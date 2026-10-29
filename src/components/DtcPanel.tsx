import { useState } from "react";
import { api, DtcReport } from "../api";

export function DtcPanel({ connected }: { connected: boolean }) {
  const [report, setReport] = useState<DtcReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function read() {
    setBusy(true);
    setError(null);
    try {
      setReport(await api.readDtcs());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function clear() {
    if (!confirm("Сбросить все коды неисправностей в ЭБУ?")) return;
    setBusy(true);
    setError(null);
    try {
      await api.clearDtcs();
      await read();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  if (!connected) return <p className="hint">Сначала подключитесь к адаптеру на вкладке «Подключение».</p>;

  const section = (title: string, codes: string[]) => (
    <div className="dtc-section">
      <h3>
        {title} <span className="badge">{codes.length}</span>
      </h3>
      {codes.length === 0 ? (
        <p className="hint">нет</p>
      ) : (
        <ul>
          {codes.map((c) => (
            <li key={c} className="dtc-code">{c}</li>
          ))}
        </ul>
      )}
    </div>
  );

  return (
    <section className="card">
      <h2>Коды неисправностей (DTC)</h2>
      <div className="form-row">
        <button className="primary" onClick={read} disabled={busy}>
          {busy ? "Чтение…" : "Прочитать ошибки"}
        </button>
        <button onClick={clear} disabled={busy || !report}>
          Сбросить ошибки
        </button>
      </div>
      {error && <p className="error">Ошибка: {error}</p>}
      {report && (
        <>
          {section("Сохранённые (Mode 03)", report.stored)}
          {section("Ожидающие (Mode 07)", report.pending)}
          {section("Постоянные (Mode 0A)", report.permanent)}
        </>
      )}
    </section>
  );
}
