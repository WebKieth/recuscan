use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
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
    /// Активная запись мониторинга в CSV (Arc — разделяется с задачей опроса).
    pub log: Arc<Mutex<Option<CsvLogger>>>,
}

/// Пишущая CSV-сессия: по одной строке на цикл опроса, колонки — выбранные PID'ы.
pub struct CsvLogger {
    writer: BufWriter<File>,
    /// Колонки в порядке записи (ключи PID'ов).
    pub keys: Vec<String>,
    /// Сколько строк данных записано.
    pub rows: u64,
    pub path: PathBuf,
}

impl CsvLogger {
    pub fn new(writer: BufWriter<File>, keys: Vec<String>, path: PathBuf) -> Self {
        Self {
            writer,
            keys,
            rows: 0,
            path,
        }
    }

    /// Строка значений на текущий момент; для ещё не опрошенных PID'ов — пустая ячейка.
    /// Сбрасываем на диск после каждой строки: лог должен пережить аварийное закрытие.
    pub fn write_row(&mut self, ts: u64, latest: &HashMap<String, f64>) {        let mut line = String::with_capacity(16 + self.keys.len() * 10);
        line.push_str(&ts.to_string());
        for k in &self.keys {
            line.push(',');
            if let Some(v) = latest.get(k.as_str()) {
                // До 3 знаков после запятой, без хвостовых нулей
                let s = format!("{v:.3}");
                let s = s.trim_end_matches('0').trim_end_matches('.');
                line.push_str(if s.is_empty() || s == "-0" { "0" } else { s });
            }
        }
        line.push('\n');
        if self
            .writer
            .write_all(line.as_bytes())
            .and_then(|_| self.writer.flush())
            .is_ok()
        {
            self.rows += 1;
        }
    }

    pub fn flush(&mut self) {
        let _ = self.writer.flush();
    }
}
