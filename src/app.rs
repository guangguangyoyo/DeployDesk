use crate::deploy;
use crate::models::{
    LogEntry, Project, Release, AUTH_METHOD_KEY, AUTH_METHOD_PASSWORD, DEPLOY_MODE_BUILD,
    DEPLOY_MODE_UPLOAD, MIN_RELEASES,
};
use crate::storage::Storage;
use slint::{ComponentHandle, ModelRc, SharedString, Timer, VecModel};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

slint::include_modules!();

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
    notice: Option<String>,
    error: Option<String>,
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
                notice: None,
                error: None,
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
                notice: None,
                error: Some(error),
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
        ui.on_refresh_projects(move || {
            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    state.reload_projects();
                }
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
        ui.on_open_new_project(move || {
            if let Some(ui) = weak.upgrade() {
                {
                    let mut state = state.lock().expect("state lock poisoned");
                    state.form_is_new = true;
                    state.dialog_kind = "project".to_string();
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
                }
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
            state.error = Some(format!("请补全必填项：{}", missing.join("、")));
        }
        render(ui, state);
        return;
    }

    let (storage, was_new) = {
        let state = state.lock().expect("state lock poisoned");
        (state.storage.clone(), state.form_is_new)
    };

    let Some(storage) = storage else {
        {
            let mut state = state.lock().expect("state lock poisoned");
            state.error = Some("配置目录不可用，无法保存项目".to_string());
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
                state.error = None;
            }
            render(ui, state);
            if was_new {
                spawn_check_connection(state.clone(), weak, saved_project);
            }
        }
        Err(error) => {
            {
                let mut state = state.lock().expect("state lock poisoned");
                state.error = Some(error);
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

    ui.set_project_rows(model_from_vec(project_rows));
    ui.set_log_rows(model_from_vec(log_rows));
    ui.set_release_rows(model_from_vec(release_rows.clone()));
    ui.set_release_count(release_rows.len() as i32);
    ui.set_project_count(state.projects.len() as i32);
    ui.set_online_count(online_count as i32);
    ui.set_active_task_count(active_count as i32);
    ui.set_log_count(state.logs.len() as i32);
    ui.set_success_log_count(success_log_count as i32);
    ui.set_failed_log_count(failed_log_count as i32);
    ui.set_dialog_kind(state.dialog_kind.clone().into());
    ui.set_notice_message(option_string(&state.notice));
    ui.set_error_message(option_string(&state.error));

    if state.form_is_new {
        ui.set_form_title("新建项目".into());
        ui.set_form_subtitle("创建一个新的前端发布目标，保存后会立即检测服务器连接。".into());
    } else {
        ui.set_form_title("编辑项目".into());
        ui.set_form_subtitle("调整项目发布配置、服务器路径和认证方式。".into());
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

fn option_string(value: &Option<String>) -> SharedString {
    value.clone().unwrap_or_default().into()
}

fn project_to_row(project: &Project, runtime: &HashMap<String, ProjectRuntimeState>) -> ProjectRow {
    let runtime = runtime.get(&project.id).cloned().unwrap_or_default();
    let (connection_label, connection_tone, connection_detail) = match runtime.connection {
        ConnectionStatus::Unknown => ("未检测", "muted", String::new()),
        ConnectionStatus::Checking => ("检测连接", "warning", String::new()),
        ConnectionStatus::Online => ("连通正常", "success", String::new()),
        ConnectionStatus::Offline(error) => ("连接失败", "danger", error),
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
