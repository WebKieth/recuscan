use std::sync::Arc;

use obd_core::elm327::Elm327;
use obd_core::profile::VehicleInfo;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[derive(Default)]
pub struct AppState {
    /// Активное подключение к адаптеру (разделяется с задачей мониторинга).
    pub conn: Mutex<Option<Arc<Mutex<Elm327>>>>,
    /// Задача цикла опроса PID'ов.
    pub monitor: Mutex<Option<JoinHandle<()>>>,
    /// Результат автоопределения автомобиля.
    pub vehicle: Mutex<Option<VehicleInfo>>,
}
