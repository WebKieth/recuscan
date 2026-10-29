#[cfg(target_os = "android")]
mod bt;
mod commands;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_btspp::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::connect_wifi,
            commands::connect_serial,
            commands::connect_bt,
            commands::list_serial_ports,
            commands::disconnect,
            commands::vehicle_info,
            commands::available_pids,
            commands::read_dtcs,
            commands::clear_dtcs,
            commands::read_readiness,
            commands::read_freeze_frame,
            commands::start_monitoring,
            commands::stop_monitoring,
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска RecuScan");
}
