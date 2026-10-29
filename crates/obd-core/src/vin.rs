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
    ("5UX", "BMW"),
    ("4US", "BMW"),
    ("WDB", "Mercedes-Benz"),
    ("WDD", "Mercedes-Benz"),
    ("WDC", "Mercedes-Benz"),
    ("55S", "Mercedes-Benz"),
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
    ("JTH", "Lexus"),
    ("JT", "Toyota"),
    ("JHM", "Honda"),
    ("1HG", "Honda"),
    ("2HG", "Honda"),
    ("SHH", "Honda"),
    ("JN1", "Nissan"),
    ("JN8", "Nissan"),
    ("1N4", "Nissan"),
    ("3N1", "Nissan"),
    ("5N1", "Nissan"),
    ("JM1", "Mazda"),
    ("JMZ", "Mazda"),
    ("JF1", "Subaru"),
    ("JF2", "Subaru"),
    ("JA3", "Mitsubishi"),
    ("JA4", "Mitsubishi"),
    ("KMH", "Hyundai"),
    ("KM8", "Hyundai"),
    ("5NP", "Hyundai"),
    ("KNA", "Kia"),
    ("KND", "Kia"),
    ("5XY", "Kia"),
    ("XTA", "Lada (АвтоВАЗ)"),
    ("XUU", "УАЗ"),
    ("X7M", "Hyundai (РФ)"),
    ("1G1", "Chevrolet"),
    ("KL1", "Chevrolet"),
    ("1FA", "Ford"),
    ("1FT", "Ford"),
    ("5YJ", "Tesla"),
    ("7SA", "Tesla"),
    ("YV1", "Volvo"),
    ("SAL", "Land Rover"),
    ("SJN", "Nissan"),
    ("W0L", "Opel"),
    ("VSS", "SEAT"),
    ("TMB", "Škoda"),
    ("UU1", "Dacia"),
    ("LSV", "Volkswagen (Китай)"),
    ("LSG", "GM (Китай)"),
    ("LFP", "FAW"),
    ("L6T", "Geely"),
    ("LVV", "Chery"),
    ("LGW", "Great Wall / Haval"),
];

/// Год выпуска по 10-му символу VIN (ISO 3779, 30-летний цикл).
/// `current_year` нужен для разрешения неоднозначности цикла (1980/2010 и т.д.).
pub fn model_year(vin: &str, current_year: u16) -> Option<u16> {
    let c = vin.chars().nth(9)?;
    let mut year = match c {
        '1'..='9' => 2001 + (c as u16 - '1' as u16), // 2001-2009
        'A'..='H' => 2010 + (c as u16 - 'A' as u16), // 2010-2017
        'J'..='N' => 2018 + (c as u16 - 'J' as u16), // 2018-2022
        'P' => 2023, // Q пропущена
        'R'..='T' => 2024 + (c as u16 - 'R' as u16), // 2024-2026
        'V'..='Y' => 2027 + (c as u16 - 'V' as u16), // 2027-2030 (U пропущена)
        _ => return None,
    };
    // Цикл повторяется каждые 30 лет: берём прошлый виток, если год в будущем
    if year > current_year {
        year -= 30;
    }
    Some(year)
}

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
        assert_eq!(manufacturer("JN112345678901234"), Some("Nissan"));
        assert_eq!(manufacturer("5UX12345678901234"), Some("BMW"));
        assert_eq!(manufacturer("QQQ12345678901234"), None);
    }

    #[test]
    fn decodes_model_year() {
        // 10-й символ (индекс 9): K = 2019
        assert_eq!(model_year("WVWZZZ3CZKE123456", 2026), Some(2019));
        // Цифры: 5 = 2005
        assert_eq!(model_year("JTDBE32K253123456", 2026), Some(2005));
        // Год в будущем цикле → прошлый виток: V = 2027 → 1997
        assert_eq!(model_year("WP0ZZZ99ZVS392124", 2026), Some(1997));
        // T = 2026, текущий год — ок
        assert_eq!(model_year("XTA211500T1234567", 2026), Some(2026));
        // Z не используется как код года
        assert_eq!(model_year("WVWZZZ3CZZE123456", 2026), None);
    }
}
