use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub local_path: String,
    pub build_command: String,
    pub remote_host: String,
    pub remote_port: u16,
    pub username: String,
    pub auth_method: String, // "password" or "key"
    pub password_or_key: String,
    pub remote_deploy_path: String,
    #[serde(default = "default_build_output_dir")]
    pub build_output_dir: String,
    #[serde(default = "default_symlink_name")]
    pub symlink_name: String,
    #[serde(default = "default_deploy_mode")]
    pub deploy_mode: String, // "build" or "upload"
    #[serde(default = "default_max_releases")]
    pub max_releases: u32,
}

fn default_max_releases() -> u32 {
    15
}

fn default_deploy_mode() -> String {
    "build".to_string()
}

fn default_build_output_dir() -> String {
    "dist".to_string()
}

fn default_symlink_name() -> String {
    "current".to_string()
}

fn get_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|mut path| {
            if !path.exists() {
                let _ = fs::create_dir_all(&path);
            }
            path.push("projects.json");
            path
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_projects(app: AppHandle) -> Result<Vec<Project>, String> {
    let path = get_config_path(&app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read config file: {}", e))?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse config file: {}", e))
}

#[tauri::command]
pub fn save_project(app: AppHandle, mut project: Project) -> Result<Project, String> {
    let mut projects = get_projects(app.clone())?;
    
    if project.id.is_empty() {
        project.id = Uuid::new_v4().to_string();
        projects.push(project.clone());
    } else {
        let mut found = false;
        for p in projects.iter_mut() {
            if p.id == project.id {
                *p = project.clone();
                found = true;
                break;
            }
        }
        if !found {
            projects.push(project.clone());
        }
    }

    let path = get_config_path(&app)?;
    let content = serde_json::to_string_pretty(&projects).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;

    Ok(project)
}

#[tauri::command]
pub fn delete_project(app: AppHandle, id: String) -> Result<(), String> {
    let mut projects = get_projects(app.clone())?;
    projects.retain(|p| p.id != id);
    let path = get_config_path(&app)?;
    let content = serde_json::to_string_pretty(&projects).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}
