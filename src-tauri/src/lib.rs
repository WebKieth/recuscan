mod commands;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::connect_wifi,
            commands::disconnect,
            commands::vehicle_info,
            commands::available_pids,
            commands::read_dtcs,
            commands::clear_dtcs,
            commands::start_monitoring,
            commands::stop_monitoring,
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска RecuScan");
}
