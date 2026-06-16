use crate::deploy;
use crate::models::{
    LogEntry, Project, Release, AUTH_METHOD_KEY, AUTH_METHOD_PASSWORD, DEPLOY_MODE_BUILD,
    DEPLOY_MODE_UPLOAD, MIN_RELEASES,
};
use crate::storage::Storage;
use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, ModelRc, SharedString, Timer, VecModel};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

slint::include_modules!();

const CONFIG_EXPORT_FORMAT: &str = "deploydesk-projects";
const CONFIG_EXPORT_VERSION: u32 = 2;
const EXPORT_SECRET_PREFIX: &str = "deploydesk-secret-v1";
const EXPORT_SECRET_MARKER: &[u8] = b"DeployDeskSecretV1\0";
const EXPORT_SECRET_STREAM_LABEL: &[u8] = b"DeployDesk export secret stream v1";

#[derive(Clone, Debug)]
enum ConnectionStatus {
    Unknown,
    Checking,
    Online,
    Offline(String),
}

#[derive(Clone, Debug)]
enum DeployState {
    Idle,
    Running,
    Success(String),
    Error(String),
}

#[derive(Clone, Debug)]
struct ProjectRuntimeState {
    connection: ConnectionStatus,
    deploy: DeployState,
}

impl Default for ProjectRuntimeState {
    fn default() -> Self {
        Self {
            connection: ConnectionStatus::Unknown,
            deploy: DeployState::Idle,
        }
    }
}

#[derive(Clone, Debug)]
struct HistoryState {
    project: Project,
    releases: Vec<Release>,
    loading: bool,
    error: Option<String>,
    success: Option<String>,
}

