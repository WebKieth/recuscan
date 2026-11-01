use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use obd_core::elm327::Elm327;
use obd_core::profile::VehicleInfo;
use obd_core::readiness::{self, ReadinessItem};
use obd_core::transport::TcpTransport;
use obd_core::{dtc, pid, vin, ObdError};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::logfmt::{self, ExportFormat, ParsedLog};
use crate::settings::{self, Settings};
use crate::state::{AppState, CsvLogger};

#[cfg(target_os = "android")]
use base64::Engine as _;
#[cfg(target_os = "android")]
use tauri_plugin_btspp::{ExportFileEntry, PickedUri, SaveExportArgs};
#[cfg(not(target_os = "android"))]
use tauri_plugin_dialog::DialogExt;

fn err(e: ObdError) -> String {
    e.to_string()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Serialize, Clone)]
pub struct Sample {
    pub key: String,
    pub name: String,
    pub unit: String,
    pub value: f64,
    pub ts: u64,
}

#[derive(Serialize)]
pub struct PidInfo {
    pub key: String,
    pub name: String,
    pub unit: String,
}

#[derive(Serialize)]
pub struct DtcEntry {
    pub code: String,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct DtcReport {
    /// Горит ли Check Engine (None — ЭБУ не ответил на PID 0101).
    pub mil_on: Option<bool>,
    /// Число сохранённых кодов по данным ЭБУ.
    pub mil_dtc_count: Option<u8>,
    pub stored: Vec<DtcEntry>,
    pub pending: Vec<DtcEntry>,
    pub permanent: Vec<DtcEntry>,
}

/// Подключение к Wi-Fi ELM327 адаптеру + автоопределение автомобиля.
/// `protocol`: None — авто; Some(3/4/5) — форсировать K-line (ISO 9141-2 / KWP2000).
#[tauri::command]
pub async fn connect_wifi(
    state: State<'_, AppState>,
    host: String,
    port: u16,
    protocol: Option<u8>,
) -> Result<VehicleInfo, String> {
    let transport = TcpTransport::connect(&host, port).await.map_err(err)?;
    let mut elm = Elm327::init_with_protocol(Box::new(transport), protocol)
        .await
        .map_err(err)?;
    let info = probe_vehicle(&mut elm).await;

    *state.vehicle.lock().await = Some(info.clone());
    *state.conn.lock().await = Some(Arc::new(Mutex::new(elm)));
    Ok(info)
}

/// Подключение к USB/Serial ELM327 адаптеру (типичный бодрейт — 38400).
#[tauri::command]
pub async fn connect_serial(
    state: State<'_, AppState>,
    path: String,
    baud: u32,
    protocol: Option<u8>,
) -> Result<VehicleInfo, String> {
    #[cfg(feature = "serial")]
    {
        let transport = obd_core::transport::SerialTransport::open(&path, baud).map_err(err)?;
        let mut elm = Elm327::init_with_protocol(Box::new(transport), protocol)
            .await
            .map_err(err)?;
        let info = probe_vehicle(&mut elm).await;

        *state.vehicle.lock().await = Some(info.clone());
        *state.conn.lock().await = Some(Arc::new(Mutex::new(elm)));
        return Ok(info);
    }
    #[cfg(not(feature = "serial"))]
    {
        let _ = (state, path, baud, protocol);
        Err("сборка без поддержки Serial (feature `serial` отключён)".into())
    }
}

/// Список последовательных портов для USB-подключения.
#[tauri::command]
pub async fn list_serial_ports() -> Result<Vec<String>, String> {
    #[cfg(feature = "serial")]
    return Ok(obd_core::transport::list_serial_ports());
    #[cfg(not(feature = "serial"))]
    Ok(Vec::new())
}

/// Подключение к Bluetooth Classic SPP адаптеру (только Android;
/// на iOS SPP заблокирован Apple — там только Wi-Fi адаптеры).
#[tauri::command]
pub async fn connect_bt(
    app: AppHandle,
    state: State<'_, AppState>,
    address: String,
    protocol: Option<u8>,
) -> Result<VehicleInfo, String> {
    #[cfg(target_os = "android")]
    {
        let plugin = app
            .state::<tauri_plugin_btspp::BtSpp<tauri::Wry>>()
            .inner()
            .clone();
        plugin.connect(address).map_err(|e| e.to_string())?;
        let transport = crate::bt::BtTransport::new(plugin);
        let mut elm = Elm327::init_with_protocol(Box::new(transport), protocol)
            .await
            .map_err(err)?;
        let info = probe_vehicle(&mut elm).await;

        *state.vehicle.lock().await = Some(info.clone());
        *state.conn.lock().await = Some(Arc::new(Mutex::new(elm)));
        return Ok(info);
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, state, address, protocol);
        Err("Bluetooth SPP доступен только на Android (на iOS SPP заблокирован Apple)".into())
    }
}

