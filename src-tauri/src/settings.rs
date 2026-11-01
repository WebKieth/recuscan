//! Настройки приложения (папка и формат экспорта логов), JSON в app_config_dir.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    /// "csv" | "json" | "xls"; None — CSV.
    pub export_format: Option<String>,
    /// Папка экспорта: путь (десктоп) или SAF tree URI content:// (Android).
    /// None — папка по умолчанию.
    pub export_dir: Option<String>,
    /// Отображаемое имя папки (актуально для SAF URI, где виден только content://).
    pub export_dir_label: Option<String>,
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("config dir недоступна: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("не создать {}: {e}", dir.display()))?;
    Ok(dir.join("settings.json"))
}

pub fn load(app: &AppHandle) -> Settings {
    let Ok(path) = settings_path(app) else {
        return Settings::default();
    };
    let Ok(bytes) = std::fs::read(path) else {
        return Settings::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app)?;
    let bytes = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, bytes).map_err(|e| format!("не записать {}: {e}", path.display()))
}
