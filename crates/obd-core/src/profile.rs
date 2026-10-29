//! Профиль автомобиля: результат автоопределения при подключении.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct VehicleInfo {
    pub vin: Option<String>,
    pub manufacturer: Option<String>,
    /// Год выпуска по 10-му символу VIN.
    pub model_year: Option<u16>,
    pub protocol: Option<String>,
    pub calibration_id: Option<String>,
    /// Имя ЭБУ (Mode 09 PID 0A).
    pub ecu_name: Option<String>,
    /// Ключи поддерживаемых PID'ов из реестра [`crate::pid`].
    pub supported_pids: Vec<String>,
}
