import { useState } from "react";
import { api, DtcEntry, DtcReport, FreezeFrame, ReadinessItem } from "../api";

export function DtcPanel({ connected }: { connected: boolean }) {
  const [report, setReport] = useState<DtcReport | null>(null);
  const [readiness, setReadiness] = useState<ReadinessItem[] | null>(null);
  const [freezeFrame, setFreezeFrame] = useState<FreezeFrame | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function read() {
    setBusy(true);
    setError(null);
    try {
      setReport(await api.readDtcs());
      // Готовность мониторов подтягиваем вместе — важно для техосмотра
      setReadiness(await api.readReadiness().catch(() => null));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function loadFreezeFrame() {
    setBusy(true);
    setError(null);
    try {
      setFreezeFrame(await api.readFreezeFrame());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function clear() {
    const ok = confirm(
      "Сбросить коды неисправностей?\n\n" +
        "Рекомендуется: двигатель заглушен, зажигание включено.\n" +
        "Прошивка ЭБУ не затрагивается — стирается только диагностическая память.\n" +
        "Внимание: также сбросятся readiness-мониторы (нужен drive cycle для техосмотра)."
    );
    if (!ok) return;
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

  if (!connected) {
    return <p className="hint">Сначала подключитесь к адаптеру на вкладке «Подключение».</p>;
  }

  const section = (title: string, entries: DtcEntry[]) => (
    <div className="dtc-section">
      <h3>
        {title} <span className="badge">{entries.length}</span>
      </h3>
      {entries.length === 0 ? (
        <p className="hint">нет</p>
      ) : (
        <ul>
          {entries.map((e) => (
            <li key={e.code} className="dtc-item">
              <span className="dtc-code">{e.code}</span>
              <span className="dtc-desc">
                {e.description ?? "Описание не найдено (вероятно, код специфичен для производителя)"}
              </span>
            </li>
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
        <button onClick={loadFreezeFrame} disabled={busy || !report}>
          Freeze frame
        </button>
        {report?.mil_on != null && (
          <span className={report.mil_on ? "status mil-on" : "status ok"}>
            Check Engine: {report.mil_on ? "ГОРИТ" : "не горит"}
            {report.mil_dtc_count != null && ` · кодов по ЭБУ: ${report.mil_dtc_count}`}
          </span>
        )}
      </div>
      {error && <p className="error">Ошибка: {error}</p>}
      {report && (
        <>
          {section("Сохранённые (Mode 03)", report.stored)}
          {section("Ожидающие (Mode 07)", report.pending)}
          {section("Постоянные (Mode 0A)", report.permanent)}
          {report.permanent.length > 0 && (
            <p className="hint">
              Постоянные коды нельзя стереть вручную — ЭБУ удалит их сам после нескольких
              поездок, в которых неисправность больше не проявляется.
            </p>
          )}
        </>
      )}
      {readiness && (
        <div className="dtc-section">
          <h3>Готовность мониторов (важно для техосмотра)</h3>
          <table className="info-table">
            <tbody>
              {readiness.map((r) => (
                <tr key={r.name}>
                  <td>{r.name}</td>
                  <td className={r.complete ? "status ok" : "status mil-on"}>
                    {r.complete ? "✓ готов" : "✗ не завершён"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {freezeFrame && (
        <div className="dtc-section">
          <h3>
            Freeze frame — снимок в момент ошибки{" "}
            {freezeFrame.dtc && <span className="dtc-code">{freezeFrame.dtc}</span>}
          </h3>
          {freezeFrame.description && <p>{freezeFrame.description}</p>}
          <table className="info-table">
            <tbody>
              {freezeFrame.entries.map((e) => (
                <tr key={e.key}>
                  <td>{e.name}</td>
                  <td>
                    {e.value.toFixed(1)} {e.unit}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
