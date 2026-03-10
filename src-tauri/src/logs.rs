use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use chrono::Local;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: String,
    pub project_name: String,
    pub action: String, // "发布" | "回退"
    pub status: String, // "成功" | "失败"
    pub details: String,
}

fn get_logs_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|mut path| {
            if !path.exists() {
                let _ = fs::create_dir_all(&path);
            }
            path.push("logs.json");
            path
        })
        .map_err(|e| e.to_string())
}

pub fn add_log(app: &AppHandle, project_name: &str, action: &str, status: &str, details: &str) -> Result<(), String> {
    let path = get_logs_path(app)?;
    let mut logs = if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str::<Vec<LogEntry>>(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    let new_entry = LogEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        project_name: project_name.to_string(),
        action: action.to_string(),
        status: status.to_string(),
        details: details.to_string(),
    };

    logs.insert(0, new_entry);
    if logs.len() > 100 {
        logs.truncate(100);
    }

    let content = serde_json::to_string_pretty(&logs).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub fn get_logs(app: AppHandle) -> Result<Vec<LogEntry>, String> {
    let path = get_logs_path(&app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}
