import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface VehicleInfo {
  vin: string | null;
  manufacturer: string | null;
  protocol: string | null;
  calibration_id: string | null;
  supported_pids: string[];
}

export interface DtcReport {
  stored: string[];
  pending: string[];
  permanent: string[];
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

export const api = {
  connectWifi: (host: string, port: number) =>
    invoke<VehicleInfo>("connect_wifi", { host, port }),
  disconnect: () => invoke<void>("disconnect"),
  vehicleInfo: () => invoke<VehicleInfo | null>("vehicle_info"),
  availablePids: () => invoke<PidInfo[]>("available_pids"),
  readDtcs: () => invoke<DtcReport>("read_dtcs"),
  clearDtcs: () => invoke<void>("clear_dtcs"),
  startMonitoring: (keys: string[]) => invoke<void>("start_monitoring", { keys }),
  stopMonitoring: () => invoke<void>("stop_monitoring"),
};

/** Подписка на поток живых данных. Возвращает функцию отписки. */
export function onSample(cb: (s: Sample) => void): Promise<() => void> {
  return listen<Sample>("obd-sample", (e) => cb(e.payload));
}