struct AppState {
    storage: Option<Storage>,
    projects: Vec<Project>,
    logs: Vec<LogEntry>,
    runtime: HashMap<String, ProjectRuntimeState>,
    form_is_new: bool,
    dialog_kind: String,
    deploy_project: Option<Project>,
    history: Option<HistoryState>,
    delete_confirm: Option<Project>,
    rollback_confirm: Option<(Project, String)>,
    pending_import_config_path: Option<PathBuf>,
    notice: Option<String>,
    error: Option<String>,
    form_error: Option<String>,
    config_password_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectConfigExport {
    format: String,
    version: u32,
    projects: Vec<ProjectExportEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectExportEntry {
    project: Project,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    key: Option<ProjectExportKey>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectExportKey {
    file_name: String,
    encrypted_content: String,
}

struct ImportedProjectConfig {
    projects: Vec<Project>,
    key_count: usize,
    password_count: usize,
}

impl AppState {
    fn new() -> Self {
        let mut state = match Storage::new() {
            Ok(storage) => Self {
                storage: Some(storage),
                projects: Vec::new(),
                logs: Vec::new(),
                runtime: HashMap::new(),
                form_is_new: true,
                dialog_kind: "none".to_string(),
                deploy_project: None,
                history: None,
                delete_confirm: None,
                rollback_confirm: None,
                pending_import_config_path: None,
                notice: None,
                error: None,
                form_error: None,
                config_password_error: None,
            },
            Err(error) => Self {
                storage: None,
                projects: Vec::new(),
                logs: Vec::new(),
                runtime: HashMap::new(),
                form_is_new: true,
                dialog_kind: "none".to_string(),
                deploy_project: None,
                history: None,
                delete_confirm: None,
                rollback_confirm: None,
                pending_import_config_path: None,
                notice: None,
                error: Some(error),
                form_error: None,
                config_password_error: None,
            },
        };

        state.reload_projects();
        state.reload_logs();
        state
    }

    fn storage(&self) -> Option<&Storage> {
        self.storage.as_ref()
    }

    fn reload_projects(&mut self) {
        let Some(storage) = self.storage() else {
            return;
        };

        match storage.get_projects() {
            Ok(projects) => {
                self.projects = projects;
                self.runtime.retain(|project_id, _| {
                    self.projects
                        .iter()
                        .any(|project| &project.id == project_id)
                });
                for project in &self.projects {
                    self.runtime.entry(project.id.clone()).or_default();
                }
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn reload_logs(&mut self) {
        let Some(storage) = self.storage() else {
            return;
        };

        match storage.get_logs() {
            Ok(logs) => {
                self.logs = logs;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }
}

pub fn run() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    ui.set_form_data(project_to_form(&Project::blank()));
    ui.set_app_version(format!("v{}", env!("CARGO_PKG_VERSION")).into());

    let state = Arc::new(Mutex::new(AppState::new()));
    install_callbacks(&ui, state.clone());
    render(&ui, &state);
    let startup_state = state.clone();
    let startup_weak = ui.as_weak();
    Timer::single_shot(Duration::from_millis(0), move || {
        spawn_startup_connection_checks(startup_state.clone(), startup_weak.clone());
    });
    ui.run()
}

fn install_callbacks(ui: &MainWindow, state: Arc<Mutex<AppState>>) {
    let weak = ui.as_weak();

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_select_projects(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_active_page("projects".into());
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_select_logs(move || {
            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    state.reload_logs();
                }
                ui.set_active_page("logs".into());
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_select_settings(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_active_page("settings".into());
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_refresh_logs(move || {
            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    state.reload_logs();
                }
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_export_projects(move || {
            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    state.dialog_kind = "export-password".to_string();
                    state.pending_import_config_path = None;
                    state.config_password_error = None;
                    state.error = None;
                }
                ui.set_config_password(SharedString::new());
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_import_projects(move || {
            if let Some(ui) = weak.upgrade() {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                else {
                    return;
                };

                {
                    let mut state = state.lock().expect("state lock poisoned");
                    state.dialog_kind = "import-password".to_string();
                    state.pending_import_config_path = Some(path);
                    state.config_password_error = None;
                    state.error = None;
                }
                ui.set_config_password(SharedString::new());
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_confirm_export_projects(move |password| {
            if let Some(ui) = weak.upgrade() {
                let password = password.to_string();
                if !accept_config_password(&ui, &state, &password, "请输入导出密码", true) {
                    return;
                }
                export_project_config(&ui, &state, &password);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_confirm_import_projects(move |password| {
            if let Some(ui) = weak.upgrade() {
                let password = password.to_string();
                if !accept_config_password(&ui, &state, &password, "请输入导入密码", false) {
                    return;
                }
                import_project_config(&ui, &state, &password);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_open_new_project(move || {
            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    state.form_is_new = true;
                    state.dialog_kind = "project".to_string();
                    state.form_error = None;
                    state.error = None;
                }
                ui.set_form_data(project_to_form(&Project::blank()));
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_open_edit_project(move |id| {
            let project = {
                let state = state.lock().expect("state lock poisoned");
                state
                    .projects
                    .iter()
                    .find(|project| project.id == id.as_str())
                    .cloned()
            };

            let Some(project) = project else {
                return;
            };

            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    state.form_is_new = false;
                    state.dialog_kind = "project".to_string();
                    state.form_error = None;
                    state.error = None;
                }
                ui.set_form_data(project_to_form(&project));
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_save_project(move |form| {
            if let Some(ui) = weak.upgrade() {
                save_project_from_form(&ui, &state, weak.clone(), form);
            }
        });
    }

    {
        let weak = weak.clone();
        ui.on_choose_local_path(move || {
            let Some(path) = rfd::FileDialog::new().pick_folder() else {
                return;
            };
            if let Some(ui) = weak.upgrade() {
                let mut form = ui.get_form_data();
                form.local_path = path.display().to_string().into();
                ui.set_form_data(form);
            }
        });
    }

    {
        let weak = weak.clone();
        ui.on_choose_key_file(move || {
            let Some(path) = rfd::FileDialog::new().pick_file() else {
                return;
            };
            if let Some(ui) = weak.upgrade() {
                let mut form = ui.get_form_data();
                form.password_or_key = path.display().to_string().into();
                ui.set_form_data(form);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_open_delete_project(move |id| {
            let project = {
                let state = state.lock().expect("state lock poisoned");
                state
                    .projects
                    .iter()
                    .find(|project| project.id == id.as_str())
                    .cloned()
            };

            let Some(project) = project else {
                return;
            };

            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    state.delete_confirm = Some(project);
                    state.dialog_kind = "delete".to_string();
                }
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_confirm_delete(move || {
            if let Some(ui) = weak.upgrade() {
                confirm_delete_project(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_check_connection(move |id| {
            let project = find_project(&state, &id);
            if let Some(project) = project {
                spawn_check_connection(state.clone(), weak.clone(), project);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_open_deploy(move |id| {
            let project = find_project(&state, &id);
            let Some(project) = project else {
                return;
            };

            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    state.deploy_project = Some(project);
                    state.dialog_kind = "deploy".to_string();
                    state.error = None;
                }
                ui.set_deploy_remark(SharedString::new());
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_confirm_deploy(move |remark| {
            let project = {
                let mut state = state.lock().expect("state lock poisoned");
                let project = state.deploy_project.take();
                state.dialog_kind = "none".to_string();
                project
            };

            if let Some(project) = project {
                if let Some(ui) = weak.upgrade() {
                    render(&ui, &state);
                }
                spawn_deployment(state.clone(), weak.clone(), project, remark.to_string());
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_open_history(move |id| {
            let project = find_project(&state, &id);
            let Some(project) = project else {
                return;
            };

            {
                let mut state = state.lock().expect("state lock poisoned");
                state.history = Some(HistoryState {
                    project: project.clone(),
                    releases: Vec::new(),
                    loading: true,
                    error: None,
                    success: None,
                });
                state.dialog_kind = "history".to_string();
            }

            if let Some(ui) = weak.upgrade() {
                render(&ui, &state);
            }
            spawn_load_releases(state.clone(), weak.clone(), project);
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_refresh_history(move || {
            let project = {
                let mut state = state.lock().expect("state lock poisoned");
                let project = state
                    .history
                    .as_ref()
                    .map(|history| history.project.clone());
                if let Some(history) = state.history.as_mut() {
                    history.loading = true;
                    history.error = None;
                    history.success = None;
                }
                project
            };

            if let Some(project) = project {
                if let Some(ui) = weak.upgrade() {
                    render(&ui, &state);
                }
                spawn_load_releases(state.clone(), weak.clone(), project);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_request_rollback(move |release| {
            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    if let Some(history) = state.history.as_ref() {
                        state.rollback_confirm =
                            Some((history.project.clone(), release.to_string()));
                        state.dialog_kind = "rollback".to_string();
                    }
                }
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_confirm_rollback(move || {
            let rollback = {
                let mut state = state.lock().expect("state lock poisoned");
                let rollback = state.rollback_confirm.take();
                if let Some(history) = state.history.as_mut() {
                    history.loading = true;
                    history.error = None;
                    history.success = None;
                    state.dialog_kind = "history".to_string();
                } else {
                    state.dialog_kind = "none".to_string();
                }
                rollback
            };

            if let Some((project, release)) = rollback {
                if let Some(ui) = weak.upgrade() {
                    render(&ui, &state);
                }
                spawn_rollback(state.clone(), weak.clone(), project, release);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_close_dialog(move || {
            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    match state.dialog_kind.as_str() {
                        "rollback" => {
                            state.rollback_confirm = None;
                            state.dialog_kind = if state.history.is_some() {
                                "history".to_string()
                            } else {
                                "none".to_string()
                            };
                        }
                        "history" => {
                            state.history = None;
                            state.dialog_kind = "none".to_string();
                        }
                        "delete" => {
                            state.delete_confirm = None;
                            state.dialog_kind = "none".to_string();
                        }
                        _ => {
                            state.deploy_project = None;
                            state.dialog_kind = "none".to_string();
                        }
                    }
                    state.pending_import_config_path = None;
                    state.form_error = None;
                    state.config_password_error = None;
                }
                ui.set_config_password(SharedString::new());
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_dismiss_notice(move || {
            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    let notice = state.notice.take();
                    if let Some(notice) = notice.as_deref() {
                        clear_deploy_result(&mut state, notice);
                    }
                }
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_dismiss_error(move || {
            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    let error = state.error.take();
                    if let Some(error) = error.as_deref() {
                        clear_deploy_result(&mut state, error);
                    }
                }
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_dismiss_history_success(move || {
            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    if let Some(history) = state.history.as_mut() {
                        history.success = None;
                    }
                }
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_dismiss_form_error(move || {
            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    state.form_error = None;
                }
                render(&ui, &state);
            }
        });
    }

    {
        let weak = weak.clone();
        let state = state.clone();
        ui.on_dismiss_history_error(move || {
            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    if let Some(history) = state.history.as_mut() {
                        history.error = None;
                    }
                }
                render(&ui, &state);
            }
        });
    }
}

fn clear_deploy_result(state: &mut AppState, message: &str) {
    for runtime in state.runtime.values_mut() {
        if matches!(&runtime.deploy, DeployState::Success(success) if success == message)
            || matches!(&runtime.deploy, DeployState::Error(error) if error == message)
        {
            runtime.deploy = DeployState::Idle;
        }
    }
}

fn accept_config_password(
    ui: &MainWindow,
    state: &Arc<Mutex<AppState>>,
    password: &str,
    empty_message: &str,
    close_dialog: bool,
) -> bool {
    let password = password.trim();
    if password.is_empty() {
        {
            let mut state = state.lock().expect("state lock poisoned");
            state.config_password_error = Some(empty_message.to_string());
        }
        render(ui, state);
        return false;
    }

    {
        let mut state = state.lock().expect("state lock poisoned");
        if close_dialog {
            state.dialog_kind = "none".to_string();
        }
        state.config_password_error = None;
    }
    if close_dialog {
        ui.set_config_password(SharedString::new());
    }
    render(ui, state);
    true
}

fn export_project_config(ui: &MainWindow, state: &Arc<Mutex<AppState>>, export_password: &str) {
    let projects = {
        let state = state.lock().expect("state lock poisoned");
        state.projects.clone()
    };

    let Some(mut path) = rfd::FileDialog::new()
        .add_filter("JSON", &["json"])
        .set_file_name("deploydesk-projects.json")
        .save_file()
    else {
        return;
    };

    if path.extension().is_none() {
        path.set_extension("json");
    }

    let result = build_project_export(&projects, export_password)
        .and_then(|export| {
            serde_json::to_string_pretty(&export).map_err(|e| format!("序列化项目配置失败: {}", e))
        })
        .and_then(|content| {
            fs::write(&path, content)
                .map_err(|e| format!("导出项目配置失败 {}: {}", path.display(), e))
        });

    {
        let mut state = state.lock().expect("state lock poisoned");
        match result {
            Ok(()) => {
                state.notice = Some(format!("已导出 {} 个项目配置", projects.len()));
                state.error = None;
            }
            Err(error) => {
                state.error = Some(error);
                state.notice = None;
            }
        }
    }

    render(ui, state);
}

fn import_project_config(ui: &MainWindow, state: &Arc<Mutex<AppState>>, export_password: &str) {
    let path = {
        let state = state.lock().expect("state lock poisoned");
        state.pending_import_config_path.clone()
    };

    let Some(path) = path else {
        {
            let mut state = state.lock().expect("state lock poisoned");
            state.config_password_error = Some("请先选择要导入的项目配置文件".to_string());
        }
        render(ui, state);
        return;
    };

    let storage = {
        let state = state.lock().expect("state lock poisoned");
        state.storage.clone()
    };

    let Some(storage) = storage else {
        {
            let mut state = state.lock().expect("state lock poisoned");
            state.config_password_error = Some("配置目录不可用，无法导入项目配置".to_string());
        }
        render(ui, state);
        return;
    };

    let import_result = fs::read_to_string(&path)
        .map_err(|e| format!("读取项目配置文件失败 {}: {}", path.display(), e))
        .and_then(|content| parse_project_import(&content, &storage, export_password));

    let imported = match import_result {
        Ok(imported) => imported,
        Err(error) => {
            {
                let mut state = state.lock().expect("state lock poisoned");
                state.config_password_error = Some(error);
            }
            render(ui, state);
            return;
        }
    };

    match storage.save_projects(&imported.projects) {
        Ok(()) => {
            let imported_count = imported.projects.len();
            let missing_count = count_local_path_errors(&imported.projects);
            {
                let mut state = state.lock().expect("state lock poisoned");
                state.reload_projects();
                state.dialog_kind = "none".to_string();
                state.pending_import_config_path = None;
                state.config_password_error = None;
                if missing_count > 0 {
                    state.error = Some(format!(
                        "已导入 {} 个项目配置，其中 {} 个本地目录不可用",
                        imported_count, missing_count
                    ));
                    state.notice = None;
                } else {
                    let mut notice = format!("已导入 {} 个项目配置", imported_count);
                    if imported.key_count > 0 {
                        notice.push_str(&format!("，已恢复 {} 个密钥文件", imported.key_count));
                    }
                    if imported.password_count > 0 {
                        notice.push_str(&format!("，已解密 {} 个密码", imported.password_count));
                    }
                    state.notice = Some(notice);
                    state.error = None;
                }
            }
            ui.set_active_page("projects".into());
            ui.set_config_password(SharedString::new());
            render(ui, state);
        }
        Err(error) => {
            {
                let mut state = state.lock().expect("state lock poisoned");
                state.config_password_error = Some(error);
            }
            render(ui, state);
        }
    }
}

fn build_project_export(
    projects: &[Project],
    export_password: &str,
) -> Result<ProjectConfigExport, String> {
    let mut export_projects = Vec::with_capacity(projects.len());

    for project in projects {
        let mut export_project = project.clone();
        let mut export_key = None;

        if export_project.auth_method == AUTH_METHOD_PASSWORD {
            if !export_project.password_or_key.trim().is_empty() {
                export_project.password_or_key =
                    encrypt_export_secret(&export_project.password_or_key, export_password);
            }
        } else if export_project.auth_method == AUTH_METHOD_KEY {
            let key_path_text = export_project.password_or_key.trim();
            if key_path_text.is_empty() {
                return Err(format!(
                    "项目「{}」未配置私钥路径，无法导出密钥",
                    project.name
                ));
            }

            let key_path = Path::new(key_path_text);
            let content = fs::read_to_string(key_path).map_err(|e| {
                format!(
                    "读取项目「{}」私钥失败 {}: {}",
                    project.name,
                    key_path.display(),
                    e
                )
            })?;
            let file_name = key_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("identity.key")
                .to_string();
            export_key = Some(ProjectExportKey {
                file_name,
                encrypted_content: encrypt_export_secret(&content, export_password),
            });
        }

        export_projects.push(ProjectExportEntry {
            project: export_project,
            key: export_key,
        });
    }

    Ok(ProjectConfigExport {
        format: CONFIG_EXPORT_FORMAT.to_string(),
        version: CONFIG_EXPORT_VERSION,
        projects: export_projects,
    })
}

fn parse_project_import(
    content: &str,
    storage: &Storage,
    export_password: &str,
) -> Result<ImportedProjectConfig, String> {
    let export: ProjectConfigExport =
        serde_json::from_str(content).map_err(|e| format!("解析项目配置文件失败: {}", e))?;

    if export.format != CONFIG_EXPORT_FORMAT {
        return Err("项目配置文件格式不正确".to_string());
    }

    if export.version != CONFIG_EXPORT_VERSION {
        return Err(format!("不支持的项目配置版本: {}", export.version));
    }

    let mut projects = Vec::with_capacity(export.projects.len());
    let mut key_count = 0;
    let mut password_count = 0;

    for entry in export.projects {
        let mut project = entry.project;

        if project.auth_method == AUTH_METHOD_KEY {
            if let Some(key) = entry.key {
                let key_content = decrypt_export_secret(&key.encrypted_content, export_password)?;
                let key_path =
                    storage.save_imported_key(&project.id, &key.file_name, &key_content)?;
                project.password_or_key = key_path.display().to_string();
                key_count += 1;
            } else {
                return Err(format!("项目「{}」缺少导出的私钥内容", project.name));
            }
        } else if project.auth_method == AUTH_METHOD_PASSWORD
            && !project.password_or_key.trim().is_empty()
        {
            project.password_or_key =
                decrypt_export_secret(&project.password_or_key, export_password)?;
            password_count += 1;
        }

        project.normalize();
        projects.push(project);
    }

    Ok(ImportedProjectConfig {
        projects,
        key_count,
        password_count,
    })
}

fn encrypt_export_secret(secret: &str, export_password: &str) -> String {
    let nonce = *uuid::Uuid::new_v4().as_bytes();
    let mut plain = Vec::with_capacity(EXPORT_SECRET_MARKER.len() + secret.len());
    plain.extend_from_slice(EXPORT_SECRET_MARKER);
    plain.extend_from_slice(secret.as_bytes());
    let encrypted = crypt_secret_bytes(&plain, export_password, &nonce);
    format!(
        "{}:{}:{}",
        EXPORT_SECRET_PREFIX,
        hex_encode(&nonce),
        hex_encode(&encrypted)
    )
}

fn decrypt_export_secret(value: &str, export_password: &str) -> Result<String, String> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0] != EXPORT_SECRET_PREFIX {
        return Err("导入配置中的凭据不是受支持的加密格式".to_string());
    }

    let nonce = hex_decode(parts[1]).map_err(|e| format!("解析加密凭据 nonce 失败: {}", e))?;
    let encrypted = hex_decode(parts[2]).map_err(|e| format!("解析加密凭据失败: {}", e))?;
    let decrypted = crypt_secret_bytes(&encrypted, export_password, &nonce);
    if !decrypted.starts_with(EXPORT_SECRET_MARKER) {
        return Err("导入密码不正确或配置文件已损坏".to_string());
    }

    String::from_utf8(decrypted[EXPORT_SECRET_MARKER.len()..].to_vec())
        .map_err(|_| "导入密码不正确或配置文件已损坏".to_string())
}

fn crypt_secret_bytes(input: &[u8], export_password: &str, nonce: &[u8]) -> Vec<u8> {
    let mut state = password_stream_seed(export_password, nonce);
    input
        .iter()
        .map(|byte| byte ^ next_secret_stream_byte(&mut state))
        .collect()
}

fn password_stream_seed(export_password: &str, nonce: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in EXPORT_SECRET_STREAM_LABEL
        .iter()
        .chain(export_password.as_bytes().iter())
        .chain(nonce.iter())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    if hash == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        hash
    }
}

fn next_secret_stream_byte(state: &mut u64) -> u8 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    (value >> 32) as u8
}

fn hex_encode(bytes: &[u8]) -> String {
    const CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(CHARS[(byte >> 4) as usize] as char);
        encoded.push(CHARS[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn hex_decode(value: &str) -> Result<Vec<u8>, String> {
    if value.len() % 2 != 0 {
        return Err("长度不是偶数".to_string());
    }

    let mut decoded = Vec::with_capacity(value.len() / 2);
    let bytes = value.as_bytes();
    for chunk in bytes.chunks_exact(2) {
        let high = hex_value(chunk[0])?;
        let low = hex_value(chunk[1])?;
        decoded.push((high << 4) | low);
    }
    Ok(decoded)
}

fn hex_value(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("非法十六进制字符: {}", byte as char)),
    }
}

fn count_local_path_errors(projects: &[Project]) -> usize {
    projects
        .iter()
        .filter(|project| local_project_path_error(project).is_some())
        .count()
}

fn find_project(state: &Arc<Mutex<AppState>>, id: &SharedString) -> Option<Project> {
    let state = state.lock().expect("state lock poisoned");
    state
        .projects
        .iter()
        .find(|project| project.id == id.as_str())
        .cloned()
}

fn spawn_startup_connection_checks(state: Arc<Mutex<AppState>>, weak: slint::Weak<MainWindow>) {
    let projects = {
        let state = state.lock().expect("state lock poisoned");
        state.projects.clone()
    };

    for project in projects {
        spawn_check_connection(state.clone(), weak.clone(), project);
    }
}

fn save_project_from_form(
    ui: &MainWindow,
    state: &Arc<Mutex<AppState>>,
    weak: slint::Weak<MainWindow>,
    form: ProjectFormData,
) {
    let project = form_to_project(form);
    let missing = project_form_missing_fields(&project);
    if !missing.is_empty() {
        {
            let mut state = state.lock().expect("state lock poisoned");
            state.form_error = Some(format!("请补全必填项：{}", missing.join("、")));
        }
        render(ui, state);
        return;
    }

    let storage = {
        let state = state.lock().expect("state lock poisoned");
        state.storage.clone()
    };

    let Some(storage) = storage else {
        {
            let mut state = state.lock().expect("state lock poisoned");
            state.form_error = Some("配置目录不可用，无法保存项目".to_string());
        }
        render(ui, state);
        return;
    };

    match storage.save_project(project) {
        Ok(saved_project) => {
            {
                let mut state = state.lock().expect("state lock poisoned");
                state.reload_projects();
                state.dialog_kind = "none".to_string();
                state.notice = Some(format!("已保存项目：{}", saved_project.name));
                state.form_error = None;
                state.error = None;
            }
            render(ui, state);
            spawn_check_connection(state.clone(), weak, saved_project);
        }
        Err(error) => {
            {
                let mut state = state.lock().expect("state lock poisoned");
                state.form_error = Some(error);
            }
            render(ui, state);
        }
    }
}

fn confirm_delete_project(ui: &MainWindow, state: &Arc<Mutex<AppState>>) {
    let (storage, project) = {
        let mut state = state.lock().expect("state lock poisoned");
        (state.storage.clone(), state.delete_confirm.take())
    };

    let Some(project) = project else {
        return;
    };

    let Some(storage) = storage else {
        {
            let mut state = state.lock().expect("state lock poisoned");
            state.error = Some("配置目录不可用，无法删除项目".to_string());
            state.dialog_kind = "none".to_string();
        }
        render(ui, state);
        return;
    };

    match storage.delete_project(&project.id) {
        Ok(()) => {
            let mut state = state.lock().expect("state lock poisoned");
            state.reload_projects();
            state.notice = Some(format!("已删除项目：{}", project.name));
            state.error = None;
            state.dialog_kind = "none".to_string();
        }
        Err(error) => {
            let mut state = state.lock().expect("state lock poisoned");
            state.error = Some(error);
            state.dialog_kind = "none".to_string();
        }
    }

    render(ui, state);
}

fn spawn_check_connection(
    state: Arc<Mutex<AppState>>,
    weak: slint::Weak<MainWindow>,
    project: Project,
) {
    {
        let mut state = state.lock().expect("state lock poisoned");
        state
            .runtime
            .entry(project.id.clone())
            .or_default()
            .connection = ConnectionStatus::Checking;
    }
    render_now(&weak, &state);

    std::thread::spawn(move || {
        let project_id = project.id.clone();
        let result = deploy::check_connection(&project);
        {
            let mut state = state.lock().expect("state lock poisoned");
            let runtime = state.runtime.entry(project_id).or_default();
            runtime.connection = match result {
                Ok(_) => ConnectionStatus::Online,
                Err(error) => ConnectionStatus::Offline(error),
            };
        }
        invoke_render(weak, state);
    });
}

fn spawn_deployment(
    state: Arc<Mutex<AppState>>,
    weak: slint::Weak<MainWindow>,
    project: Project,
    remark: String,
) {
    let storage = {
        let mut state = state.lock().expect("state lock poisoned");
        state.runtime.entry(project.id.clone()).or_default().deploy = DeployState::Running;
        state.notice = None;
        state.error = None;
        state.storage.clone()
    };
    render_now(&weak, &state);

    let Some(storage) = storage else {
        {
            let mut state = state.lock().expect("state lock poisoned");
            state.error = Some("配置目录不可用，无法发布".to_string());
            state.runtime.entry(project.id).or_default().deploy = DeployState::Idle;
        }
        render_now(&weak, &state);
        return;
    };

    std::thread::spawn(move || {
        let project_id = project.id.clone();
        let result = deploy::run_deployment_with_log(&storage, project, remark);
        let logs_result = storage.get_logs();

        {
            let mut state = state.lock().expect("state lock poisoned");
            match result {
                Ok(message) => {
                    state.runtime.entry(project_id).or_default().deploy =
                        DeployState::Success(message.clone());
                    state.notice = Some(message);
                    state.error = None;
                }
                Err(error) => {
                    state.runtime.entry(project_id).or_default().deploy =
                        DeployState::Error(error.clone());
                    state.error = Some(error);
                }
            }

            match logs_result {
                Ok(logs) => state.logs = logs,
                Err(error) => state.error = Some(error),
            }
        }
        invoke_render(weak, state);
    });
}

fn spawn_load_releases(
    state: Arc<Mutex<AppState>>,
    weak: slint::Weak<MainWindow>,
    project: Project,
) {
    std::thread::spawn(move || {
        let project_id = project.id.clone();
        let result = deploy::get_releases(&project);

        {
            let mut state = state.lock().expect("state lock poisoned");
            if let Some(history) = state.history.as_mut() {
                if history.project.id == project_id {
                    history.loading = false;
                    match result {
                        Ok(releases) => {
                            history.releases = releases;
                            history.error = None;
                        }
                        Err(error) => history.error = Some(error),
                    }
                }
            }
        }
        invoke_render(weak, state);
    });
}

fn spawn_rollback(
    state: Arc<Mutex<AppState>>,
    weak: slint::Weak<MainWindow>,
    project: Project,
    release: String,
) {
    let storage = {
        let state = state.lock().expect("state lock poisoned");
        state.storage.clone()
    };

    let Some(storage) = storage else {
        {
            let mut state = state.lock().expect("state lock poisoned");
            state.error = Some("配置目录不可用，无法回滚".to_string());
            if let Some(history) = state.history.as_mut() {
                history.loading = false;
            }
        }
        render_now(&weak, &state);
        return;
    };

    std::thread::spawn(move || {
        let project_id = project.id.clone();
        let result = deploy::rollback_with_log(&storage, project.clone(), release);
        let logs_result = storage.get_logs();
        let releases_result = if result.is_ok() {
            Some(deploy::get_releases(&project))
        } else {
            None
        };

        {
            let mut state = state.lock().expect("state lock poisoned");
            match result {
                Ok(message) => {
                    state.notice = Some(message.clone());
                    state.error = None;
                    if let Some(history) = state.history.as_mut() {
                        if history.project.id == project_id {
                            history.success = Some(message);
                        }
                    }
                }
                Err(error) => {
                    state.error = Some(error.clone());
                    if let Some(history) = state.history.as_mut() {
                        if history.project.id == project_id {
                            history.error = Some(error);
                        }
                    }
                }
            }

            if let Some(history) = state.history.as_mut() {
                if history.project.id == project_id {
                    history.loading = false;
                    if let Some(releases_result) = releases_result {
                        match releases_result {
                            Ok(releases) => history.releases = releases,
                            Err(error) => history.error = Some(error),
                        }
                    }
                }
            }

            match logs_result {
                Ok(logs) => state.logs = logs,
                Err(error) => state.error = Some(error),
            }
        }
        invoke_render(weak, state);
    });
}

fn render_now(weak: &slint::Weak<MainWindow>, state: &Arc<Mutex<AppState>>) {
    if let Some(ui) = weak.upgrade() {
        render(&ui, state);
    }
}

fn invoke_render(weak: slint::Weak<MainWindow>, state: Arc<Mutex<AppState>>) {
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = weak.upgrade() {
            render(&ui, &state);
        }
    });
}

fn render(ui: &MainWindow, state: &Arc<Mutex<AppState>>) {
    let state = state.lock().expect("state lock poisoned");
    let project_rows = state
        .projects
        .iter()
        .map(|project| project_to_row(project, &state.runtime))
        .collect::<Vec<_>>();
    let project_grid_rows = group_project_grid_rows(&project_rows);
    let log_rows = state.logs.iter().map(log_to_row).collect::<Vec<_>>();
    let release_rows = state
        .history
        .as_ref()
        .map(|history| {
            history
                .releases
                .iter()
                .map(release_to_row)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let online_count = state
        .runtime
        .values()
        .filter(|runtime| matches!(runtime.connection, ConnectionStatus::Online))
        .count();
    let active_count = state
        .runtime
        .values()
        .filter(|runtime| {
            matches!(runtime.connection, ConnectionStatus::Checking)
                || matches!(runtime.deploy, DeployState::Running)
        })
        .count()
        + usize::from(
            state
                .history
                .as_ref()
                .is_some_and(|history| history.loading),
        );
    let success_log_count = state.logs.iter().filter(|log| log.status == "成功").count();
    let failed_log_count = state.logs.len().saturating_sub(success_log_count);
    let local_path_error_count = count_local_path_errors(&state.projects);

    ui.set_project_grid_rows(model_from_vec(project_grid_rows));
    ui.set_log_rows(model_from_vec(log_rows));
    ui.set_release_rows(model_from_vec(release_rows.clone()));
    ui.set_release_count(release_rows.len() as i32);
    ui.set_project_count(state.projects.len() as i32);
    ui.set_local_path_error_count(local_path_error_count as i32);
    ui.set_online_count(online_count as i32);
    ui.set_active_task_count(active_count as i32);
    ui.set_log_count(state.logs.len() as i32);
    ui.set_success_log_count(success_log_count as i32);
    ui.set_failed_log_count(failed_log_count as i32);
    ui.set_dialog_kind(state.dialog_kind.clone().into());
    ui.set_notice_message(option_string(&state.notice));
    ui.set_error_message(option_string(&state.error));
    ui.set_form_error(option_string(&state.form_error));
    ui.set_config_password_error(option_string(&state.config_password_error));

    if state.form_is_new {
        ui.set_form_title("新建项目".into());
        ui.set_form_subtitle("创建一个新的前端发布目标，保存后会立即检测服务器连接。".into());
    } else {
        ui.set_form_title("编辑项目".into());
        ui.set_form_subtitle("调整项目发布配置、服务器路径和认证方式，保存后会重新检测。".into());
    }

    if let Some(project) = state.deploy_project.as_ref() {
        ui.set_deploy_project_name(project.name.clone().into());
        ui.set_deploy_project_path(project.remote_deploy_path.clone().into());
    } else {
        ui.set_deploy_project_name(SharedString::new());
        ui.set_deploy_project_path(SharedString::new());
    }

    if let Some(history) = state.history.as_ref() {
        ui.set_history_project_name(history.project.name.clone().into());
        ui.set_history_project_path(history.project.remote_deploy_path.clone().into());
        ui.set_history_loading(history.loading);
        ui.set_history_error(option_string(&history.error));
        ui.set_history_success(option_string(&history.success));
    } else {
        ui.set_history_project_name(SharedString::new());
        ui.set_history_project_path(SharedString::new());
        ui.set_history_loading(false);
        ui.set_history_error(SharedString::new());
        ui.set_history_success(SharedString::new());
    }

    match state.dialog_kind.as_str() {
        "delete" => {
            if let Some(project) = state.delete_confirm.as_ref() {
                ui.set_confirm_title("删除项目".into());
                ui.set_confirm_detail(format!("确定要删除项目「{}」吗？", project.name).into());
                ui.set_confirm_hint("此操作只删除本地配置，不会删除远程服务器文件。".into());
            }
        }
        "rollback" => {
            if let Some((project, release)) = state.rollback_confirm.as_ref() {
                ui.set_confirm_title("确认回滚".into());
                ui.set_confirm_detail(format!("回滚项目「{}」", project.name).into());
                ui.set_confirm_hint(format!("远程软链接将指向版本 {}。", release).into());
            }
        }
        _ => {
            ui.set_confirm_title(SharedString::new());
            ui.set_confirm_detail(SharedString::new());
            ui.set_confirm_hint(SharedString::new());
        }
    }
}

fn model_from_vec<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    Rc::new(VecModel::from(items)).into()
}

fn group_project_grid_rows(rows: &[ProjectRow]) -> Vec<ProjectGridRow> {
    let mut grid_rows = Vec::with_capacity((rows.len() + 1) / 2);

    for chunk in rows.chunks(2) {
        let left = chunk[0].clone();
        let has_right = chunk.len() > 1;
        let right = if has_right {
            chunk[1].clone()
        } else {
            left.clone()
        };
        grid_rows.push(ProjectGridRow {
            left,
            right,
            has_right,
        });
    }

    grid_rows
}

fn option_string(value: &Option<String>) -> SharedString {
    value.clone().unwrap_or_default().into()
}

fn project_to_row(project: &Project, runtime: &HashMap<String, ProjectRuntimeState>) -> ProjectRow {
    let runtime = runtime.get(&project.id).cloned().unwrap_or_default();
    let local_path_error = local_project_path_error(project);
    let (connection_label, connection_tone, connection_detail) =
        if let Some(error) = local_path_error {
            ("本地目录缺失", "danger", error)
        } else {
            match runtime.connection {
                ConnectionStatus::Unknown => ("未检测", "muted", String::new()),
                ConnectionStatus::Checking => ("检测连接", "warning", String::new()),
                ConnectionStatus::Online => ("连通正常", "success", String::new()),
                ConnectionStatus::Offline(error) => ("连接失败", "danger", error),
            }
        };

    let (deploy_label, deploy_tone, deploy_detail, deploy_running) = match runtime.deploy {
        DeployState::Idle => ("", "muted", connection_detail, false),
        DeployState::Running => ("正在部署", "accent", "后台发布任务运行中".to_string(), true),
        DeployState::Success(message) => ("部署成功", "success", message, false),
        DeployState::Error(error) => ("部署失败", "danger", error, false),
    };

    ProjectRow {
        id: project.id.clone().into(),
        name: project.name.clone().into(),
        local_path: project.local_path.clone().into(),
        server: project.server_label().into(),
        remote_path: project.remote_deploy_path.clone().into(),
        deploy_mode_label: if project.uses_build_step() {
            "构建并发布"
        } else {
            "直接上传"
        }
        .into(),
        output_dir: project.build_output_dir.clone().into(),
        symlink_name: project.symlink_name.clone().into(),
        max_releases: project.max_releases as i32,
        connection_label: connection_label.into(),
        connection_tone: connection_tone.into(),
        deploy_label: deploy_label.into(),
        deploy_tone: deploy_tone.into(),
        deploy_detail: deploy_detail.into(),
        deploy_running,
    }
}

fn local_project_path_error(project: &Project) -> Option<String> {
    let local_path = project.local_path.trim();
    if local_path.is_empty() {
        return Some("本地项目根目录未配置".to_string());
    }

    let path = Path::new(local_path);
    if !path.exists() {
        return Some(format!("本地项目根目录不存在: {}", path.display()));
    }

    if !path.is_dir() {
        return Some(format!("本地项目根路径不是目录: {}", path.display()));
    }

    None
}

fn log_to_row(log: &LogEntry) -> LogRow {
    LogRow {
        project_name: log.project_name.clone().into(),
        action: log.action.clone().into(),
        status: log.status.clone().into(),
        details: log.details.clone().into(),
        timestamp: log.timestamp.clone().into(),
        status_tone: if log.status == "成功" {
            "success"
        } else {
            "danger"
        }
        .into(),
    }
}

fn release_to_row(release: &Release) -> ReleaseRow {
    ReleaseRow {
        name: release.name.clone().into(),
        remark: release.remark.clone().into(),
        status_label: if release.is_current {
            "当前"
        } else {
            "历史"
        }
        .into(),
        status_tone: if release.is_current {
            "success"
        } else {
            "muted"
        }
        .into(),
        is_current: release.is_current,
    }
}

fn project_to_form(project: &Project) -> ProjectFormData {
    ProjectFormData {
        id: project.id.clone().into(),
        name: project.name.clone().into(),
        local_path: project.local_path.clone().into(),
        build_command: project.build_command.clone().into(),
        remote_host: project.remote_host.clone().into(),
        remote_port: i32::from(project.remote_port),
        username: project.username.clone().into(),
        auth_method: project.auth_method.clone().into(),
        password_or_key: project.password_or_key.clone().into(),
        remote_deploy_path: project.remote_deploy_path.clone().into(),
        build_output_dir: project.build_output_dir.clone().into(),
        symlink_name: project.symlink_name.clone().into(),
        deploy_mode: project.deploy_mode.clone().into(),
        max_releases: project.max_releases as i32,
    }
}

fn form_to_project(form: ProjectFormData) -> Project {
    let remote_port = u16::try_from(form.remote_port.clamp(1, 65_535)).unwrap_or(22);
    let max_releases =
        u32::try_from(form.max_releases.max(MIN_RELEASES as i32)).unwrap_or(MIN_RELEASES);
    let auth_method = if form.auth_method.as_str() == AUTH_METHOD_KEY {
        AUTH_METHOD_KEY
    } else {
        AUTH_METHOD_PASSWORD
    };
    let deploy_mode = if form.deploy_mode.as_str() == DEPLOY_MODE_UPLOAD {
        DEPLOY_MODE_UPLOAD
    } else {
        DEPLOY_MODE_BUILD
    };

    Project {
        id: form.id.to_string(),
        name: form.name.to_string(),
        local_path: form.local_path.to_string(),
        build_command: form.build_command.to_string(),
        remote_host: form.remote_host.to_string(),
        remote_port,
        username: form.username.to_string(),
        auth_method: auth_method.to_string(),
        password_or_key: form.password_or_key.to_string(),
        remote_deploy_path: form.remote_deploy_path.to_string(),
        build_output_dir: form.build_output_dir.to_string(),
        symlink_name: form.symlink_name.to_string(),
        deploy_mode: deploy_mode.to_string(),
        max_releases,
    }
}

fn project_form_missing_fields(project: &Project) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if project.name.trim().is_empty() {
        fields.push("项目名称");
    }
    if project.local_path.trim().is_empty() {
        fields.push("本地项目根目录");
    }
    if project.remote_host.trim().is_empty() {
        fields.push("远程主机");
    }
    if project.remote_deploy_path.trim().is_empty() {
        fields.push("远程部署路径");
    }
    if project.max_releases < MIN_RELEASES {
        fields.push("最大保留版本数");
    }
    fields
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("deploydesk-app-test-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn local_project_path_error_detects_missing_directory() {
        let mut project = Project::blank();
        project.local_path = std::env::temp_dir()
            .join(format!("deploydesk-missing-{}", Uuid::new_v4()))
            .display()
            .to_string();

        let error = local_project_path_error(&project).unwrap();
        assert!(error.contains("本地项目根目录不存在"));
    }

    #[test]
    fn local_project_path_error_detects_file_path() {
        let dir = TestDir::new();
        let file_path = dir.path.join("package.json");
        fs::write(&file_path, "{}").unwrap();

        let mut project = Project::blank();
        project.local_path = file_path.display().to_string();

        let error = local_project_path_error(&project).unwrap();
        assert!(error.contains("本地项目根路径不是目录"));
    }

    #[test]
    fn local_project_path_error_accepts_existing_directory() {
        let dir = TestDir::new();
        let mut project = Project::blank();
        project.local_path = dir.path.display().to_string();

        assert!(local_project_path_error(&project).is_none());
    }

    #[test]
    fn export_password_is_encrypted_and_round_trips() {
        let password = "s3cret-密码";
        let export_password = "bundle-password";
        let encrypted = encrypt_export_secret(password, export_password);

        assert_ne!(encrypted, password);
        assert!(!encrypted.contains(password));
        assert_eq!(
            decrypt_export_secret(&encrypted, export_password).unwrap(),
            password
        );
        assert!(decrypt_export_secret(&encrypted, "wrong-password").is_err());
    }

    #[test]
    fn project_export_encrypts_password_projects() {
        let mut project = Project::blank();
        project.name = "demo".to_string();
        project.auth_method = AUTH_METHOD_PASSWORD.to_string();
        project.password_or_key = "plain-password".to_string();

        let export = build_project_export(&[project], "bundle-password").unwrap();
        let exported_password = &export.projects[0].project.password_or_key;

        assert_ne!(exported_password, "plain-password");
        assert!(exported_password.starts_with(EXPORT_SECRET_PREFIX));
        assert_eq!(
            decrypt_export_secret(exported_password, "bundle-password").unwrap(),
            "plain-password"
        );
    }

    #[test]
    fn project_export_encrypts_key_file_content() {
        let dir = TestDir::new();
        let key_path = dir.path.join("id_rsa");
        fs::write(&key_path, "PRIVATE KEY CONTENT").unwrap();

        let mut project = Project::blank();
        project.name = "key-demo".to_string();
        project.auth_method = AUTH_METHOD_KEY.to_string();
        project.password_or_key = key_path.display().to_string();

        let export = build_project_export(&[project], "bundle-password").unwrap();
        let exported_key = export.projects[0].key.as_ref().unwrap();

        assert_eq!(exported_key.file_name, "id_rsa");
        assert_ne!(exported_key.encrypted_content, "PRIVATE KEY CONTENT");
        assert!(!exported_key
            .encrypted_content
            .contains("PRIVATE KEY CONTENT"));
        assert_eq!(
            decrypt_export_secret(&exported_key.encrypted_content, "bundle-password").unwrap(),
            "PRIVATE KEY CONTENT"
        );
    }
}
