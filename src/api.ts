import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface VehicleInfo {
  vin: string | null;
  manufacturer: string | null;
  model_year: number | null;
  protocol: string | null;
  calibration_id: string | null;
  ecu_name: string | null;
  supported_pids: string[];
}

export interface DtcEntry {
  code: string;
  description: string | null;
}

export interface DtcReport {
  mil_on: boolean | null;
  mil_dtc_count: number | null;
  stored: DtcEntry[];
  pending: DtcEntry[];
  permanent: DtcEntry[];
}

export interface ReadinessItem {
  name: string;
  supported: boolean;
  complete: boolean;
}

export interface FreezeFrameEntry {
  key: string;
  name: string;
  unit: string;
  value: number;
}

export interface FreezeFrame {
  dtc: string | null;
  description: string | null;
  entries: FreezeFrameEntry[];
}

export interface PidInfo {
  key: string;
  name: string;
  unit: string;
}

export interface Sample {
  key: string;
  name: string;
  unit: string;
  value: number;
  ts: number;
}

export interface BtDevice {
  name: string;
  address: string;
}

export interface LogSummary {
  path: string;
  rows: number;
}

export interface ExportSettings {
  format: string;
  format_label: string;
  dir_label: string;
  is_default_dir: boolean;
}

export interface LogFileInfo {
  id: string;
  name: string;
  size: number;
  mtime_ms: number;
}

export interface LogColumn {
  key: string;
  name: string;
  unit: string;
}

export interface ParsedLog {
  columns: LogColumn[];
  ts_ms: number[];
  /** Значения по колонкам: values[i][строка] соответствует columns[i]. */
  values: (number | null)[][];
}

export interface ImportedLog {
  id: string;
  name: string;
  log: ParsedLog;
}

export const api = {
  connectWifi: (host: string, port: number, protocol: number | null) =>
    invoke<VehicleInfo>("connect_wifi", { host, port, protocol }),
  connectSerial: (path: string, baud: number, protocol: number | null) =>
    invoke<VehicleInfo>("connect_serial", { path, baud, protocol }),
  connectBt: (address: string, protocol: number | null) =>
    invoke<VehicleInfo>("connect_bt", { address, protocol }),
  listBtDevices: () => invoke<BtDevice[]>("plugin:btspp|list_paired"),
  listSerialPorts: () => invoke<string[]>("list_serial_ports"),
  disconnect: () => invoke<void>("disconnect"),
  vehicleInfo: () => invoke<VehicleInfo | null>("vehicle_info"),
  availablePids: () => invoke<PidInfo[]>("available_pids"),
  readDtcs: () => invoke<DtcReport>("read_dtcs"),
  clearDtcs: () => invoke<void>("clear_dtcs"),
  readReadiness: () => invoke<ReadinessItem[]>("read_readiness"),
  readFreezeFrame: () => invoke<FreezeFrame>("read_freeze_frame"),
  startMonitoring: (keys: string[]) => invoke<void>("start_monitoring", { keys }),
  stopMonitoring: () => invoke<void>("stop_monitoring"),
  startLogging: (keys: string[]) => invoke<string>("start_logging", { keys }),
  stopLogging: () => invoke<LogSummary | null>("stop_logging"),
  getExportSettings: () => invoke<ExportSettings>("get_export_settings"),
  setExportFormat: (format: string) =>
    invoke<ExportSettings>("set_export_format", { format }),
  pickExportDir: () => invoke<ExportSettings | null>("pick_export_dir"),
  resetExportDir: () => invoke<ExportSettings>("reset_export_dir"),
  listLogs: () => invoke<LogFileInfo[]>("list_logs"),
  readLog: (id: string, name: string) => invoke<ParsedLog>("read_log", { id, name }),
  importLog: () => invoke<ImportedLog | null>("import_log"),
};

/** Подписка на поток живых данных. Возвращает функцию отписки. */
export function onSample(cb: (s: Sample) => void): Promise<() => void> {
  return listen<Sample>("obd-sample", (e) => {
    cb(e.payload);
    if (e.payload.key === "maf") {
      // Только приближение для бензина при λ≈1. Не учитывает обогащение
      // и отсечку впрыска; STFT/LTFT не являются прямым измерением расхода.
      cb({ ...e.payload, key: "fuel_rate_maf", name: "Расход (оценка по MAF)",
        unit: "л/ч", value: e.payload.value * 3600 / (14.7 * 720) });
    }
  });
}
