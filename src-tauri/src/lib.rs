#[cfg(target_os = "android")]
mod bt;
mod commands;
mod logfmt;
mod settings;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_btspp::init());
    // Диалоги выбора папки/файла нужны только на десктопе:
    // на Android экспорт и импорт идут через SAF/MediaStore в плагине btspp
    #[cfg(not(target_os = "android"))]
    let builder = builder.plugin(tauri_plugin_dialog::init());
    builder
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
            commands::start_logging,
            commands::stop_logging,
            commands::get_export_settings,
            commands::set_export_format,
            commands::pick_export_dir,
            commands::reset_export_dir,
            commands::list_logs,
            commands::read_log,
            commands::import_log,
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска RecuScan");
}
