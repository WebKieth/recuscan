//! Реестр PID'ов Mode 01 (SAE J1979) и формулы декодирования.

/// Описание одного PID'а.
#[derive(Clone)]
pub struct PidDef {
    /// Короткий ключ для API/UI (напр. "rpm").
    pub key: &'static str,
    pub pid: u8,
    pub name: &'static str,
    pub unit: &'static str,
    pub min: f64,
    pub max: f64,
    /// Декодер байт данных в физическое значение.
    pub decode: fn(&[u8]) -> Option<f64>,
}

fn a1(d: &[u8]) -> Option<f64> {
    d.first().map(|&a| a as f64)
}

fn a2(d: &[u8]) -> Option<(f64, f64)> {
    if d.len() >= 2 {
        Some((d[0] as f64, d[1] as f64))
    } else {
        None
    }
}

/// Поддерживаемые нами PID'ы Mode 01. Порядок = порядок в UI.
pub static PIDS: &[PidDef] = &[
    PidDef {
        key: "engine_load",
        pid: 0x04,
        name: "Нагрузка двигателя",
        unit: "%",
        min: 0.0,
        max: 100.0,
        decode: |d| a1(d).map(|a| a * 100.0 / 255.0),
    },
    PidDef {
        key: "coolant_temp",
        pid: 0x05,
        name: "Температура ОЖ",
        unit: "°C",
        min: -40.0,
        max: 215.0,
        decode: |d| a1(d).map(|a| a - 40.0),
    },
    PidDef {
        key: "stft1",
        pid: 0x06,
        name: "STFT банк 1 (смесь)",
        unit: "%",
        min: -100.0,
        max: 99.2,
        decode: |d| a1(d).map(|a| (a - 128.0) * 100.0 / 128.0),
    },
    PidDef {
        key: "ltft1",
        pid: 0x07,
        name: "LTFT банк 1 (смесь)",
        unit: "%",
        min: -100.0,
        max: 99.2,
        decode: |d| a1(d).map(|a| (a - 128.0) * 100.0 / 128.0),
    },
    PidDef {
        key: "stft2",
        pid: 0x08,
        name: "STFT банк 2 (смесь)",
        unit: "%",
        min: -100.0,
        max: 99.2,
        decode: |d| a1(d).map(|a| (a - 128.0) * 100.0 / 128.0),
    },
    PidDef {
        key: "ltft2",
        pid: 0x09,
        name: "LTFT банк 2 (смесь)",
        unit: "%",
        min: -100.0,
        max: 99.2,
        decode: |d| a1(d).map(|a| (a - 128.0) * 100.0 / 128.0),
    },
    PidDef {
        key: "map",
        pid: 0x0B,
        name: "Давление во впуске",
        unit: "кПа",
        min: 0.0,
        max: 255.0,
        decode: |d| a1(d),
    },
    PidDef {
        key: "rpm",
        pid: 0x0C,
        name: "Обороты двигателя",
        unit: "об/мин",
        min: 0.0,
        max: 16383.75,
        decode: |d| a2(d).map(|(a, b)| (a * 256.0 + b) / 4.0),
    },
    PidDef {
        key: "speed",
        pid: 0x0D,
        name: "Скорость",
        unit: "км/ч",
        min: 0.0,
        max: 255.0,
        decode: |d| a1(d),
    },
    PidDef {
        key: "iat",
        pid: 0x0F,
        name: "Температура впускного воздуха",
        unit: "°C",
        min: -40.0,
        max: 215.0,
        decode: |d| a1(d).map(|a| a - 40.0),
    },
    PidDef {
        key: "maf",
        pid: 0x10,
        name: "Расход воздуха (MAF)",
        unit: "г/с",
        min: 0.0,
        max: 655.35,
        decode: |d| a2(d).map(|(a, b)| (a * 256.0 + b) / 100.0),
    },
    PidDef {
        key: "throttle",
        pid: 0x11,
        name: "Положение дросселя",
        unit: "%",
        min: 0.0,
        max: 100.0,
        decode: |d| a1(d).map(|a| a * 100.0 / 255.0),
    },
    PidDef {
        key: "runtime",
        pid: 0x1F,
        name: "Время с запуска",
        unit: "с",
        min: 0.0,
        max: 65535.0,
        decode: |d| a2(d).map(|(a, b)| a * 256.0 + b),
    },
    PidDef {
        key: "fuel_level",
        pid: 0x2F,
        name: "Уровень топлива",
        unit: "%",
        min: 0.0,
        max: 100.0,
        decode: |d| a1(d).map(|a| a * 100.0 / 255.0),
    },
    PidDef {
        key: "voltage",
        pid: 0x42,
        name: "Напряжение ЭБУ",
        unit: "В",
        min: 0.0,
        max: 65.535,
        decode: |d| a2(d).map(|(a, b)| (a * 256.0 + b) / 1000.0),
    },
    PidDef {
        key: "ambient_temp",
        pid: 0x46,
        name: "Температура снаружи",
        unit: "°C",
        min: -40.0,
        max: 215.0,
        decode: |d| a1(d).map(|a| a - 40.0),
    },
    PidDef {
        key: "oil_temp",
        pid: 0x5C,
        name: "Температура масла",
        unit: "°C",
        min: -40.0,
        max: 210.0,
        decode: |d| a1(d).map(|a| a - 40.0),
    },
    PidDef {
        key: "fuel_rate",
        pid: 0x5E,
        name: "Расход топлива",
        unit: "л/ч",
        min: 0.0,
        max: 3212.75,
        decode: |d| a2(d).map(|(a, b)| (a * 256.0 + b) * 0.05),
    },
];

