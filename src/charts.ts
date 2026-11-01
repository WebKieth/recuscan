import { PidInfo } from "./api";

export interface ChartConfig { id: string; title: string; keys: string[] }
export const CHART_STORAGE = "recuscan.charts.v1";
export const LOG_CHART_STORAGE = "recuscan.logcharts.v1";
export const FUEL_PID: PidInfo = {
  key: "fuel_rate_maf", name: "Расход (MAF, бензин, λ≈1)", unit: "л/ч",
};
export const PRESETS = [
  { title: "Обороты и расход, л/ч", keys: ["rpm", "fuel_rate"], hint: "Обороты слева, расход в л/ч справа. При отсутствии PID расхода — оценка по MAF для бензина." },
  { title: "Топливные коррекции", keys: ["stft1", "ltft1", "rpm"], hint: "Поведение смеси на холостом ходу и под нагрузкой." },
  { title: "Воздух и обороты", keys: ["rpm", "maf"], hint: "Изменение расхода воздуха при изменении оборотов." },
  { title: "Впуск и нагрузка", keys: ["throttle", "engine_load", "map"], hint: "Отклик дросселя, нагрузка и давление во впуске." },
  { title: "Прогрев двигателя", keys: ["coolant_temp", "oil_temp", "iat"], hint: "Прогрев, работа термостата и температура воздуха." },
  { title: "Питание", keys: ["voltage", "rpm"], hint: "Просадки напряжения и работа генератора." },
  { title: "Лямбда-зонды", keys: ["o2_b1s1", "o2_b1s2", "rpm"], hint: "Сравнение узкополосных датчиков до и после катализатора; частота ELM327 ограничена." },
  { title: "Зажигание и обороты", keys: ["rpm", "timing_advance"], hint: "Изменение угла зажигания; само по себе не доказывает детонацию." },
  { title: "Давление топлива", keys: ["fuel_pressure", "rpm"], hint: "Провалы давления при изменении оборотов (если ЭБУ поддерживает PID 0A)." },
];

export function monitoringKeys(keys: string[], available: PidInfo[]): string[] {
  const supported = new Set(available.map((p) => p.key));
  return [...new Set(keys.flatMap((key) => key === FUEL_PID.key ? ["maf"] : [key]))]
    .filter((key) => supported.has(key));
}

export function loadCharts(storageKey: string = CHART_STORAGE): ChartConfig[] {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(storageKey) || "[]");
    if (!Array.isArray(value)) return [];
    const ids = new Set<string>();
    return value.filter((c): c is ChartConfig => {
      if (!c || typeof c.id !== "string" || ids.has(c.id) || typeof c.title !== "string" ||
          !Array.isArray(c.keys) || c.keys.length < 1 || c.keys.length > 4 ||
          !c.keys.every((k: unknown) => typeof k === "string") || new Set(c.keys).size !== c.keys.length) return false;
      ids.add(c.id);
      return true;
    });
  } catch { return []; }
}
