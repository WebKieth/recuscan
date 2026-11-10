# RecuScan

Открытый сканнер ЭБУ для ELM327 / OBD2. Без рекламы, без платных разблокировок, с автоматическим определением автомобиля.

**Стек:** Tauri v2 (Rust) + React + TypeScript + uPlot.
**Платформы:** Windows, Linux, macOS, Android.
**Автор:** [shadowof.space](https://shadowof.space) · [Telegram-канал](https://t.me/shadowof_space) · [GitHub](https://github.com/WebKieth/recuscan)

## Возможности

- 🔌 Подключение к ELM327 по Wi-Fi (TCP), Bluetooth SPP и USB (в разработке)
- 🚗 Автоопределение: протокол → VIN → производитель → поддерживаемые PID'ы
- ⚠️ Чтение и сброс ошибок (stored / pending / permanent DTC)
- 📊 Мониторинг в реальном времени: обороты, смесь (STFT/LTFT), расход топлива и др.
- 💾 Запись мониторинга в CSV — для разбора логов после поездки (Excel/Python)
- Сводные графики: до 4 цветных линий, две оси Y для разных единиц, общее окно времени 60 с. Создание, изменение, удаление и сохранение наборов на устройстве.
- Шаблон «Обороты и расход, л/ч»: обороты слева, расход справа. Используется PID расхода ЭБУ, а при его отсутствии доступна явно помеченная оценка по MAF (только бензин, стехиометрическая смесь; не дизель/ГБО).
- Диагностические шаблоны смеси, впуска, прогрева, питания, зажигания, лямбда-зондов и давления топлива. Доступность зависит от карты PID ЭБУ; узкополосные лямбда-графики через ELM327 не заменяют осциллограф.

План работ: [PLAN.md](PLAN.md)

## Разработка

Требования: [Rust](https://rustup.rs), Node.js 18+, [системные зависимости Tauri](https://v2.tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri dev      # десктоп в dev-режиме
cargo test --manifest-path src-tauri/Cargo.toml   # тесты OBD-ядра
```

Сборка релиза: `npm run tauri build`.

Android: установите Android SDK/NDK по [инструкции Tauri](https://v2.tauri.app/start/prerequisites/#android), затем `npm run tauri android init` и `npm run tauri android dev`. Bluetooth Classic SPP реализован собственным плагином `plugins/tauri-plugin-btspp` (на iOS SPP заблокирован Apple — используйте Wi-Fi адаптер). Для Android-сборки без USB-Serial: `cargo build --no-default-features` (feature `serial`).

Релизные APK по архитектурам: `npm run tauri -- android build --apk --split-per-abi` (Rust-профиль `opt-level="z"` + LTO + strip). Локальная подпись настраивается отдельно в генерируемом Android-проекте; весь `gen/` в .gitignore.

## Готовые пакеты

В [GitHub Releases](https://github.com/WebKieth/recuscan/releases) автоматически собираются Android APK (ARM64/ARMv7), Windows x64 (`.exe`, `.msi`), Ubuntu x64 (`.deb`, `.AppImage`) и macOS Universal (`.dmg`, Intel и Apple Silicon). `latest` — тестовые сборки из `main`, теги `vX.Y.Z` — стабильные релизы. Файл `SHA256SUMS.txt` содержит контрольные суммы всех пакетов.

Десктопные пакеты не подписаны сертификатами разработчика Microsoft/Apple; macOS-сборки не нотариализованы. Возможны предупреждения SmartScreen/Gatekeeper. Ubuntu-пакеты собираются на Ubuntu 22.04; для `.deb` системные зависимости устанавливает пакетный менеджер.

## Тестирование без машины

Используйте эмулятор [ELM327-emulator (OBDSim)](https://github.com/Ircama/ELM327-emulator) — поднимает ELM327 на TCP-порту, приложение подключается к нему как к обычному Wi-Fi адаптеру.
