import { useEffect, useMemo, useState } from "react";
import { api, PidInfo } from "../api";
import { LiveChart } from "./LiveChart";

const DEFAULTS = ["rpm", "speed", "stft1", "maf"];

export function Dashboard({ connected }: { connected: boolean }) {
  const [pids, setPids] = useState<PidInfo[]>([]);
  const [selected, setSelected] = useState<string[]>(DEFAULTS);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (connected) {
      api.availablePids().then(setPids).catch(() => setPids([]));
    }
  }, [connected]);

  const meta = useMemo(() => {
    const m: Record<string, PidInfo> = {
      fuel_consumption: { key: "fuel_consumption", name: "Расход топлива (оценка по MAF)", unit: "л/100км" },
    };
    for (const p of pids) m[p.key] = p;
    return m;
  }, [pids]);

  const chartKeys = useMemo(() => {
    const keys = selected.filter((k) => meta[k]);
    // виртуальный PID мгновенного расхода: нужен MAF и скорость
    if (selected.includes("maf") && selected.includes("speed")) keys.push("fuel_consumption");
    return keys;
  }, [selected, meta]);

  function toggle(key: string) {
    setSelected((s) => (s.includes(key) ? s.filter((k) => k !== key) : [...s, key]));
  }

  async function start() {
    setError(null);
    try {
      await api.startMonitoring(selected);
      setRunning(true);
    } catch (e) {
      setError(String(e));
    }
  }

  async function stop() {
    await api.stopMonitoring().catch(() => {});
    setRunning(false);
  }

  if (!connected) return <p className="hint">Сначала подключитесь к адаптеру на вкладке «Подключение».</p>;

  return (
    <section>
      <div className="card">
        <h2>Параметры мониторинга</h2>
        <div className="pid-picker">
          {pids.map((p) => (
            <label key={p.key} className={selected.includes(p.key) ? "chip on" : "chip"}>
              <input type="checkbox" checked={selected.includes(p.key)} onChange={() => toggle(p.key)} />
              {p.name}
            </label>
          ))}
        </div>
        <div className="form-row">
          {running ? (
            <button onClick={stop}>Остановить</button>
          ) : (
            <button className="primary" onClick={start} disabled={selected.length === 0}>
              Старт мониторинга
            </button>
          )}
        </div>
        {error && <p className="error">Ошибка: {error}</p>}
      </div>
      <div className="charts">
        {chartKeys.map((k) => (
          <LiveChart key={k} sampleKey={k} title={meta[k].name} unit={meta[k].unit} />
        ))}
      </div>
    </section>
  );
}
