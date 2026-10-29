import { useState } from "react";
import { api, VehicleInfo } from "../api";

interface Props {
  connected: boolean;
  onConnected: (v: VehicleInfo) => void;
  onDisconnected: () => void;
}

type Mode = "wifi" | "usb" | "bt";

export function ConnectionPanel({ connected, onConnected, onDisconnected }: Props) {
  const [mode, setMode] = useState<Mode>("wifi");
  // Стандартный адрес большинства Wi-Fi ELM327 адаптеров
  const [host, setHost] = useState("192.168.0.10");
  const [port, setPort] = useState(35000);
  const [serialPath, setSerialPath] = useState("");
  const [baud, setBaud] = useState(38400);
  const [serialPorts, setSerialPorts] = useState<string[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function connect() {
    setBusy(true);
    setError(null);
    try {
      const info =
        mode === "wifi"
          ? await api.connectWifi(host, port)
          : await api.connectSerial(serialPath, baud);
      onConnected(info);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function refreshPorts() {
    setSerialPorts(await api.listSerialPorts().catch(() => []));
  }

  async function disconnect() {
    await api.disconnect().catch(() => {});
    onDisconnected();
  }

  const MODES: Record<Mode, string> = { wifi: "Wi-Fi", usb: "USB", bt: "Bluetooth" };

  return (
    <section className="card">
      <h2>Подключение к адаптеру</h2>
      <div className="form-row">
        {(Object.keys(MODES) as Mode[]).map((m) => (
          <button
            key={m}
            className={mode === m ? "active" : ""}
            onClick={() => setMode(m)}
            disabled={connected}
          >
            {MODES[m]}
          </button>
        ))}
      </div>

      {mode === "wifi" && (
        <>
          <p className="hint">
            Подключитесь к Wi-Fi сети адаптера (обычно «WiFi_OBDII»), затем нажмите «Подключить».
          </p>
          <div className="form-row">
            <label>
              Хост
              <input value={host} onChange={(e) => setHost(e.target.value)} disabled={connected || busy} />
            </label>
            <label>
              Порт
              <input
                type="number"
                value={port}
                onChange={(e) => setPort(Number(e.target.value))}
                disabled={connected || busy}
              />
            </label>
          </div>
        </>
      )}

      {mode === "usb" && (
        <>
          <p className="hint">Проводной USB/Serial адаптер. Типичный бодрейт — 38400.</p>
          <div className="form-row">
            <label>
              Порт
              <select
                value={serialPath}
                onChange={(e) => setSerialPath(e.target.value)}
                onFocus={() => serialPorts === null && refreshPorts()}
                disabled={connected || busy}
              >
                <option value="">— выбрать —</option>
                {(serialPorts ?? []).map((p) => (
                  <option key={p} value={p}>{p}</option>
                ))}
              </select>
            </label>
            <label>
              Бодрейт
              <input
                type="number"
                value={baud}
                onChange={(e) => setBaud(Number(e.target.value))}
                disabled={connected || busy}
              />
            </label>
            <button onClick={refreshPorts} disabled={connected || busy}>
              Обновить
            </button>
          </div>
        </>
      )}

      {mode === "bt" && (
        <p className="hint">
          Bluetooth Classic SPP поддерживается на Android (сопряжение через настройки системы).
          На iOS SPP заблокирован Apple — используйте Wi-Fi адаптер.
        </p>
      )}

      <div className="form-row">
        {connected ? (
          <button onClick={disconnect}>Отключиться</button>
        ) : (
          <button
            className="primary"
            onClick={connect}
            disabled={busy || mode === "bt" || (mode === "usb" && !serialPath)}
          >
            {busy ? "Подключение…" : "Подключить"}
          </button>
        )}
      </div>
      {error && <p className="error">Ошибка: {error}</p>}
    </section>
  );
}
