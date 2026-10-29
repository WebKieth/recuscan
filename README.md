# RecuScan

Открытый сканнер ЭБУ для ELM327 / OBD2. Без рекламы, без платных разблокировок, с автоматическим определением автомобиля.

**Стек:** Tauri v2 (Rust) + React + TypeScript + uPlot.
**Платформы:** Windows, Linux, macOS, Android.

## Возможности

- 🔌 Подключение к ELM327 по Wi-Fi (TCP), Bluetooth SPP и USB (в разработке)
- 🚗 Автоопределение: протокол → VIN → производитель → поддерживаемые PID'ы
- ⚠️ Чтение и сброс ошибок (stored / pending / permanent DTC)
- 📊 Мониторинг в реальном времени: обороты, смесь (STFT/LTFT), расход топлива и др.

План работ: [PLAN.md](PLAN.md)

## Разработка

Требования: [Rust](https://rustup.rs), Node.js 18+, [системные зависимости Tauri](https://v2.tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri dev      # десктоп в dev-режиме
cargo test --manifest-path src-tauri/Cargo.toml   # тесты OBD-ядра
```

Сборка релиза: `npm run tauri build` (перед бандлингом сгенерируйте иконки: `npm run tauri icon path/to/logo.png`).

Android: `npm run tauri android init` → `npm run tauri android dev`.

## Тестирование без машины

Используйте эмулятор [ELM327-emulator (OBDSim)](https://github.com/Ircama/ELM327-emulator) — поднимает ELM327 на TCP-порту, приложение подключается к нему как к обычному Wi-Fi адаптеру.
