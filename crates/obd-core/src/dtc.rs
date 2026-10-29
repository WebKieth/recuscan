//! Декодирование кодов неисправностей (DTC) по SAE J2012
//! и их расшифровка по встроенной базе описаний.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Встроенная база описаний DTC (data/dtc.json), парсится один раз.
static DESCRIPTIONS: OnceLock<HashMap<String, String>> = OnceLock::new();

fn descriptions() -> &'static HashMap<String, String> {
    DESCRIPTIONS.get_or_init(|| {
        serde_json::from_str(include_str!("../data/dtc.json")).unwrap_or_default()
    })
}

/// Человекочитаемое описание кода. None — код не найден
/// (вероятно, производитель-специфичный: P1xxx/P2xxx/P3xxx и т.п.).
pub fn description(code: &str) -> Option<&str> {
    descriptions().get(code).map(String::as_str)
}

/// Статус диагностики из Mode 01 PID 01.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorStatus {
    /// Горит ли Check Engine (MIL).
    pub mil_on: bool,
    /// Количество сохранённых кодов по мнению ЭБУ.
    pub dtc_count: u8,
}

/// Байт A ответа на PID 0101: бит 7 — MIL, биты 0-6 — число сохранённых DTC.
pub fn decode_monitor_status(data: &[u8]) -> Option<MonitorStatus> {
    data.first().map(|&a| MonitorStatus {
        mil_on: a & 0x80 != 0,
        dtc_count: a & 0x7F,
    })
}

/// Декодирует пару байт в код вида «P0133». Пара 0x0000 — заполнитель, возвращает None.
pub fn decode_pair(a: u8, b: u8) -> Option<String> {
    if a == 0 && b == 0 {
        return None;
    }
    let system = match (a >> 6) & 0x03 {
        0 => 'P',
        1 => 'C',
        2 => 'B',
        _ => 'U',
    };
    let d1 = (a >> 4) & 0x03; // 0-3
    let d2 = a & 0x0F;
    let d3 = (b >> 4) & 0x0F;
    let d4 = b & 0x0F;
    Some(format!("{system}{d1}{d2:X}{d3:X}{d4:X}"))
}

/// Декодирует поток байт из ответа Mode 03/07/0A в список кодов.
pub fn decode_list(data: &[u8]) -> Vec<String> {
    data.chunks(2)
        .filter_map(|c| {
            if c.len() == 2 {
                decode_pair(c[0], c[1])
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_powertrain_code() {
        assert_eq!(decode_pair(0x01, 0x33).as_deref(), Some("P0133"));
        assert_eq!(decode_pair(0x01, 0x52).as_deref(), Some("P0152"));
    }

    #[test]
    fn decodes_system_prefixes() {
        assert_eq!(decode_pair(0x41, 0x00).as_deref(), Some("C0100"));
        assert_eq!(decode_pair(0x81, 0x00).as_deref(), Some("B0100"));
        assert_eq!(decode_pair(0xC1, 0x00).as_deref(), Some("U0100"));
    }

    #[test]
    fn skips_padding_pairs() {
        let data = [0x01, 0x33, 0x01, 0x52, 0x00, 0x00];
        assert_eq!(decode_list(&data), vec!["P0133", "P0152"]);
    }

    #[test]
    fn finds_descriptions() {
        assert_eq!(
            description("P0133"),
            Some("Датчик кислорода (банк 1, датчик 1): медленный отклик")
        );
        assert!(description("P0420").is_some());
        // Производитель-специфичный код вне базы
        assert_eq!(description("P1999"), None);
    }

    #[test]
    fn decodes_monitor_status() {
        // 0x83: MIL горит, 3 сохранённых кода
        let st = decode_monitor_status(&[0x83, 0x00, 0x00, 0x00]).unwrap();
        assert!(st.mil_on);
        assert_eq!(st.dtc_count, 3);
        // 0x00: MIL не горит, кодов нет
        let st = decode_monitor_status(&[0x00]).unwrap();
        assert!(!st.mil_on);
        assert_eq!(st.dtc_count, 0);
        assert_eq!(decode_monitor_status(&[]), None);
    }
}
