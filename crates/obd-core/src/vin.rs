//! VIN (Mode 09 PID 02) и определение производителя по WMI.

/// Извлекает VIN из полезной нагрузки ответа 0902.
pub fn decode_vin(payload: &[u8]) -> Option<String> {
    let vin: String = payload
        .iter()
        .filter(|b| b.is_ascii_alphanumeric())
        .map(|&b| b as char)
        .collect();
    if vin.len() >= 17 {
        Some(vin[..17].to_string())
    } else {
        None
    }
}

/// Таблица WMI (первые 2-3 символа VIN) → производитель. Наиболее распространённые;
/// расширяется по мере необходимости.
static WMI: &[(&str, &str)] = &[
    ("WBA", "BMW"),
    ("WBS", "BMW M"),
    ("WDB", "Mercedes-Benz"),
    ("WDD", "Mercedes-Benz"),
    ("WAU", "Audi"),
    ("WVW", "Volkswagen"),
    ("WV1", "Volkswagen"),
    ("WP0", "Porsche"),
    ("WF0", "Ford"),
    ("ZFA", "Fiat"),
    ("ZFF", "Ferrari"),
    ("VF1", "Renault"),
    ("VF3", "Peugeot"),
    ("VF7", "Citroën"),
    ("JTD", "Toyota"),
    ("JTE", "Toyota"),
    ("JT", "Toyota"),
    ("JHM", "Honda"),
    ("1HG", "Honda"),
    ("KMH", "Hyundai"),
    ("KNA", "Kia"),
    ("XTA", "Lada (АвтоВАЗ)"),
    ("XUU", "УАЗ"),
    ("X7M", "Hyundai (РФ)"),
    ("1G1", "Chevrolet"),
    ("1FA", "Ford"),
    ("1FT", "Ford"),
    ("5YJ", "Tesla"),
    ("7SA", "Tesla"),
    ("YV1", "Volvo"),
    ("SAL", "Land Rover"),
    ("SJN", "Nissan"),
    ("VSS", "SEAT"),
    ("TMB", "Škoda"),
    ("UU1", "Dacia"),
    ("LSV", "Volkswagen (Китай)"),
    ("LSG", "GM (Китай)"),
    ("LFP", "FAW"),
];

/// Определяет производителя по VIN (сначала ищем 3-символьный WMI, затем 2-символьный).
pub fn manufacturer(vin: &str) -> Option<&'static str> {
    WMI.iter()
        .filter(|(prefix, _)| prefix.len() == 3)
        .find(|(prefix, _)| vin.starts_with(prefix))
        .map(|(_, m)| *m)
        .or_else(|| {
            WMI.iter()
                .filter(|(prefix, _)| prefix.len() == 2)
                .find(|(prefix, _)| vin.starts_with(prefix))
                .map(|(_, m)| *m)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_vin_from_payload() {
        let payload = b"WP0ZZZ99ZTS392124\x00\x00";
        assert_eq!(decode_vin(payload).as_deref(), Some("WP0ZZZ99ZTS392124"));
    }

    #[test]
    fn rejects_short_payload() {
        assert_eq!(decode_vin(b"WP0"), None);
    }

    #[test]
    fn detects_manufacturer_by_wmi() {
        assert_eq!(manufacturer("WP0ZZZ99ZTS392124"), Some("Porsche"));
        assert_eq!(manufacturer("XTABC123456789012"), Some("Lada (АвтоВАЗ)"));
        assert_eq!(manufacturer("JTD12345678901234"), Some("Toyota"));
        assert_eq!(manufacturer("QQQ12345678901234"), None);
    }
}
