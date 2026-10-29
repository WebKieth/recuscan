use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use obd_core::elm327::Elm327;
use obd_core::profile::VehicleInfo;
use obd_core::transport::TcpTransport;
use obd_core::{dtc, pid, vin, ObdError};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::state::AppState;

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
#[tauri::command]
pub async fn connect_wifi(
    state: State<'_, AppState>,
    host: String,
    port: u16,
) -> Result<VehicleInfo, String> {
    let transport = TcpTransport::connect(&host, port).await.map_err(err)?;
    let mut elm = Elm327::init(Box::new(transport)).await.map_err(err)?;

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

    let supported_keys = supported
        .iter()
        .filter_map(|&b| pid::by_pid(b))
        .map(|d| d.key.to_string())
        .collect();

    let info = VehicleInfo {
        vin: vin_str,
        manufacturer,
        protocol,
        calibration_id,
        supported_pids: supported_keys,
    };

    *state.vehicle.lock().await = Some(info.clone());
    *state.conn.lock().await = Some(Arc::new(Mutex::new(elm)));
    Ok(info)
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
#[tauri::command]
pub async fn read_dtcs(state: State<'_, AppState>) -> Result<DtcReport, String> {
    let conn = state.conn.lock().await.clone();
    let conn = conn.ok_or(ObdError::NotConnected).map_err(err)?;
    let mut elm = conn.lock().await;

    // Статус Check Engine и число кодов по мнению ЭБУ
    let monitor = elm
        .query_pid(0x01, 0x01)
        .await
        .ok()
        .and_then(|d| dtc::decode_monitor_status(&d));

    // NO DATA у отдельного режима — не ошибка, просто нет кодов
    let read = |mode: u8, elm: &mut Elm327| async move {
        match elm.query_mode(mode).await {
            Ok(data) => dtc::decode_list(&data)
                .into_iter()
                .map(|code| DtcEntry {
                    description: dtc::description(&code).map(String::from),
                    code,
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    };

    Ok(DtcReport {
        mil_on: monitor.map(|m| m.mil_on),
        mil_dtc_count: monitor.map(|m| m.dtc_count),
        stored: read(0x03, &mut elm).await,
        pending: read(0x07, &mut elm).await,
        permanent: read(0x0A, &mut elm).await,
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

    let defs: Vec<&'static pid::PidDef> =
        keys.iter().filter_map(|k| pid::by_key(k)).collect();
    if defs.is_empty() {
        return Err("не выбрано ни одного поддерживаемого PID".into());
    }

    let handle = tokio::spawn(async move {
        let mut last_speed: Option<f64> = None;
        let mut last_maf: Option<f64> = None;
        loop {
            for def in &defs {
                let res = {
                    let mut elm = conn.lock().await;
                    elm.query_pid(0x01, def.pid).await
                };
                if let Ok(data) = res {
                    if let Some(value) = (def.decode)(&data) {
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
