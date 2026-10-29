//! Декодирование кодов неисправностей (DTC) по SAE J2012.

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
}
