import { useState } from "react";
import { ConnectionPanel } from "./components/ConnectionPanel";
import { Dashboard } from "./components/Dashboard";
import { DtcPanel } from "./components/DtcPanel";
import { VehiclePanel } from "./components/VehiclePanel";
import { VehicleInfo } from "./api";

type Tab = "connect" | "dashboard" | "dtc" | "vehicle";

const TAB_LABELS: Record<Tab, string> = {
  connect: "Подключение",
  dashboard: "Мониторинг",
  dtc: "Ошибки",
  vehicle: "Автомобиль",
};

export default function App() {
  const [tab, setTab] = useState<Tab>("connect");
  const [vehicle, setVehicle] = useState<VehicleInfo | null>(null);
  const [connected, setConnected] = useState(false);

  return (
    <div className="app">
      <header>
        <h1>RecuScan</h1>
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
      </main>
    </div>
  );
}
