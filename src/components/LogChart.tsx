import { useEffect, useRef, useState } from "react";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import { LogColumn } from "../api";
import { CHART_COLORS } from "./LiveChart";

interface Props {
  title: string;
  series: LogColumn[];
  /** Ось X, секунды (ts/1000), возрастают. */
  xs: number[];
  /** Значения по сериям: ys[i][точка] соответствует series[i]. */
  ys: (number | null)[][];
  onEdit?: () => void;
  onRemove?: () => void;
}

/**
 * Статический график записанного лога.
 * Навигация: колесо — масштаб вокруг курсора, выделение мышью — приближение,
 * двойной клик / кнопка — сброс, режим «рука» — перетаскивание по времени.
 */
export function LogChart({ title, series, xs, ys, onEdit, onRemove }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<uPlot | null>(null);
  const cleanupRef = useRef<(() => void) | null>(null);
  const fullRangeRef = useRef<{ min: number; max: number }>({ min: 0, max: 1 });
  const panRef = useRef(false);
  const [panMode, setPanMode] = useState(false);
  panRef.current = panMode;

  useEffect(() => {
    const el = containerRef.current;
    if (!el || !series.length || xs.length < 2) return;
    const units = [...new Set(series.map((s) => s.unit))];
    const scaleKey = (unit: string) => `unit${units.indexOf(unit)}`;
    const data = [xs, ...ys] as uPlot.AlignedData;
    fullRangeRef.current = { min: xs[0], max: xs[xs.length - 1] };

    const chart = new uPlot({
      width: el.clientWidth || 560,
      height: 260,
      legend: { show: true, live: true },
      cursor: { drag: { x: true, y: false }, focus: { prox: 24 } },
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
        points: { show: xs.length <= 4000 },
        value: (_u: uPlot, value: number | null) => value == null ? "—" : `${value.toFixed(1)} ${s.unit}`,
      }))],
      plugins: [{
        hooks: {
          ready: [
            (u: uPlot) => {
              const over = u.over;
              // Колесо мыши: масштаб по X вокруг позиции курсора
              over.addEventListener("wheel", (e: WheelEvent) => {
                e.preventDefault();
                const { min, max } = u.scales.x;
                if (min == null || max == null) return;
                const rect = over.getBoundingClientRect();
                const center = u.posToVal(e.clientX - rect.left, "x");
                const factor = e.deltaY < 0 ? 0.8 : 1.25;
                let nMin = center - (center - min) * factor;
                let nMax = center + (max - center) * factor;
                const full = fullRangeRef.current;
                if (nMax - nMin >= full.max - full.min) {
                  nMin = full.min;
                  nMax = full.max;
                }
                u.setScale("x", { min: nMin, max: nMax });
              }, { passive: false });

              // Режим «рука»: перетаскивание сдвигает ось времени
              let panning = false;
              let startX = 0;
              let startMin = 0;
              let startMax = 0;
              const onMove = (e: MouseEvent) => {
                if (!panning) return;
                const width = over.clientWidth || 1;
                const dVal = (e.clientX - startX) * (startMax - startMin) / width;
                u.setScale("x", { min: startMin - dVal, max: startMax - dVal });
              };
              const onUp = () => { panning = false; };
              over.addEventListener("mousedown", (e: MouseEvent) => {
                if (!panRef.current || e.button !== 0) return;
                const { min, max } = u.scales.x;
                if (min == null || max == null) return;
                panning = true;
                startX = e.clientX;
                startMin = min;
                startMax = max;
                e.preventDefault();
              });
              window.addEventListener("mousemove", onMove);
              window.addEventListener("mouseup", onUp);
              // Отписка — в destroy (ниже замыкания видны)
              cleanupRef.current = () => {
                window.removeEventListener("mousemove", onMove);
                window.removeEventListener("mouseup", onUp);
              };
            },
          ],
          destroy: [() => cleanupRef.current?.()],
        },
      }],
    }, data, el);

    chartRef.current = chart;
    const resize = new ResizeObserver(() => {
      if (el.clientWidth) chart.setSize({ width: el.clientWidth, height: 260 });
    });
    resize.observe(el);
    return () => {
      resize.disconnect();
      chart.destroy();
      chartRef.current = null;
    };
  }, [series, xs, ys]);

  // Переключение режима: «рука» отключает выделение-зум, меняет курсор
  useEffect(() => {
    const chart = chartRef.current;
    if (!chart) return;
    if (chart.cursor?.drag) chart.cursor.drag.x = !panMode;
    chart.over.style.cursor = panMode ? "grab" : "crosshair";
  }, [panMode, series, xs, ys]);

  function resetZoom() {
    const chart = chartRef.current;
    if (!chart) return;
    const full = fullRangeRef.current;
    chart.setScale("x", { min: full.min, max: full.max });
  }

  return (
    <div className="card chart-card">
      <div className="chart-head">
        <strong>{title}</strong>
        <div className="chart-actions">
          <button className={panMode ? "active" : ""} onClick={() => setPanMode((v) => !v)}
            title="Режим перетаскивания графика по времени">
            ✋ Сдвиг
          </button>
          <button onClick={resetZoom} title="Весь диапазон времени">Сброс</button>
          {onEdit && <button onClick={onEdit}>Изменить</button>}
          {onRemove && <button onClick={onRemove} aria-label={`Удалить график «${title}»`}>×</button>}
        </div>
      </div>
      <div className="plot-container" ref={containerRef} />
      <p className="hint">
        Колесо — приближение · выделение мышью — зум участка · «Сдвиг» — перетаскивание · двойной клик — сброс
      </p>
    </div>
  );
}
