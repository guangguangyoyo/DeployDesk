use crate::models::{LogEntry, Project};
use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Storage {
    app_dir: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self, String> {
        let app_dir = app_config_dir()?;
        fs::create_dir_all(&app_dir)
            .map_err(|e| format!("创建配置目录失败 {}: {}", app_dir.display(), e))?;
        Ok(Self { app_dir })
    }

    pub fn app_dir(&self) -> &PathBuf {
        &self.app_dir
    }

    pub fn projects_path(&self) -> PathBuf {
        self.app_dir.join("projects.json")
    }

    pub fn logs_path(&self) -> PathBuf {
        self.app_dir.join("logs.json")
    }

    pub fn get_projects(&self) -> Result<Vec<Project>, String> {
        let path = self.projects_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取项目配置失败 {}: {}", path.display(), e))?;
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut projects: Vec<Project> = serde_json::from_str(&content)
            .map_err(|e| format!("解析项目配置失败 {}: {}", path.display(), e))?;
        for project in &mut projects {
            project.normalize();
        }
        Ok(projects)
    }

    pub fn save_project(&self, mut project: Project) -> Result<Project, String> {
        project.normalize();
        let mut projects = self.get_projects()?;

        if let Some(existing) = projects.iter_mut().find(|item| item.id == project.id) {
            *existing = project.clone();
        } else {
            projects.push(project.clone());
        }

        self.save_projects(&projects)?;
        Ok(project)
    }

    pub fn delete_project(&self, id: &str) -> Result<(), String> {
        let mut projects = self.get_projects()?;
        projects.retain(|project| project.id != id);
        self.save_projects(&projects)
    }

    pub fn save_projects(&self, projects: &[Project]) -> Result<(), String> {
        let content = serde_json::to_string_pretty(projects)
            .map_err(|e| format!("序列化项目配置失败: {}", e))?;
        fs::write(self.projects_path(), content).map_err(|e| format!("保存项目配置失败: {}", e))
    }

    pub fn get_logs(&self) -> Result<Vec<LogEntry>, String> {
        let path = self.logs_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取操作日志失败 {}: {}", path.display(), e))?;
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        serde_json::from_str(&content)
            .map_err(|e| format!("解析操作日志失败 {}: {}", path.display(), e))
    }

    pub fn save_logs(&self, logs: &[LogEntry]) -> Result<(), String> {
        let content =
            serde_json::to_string_pretty(logs).map_err(|e| format!("序列化操作日志失败: {}", e))?;
        fs::write(self.logs_path(), content).map_err(|e| format!("保存操作日志失败: {}", e))
    }
}

fn app_config_dir() -> Result<PathBuf, String> {
    if cfg!(target_os = "windows") {
        if let Some(roaming) = std::env::var_os("APPDATA") {
            return Ok(PathBuf::from(roaming).join("com.deploydesk.app"));
        }
    }

    dirs::config_dir()
        .map(|path| path.join("com.deploydesk.app"))
        .ok_or_else(|| "无法定位系统配置目录".to_string())
}
