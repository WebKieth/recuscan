import { useEffect, useMemo, useState } from "react";
import { api, ExportSettings, PidInfo } from "../api";
import { ChartConfig, CHART_STORAGE, FUEL_PID, loadCharts, monitoringKeys, PRESETS } from "../charts";
import { CHART_COLORS, LiveChart } from "./LiveChart";

const DEFAULTS = ["rpm", "speed", "stft1", "maf"];

const FORMATS: { id: string; label: string }[] = [
  { id: "csv", label: "CSV" },
  { id: "json", label: "JSON" },
  { id: "xls", label: "XLS (Excel)" },
];

export function Dashboard({ connected }: { connected: boolean }) {
  const [pids, setPids] = useState<PidInfo[]>([]);
  const [selected, setSelected] = useState<string[]>(DEFAULTS);
  const [charts, setCharts] = useState<ChartConfig[]>(() => loadCharts());
  const [draft, setDraft] = useState<ChartConfig | null>(null);
  const [running, setRunning] = useState(false);
  const [recording, setRecording] = useState(false);
  const [logInfo, setLogInfo] = useState<string | null>(null);
  const [exportSettings, setExportSettings] = useState<ExportSettings | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!connected) { setPids([]); setRunning(false); return; }
    let active = true;
    api.availablePids().then((result) => { if (active) setPids(result); })
      .catch((e) => { if (active) setError(String(e)); });
    return () => { active = false; };
  }, [connected]);

  useEffect(() => {
    try { localStorage.setItem(CHART_STORAGE, JSON.stringify(charts)); }
    catch { setError("Не удалось сохранить графики в локальное хранилище."); }
  }, [charts]);

  useEffect(() => {
    api.getExportSettings().then(setExportSettings).catch(() => {});
  }, []);

  // Мониторинг принадлежит экрану: при уходе не оставляем скрытый опрос адаптера.
  useEffect(() => () => {
    void api.stopMonitoring().catch(() => {});
    void api.stopLogging().catch(() => {});
  }, []);

  const available = useMemo(() => pids.some((p) => p.key === "maf") ? [...pids, FUEL_PID] : pids, [pids]);
  const meta = useMemo(() => Object.fromEntries(available.map((p) => [p.key, p])), [available]);
  const keys = useMemo(() => monitoringKeys([
    ...selected, ...charts.flatMap((c) => c.keys),
  ], pids), [selected, charts, pids]);
  const singles = useMemo(() => selected.filter((k) => meta[k]).map((k) => [meta[k]]), [selected, meta]);
  const combined = useMemo(() => charts.map((c) => ({
    ...c, series: c.keys.filter((k) => meta[k]).map((k) => meta[k]),
  })), [charts, meta]);

  function toggle(key: string) {
    setSelected((s) => s.includes(key) ? s.filter((k) => k !== key) : [...s, key]);
  }

  async function start() {
    setBusy(true); setError(null);
    try { await api.startMonitoring(keys); setRunning(true); }
    catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  }

  async function stop() {
    setBusy(true);
    try {
      await api.stopMonitoring();
      setRunning(false);
      if (recording) {
        const summary = await api.stopLogging();
        setRecording(false);
        setLogInfo(summary ? `Лог сохранён (${summary.rows} строк): ${summary.path}` : null);
      }
    }
    catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  }

  async function toggleRecording() {
    setError(null);
    try {
      if (!recording) {
        const name = await api.startLogging(keys);
        setRecording(true);
        setLogInfo(`Запись: ${name} → ${exportSettings?.dir_label ?? "…"}`);
      } else {
        const summary = await api.stopLogging();
        setRecording(false);
        setLogInfo(summary ? `Лог сохранён (${summary.rows} строк): ${summary.path}` : null);
      }
    }
    catch (e) { setError(String(e)); }
  }

  async function setFormat(format: string) {
    try { setExportSettings(await api.setExportFormat(format)); }
    catch (e) { setError(String(e)); }
  }

  async function pickDir() {
    try {
      const next = await api.pickExportDir();
      if (next) setExportSettings(next);
    }
    catch (e) { setError(String(e)); }
  }

  async function resetDir() {
    try { setExportSettings(await api.resetExportDir()); }
    catch (e) { setError(String(e)); }
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

  if (!connected) return <p className="hint">Сначала подключитесь к адаптеру на вкладке «Подключение».</p>;

  return (
    <section>
      <div className="card">
        <h2>Параметры мониторинга</h2>
        <p className="hint">Отдельные графики. Параметры сводных графиков добавляются в опрос автоматически.</p>
        <div className="pid-picker">
          {available.map((p) => (
            <label key={p.key} className={selected.includes(p.key) ? "chip on" : "chip"}>
              <input type="checkbox" disabled={running || busy} checked={selected.includes(p.key)} onChange={() => toggle(p.key)} />
              {p.name}
            </label>
          ))}
        </div>
        <div className="form-row">
          {running ? <button disabled={busy} onClick={stop}>Остановить</button> :
            <button className="primary" onClick={start} disabled={busy || !keys.length}>Старт мониторинга</button>}
          <button disabled={!running || busy} onClick={toggleRecording}>
            {recording ? "■ Стоп запись" : `● Запись лога${exportSettings ? ` (${exportSettings.format_label})` : ""}`}
          </button>
          <span className="hint">В опросе: {keys.length} PID. Чем их больше, тем ниже частота обновления.</span>
        </div>
        {logInfo && <p className="hint">{logInfo}</p>}
        {error && <p role="alert" className="error">Ошибка: {error}</p>}
      </div>

      <div className="card">
        <h2>Экспорт логов</h2>
        <p className="hint">
          Запись идёт во временный файл и при остановке экспортируется в выбранной папке и формате.
          Открыть записанные логи можно на вкладке «Анализ логов».
        </p>
        <div className="form-row">
          <label>Формат
            <span className="pid-picker" role="group" aria-label="Формат экспорта">
              {FORMATS.map((f) => (
                <button key={f.id}
                  className={exportSettings?.format === f.id ? "chip on" : "chip"}
                  disabled={recording}
                  onClick={() => setFormat(f.id)}>
                  {f.label}
                </button>
              ))}
            </span>
          </label>
        </div>
        <div className="form-row">
          <label>Папка экспорта
            <span className="export-dir">
              <strong>{exportSettings?.dir_label ?? "…"}</strong>
              {exportSettings?.is_default_dir && <span className="badge">по умолчанию</span>}
            </span>
          </label>
          <button disabled={recording} onClick={pickDir}>Сменить…</button>
          {exportSettings && !exportSettings.is_default_dir && (
            <button disabled={recording} onClick={resetDir}>Сбросить</button>
          )}
        </div>
        {exportSettings?.format === "xls" && (
          <p className="hint">XLS — только экспорт: в «Анализе логов» открываются CSV и JSON. Для анализа записывайте CSV или JSON.</p>
        )}
      </div>

      <div className="card">
        <h2>Конструктор сводных графиков</h2>
        <p className="hint">До 4 линий и 2 единиц измерения: левая и правая оси Y. Общая ось X — время. Наборы сохраняются на этом устройстве.</p>
        {running && <p className="hint">Для изменения набора остановите мониторинг.</p>}
        <button disabled={running || busy} onClick={() => create()}>+ Создать график</button>
        <div className="presets">
          {PRESETS.map((preset, index) => {
            const presetKeys = preset.keys.map((k) => k === "fuel_rate" && !meta[k] ? FUEL_PID.key : k);
            const supported = presetKeys.every((k) => meta[k]);
            return <div key={preset.title} className="preset">
              <button disabled={!supported || running || busy} onClick={() => create(preset.title, presetKeys)}>{preset.title}</button>
              <span className="hint">{preset.hint}{!supported && " Не все параметры доступны на этом ЭБУ."}</span>
              {index === 0 && <span className="hint">Оценка MAF предполагает λ≈1: не учитывает обогащение и отсечку топлива, не подходит для дизеля и ГБО.</span>}
            </div>;
          })}
        </div>
        {draft && <div className="chart-builder">
          <div className="form-row"><label>Название<input maxLength={100} value={draft.title} disabled={running} onChange={(e) => setDraft({ ...draft, title: e.target.value })} /></label></div>
          <div className="pid-picker">
            {available.map((p) => {
              const index = draft.keys.indexOf(p.key);
              const units = new Set(draft.keys.map((k) => meta[k]?.unit).filter(Boolean));
              const disabled = running || (index < 0 && (draft.keys.length >= 4 || (units.size >= 2 && !units.has(p.unit))));
              return <label key={p.key} className={index >= 0 ? "chip on" : "chip"} style={index >= 0 ? { color: CHART_COLORS[index], borderColor: CHART_COLORS[index] } : undefined}>
                <input type="checkbox" checked={index >= 0} disabled={disabled} onChange={() => setDraft({ ...draft, keys: index >= 0 ? draft.keys.filter((k) => k !== p.key) : [...draft.keys, p.key] })} />
                {p.name} ({p.unit})
              </label>;
            })}
          </div>
          <p className="hint">Выбрано: {draft.keys.length}/4. Первая единица — левая шкала, вторая — правая.</p>
          {draft.keys.some((k) => !meta[k]) && <p className="error">Некоторые сохранённые параметры недоступны. Удалите их из набора: {draft.keys.filter((k) => !meta[k]).map((k) => <button key={k} onClick={() => setDraft({ ...draft, keys: draft.keys.filter((key) => key !== k) })}>{k} ×</button>)}</p>}
          <div className="form-row">
            <button className="primary" disabled={running || !draft.title.trim() || !draft.keys.length || draft.keys.some((k) => !meta[k])} onClick={save}>Сохранить график</button>
            <button onClick={() => setDraft(null)}>Отмена</button>
          </div>
        </div>}
      </div>

      <div className="charts">
        {combined.map((c) => <div key={c.id}>
          <LiveChart title={c.title} series={c.series}
            onEdit={!running && !busy ? () => setDraft({ id: c.id, title: c.title, keys: c.keys }) : undefined}
            onRemove={!running && !busy ? () => setCharts((prev) => prev.filter((item) => item.id !== c.id)) : undefined} />
          {c.series.length !== c.keys.length && <p className="hint">Часть параметров этого набора недоступна на текущем ЭБУ.</p>}
        </div>)}
        {singles.map((series) => <LiveChart key={series[0].key} title={series[0].name} series={series} />)}
      </div>
    </section>
  );
}
