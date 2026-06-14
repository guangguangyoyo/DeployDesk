use crate::models::LogEntry;
use crate::storage::Storage;
use chrono::Local;
use uuid::Uuid;

const MAX_LOG_ENTRIES: usize = 100;

pub fn add_log(
    storage: &Storage,
    project_name: &str,
    action: &str,
    status: &str,
    details: &str,
) -> Result<(), String> {
    let mut logs = storage.get_logs()?;

    logs.insert(
        0,
        LogEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            project_name: project_name.to_string(),
            action: action.to_string(),
            status: status.to_string(),
            details: details.to_string(),
        },
    );

    if logs.len() > MAX_LOG_ENTRIES {
        logs.truncate(MAX_LOG_ENTRIES);
    }

    storage.save_logs(&logs)
}