/// Автоопределение автомобиля после установки связи: протокол, карта PID'ов,
/// VIN → производитель, калибровка ЭБУ.
async fn probe_vehicle(elm: &mut Elm327) -> VehicleInfo {
    // Протокол, выбранный адаптером автоматически
    let protocol = elm.describe_protocol().await.ok();

    // Карта поддерживаемых PID'ов (0100, 0120, 0140, ...)
    let mut supported: Vec<u8> = Vec::new();
    let mut base = 0x00u8;
    loop {
        match elm.query_pid(0x01, base).await {
            Ok(mask) => {
                let pids = pid::decode_bitmask(base + 1, &mask);
                supported.extend(&pids);
                if pids.contains(&(base + 0x20)) {
                    base += 0x20;
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    // VIN → производитель
    let vin_str = elm
        .query_pid(0x09, 0x02)
        .await
        .ok()
        .and_then(|p| vin::decode_vin(&p));
    let manufacturer = vin_str
        .as_deref()
        .and_then(vin::manufacturer)
        .map(String::from);
    let calibration_id = elm
        .query_pid(0x09, 0x04)
        .await
        .ok()
        .map(|p| String::from_utf8_lossy(&p).trim().to_string());
    // Имя ЭБУ (090A) — полезно при поиске прошивок и TSB
    let ecu_name = elm
        .query_pid(0x09, 0x0A)
        .await
        .ok()
        .map(|p| String::from_utf8_lossy(&p).trim().to_string())
        .filter(|s| !s.is_empty());
    // Год выпуска из 10-го символа VIN
    let model_year = vin_str
        .as_deref()
        .and_then(|v| vin::model_year(v, current_year()));

    let supported_keys = supported
        .iter()
        .filter_map(|&b| pid::by_pid(b))
        .map(|d| d.key.to_string())
        .collect();

    VehicleInfo {
        vin: vin_str,
        manufacturer,
        model_year,
        protocol,
        calibration_id,
        ecu_name,
        supported_pids: supported_keys,
    }
}

/// Текущий год (приблизительно; точности хватает для 30-летнего цикла VIN).
fn current_year() -> u16 {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86_400;
    1970 + (days / 365) as u16
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(h) = state.monitor.lock().await.take() {
        h.abort();
    }
    state.conn.lock().await.take();
    state.vehicle.lock().await.take();
    Ok(())
}

#[tauri::command]
pub async fn vehicle_info(state: State<'_, AppState>) -> Result<Option<VehicleInfo>, String> {
    Ok(state.vehicle.lock().await.clone())
}

/// PID'ы, доступные для мониторинга (по карте поддержки ЭБУ, если известна).
#[tauri::command]
pub async fn available_pids(state: State<'_, AppState>) -> Result<Vec<PidInfo>, String> {
    let vehicle = state.vehicle.lock().await.clone();
    let defs: Vec<_> = match &vehicle {
        Some(v) if !v.supported_pids.is_empty() => pid::PIDS
            .iter()
            .filter(|d| v.supported_pids.iter().any(|k| k == d.key))
            .cloned()
            .collect(),
        _ => pid::PIDS.to_vec(),
    };
    Ok(defs
        .iter()
        .map(|d| PidInfo {
            key: d.key.into(),
            name: d.name.into(),
            unit: d.unit.into(),
        })
        .collect())
}

/// Чтение ошибок: сохранённые (03), ожидающие (07), постоянные (0A) + статус MIL.
/// Описания подбираются с учётом марки из автоопределённого VIN.
#[tauri::command]
pub async fn read_dtcs(state: State<'_, AppState>) -> Result<DtcReport, String> {
    let conn = state.conn.lock().await.clone();
    let conn = conn.ok_or(ObdError::NotConnected).map_err(err)?;
    let manufacturer = state
        .vehicle
        .lock()
        .await
        .as_ref()
        .and_then(|v| v.manufacturer.clone());
    let mut elm = conn.lock().await;

    // Статус Check Engine и число кодов по мнению ЭБУ
    let monitor = elm
        .query_pid(0x01, 0x01)
        .await
        .ok()
        .and_then(|d| dtc::decode_monitor_status(&d));

    Ok(DtcReport {
        mil_on: monitor.map(|m| m.mil_on),
        mil_dtc_count: monitor.map(|m| m.dtc_count),
        stored: read_dtc_mode(&mut elm, 0x03, manufacturer.as_deref()).await,
        pending: read_dtc_mode(&mut elm, 0x07, manufacturer.as_deref()).await,
        permanent: read_dtc_mode(&mut elm, 0x0A, manufacturer.as_deref()).await,
    })
}

/// Прочитать коды одного режима (03/07/0A) с расшифровкой по базе.
/// NO DATA — не ошибка, просто нет кодов.
async fn read_dtc_mode(
    elm: &mut Elm327,
    mode: u8,
    manufacturer: Option<&str>,
) -> Vec<DtcEntry> {
    match elm.query_mode(mode).await {
        Ok(data) => dtc::decode_list(&data)
            .into_iter()
            .map(|code| DtcEntry {
                description: dtc::description_for(&code, manufacturer).map(String::from),
                code,
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Готовность диагностических мониторов (для техосмотра после сброса ошибок).
#[tauri::command]
pub async fn read_readiness(state: State<'_, AppState>) -> Result<Vec<ReadinessItem>, String> {
    let conn = state.conn.lock().await.clone();
    let conn = conn.ok_or(ObdError::NotConnected).map_err(err)?;
    let mut elm = conn.lock().await;
    let data = elm.query_pid(0x01, 0x01).await.map_err(err)?;
    Ok(readiness::decode_readiness(&data))
}

#[derive(Serialize)]
pub struct FreezeFrameEntry {
    pub key: String,
    pub name: String,
    pub unit: String,
    pub value: f64,
}

#[derive(Serialize)]
pub struct FreezeFrame {
    /// Код, вызвавший регистрацию freeze frame (Mode 02 PID 02).
    pub dtc: Option<String>,
    pub description: Option<String>,
    pub entries: Vec<FreezeFrameEntry>,
}

/// Freeze frame — снимок параметров двигателя в момент регистрации ошибки (Mode 02).
#[tauri::command]
pub async fn read_freeze_frame(state: State<'_, AppState>) -> Result<FreezeFrame, String> {
    let conn = state.conn.lock().await.clone();
    let conn = conn.ok_or(ObdError::NotConnected).map_err(err)?;
    let manufacturer = state
        .vehicle
        .lock()
        .await
        .as_ref()
        .and_then(|v| v.manufacturer.clone());
    let mut elm = conn.lock().await;

    let causative = elm
        .query_pid(0x02, 0x02)
        .await
        .ok()
        .and_then(|d| {
            if d.len() >= 2 {
                dtc::decode_pair(d[0], d[1])
            } else {
                None
            }
        });
    let description = causative
        .as_deref()
        .and_then(|c| dtc::description_for(c, manufacturer.as_deref()))
        .map(String::from);

    let mut entries = Vec::new();
    for def in pid::PIDS {
        if let Ok(data) = elm.query_pid(0x02, def.pid).await {
            if let Some(value) = (def.decode)(&data) {
                entries.push(FreezeFrameEntry {
                    key: def.key.into(),
                    name: def.name.into(),
                    unit: def.unit.into(),
                    value,
                });
            }
        }
    }
    if entries.is_empty() && causative.is_none() {
        return Err("ЭБУ не вернул freeze frame (нет сохранённой ошибки)".into());
    }
    Ok(FreezeFrame {
        dtc: causative,
        description,
        entries,
    })
}

/// Сброс ошибок (Mode 04).
///
/// Безопасно для прошивки: Mode 04 стирает только диагностическую память
/// (коды, freeze frame, readiness-флаги, адаптации топливных коррекций).
/// Permanent DTC (Mode 0A) стандарт запрещает стирать — они уходят сами
/// после нескольких поездок без неисправности. Многие ЭБУ отклоняют сброс
/// при работающем двигателе — рекомендуется зажигание ON, двигатель заглушен.
#[tauri::command]
pub async fn clear_dtcs(state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.conn.lock().await.clone();
    let conn = conn.ok_or(ObdError::NotConnected).map_err(err)?;
    let mut elm = conn.lock().await;
    elm.query_mode(0x04).await.map_err(err)?;
    Ok(())
}

/// Запуск цикла опроса выбранных PID'ов; данные стримятся событием `obd-sample`.
#[tauri::command]
pub async fn start_monitoring(
    app: AppHandle,
    state: State<'_, AppState>,
    keys: Vec<String>,
) -> Result<(), String> {
    if let Some(h) = state.monitor.lock().await.take() {
        h.abort();
    }
    let conn = state.conn.lock().await.clone();
    let conn = conn.ok_or(ObdError::NotConnected).map_err(err)?;
    let log = state.log.clone();

    let defs: Vec<&'static pid::PidDef> =
        keys.iter().filter_map(|k| pid::by_key(k)).collect();
    if defs.is_empty() {
        return Err("не выбрано ни одного поддерживаемого PID".into());
    }

    let handle = tokio::spawn(async move {
        let mut last_speed: Option<f64> = None;
        let mut last_maf: Option<f64> = None;
        // Последние значения всех PID'ов — для строки CSV за цикл опроса
        let mut latest: HashMap<String, f64> = HashMap::new();
        loop {
            for def in &defs {
                let res = {
                    let mut elm = conn.lock().await;
                    elm.query_pid(0x01, def.pid).await
                };
                if let Ok(data) = res {
                    if let Some(value) = (def.decode)(&data) {
                        latest.insert(def.key.into(), value);
                        match def.key {
                            "speed" => last_speed = Some(value),
                            "maf" => last_maf = Some(value),
                            _ => {}
                        }
                        let _ = app.emit(
                            "obd-sample",
                            Sample {
                                key: def.key.into(),
                                name: def.name.into(),
                                unit: def.unit.into(),
                                value,
                                ts: now_ms(),
                            },
                        );
                        // Мгновенный расход: оценка по MAF для бензина
                        // (стехиометрия 14.7:1, плотность бензина ~720 г/л)
                        if matches!(def.key, "speed" | "maf") {
                            if let (Some(spd), Some(maf)) = (last_speed, last_maf) {
                                let liters_per_hour = maf * 3600.0 / (14.7 * 720.0);
                                let per_100km = if spd > 5.0 {
                                    liters_per_hour / spd * 100.0
                                } else {
                                    0.0
                                };
                                latest.insert("fuel_consumption".into(), per_100km);
                                let _ = app.emit(
                                    "obd-sample",
                                    Sample {
                                        key: "fuel_consumption".into(),
                                        name: "Расход топлива (оценка)".into(),
                                        unit: "л/100км".into(),
                                        value: per_100km,
                                        ts: now_ms(),
                                    },
                                );
                            }
                        }
                    }
                }
            }
            // Строка CSV за цикл (если включена запись); лок не держим во время сна
            {
                let mut guard = log.lock().await;
                if let Some(logger) = guard.as_mut() {
                    logger.write_row(now_ms(), &latest);
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });
    *state.monitor.lock().await = Some(handle);
    Ok(())
}

#[tauri::command]
pub async fn stop_monitoring(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(h) = state.monitor.lock().await.take() {
        h.abort();
    }
    Ok(())
}

#[derive(Serialize)]
pub struct LogSummary {
    /// Отображаемый путь к экспортированному файлу (label, не обязательно fs-путь).
    pub path: String,
    pub rows: u64,
}

/// Начать запись мониторинга. Пишется временный CSV (переживает аварийное
/// закрытие), при остановке конвертируется в выбранный формат и экспортируется
/// в настроенную папку. Возвращает имя будущего файла.
/// Если в опросе есть и скорость, и MAF — добавляет колонку расхода (л/100км).
#[tauri::command]
pub async fn start_logging(
    app: AppHandle,
    state: State<'_, AppState>,
    keys: Vec<String>,
) -> Result<String, String> {
    let mut keys: Vec<String> = keys;
    if keys.iter().any(|k| k == "speed") && keys.iter().any(|k| k == "maf") {
        keys.push("fuel_consumption".into());
    }
    let (path, file) = create_temp_log_file(&app)?;
    let fmt = current_format(&app);
    // Имя итогового файла (совпадает с тем, что вернёт stop_logging)
    let export_display_name = export_name(&path, fmt.extension());
    let mut writer = std::io::BufWriter::new(file);
    // BOM — чтобы Excel корректно открыл UTF-8 с русскими именами колонок
    use std::io::Write;
    writer
        .write_all("\u{FEFF}".as_bytes())
        .and_then(|_| writeln!(writer, "ts_ms,{}", csv_header(&keys)))
        .and_then(|_| writer.flush())
        .map_err(|e| format!("не записать заголовок в {}: {e}", path.display()))?;

    *state.log.lock().await = Some(CsvLogger::new(writer, keys, path));
    Ok(export_display_name)
}

/// Завершить запись и экспортировать лог в настроенной папке и формате.
/// Возвращает отображаемый путь и число строк (или None, если запись не велась).
#[tauri::command]
pub async fn stop_logging(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<LogSummary>, String> {
    let logger = state.log.lock().await.take();
    let Some(mut logger) = logger else {
        return Ok(None);
    };
    logger.flush();
    let rows = logger.rows;

    let temp_bytes = std::fs::read(&logger.path)
        .map_err(|e| format!("не прочитать временный лог {}: {e}", logger.path.display()))?;
    let settings = settings::load(&app);
    let fmt = current_format(&app);
    let (data, name) = if fmt == ExportFormat::Csv {
        // CSV: временный файл уже в нужном виде, только имя с датой старта записи
        let name = logger
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| log_filename("csv"));
        (temp_bytes, name)
    } else {
        // Конвертация: восстанавливаем ключи/единицы по реестру PID'ов
        let mut log = logfmt::parse_csv(&temp_bytes)?;
        log.columns = columns_for_keys(&logger.keys);
        (
            logfmt::render(fmt, &log),
            export_name(&logger.path, fmt.extension()),
        )
    };
    let label = write_export(&app, &settings, &name, fmt.mime(), &data)?;
    let _ = std::fs::remove_file(&logger.path);
    Ok(Some(LogSummary { path: label, rows }))
}

/// Имя экспорта из имени временного файла с подменой расширения.
fn export_name(temp_path: &std::path::Path, ext: &str) -> String {
    temp_path
        .file_stem()
        .map(|s| format!("{}.{ext}", s.to_string_lossy()))
        .unwrap_or_else(|| log_filename(ext))
}

/// Заголовок CSV: человекочитаемые имена с единицами, где PID известен реестру.
fn csv_header(keys: &[String]) -> String {
    columns_for_keys(keys)
        .iter()
        .map(|c| {
            if c.unit.is_empty() {
                c.name.clone()
            } else {
                format!("{} [{}]", c.name, c.unit)
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Метаданные колонок лога по ключам PID'ов (для конвертации в JSON/XLS).
fn columns_for_keys(keys: &[String]) -> Vec<logfmt::LogColumn> {
    keys.iter()
        .map(|k| {
            let (name, unit) = match k.as_str() {
                "fuel_consumption" => ("Расход топлива (оценка)", "л/100км"),
                _ => pid::by_key(k).map(|d| (d.name, d.unit)).unwrap_or((k.as_str(), "")),
            };
            logfmt::LogColumn {
                key: k.clone(),
                name: name.to_string(),
                unit: unit.to_string(),
            }
        })
        .collect()
}

/// Временный файл записи: приватный кэш — всегда доступен для записи,
/// в т.ч. на Android со scoped storage. Экспорт — при остановке записи.
fn create_temp_log_file(app: &AppHandle) -> Result<(PathBuf, std::fs::File), String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("cache dir недоступна: {e}"))?
        .join("recuscan");
    std::fs::create_dir_all(&dir).map_err(|e| format!("не создать {}: {e}", dir.display()))?;
    let path = dir.join(log_filename("csv"));
    let file = std::fs::File::create(&path)
        .map_err(|e| format!("не создать {}: {e}", path.display()))?;
    Ok((path, file))
}

fn current_format(app: &AppHandle) -> ExportFormat {
    ExportFormat::parse(
        settings::load(app)
            .export_format
            .as_deref()
            .unwrap_or("csv"),
    )
}

/// Записать готовый файл экспорта: SAF/MediaStore на Android, fs на десктопе.
/// Возвращает отображаемый путь (label).
fn write_export(
    app: &AppHandle,
    settings: &Settings,
    file_name: &str,
    mime: &str,
    data: &[u8],
) -> Result<String, String> {
    #[cfg(target_os = "android")]
    {
        let plugin = app
            .state::<tauri_plugin_btspp::BtSpp<tauri::Wry>>()
            .inner()
            .clone();
        let resp = plugin
            .save_export(SaveExportArgs {
                tree_uri: saf_dir(settings),
                file_name: file_name.to_string(),
                mime: mime.to_string(),
                data: base64::engine::general_purpose::STANDARD.encode(data),
            })
            .map_err(|e| e.to_string())?;
        return Ok(resp.label);
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = mime; // на десктопе тип файла определяется расширением
        let dir = settings
            .export_dir
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| default_export_dir(app));
        std::fs::create_dir_all(&dir).map_err(|e| format!("не создать {}: {e}", dir.display()))?;
        let path = dir.join(file_name);
        std::fs::write(&path, data).map_err(|e| format!("не записать {}: {e}", path.display()))?;
        return Ok(path.display().to_string());
    }
}

/// SAF tree URI из настроек (Android); None — папка по умолчанию (Загрузки/recuscan).
#[cfg(target_os = "android")]
fn saf_dir(settings: &Settings) -> Option<String> {
    settings
        .export_dir
        .clone()
        .filter(|d| d.starts_with("content://"))
}

/// Папка экспорта по умолчанию на десктопе: Документы → Загрузки → temp.
#[cfg(not(target_os = "android"))]
fn default_export_dir(app: &AppHandle) -> PathBuf {
    let p = app.path();
    p.document_dir()
        .ok()
        .or_else(|| p.download_dir().ok())
        .unwrap_or_else(std::env::temp_dir)
        .join("recuscan")
}

// ---------- Настройки экспорта ----------

#[derive(Serialize)]
pub struct ExportSettingsDto {
    pub format: String,
    pub format_label: String,
    pub dir_label: String,
    pub is_default_dir: bool,
}

fn settings_dto(app: &AppHandle, settings: &Settings) -> ExportSettingsDto {
    let fmt = ExportFormat::parse(settings.export_format.as_deref().unwrap_or("csv"));
    let (dir_label, is_default_dir) = match &settings.export_dir {
        Some(dir) => (
            settings
                .export_dir_label
                .clone()
                .filter(|l| !l.is_empty())
                .unwrap_or_else(|| dir.clone()),
            false,
        ),
        None => (default_dir_label(app), true),
    };
    ExportSettingsDto {
        format: fmt.as_str().into(),
        format_label: fmt.label().into(),
        dir_label,
        is_default_dir,
    }
}

#[cfg(target_os = "android")]
fn default_dir_label(_app: &AppHandle) -> String {
    // MediaStore Downloads/recuscan (на API < 29 Kotlin падает в app-specific папку)
    "Загрузки/recuscan".into()
}

#[cfg(not(target_os = "android"))]
fn default_dir_label(app: &AppHandle) -> String {
    default_export_dir(app).display().to_string()
}

#[tauri::command]
pub async fn get_export_settings(app: AppHandle) -> Result<ExportSettingsDto, String> {
    Ok(settings_dto(&app, &settings::load(&app)))
}

#[tauri::command]
pub async fn set_export_format(app: AppHandle, format: String) -> Result<ExportSettingsDto, String> {
    let fmt = ExportFormat::parse(&format);
    let mut s = settings::load(&app);
    s.export_format = Some(fmt.as_str().into());
    settings::save(&app, &s)?;
    Ok(settings_dto(&app, &s))
}

/// Выбор папки экспорта: нативный диалог (десктоп) или SAF-пикер (Android).
/// None — пользователь отменил выбор.
#[tauri::command]
pub async fn pick_export_dir(app: AppHandle) -> Result<Option<ExportSettingsDto>, String> {
    #[cfg(target_os = "android")]
    {
        let plugin = app
            .state::<tauri_plugin_btspp::BtSpp<tauri::Wry>>()
            .inner()
            .clone();
        let PickedUri { uri, name } = plugin.pick_folder().map_err(|e| e.to_string())?;
        let Some(uri) = uri else {
            return Ok(None);
        };
        let mut s = settings::load(&app);
        s.export_dir = Some(uri);
        s.export_dir_label = name;
        settings::save(&app, &s)?;
        return Ok(Some(settings_dto(&app, &s)));
    }
    #[cfg(not(target_os = "android"))]
    {
        let app2 = app.clone();
        let folder = tauri::async_runtime::spawn_blocking(move || {
            app2.dialog().file().blocking_pick_folder()
        })
        .await
        .map_err(|e| e.to_string())?;
        let Some(folder) = folder else {
            return Ok(None);
        };
        let path = folder
            .into_path()
            .map_err(|_| "выбранный объект не является путём на диске".to_string())?;
        let mut s = settings::load(&app);
        s.export_dir = Some(path.display().to_string());
        s.export_dir_label = None; // путь сам говорящий
        settings::save(&app, &s)?;
        return Ok(Some(settings_dto(&app, &s)));
    }
}

/// Сбросить папку экспорта на папку по умолчанию.
#[tauri::command]
pub async fn reset_export_dir(app: AppHandle) -> Result<ExportSettingsDto, String> {
    let mut s = settings::load(&app);
    s.export_dir = None;
    s.export_dir_label = None;
    settings::save(&app, &s)?;
    Ok(settings_dto(&app, &s))
}

// ---------- Анализ логов: список, чтение, импорт ----------

#[derive(Serialize)]
pub struct LogFileInfo {
    /// Идентификатор для read_log: fs-путь или content:// URI.
    pub id: String,
    pub name: String,
    pub size: u64,
    pub mtime_ms: u64,
}

#[derive(Serialize)]
pub struct ParsedLogDto {
    pub columns: Vec<logfmt::LogColumn>,
    pub ts_ms: Vec<u64>,
    /// Значения по колонкам: values[i][строка] соответствует columns[i].
    pub values: Vec<Vec<Option<f64>>>,
}

impl From<ParsedLog> for ParsedLogDto {
    fn from(log: ParsedLog) -> Self {
        Self {
            columns: log.columns,
            ts_ms: log.ts_ms,
            values: log.values,
        }
    }
}

#[derive(Serialize)]
pub struct ImportedLog {
    pub id: String,
    pub name: String,
    pub log: ParsedLogDto,
}

fn log_extension(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or("").trim()
}

/// Разбор по расширению имени; без расширения — пробуем CSV, затем JSON.
fn parse_log_bytes(name: &str, bytes: &[u8]) -> Result<ParsedLogDto, String> {
    let ext = log_extension(name).to_lowercase();
    match ext.as_str() {
        "csv" | "json" | "xls" => logfmt::parse(&ext, bytes).map(ParsedLogDto::from),
        _ => logfmt::parse("csv", bytes)
            .or_else(|_| logfmt::parse("json", bytes))
            .map(ParsedLogDto::from)
            .map_err(|e| format!("не удалось разобрать лог ни как CSV, ни как JSON: {e}")),
    }
}

/// Список записанных логов в папке экспорта (выбранной или по умолчанию).
#[tauri::command]
pub async fn list_logs(app: AppHandle) -> Result<Vec<LogFileInfo>, String> {
    let settings = settings::load(&app);
    #[cfg(target_os = "android")]
    {
        let plugin = app
            .state::<tauri_plugin_btspp::BtSpp<tauri::Wry>>()
            .inner()
            .clone();
        let files: Vec<ExportFileEntry> =
            plugin.list_exports(saf_dir(&settings)).map_err(|e| e.to_string())?;
        let mut logs: Vec<LogFileInfo> = files
            .into_iter()
            .filter(|f| matches!(log_extension(&f.name).to_lowercase().as_str(), "csv" | "json"))
            .map(|f| LogFileInfo {
                id: f.uri,
                name: f.name,
                size: f.size.max(0) as u64,
                mtime_ms: f.mtime.max(0) as u64,
            })
            .collect();
        logs.sort_by(|a, b| b.mtime_ms.cmp(&a.mtime_ms));
        logs.truncate(200);
        return Ok(logs);
    }
    #[cfg(not(target_os = "android"))]
    {
        let dir = settings
            .export_dir
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| default_export_dir(&app));
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return Ok(Vec::new()), // папки ещё нет — логов не записывали
        };
        let mut logs: Vec<LogFileInfo> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                matches!(
                    log_extension(&e.file_name().to_string_lossy()).to_lowercase().as_str(),
                    "csv" | "json"
                )
            })
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                let mtime_ms = meta
                    .modified()
                    .ok()?
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                Some(LogFileInfo {
                    id: e.path().display().to_string(),
                    name: e.file_name().to_string_lossy().into_owned(),
                    size: meta.len(),
                    mtime_ms,
                })
            })
            .collect();
        logs.sort_by(|a, b| b.mtime_ms.cmp(&a.mtime_ms));
        logs.truncate(200);
        return Ok(logs);
    }
}

/// Прочитать и разобрать лог по идентификатору из list_logs/import.
#[tauri::command]
pub async fn read_log(app: AppHandle, id: String, name: String) -> Result<ParsedLogDto, String> {
    let bytes = read_log_bytes(&app, &id)?;
    parse_log_bytes(&name, &bytes)
}

/// Импорт лога из произвольного места: диалог выбора файла (десктоп)
/// или SAF-пикер документа (Android). None — отмена.
#[tauri::command]
pub async fn import_log(app: AppHandle) -> Result<Option<ImportedLog>, String> {
    #[cfg(target_os = "android")]
    {
        let plugin = app
            .state::<tauri_plugin_btspp::BtSpp<tauri::Wry>>()
            .inner()
            .clone();
        let PickedUri { uri, name } = plugin.pick_log_file().map_err(|e| e.to_string())?;
        let Some(uri) = uri else {
            return Ok(None);
        };
        let name = name.unwrap_or_else(|| "лог.csv".into());
        let bytes = read_log_bytes(&app, &uri)?;
        let log = parse_log_bytes(&name, &bytes)?;
        return Ok(Some(ImportedLog { id: uri, name, log }));
    }
    #[cfg(not(target_os = "android"))]
    {
        let app2 = app.clone();
        let file = tauri::async_runtime::spawn_blocking(move || {
            app2
                .dialog()
                .file()
                .add_filter("Логи RecuScan", &["csv", "json"])
                .blocking_pick_file()
        })
        .await
        .map_err(|e| e.to_string())?;
        let Some(file) = file else {
            return Ok(None);
        };
        let path = file
            .into_path()
            .map_err(|_| "выбранный объект не является путём на диске".to_string())?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "лог.csv".into());
        let bytes = std::fs::read(&path)
            .map_err(|e| format!("не прочитать {}: {e}", path.display()))?;
        let log = parse_log_bytes(&name, &bytes)?;
        return Ok(Some(ImportedLog {
            id: path.display().to_string(),
            name,
            log,
        }));
    }
}

/// Байты лога по идентификатору: content:// через Kotlin, file:// и пути — напрямую.
fn read_log_bytes(app: &AppHandle, id: &str) -> Result<Vec<u8>, String> {
    #[cfg(not(target_os = "android"))]
    let _ = app; // content:// на десктопе не бывает, читаем только пути
    if id.starts_with("content://") {        #[cfg(target_os = "android")]
        {
            let plugin = app
                .state::<tauri_plugin_btspp::BtSpp<tauri::Wry>>()
                .inner()
                .clone();
            let resp = plugin
                .read_export(id.to_string())
                .map_err(|e| e.to_string())?;
            return base64::engine::general_purpose::STANDARD
                .decode(resp.data)
                .map_err(|e| format!("битый base64 из плагина: {e}"));
        }
        #[cfg(not(target_os = "android"))]
        {
            return Err("content:// URI доступны только на Android".into());
        }
    }
    let path = id.strip_prefix("file://").unwrap_or(id);
    std::fs::read(path).map_err(|e| format!("не прочитать {path}: {e}"))
}

/// Имя файла вида recuscan-20261031-153012.<ext> (UTC; civil-from-days по Хиннанту).
fn log_filename(ext: &str) -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, rem % 3600 / 60, rem % 60);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("recuscan-{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}.{ext}")
}
