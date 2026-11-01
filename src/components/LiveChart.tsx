import { useEffect, useRef, useState } from "react";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import { onSample, PidInfo } from "../api";

export const CHART_COLORS = ["#4fc3f7", "#aed581", "#ffb74d", "#f06292"];
const WINDOW_SEC = 60;

interface Props {
  series: PidInfo[];
  title: string;
  onEdit?: () => void;
  onRemove?: () => void;
}

export function LiveChart({ series, title, onEdit, onRemove }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [current, setCurrent] = useState<Record<string, number>>({});

  useEffect(() => {
    const el = containerRef.current;
    if (!el || !series.length) return;
    setCurrent({});
    const units = [...new Set(series.map((s) => s.unit))];
    const scaleKey = (unit: string) => `unit${units.indexOf(unit)}`;
    const data: (number | null)[][] = Array.from({ length: series.length + 1 }, () => []);
    let active = true;
    let unsubscribe: (() => void) | undefined;
    let dirty = false;
    let latest: Record<string, number> = {};
    const chart = new uPlot({
      width: el.clientWidth || 560,
      height: 240,
      legend: { show: false },
      scales: Object.fromEntries([
        ["x", { time: true }],
        ...units.map((unit) => [scaleKey(unit), { auto: true }]),
      ]),
      axes: [
        { stroke: "#9aa4b2", grid: { show: false } },
        ...units.map((unit, i) => ({
          scale: scaleKey(unit), label: unit, size: 48, labelSize: 18,
          side: i % 2 === 0 ? 3 : 1,
          stroke: CHART_COLORS[series.findIndex((s) => s.unit === unit)],
          grid: { show: i === 0, stroke: "#2a2f3a" },
        })),
      ],
      series: [{}, ...series.map((s, i) => ({
        label: s.name, scale: scaleKey(s.unit), stroke: CHART_COLORS[i],
        width: 2, spanGaps: true,
        value: (_u: uPlot, value: number | null) => value == null ? "—" : `${value.toFixed(1)} ${s.unit}`,
      }))],
    }, data as uPlot.AlignedData, el);

    const unlisten = onSample((s) => {
      if (!active || !Number.isFinite(s.value) || !Number.isFinite(s.ts)) return;
      const index = series.findIndex((p) => p.key === s.key);
      if (index < 0) return;
      const ts = s.ts / 1000;
      const xs = data[0] as number[];
      // ELM327 опрашивает PID последовательно: не выдаём старое значение за новый замер.
      if (xs.length && ts < xs[xs.length - 1]) return;
      if (!xs.length || ts !== xs[xs.length - 1]) {
        xs.push(ts);
        for (let i = 1; i < data.length; i++) data[i].push(null);
      }
      data[index + 1][xs.length - 1] = s.value;
      let remove = 0;
      while (remove < xs.length && xs[remove] < ts - WINDOW_SEC) remove++;
      if (remove) data.forEach((column) => column.splice(0, remove));
      latest = { ...latest, [s.key]: s.value };
      dirty = true;
    });
    // Обрабатываем также отказ подписки и размонтирование до её завершения.
    void unlisten.then((off) => { if (!active) off(); else unsubscribe = off; }).catch(() => {});
    const timer = setInterval(() => {
      if (!dirty) return;
      chart.setData(data as uPlot.AlignedData);
      setCurrent(latest);
      dirty = false;
    }, 400);
    const resize = new ResizeObserver(() => {
      if (el.clientWidth) chart.setSize({ width: el.clientWidth, height: 240 });
    });
    resize.observe(el);
    return () => {
      active = false;
      unsubscribe?.();
      clearInterval(timer);
      resize.disconnect();
      chart.destroy();
    };
  }, [series]);

  return (
    <div className="card chart-card">
      <div className="chart-head">
        <strong>{title}</strong>
        <div className="chart-actions">
          {onEdit && <button onClick={onEdit}>Изменить</button>}
          {onRemove && <button onClick={onRemove} aria-label={`Удалить график «${title}»`}>×</button>}
        </div>
      </div>
      <div className="chart-legend">
        {series.map((s, i) => (
          <span key={s.key} style={{ color: CHART_COLORS[i] }}>
            ● {s.name}: <b>{current[s.key] === undefined ? "—" : current[s.key].toFixed(1)} {s.unit}</b>
          </span>
        ))}
      </div>
      <div className="plot-container" ref={containerRef} />
      <p className="hint">Время · последние {WINDOW_SEC} с · шкалы по единицам измерения</p>
    </div>
  );
}
