//! Профиль автомобиля: результат автоопределения при подключении.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct VehicleInfo {
    pub vin: Option<String>,
    pub manufacturer: Option<String>,
    pub protocol: Option<String>,
    pub calibration_id: Option<String>,
    /// Ключи поддерживаемых PID'ов из реестра [`crate::pid`].
    pub supported_pids: Vec<String>,
}
