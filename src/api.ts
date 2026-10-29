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

export const api = {
  connectWifi: (host: string, port: number) =>
    invoke<VehicleInfo>("connect_wifi", { host, port }),
  connectSerial: (path: string, baud: number) =>
    invoke<VehicleInfo>("connect_serial", { path, baud }),
  connectBt: (address: string) =>
    invoke<VehicleInfo>("connect_bt", { address }),
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
};

/** Подписка на поток живых данных. Возвращает функцию отписки. */
export function onSample(cb: (s: Sample) => void): Promise<() => void> {
  return listen<Sample>("obd-sample", (e) => cb(e.payload));
}
