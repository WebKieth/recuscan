import { useCallback, useEffect, useMemo, useState } from "react";
import { api, ImportedLog, LogFileInfo } from "../api";
import { ChartConfig, loadCharts, LOG_CHART_STORAGE } from "../charts";
import { CHART_COLORS } from "./LiveChart";
import { LogChart } from "./LogChart";

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} Б`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} КБ`;
  return `${(bytes / 1024 / 1024).toFixed(1)} МБ`;
}

function formatDuration(ms: number): string {
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s} с`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} мин ${s % 60} с`;
  return `${Math.floor(m / 60)} ч ${m % 60} мин`;
}

export function LogAnalysis() {
  const [files, setFiles] = useState<LogFileInfo[] | null>(null);
  const [listError, setListError] = useState<string | null>(null);
  const [current, setCurrent] = useState<ImportedLog | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [charts, setCharts] = useState<ChartConfig[]>(() => loadCharts(LOG_CHART_STORAGE));
  const [draft, setDraft] = useState<ChartConfig | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(() => {
    setListError(null);
    api.listLogs().then(setFiles).catch((e) => {
      setFiles([]);
      setListError(String(e));
    });
  }, []);

  useEffect(refresh, [refresh]);

  useEffect(() => {
    try { localStorage.setItem(LOG_CHART_STORAGE, JSON.stringify(charts)); }
    catch { setError("Не удалось сохранить графики в локальное хранилище."); }
  }, [charts]);

  const log = current?.log ?? null;
  const meta = useMemo(
    () => Object.fromEntries((log?.columns ?? []).map((c) => [c.key, c])),
    [log],
  );
  const combined = useMemo(
    () => charts.map((c) => ({ ...c, series: c.keys.filter((k) => meta[k]).map((k) => meta[k]) })),
    [charts, meta],
  );
  const singles = useMemo(
    () => selected.filter((k) => meta[k]).map((k) => [meta[k]]),
    [selected, meta],
  );

  /** Ключи колонок, которые есть в загруженном логе и отсутствуют в данных, не показываем. */
  const xs = useMemo(() => (log ? log.ts_ms.map((t) => t / 1000) : []), [log]);
  const columnValues = useMemo(() => {
    if (!log) return {} as Record<string, (number | null)[]>;
    return Object.fromEntries(log.columns.map((c, i) => [c.key, log.values[i]]));
  }, [log]);

  function openLog(id: string, name: string) {
    setBusy(true);
    setError(null);
    api.readLog(id, name)
      .then((parsed) => {
        setCurrent({ id, name, log: parsed });
        // Стартовый набор: обороты+скорость, если есть; иначе первая колонка
        const keys = parsed.columns.map((c) => c.key);
        const defaults = ["rpm", "speed"].filter((k) => keys.includes(k));
        setSelected(defaults.length ? defaults : keys.slice(0, 1));
      })
      .catch((e) => setError(String(e)))
      .finally(() => setBusy(false));
  }

  async function importFile() {
    setBusy(true);
    setError(null);
    try {
      const imported = await api.importLog();
      if (imported) {
        setCurrent(imported);
        const keys = imported.log.columns.map((c) => c.key);
        const defaults = ["rpm", "speed"].filter((k) => keys.includes(k));
        setSelected(defaults.length ? defaults : keys.slice(0, 1));
      }
    }
    catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  }

  function toggle(key: string) {
    setSelected((s) => s.includes(key) ? s.filter((k) => k !== key) : [...s, key]);
  }

  function create(title = "Сводный график", presetKeys: string[] = []) {
    setDraft({ id: crypto.randomUUID(), title, keys: presetKeys });
  }

  function save() {
    if (!draft || !draft.title.trim() || !draft.keys.length) return;
    const config = { ...draft, title: draft.title.trim() };
    setCharts((prev) => prev.some((c) => c.id === config.id)
      ? prev.map((c) => c.id === config.id ? config : c) : [...prev, config]);
    setDraft(null);
  }

  const duration = log && log.ts_ms.length > 1
    ? log.ts_ms[log.ts_ms.length - 1] - log.ts_ms[0]
    : 0;

  return (
    <section>
      <div className="card">
        <h2>Источник лога</h2>
        <div className="form-row">
          <button className="primary" disabled={busy} onClick={importFile}>
            Импорт файла…
          </button>
          <button disabled={busy} onClick={refresh}>Обновить список</button>
          <span className="hint">Логи из папки экспорта по умолчанию (CSV, JSON).</span>
        </div>
        {listError && <p role="alert" className="error">Список логов: {listError}</p>}
        {files && files.length === 0 && !listError && (
          <p className="hint">Записанных логов пока нет. Включите запись на вкладке «Мониторинг».</p>
        )}
        {files && files.length > 0 && (
          <ul className="log-list">
            {files.map((f) => (
              <li key={f.id}>
                <button
                  className={current?.id === f.id ? "log-item active" : "log-item"}
                  disabled={busy}
                  onClick={() => openLog(f.id, f.name)}
                >
                  <span className="log-name">{f.name}</span>
                  <span className="log-meta">
                    {new Date(f.mtime_ms).toLocaleString("ru-RU")} · {formatSize(f.size)}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}
        {error && <p role="alert" className="error">Ошибка: {error}</p>}
      </div>

      {log && current && (
        <>
          <div className="card">
            <h2>{current.name}</h2>
            <table className="info-table">
              <tbody>
                <tr><td>Строк данных</td><td>{log.ts_ms.length}</td></tr>
                <tr><td>Период</td><td>
                  {new Date(log.ts_ms[0]).toLocaleString("ru-RU")} — {new Date(log.ts_ms[log.ts_ms.length - 1]).toLocaleTimeString("ru-RU")}
                </td></tr>
                <tr><td>Длительность</td><td>{formatDuration(duration)}</td></tr>
                <tr><td>Параметров</td><td>{log.columns.length}</td></tr>
              </tbody>
            </table>

            <h3>Отдельные графики</h3>
            <div className="pid-picker">
              {log.columns.map((c) => (
                <label key={c.key} className={selected.includes(c.key) ? "chip on" : "chip"}>
                  <input type="checkbox" checked={selected.includes(c.key)} onChange={() => toggle(c.key)} />
                  {c.name}{c.unit ? ` (${c.unit})` : ""}
                </label>
              ))}
            </div>
          </div>

          <div className="card">
            <h2>Конструктор сводных графиков</h2>
            <p className="hint">До 4 линий и 2 единиц измерения: левая и правая оси Y. Общая ось X — время. Наборы сохраняются на этом устройстве.</p>
            <button onClick={() => create()}>+ Создать график</button>
            {draft && (
              <div className="chart-builder">
                <div className="form-row">
                  <label>Название
                    <input maxLength={100} value={draft.title}
                      onChange={(e) => setDraft({ ...draft, title: e.target.value })} />
                  </label>
                </div>
                <div className="pid-picker">
                  {log.columns.map((c) => {
                    const index = draft.keys.indexOf(c.key);
                    const units = new Set(draft.keys.map((k) => meta[k]?.unit).filter(Boolean));
                    const disabled = index < 0 && (draft.keys.length >= 4 || (units.size >= 2 && !units.has(c.unit)));
                    return (
                      <label key={c.key} className={index >= 0 ? "chip on" : "chip"}
                        style={index >= 0 ? { color: CHART_COLORS[index], borderColor: CHART_COLORS[index] } : undefined}>
                        <input type="checkbox" checked={index >= 0} disabled={disabled}
                          onChange={() => setDraft({
                            ...draft,
                            keys: index >= 0 ? draft.keys.filter((k) => k !== c.key) : [...draft.keys, c.key],
                          })} />
                        {c.name}{c.unit ? ` (${c.unit})` : ""}
                      </label>
                    );
                  })}
                </div>
                <p className="hint">Выбрано: {draft.keys.length}/4. Первая единица — левая шкала, вторая — правая.</p>
                <div className="form-row">
                  <button className="primary" disabled={!draft.title.trim() || !draft.keys.length} onClick={save}>
                    Сохранить график
                  </button>
                  <button onClick={() => setDraft(null)}>Отмена</button>
                </div>
              </div>
            )}
          </div>

          <div className="charts">
            {combined.map((c) => (
              <div key={c.id}>
                {c.series.length > 0 && (
                  <LogChart title={c.title} series={c.series} xs={xs}
                    ys={c.series.map((s) => columnValues[s.key])}
                    onEdit={() => setDraft({ id: c.id, title: c.title, keys: c.keys })}
                    onRemove={() => setCharts((prev) => prev.filter((item) => item.id !== c.id))} />
                )}
                {c.series.length !== c.keys.length && (
                  <p className="hint">Часть параметров набора «{c.title}» отсутствует в этом логе.</p>
                )}
              </div>
            ))}
            {singles.map((series) => (
              <LogChart key={series[0].key} title={series[0].name} series={series} xs={xs}
                ys={series.map((s) => columnValues[s.key])} />
            ))}
          </div>
        </>
      )}
      {!log && (
        <p className="hint">Выберите лог из списка выше или импортируйте файл.</p>
      )}
    </section>
  );
}
