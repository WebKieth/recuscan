import { useState } from "react";
import { api, VehicleInfo } from "../api";

interface Props {
  connected: boolean;
  onConnected: (v: VehicleInfo) => void;
  onDisconnected: () => void;
}

export function ConnectionPanel({ connected, onConnected, onDisconnected }: Props) {
  // Стандартный адрес большинства Wi-Fi ELM327 адаптеров
  const [host, setHost] = useState("192.168.0.10");
  const [port, setPort] = useState(35000);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function connect() {
    setBusy(true);
    setError(null);
    try {
      const info = await api.connectWifi(host, port);
      onConnected(info);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function disconnect() {
    await api.disconnect().catch(() => {});
    onDisconnected();
  }

  return (
    <section className="card">
      <h2>Подключение к адаптеру</h2>
      <p className="hint">
        Wi-Fi адаптер: подключитесь к его сети (обычно «WiFi_OBDII»), затем нажмите
        «Подключить». Bluetooth SPP и USB — в разработке.
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
      <div className="form-row">
        {connected ? (
          <button onClick={disconnect}>Отключиться</button>
        ) : (
          <button className="primary" onClick={connect} disabled={busy}>
            {busy ? "Подключение…" : "Подключить"}
          </button>
        )}
      </div>
      {error && <p className="error">Ошибка: {error}</p>}
    </section>
  );
}
