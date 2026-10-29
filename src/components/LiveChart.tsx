import { useEffect, useRef, useState } from "react";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import { onSample } from "../api";

const WINDOW_SEC = 60; // скользящее окно графика
const COLORS = ["#4fc3f7", "#aed581", "#ffb74d", "#f06292", "#ba68c8"];
let colorIdx = 0;

interface Props {
  sampleKey: string;
  title: string;
  unit: string;
}

export function LiveChart({ sampleKey, title, unit }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const dataRef = useRef<[number[], number[]]>([[], []]);
  const [current, setCurrent] = useState<number | null>(null);
  const [color] = useState(() => COLORS[colorIdx++ % COLORS.length]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const options: uPlot.Options = {
      width: el.clientWidth || 560,
      height: 170,
      legend: { show: false },
      cursor: { show: false },
      scales: { x: { time: true } },
      axes: [{ show: false }, { stroke: "#9aa4b2", grid: { stroke: "#2a2f3a" } }],
      series: [{}, { stroke: color, width: 2, fill: `${color}22` }],
    };
    const chart = new uPlot(options, [[], []] as unknown as uPlot.AlignedData, el);

    const unlisten = onSample((s) => {
      if (s.key !== sampleKey) return;
      const [xs, ys] = dataRef.current;
      xs.push(s.ts / 1000);
      ys.push(s.value);
      const cutoff = xs[xs.length - 1] - WINDOW_SEC;
      while (xs.length && xs[0] < cutoff) {
        xs.shift();
        ys.shift();
      }
      setCurrent(s.value);
    });

    const timer = setInterval(() => {
      chart.setData([dataRef.current[0], dataRef.current[1]] as uPlot.AlignedData);
    }, 400);

    const resize = new ResizeObserver(() => {
      chart.setSize({ width: el.clientWidth, height: 170 });
    });
    resize.observe(el);

    return () => {
      unlisten.then((f) => f());
      clearInterval(timer);
      resize.disconnect();
      chart.destroy();
    };
  }, [sampleKey, color]);

  return (
    <div className="card chart-card">
      <div className="chart-head">
        <span>{title}</span>
        <span className="chart-value" style={{ color }}>
          {current !== null ? `${current.toFixed(1)} ${unit}` : "—"}
        </span>
      </div>
      <div ref={containerRef} />
    </div>
  );
}
