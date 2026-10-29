# RecuScan — план работ

Сканнер ЭБУ (ELM327 / OBD2, SAE J1979) без рекламы и paywall'ов.
Кроссплатформенно: Windows, Linux, macOS, Android (Tauri v2; iOS — теоретически, но не цель).

## Принципы

- **Нет рекламы, нет подписок.** Весь функционал доступен сразу.
- **Автоопределение вместо ручного выбора профиля:** протокол → VIN → производитель → поддерживаемые PID'ы.
- **Rust-ядро, тонкий UI:** вся логика протокола в Rust, React только рисует.

## Архитектура

```
React + TS (Vite)          src/                — UI: графики (uPlot), DTC, подключение
        │  invoke / events (obd-sample)
Tauri v2 commands          src-tauri/src/commands.rs
        │
OBD-ядро                   src-tauri/src/obd/
        ├── elm327.rs      — AT-стек адаптера (init, запросы, разбор ответов)
        ├── transport.rs   — trait Transport: Wi-Fi TCP, USB Serial (feature), BT SPP (позже)
        ├── pid.rs         — реестр PID'ов Mode 01 + формулы (SAE J1979)
        ├── dtc.rs         — декодирование ошибок P/C/B/U (Mode 03/07/0A, сброс Mode 04)
        ├── vin.rs         — VIN (Mode 09 PID 02) + таблица WMI → производитель
        └── profile.rs     — VehicleInfo: автоопределение профиля
```

Транспорты по приоритету: **Wi-Fi TCP** (работает везде, быстрый старт) → **Bluetooth Classic SPP** (Android через собственный Kotlin-плагин, на iOS SPP заблокирован Apple) → USB Serial (feature `serial`, десктоп).

## Этапы

### Phase 0 — Каркас репозитория ✅ (этот коммит)
- [x] Tauri v2 + React + TS + Vite + uPlot
- [x] Трейт `Transport`, TCP-транспорт
- [x] ELM327 init-стек (ATZ/ATE0/ATL0/ATS0/ATH0/ATSP0), разбор ответов
- [x] Реестр PID'ов с формулами, юнит-тесты декодеров

### Phase 1 — Связь и автоопределение
- [ ] Подключение по Wi-Fi из UI (дефолт 192.168.0.10:35000)
- [ ] Автовыбор протокола (ATSP0 + ATDP), подтверждение связи с ЭБУ (0100)
- [ ] Карта поддерживаемых PID'ов (0100/0120/0140…)
- [ ] VIN (0902) → WMI → производитель; Calibration ID (0904), ECU name (090A)
- [ ] Экран «Автомобиль» с результатами автоопределения

### Phase 2 — Ошибки (DTC)
- [ ] Чтение: stored (03), pending (07), permanent (0A)
- [ ] Сброс (04) с подтверждением в UI
- [ ] Локальная база расшифровок кодов (JSON), человекочитаемые описания на русском

### Phase 3 — Мониторинг в реальном времени
- [ ] Цикл опроса выбранных PID'ов в tokio-таске, события `obd-sample` в UI
- [ ] Графики: обороты (0C), скорость (0D), нагрузка (04), коррекции смеси STFT/LTFT (06–09), MAF (10)
- [ ] Мгновенный расход: PID 5E либо оценка по MAF (бензин: L/ч ≈ MAF·3600/(14.7·720)); L/100км при скорости > 0
- [ ] Адаптивная частота опроса (быстрые PID чаще), индикатор samples/sec

### Phase 4 — USB Serial + эмулятор
- [ ] `serialport` за feature-флагом, выбор порта/бодрейта в UI
- [ ] Интеграционные тесты против OBDSim (эмулятор ELM327) в CI

### Phase 5 — Android + Bluetooth SPP
- [ ] `tauri android init`, сборка APK/AAB
- [ ] Собственный Tauri-плагин: Kotlin BluetoothSocket (RFCOMM), pairing, permissions (BLUETOOTH_CONNECT на API 31+)
- [ ] Выбор сопряжённого устройства в UI
- [ ] Десктопный BT: BlueZ (Linux) / IOBluetooth (macOS) — по возможности

### Phase 6 — Полировка
- [ ] Сохранение сессий/тrips, экспорт CSV
- [ ] Freeze frame (Mode 02), readiness monitors (Mode 01 PID 01)
- [ ] Иконки, splashscreen, сборка бандлов (`npm run tauri icon logo.png`)
- [ ] CI: lint + тесты + сборка под Linux/macOS/Android

## Тестирование

- Юнит-тесты Rust: парсер ответов, DTC, PID-формулы, VIN (`cargo test`).
- Эмулятор: [OBDSim](https://github.com/Ircama/ELM327-emulator) — прогон всего стека без машины.
- Реальный адаптер: чек-лист (Wi-Fi подключение, авто-VIN, DTC, 5 мин мониторинга без разрывов).
