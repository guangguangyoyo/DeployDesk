pub mod projects;
pub mod deploy;
pub mod logs;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            projects::get_projects,
            projects::save_project,
            projects::delete_project,
            deploy::run_deployment,
            deploy::get_releases,
            deploy::rollback,
            deploy::check_connection,
            logs::get_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