pub fn by_key(key: &str) -> Option<&'static PidDef> {
    PIDS.iter().find(|p| p.key == key)
}

pub fn by_pid(pid: u8) -> Option<&'static PidDef> {
    PIDS.iter().find(|p| p.pid == pid)
}

/// Декодирует битовую маску поддерживаемых PID'ов.
/// `first_pid` — первый PID, покрываемый маской (0x01 для ответа на 0100, 0x21 для 0120 и т.д.).
pub fn decode_bitmask(first_pid: u8, mask: &[u8]) -> Vec<u8> {
    let mut supported = Vec::new();
    for (byte_idx, byte) in mask.iter().enumerate() {
        for bit in 0..8u8 {
            if byte & (0x80 >> bit) != 0 {
                supported.push(first_pid + byte_idx as u8 * 8 + bit);
            }
        }
    }
    supported
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpm_formula() {
        // 0x1AF8 = 6904 → 6904/4 = 1726 об/мин
        let def = by_key("rpm").unwrap();
        assert_eq!((def.decode)(&[0x1A, 0xF8]), Some(1726.0));
    }

    #[test]
    fn fuel_trim_formula() {
        // 128 → 0%, 255 → +99.2%, 0 → -100%
        let def = by_key("stft1").unwrap();
        assert_eq!((def.decode)(&[128]), Some(0.0));
        assert!(((def.decode)(&[200]).unwrap() - 56.25).abs() < 0.01);
    }

    #[test]
    fn coolant_temp_formula() {
        let def = by_key("coolant_temp").unwrap();
        assert_eq!((def.decode)(&[0x7B]), Some(83.0));
    }

    #[test]
    fn bitmask_decoding() {
        // 0x80 в первом байте → PID 0x01
        assert_eq!(decode_bitmask(0x01, &[0x80, 0, 0, 0]), vec![0x01]);
        // Типичная маска BE1FA813: поддерживаются 0x01, 0x03-0x07, 0x0B-0x0E, 0x10, ...
        let pids = decode_bitmask(0x01, &[0xBE, 0x1F, 0xA8, 0x13]);
        assert!(pids.contains(&0x01));
        assert!(pids.contains(&0x0C));
        assert!(pids.contains(&0x0D));
        assert!(!pids.contains(&0x02));
        // Маска для 0120 покрывает 0x21..0x40
        assert_eq!(decode_bitmask(0x21, &[0x80, 0, 0, 0]), vec![0x21]);
    }
}
