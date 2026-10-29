//! Готовность диагностических мониторов (Mode 01 PID 01, байты B/C/D).
//! Важно для техосмотра: после сброса ошибок мониторы должны «отъездиться».

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ReadinessItem {
    pub name: &'static str,
    /// Поддерживается ли монитор этим ЭБУ.
    pub supported: bool,
    /// Завершён ли цикл самодиагностики.
    pub complete: bool,
}

/// Непрерывные мониторы: бит 4-6 байта B (0 = готов).
const CONTINUOUS: &[(u8, &str)] = &[
    (4, "Пропуски зажигания"),
    (5, "Топливная система"),
    (6, "Компоненты"),
];

/// Непрерывные... точнее периодические мониторы: байт C — поддержка, байт D — готовность (0 = готов).
const PERIODIC: &[&str] = &[
    "Катализатор",
    "Катализатор с подогревом",
    "Система EVAP",
    "Вторичный воздух",
    "Хладагент кондиционера",
    "Кислородный датчик",
    "Подогрев кислородного датчика",
    "EGR / VVT",
];

/// data — 4 байта ответа на PID 0101.
pub fn decode_readiness(data: &[u8]) -> Vec<ReadinessItem> {
    if data.len() < 4 {
        return Vec::new();
    }
    let (b, c, d) = (data[1], data[2], data[3]);
    let mut out = Vec::new();
    for &(bit, name) in CONTINUOUS {
        out.push(ReadinessItem {
            name,
            supported: true,
            complete: b & (1 << bit) == 0,
        });
    }
    for (bit, name) in PERIODIC.iter().enumerate() {
        let supported = c & (1 << bit) != 0;
        if supported {
            out.push(ReadinessItem {
                name,
                supported: true,
                complete: d & (1 << bit) == 0,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_readiness() {
        // B=0x00: все непрерывные готовы; C=0x21: поддерживаются катализатор (bit0) и O2 (bit5);
        // D=0x01: катализатор не готов, O2 готов
        let items = decode_readiness(&[0x00, 0x00, 0x21, 0x01]);
        assert!(items.iter().any(|i| i.name == "Пропуски зажигания" && i.complete));
        let cat = items.iter().find(|i| i.name == "Катализатор").unwrap();
        assert!(!cat.complete);
        let o2 = items.iter().find(|i| i.name == "Кислородный датчик").unwrap();
        assert!(o2.complete);
        // EVAP не поддерживается — отсутствует
        assert!(!items.iter().any(|i| i.name == "Система EVAP"));
    }

    #[test]
    fn short_data_is_empty() {
        assert!(decode_readiness(&[0x00]).is_empty());
    }
}
