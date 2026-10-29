import { useState } from "react";
import { api, BtDevice, VehicleInfo } from "../api";

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
  const [btAddress, setBtAddress] = useState("");
  const [btDevices, setBtDevices] = useState<BtDevice[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function connect() {
    setBusy(true);
    setError(null);
    try {
      const info =
        mode === "wifi"
          ? await api.connectWifi(host, port)
          : mode === "usb"
            ? await api.connectSerial(serialPath, baud)
            : await api.connectBt(btAddress);
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

  async function refreshBt() {
    setError(null);
    try {
      setBtDevices(await api.listBtDevices());
    } catch (e) {
      setError(String(e));
      setBtDevices([]);
    }
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
        <>
          <p className="hint">
            Сначала сопрягите адаптер в настройках Bluetooth Android (обычно имя «OBDII»,
            PIN 1234 или 0000), затем выберите его ниже. iOS не поддерживает Bluetooth SPP.
          </p>
          <div className="form-row">
            <label>
              Устройство
              <select
                value={btAddress}
                onChange={(e) => setBtAddress(e.target.value)}
                onFocus={() => btDevices === null && refreshBt()}
                disabled={connected || busy}
              >
                <option value="">— выбрать —</option>
                {(btDevices ?? []).map((d) => (
                  <option key={d.address} value={d.address}>
                    {d.name} ({d.address})
                  </option>
                ))}
              </select>
            </label>
            <button onClick={refreshBt} disabled={connected || busy}>
              Обновить
            </button>
          </div>
        </>
      )}

      <div className="form-row">
        {connected ? (
          <button onClick={disconnect}>Отключиться</button>
        ) : (
          <button
            className="primary"
            onClick={connect}
            disabled={busy || (mode === "usb" && !serialPath) || (mode === "bt" && !btAddress)}
          >
            {busy ? "Подключение…" : "Подключить"}
          </button>
        )}
      </div>
      {error && <p className="error">Ошибка: {error}</p>}
    </section>
  );
}
