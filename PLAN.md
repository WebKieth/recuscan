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

### Phase 1 — Связь и автоопределение ✅
- [x] Подключение по Wi-Fi из UI (дефолт 192.168.0.10:35000)
- [x] Автовыбор протокола (ATSP0 + ATDP), проверка связи с ЭБУ (0100)
- [x] Fallback при сбое авто-протокола: ручной перебор CAN 6-9, ISO/KWP/J1850 3-5 (для дешёвых клонов ELM)
- [x] Карта поддерживаемых PID'ов (0100/0120/0140…)
- [x] VIN (0902) → WMI → производитель (~50 WMI); год выпуска по 10-му символу VIN
- [x] Calibration ID (0904), ECU name (090A)
- [x] Экран «Автомобиль» с результатами автоопределения

### Phase 2 — Ошибки (DTC)
- [x] Чтение: stored (03), pending (07), permanent (0A)
- [x] Сброс (04) с подтверждением и предупреждениями в UI
- [x] Встроенная база расшифровок (~230 generic-кодов, data/dtc.json), описания на русском
- [x] Статус Check Engine (MIL) и счётчик кодов из PID 0101
- [ ] Расширение базы: manufacturer-specific коды по маркам (после автоопределения VIN)

### Phase 3 — Мониторинг в реальном времени
- [x] Цикл опроса выбранных PID'ов в tokio-таске, события `obd-sample` в UI
- [x] Графики: обороты (0C), скорость (0D), нагрузка (04), коррекции смеси STFT/LTFT (06–09), MAF (10)
- [x] Мгновенный расход: PID 5E либо оценка по MAF (бензин: L/ч ≈ MAF·3600/(14.7·720)); L/100км при скорости > 0
- [ ] Адаптивная частота опроса (быстрые PID чаще), индикатор samples/sec

### Phase 4 — USB Serial + эмулятор
- [x] `serialport` за feature-флагом (default on; для Android-сборки `--no-default-features`), выбор порта/бодрейта в UI
- [ ] Интеграционные тесты против OBDSim (эмулятор ELM327) в CI

### Phase 5 — Android + Bluetooth SPP
- [ ] `tauri android init` — требует Android SDK/NDK (см. https://v2.tauri.app/start/prerequisites/#android)
- [x] Плагин `plugins/tauri-plugin-btspp`: Kotlin BluetoothSocket (RFCOMM), permissions BLUETOOTH_CONNECT, список сопряжённых устройств
- [x] `BtTransport` (src-tauri/src/bt.rs): байты через JNI-мост (base64), реализует `Transport`
- [x] Вкладка Bluetooth в UI: список сопряжённых устройств, подключение
- [x] Итерации по API tauri-android 2.12: `invoke.getArgs()`, `@PermissionCallback`, `project(":tauri-android")` в gradle — debug APK собирается
- [ ] Прогон на реальном устройстве (сопряжение адаптера, SPP-коннект, мониторинг)

### Phase 6 — Полировка
- [x] Freeze frame (Mode 02) — снимок параметров в момент ошибки
- [x] Readiness-мониторы (PID 0101) — готовность к техосмотру
- [x] Брендовые базы DTC по VIN (VAG, Toyota, Ford, Lada, BMW, Mercedes, Hyundai/Kia, Nissan — 8 марок)
- [x] Запись мониторинга в CSV (кнопка на дашборде, BOM для Excel, путь к файлу показывается в UI)
- [x] Экспорт логов: форматы CSV / JSON / XLS (SpreadsheetML), выбор папки; запись во временный файл, экспорт при остановке
- [x] Папка экспорта по умолчанию вне android/data: Android — MediaStore `Download/recuscan` или SAF-папка на выбор; десктоп — `~/Documents/recuscan` с диалогом выбора
- [x] Вкладка «Анализ логов»: список логов из папки экспорта, импорт CSV/JSON, графики с зумом/панорамой (колесо, выделение, «рука»), конструктор сводных графиков
- [ ] Сохранение сессий/trips, просмотр истории логов в приложении
- [ ] Расширение брендовых баз (Mazda, Subaru, Mitsubishi, GM)
- [ ] Иконки, splashscreen, сборка бандлов
- [ ] CI: lint + тесты + сборка под Linux/macOS/Android

## Тестирование

- Юнит-тесты Rust: парсер ответов, DTC, PID-формулы, VIN (`cargo test`).
- Эмулятор: [OBDSim](https://github.com/Ircama/ELM327-emulator) — прогон всего стека без машины.
- Реальный адаптер: чек-лист (Wi-Fi подключение, авто-VIN, DTC, 5 мин мониторинга без разрывов).
