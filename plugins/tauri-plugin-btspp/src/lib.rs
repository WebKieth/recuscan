//! Tauri-плагин: Bluetooth Classic SPP (RFCOMM) для ELM327 на Android.

mod commands;
mod error;
mod models;

#[cfg(target_os = "android")]
mod android_impl;
#[cfg(not(target_os = "android"))]
mod desktop;

pub use error::{Error, Result};
pub use models::*;

#[cfg(target_os = "android")]
pub use android_impl::BtSpp;
#[cfg(not(target_os = "android"))]
pub use desktop::BtSpp;

use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

/// Инициализация плагина.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("btspp")
        .invoke_handler(tauri::generate_handler![commands::list_paired])
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let bt = android_impl::init(app, api)?;
            #[cfg(not(target_os = "android"))]
            let bt = desktop::init(app, api)?;
            app.manage(bt);
            Ok(())
        })
        .build()
}
