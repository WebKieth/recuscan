import { useState } from "react";
import { ConnectionPanel } from "./components/ConnectionPanel";
import { Dashboard } from "./components/Dashboard";
import { DtcPanel } from "./components/DtcPanel";
import { LogAnalysis } from "./components/LogAnalysis";
import { VehiclePanel } from "./components/VehiclePanel";
import { VehicleInfo } from "./api";
import logo from "./assets/logo.png";

type Tab = "connect" | "dashboard" | "dtc" | "vehicle" | "analysis";

const TAB_LABELS: Record<Tab, string> = {
  connect: "Подключение",
  dashboard: "Мониторинг",
  dtc: "Ошибки",
  vehicle: "Автомобиль",
  analysis: "Анализ логов",
};

export default function App() {
  const [tab, setTab] = useState<Tab>("connect");
  const [vehicle, setVehicle] = useState<VehicleInfo | null>(null);
  const [connected, setConnected] = useState(false);

  return (
    <div className="app">
      <header>
        <img src={logo} className="logo" alt="RecuScan" />
        <nav>
          {(Object.keys(TAB_LABELS) as Tab[]).map((t) => (
            <button
              key={t}
              className={tab === t ? "active" : ""}
              onClick={() => setTab(t)}
            >
              {TAB_LABELS[t]}
            </button>
          ))}
        </nav>
        <span className={connected ? "status ok" : "status"}>
          {connected ? "● подключено" : "○ не подключено"}
        </span>
      </header>
      <main>
        {tab === "connect" && (
          <ConnectionPanel
            connected={connected}
            onConnected={(v) => {
              setVehicle(v);
              setConnected(true);
              setTab("vehicle");
            }}
            onDisconnected={() => {
              setConnected(false);
              setVehicle(null);
            }}
          />
        )}
        {tab === "dashboard" && <Dashboard connected={connected} />}
        {tab === "dtc" && <DtcPanel connected={connected} />}
        {tab === "vehicle" && <VehiclePanel vehicle={vehicle} />}
        {tab === "analysis" && <LogAnalysis />}
      </main>
      <footer>
        <span className="credit">
          developed by{" "}
          <a href="https://shadowof.space" target="_blank" rel="noreferrer">
            shadowof.space
          </a>
        </span>
        <div className="social">
          <a
            href="https://github.com/WebKieth/recuscan"
            target="_blank"
            rel="noreferrer"
            aria-label="GitHub"
            title="GitHub"
          >
            <svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor" aria-hidden="true">
              <path d="M12 .5C5.65.5.5 5.65.5 12c0 5.08 3.29 9.39 7.86 10.91.58.11.79-.25.79-.55 0-.27-.01-1.17-.02-2.12-3.2.7-3.88-1.36-3.88-1.36-.52-1.33-1.28-1.68-1.28-1.68-1.04-.71.08-.7.08-.7 1.15.08 1.76 1.18 1.76 1.18 1.03 1.76 2.69 1.25 3.35.96.1-.75.4-1.25.72-1.54-2.55-.29-5.24-1.28-5.24-5.69 0-1.26.45-2.28 1.18-3.09-.12-.29-.51-1.46.11-3.05 0 0 .96-.31 3.15 1.18a10.9 10.9 0 0 1 5.74 0c2.19-1.49 3.15-1.18 3.15-1.18.62 1.59.23 2.76.11 3.05.74.81 1.18 1.83 1.18 3.09 0 4.42-2.7 5.4-5.27 5.68.41.36.78 1.06.78 2.14 0 1.55-.01 2.79-.01 3.17 0 .31.21.67.8.55A11.51 11.51 0 0 0 23.5 12C23.5 5.65 18.35.5 12 .5z" />
            </svg>
          </a>
          <a
            href="https://t.me/shadowof_space"
            target="_blank"
            rel="noreferrer"
            aria-label="Telegram"
            title="Telegram"
          >
            <svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor" aria-hidden="true">
              <path d="M23.91 3.79 20.3 20.84c-.25 1.21-.98 1.5-2 1.16l-5.5-4.05-2.65 2.55c-.29.3-.54.54-1.1.54l.39-5.56L19.54 6.1c.44-.39-.1-.61-.68-.22L6.7 14.06l-5.45-1.7C.07 11.96.04 11.2 1.1 10.76l21.3-8.2c1-.37 1.86.24 1.51 1.23z" />
            </svg>
          </a>
        </div>
      </footer>
    </div>
  );
}
