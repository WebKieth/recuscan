import { VehicleInfo } from "../api";

export function VehiclePanel({ vehicle }: { vehicle: VehicleInfo | null }) {
  if (!vehicle) {
    return <p className="hint">Информация об автомобиле появится после подключения к адаптеру.</p>;
  }

  const rows: Array<[string, string]> = [
    ["VIN", vehicle.vin ?? "не определён"],
    ["Производитель", vehicle.manufacturer ?? "не определён"],
    ["Протокол OBD", vehicle.protocol ?? "не определён"],
    ["Калибровка ЭБУ", vehicle.calibration_id ?? "нет данных"],
  ];

  return (
    <section className="card">
      <h2>Автоопределение автомобиля</h2>
      <table className="info-table">
        <tbody>
          {rows.map(([k, v]) => (
            <tr key={k}>
              <td>{k}</td>
              <td>{v}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <h3>Поддерживаемые параметры ({vehicle.supported_pids.length})</h3>
      <div className="pid-picker">
        {vehicle.supported_pids.map((k) => (
          <span key={k} className="chip">{k}</span>
        ))}
      </div>
    </section>
  );
}
