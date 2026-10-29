//! Ядро RecuScan: общение с ELM327-адаптером и протокол OBD2 (SAE J1979).

pub mod dtc;
pub mod elm327;
pub mod pid;
pub mod profile;
pub mod readiness;
pub mod transport;
pub mod vin;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObdError {
    #[error("ошибка ввода-вывода: {0}")]
    Io(#[from] std::io::Error),
    #[error("таймаут ответа адаптера")]
    Timeout,
    #[error("ЭБУ не вернул данные (NO DATA)")]
    NoData,
    #[error("не удалось установить связь с ЭБУ")]
    ConnectionFailed,
    #[error("адаптер отклонил команду: {0}")]
    Adapter(String),
    #[error("некорректный ответ адаптера: {0}")]
    Parse(String),
    #[error("нет активного подключения")]
    NotConnected,
}

pub type Result<T> = std::result::Result<T, ObdError>;
