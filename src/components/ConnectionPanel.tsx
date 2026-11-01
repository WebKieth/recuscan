import { useState } from "react";
import { api, BtDevice, VehicleInfo } from "../api";

interface Props {
  connected: boolean;
  onConnected: (v: VehicleInfo) => void;
  onDisconnected: () => void;
}

type Mode = "wifi" | "usb" | "bt";

/** Протоколы ELM327 (ATSP). null — авто с последующим перебором. */
const PROTOCOLS: { value: number | null; label: string }[] = [
  { value: null, label: "Авто (перебор протоколов)" },
  { value: 3, label: "ISO 9141-2 (K-line; VW/VAZ ~1996–2005)" },
  { value: 4, label: "ISO 14230-4 KWP2000 (5-бод инициализация)" },
  { value: 5, label: "ISO 14230-4 KWP2000 (быстрая инициализация)" },
  { value: 6, label: "ISO 15765-4 CAN (11 бит, 500 кбит)" },
  { value: 7, label: "ISO 15765-4 CAN (29 бит, 500 кбит)" },
  { value: 8, label: "ISO 15765-4 CAN (11 бит, 250 кбит)" },
  { value: 9, label: "ISO 15765-4 CAN (29 бит, 250 кбит)" },
];

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
  const [protocol, setProtocol] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function connect() {
    setBusy(true);
    setError(null);
    try {
      const info =
        mode === "wifi"
          ? await api.connectWifi(host, port, protocol)
          : mode === "usb"
            ? await api.connectSerial(serialPath, baud, protocol)
            : await api.connectBt(btAddress, protocol);
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
        <label>
          Протокол
          <select
            value={protocol === null ? "" : String(protocol)}
            onChange={(e) => setProtocol(e.target.value === "" ? null : Number(e.target.value))}
            disabled={connected || busy}
          >
            {PROTOCOLS.map((p) => (
              <option key={String(p.value)} value={p.value === null ? "" : p.value}>
                {p.label}
              </option>
            ))}
          </select>
        </label>
      </div>
      <p className="hint">
        Если «Авто» не подключается к старому авто (K-line): выберите ISO 9141-2, при неудаче —
        KWP2000. Зажигание должно быть включено.
      </p>

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
