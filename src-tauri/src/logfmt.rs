//! Форматы экспорта логов мониторинга (CSV / JSON / XLS-SpreadsheetML)
//! и разбор записанных логов для вкладки «Анализ логов».

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct LogColumn {
    pub key: String,
    pub name: String,
    pub unit: String,
}

/// Разобранный лог в колоночном виде (удобно для графиков и invoke-сериализации).
#[derive(Debug, Clone)]
pub struct ParsedLog {
    pub columns: Vec<LogColumn>,
    /// Метки времени строк (мс с эпохи), возрастают.
    pub ts_ms: Vec<u64>,
    /// Значения по колонкам: values[i][row] соответствует columns[i].
    pub values: Vec<Vec<Option<f64>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Json,
    Xls,
}

impl ExportFormat {
    pub fn parse(s: &str) -> Self {
        match s {
            "json" => Self::Json,
            "xls" => Self::Xls,
            _ => Self::Csv,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Xls => "xls",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Csv => "CSV",
            Self::Json => "JSON",
            Self::Xls => "XLS (Excel)",
        }
    }

    pub fn extension(&self) -> &'static str {
        self.as_str()
    }

    pub fn mime(&self) -> &'static str {
        match self {
            Self::Csv => "text/csv",
            Self::Json => "application/json",
            Self::Xls => "application/vnd.ms-excel",
        }
    }
}

/// Отрисовка лога в выбранном формате. CSV здесь не рендерим: исходный CSV
/// уже записан логгером во временный файл и копируется как есть.
pub fn render(fmt: ExportFormat, log: &ParsedLog) -> Vec<u8> {
    match fmt {
        ExportFormat::Json => render_json(log),
        ExportFormat::Xls => render_xls(log),
        ExportFormat::Csv => render_csv(log),
    }
}

fn fmt_num(v: f64) -> String {
    // До 3 знаков после запятой, без хвостовых нулей (как в CsvLogger)
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".into()
    } else {
        s.into()
    }
}

fn display_name(c: &LogColumn) -> String {
    if c.unit.is_empty() {
        c.name.clone()
    } else {
        format!("{} [{}]", c.name, c.unit)
    }
}

fn render_csv(log: &ParsedLog) -> Vec<u8> {
    let mut out = String::from("\u{FEFF}ts_ms");
    for c in &log.columns {
        out.push(',');
        out.push_str(&display_name(c));
    }
    out.push('\n');
    for (row, ts) in log.ts_ms.iter().enumerate() {
        out.push_str(&ts.to_string());
        for col in &log.values {
            out.push(',');
            if let Some(Some(v)) = col.get(row) {
                out.push_str(&fmt_num(*v));
            }
        }
        out.push('\n');
    }
    out.into_bytes()
}

fn render_json(log: &ParsedLog) -> Vec<u8> {
    // Ручная сборка: компактные числа без сериализации f64 как 12.0
    let mut out = String::with_capacity(log.ts_ms.len() * log.columns.len() * 8 + 512);
    out.push_str("{\"app\":\"recuscan\",\"format\":1,\"columns\":[");
    for (i, c) in log.columns.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"key\":{},\"name\":{},\"unit\":{}}}",
            json_str(&c.key),
            json_str(&c.name),
            json_str(&c.unit)
        ));
    }
    out.push_str("],\"rows\":[");
    for (row, ts) in log.ts_ms.iter().enumerate() {
        if row > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&ts.to_string());
        for col in &log.values {
            out.push(',');
            match col.get(row) {
                Some(Some(v)) => out.push_str(&fmt_num(*v)),
                _ => out.push_str("null"),
            }
        }
        out.push(']');
    }
    out.push_str("]}");
    out.into_bytes()
}

fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// SpreadsheetML 2003: открывается Excel и LibreOffice как .xls без внешних зависимостей.
fn render_xls(log: &ParsedLog) -> Vec<u8> {
    let mut out = String::with_capacity(log.ts_ms.len() * log.columns.len() * 12 + 1024);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<?mso-application progid=\"Excel.Sheet\"?>\n");
    out.push_str("<Workbook xmlns=\"urn:schemas-microsoft-com:office:spreadsheet\" xmlns:ss=\"urn:schemas-microsoft-com:office:spreadsheet\">");
    out.push_str("<Worksheet ss:Name=\"Лог\"><Table>");
    out.push_str("<Row><Cell><Data ss:Type=\"String\">ts_ms</Data></Cell>");
    for c in &log.columns {
        out.push_str("<Cell><Data ss:Type=\"String\">");
        out.push_str(&xml_escape(&display_name(c)));
        out.push_str("</Data></Cell>");
    }
    out.push_str("</Row>");
    for (row, ts) in log.ts_ms.iter().enumerate() {
        out.push_str("<Row><Cell><Data ss:Type=\"Number\">");
        out.push_str(&ts.to_string());
        out.push_str("</Data></Cell>");
        for col in &log.values {
            match col.get(row) {
                Some(Some(v)) => {
                    out.push_str("<Cell><Data ss:Type=\"Number\">");
                    out.push_str(&fmt_num(*v));
                    out.push_str("</Data></Cell>");
                }
                _ => out.push_str("<Cell/>"),
            }
        }
        out.push_str("</Row>");
    }
    out.push_str("</Table></Worksheet></Workbook>");
    out.into_bytes()
}

/// Разбор лога по расширению файла. XLS — только экспорт: обратный разбор
/// SpreadsheetML не поддержан, импортируйте CSV/JSON.
pub fn parse(ext: &str, bytes: &[u8]) -> Result<ParsedLog, String> {
    match ext {
        "csv" => parse_csv(bytes),
        "json" => parse_json(bytes),
        "xls" => Err("импорт XLS не поддержан (формат только для экспорта); сохраняйте копию лога в CSV или JSON".into()),
        other => Err(format!("неизвестный формат лога: .{other}")),
    }
}

/// Разбор "Имя [единица]" → (name, unit). Без скобок — вся строка имя.
fn split_name_unit(header: &str) -> (String, String) {
    let h = header.trim();
    if let Some(rest) = h.strip_suffix(']') {
        if let Some(pos) = rest.rfind(" [") {
            return (rest[..pos].trim().to_string(), rest[pos + 2..].trim().to_string());
        }
    }
    (h.to_string(), String::new())
}

pub fn parse_csv(bytes: &[u8]) -> Result<ParsedLog, String> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(&text);
    let mut lines = text.lines();
    let header = lines.next().ok_or("пустой файл лога")?;
    let mut headers = header.split(',');
    if headers.next().map(str::trim) != Some("ts_ms") {
        return Err("не похоже на лог RecuScan: первая колонка должна быть ts_ms".into());
    }
    let names: Vec<&str> = headers.collect();
    let columns: Vec<LogColumn> = names
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let (name, unit) = split_name_unit(h);
            LogColumn {
                key: format!("c{i}"),
                name,
                unit,
            }
        })
        .collect();

    let mut ts_ms = Vec::new();
    let mut values: Vec<Vec<Option<f64>>> = vec![Vec::new(); columns.len()];
    for (line_no, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut cells = line.split(',');
        let ts: u64 = match cells.next().map(str::trim).unwrap_or("").parse() {
            Ok(v) => v,
            Err(_) => return Err(format!("строка {}: неверная метка времени", line_no + 2)),
        };
        ts_ms.push(ts);
        for col in values.iter_mut() {
            let cell = cells.next().map(str::trim).unwrap_or("");
            col.push(if cell.is_empty() {
                None
            } else {
                cell.parse::<f64>().ok()
            });
        }
    }
    if ts_ms.is_empty() {
        return Err("в логе нет строк данных".into());
    }
    Ok(ParsedLog {
        columns,
        ts_ms,
        values,
    })
}

pub fn parse_json(bytes: &[u8]) -> Result<ParsedLog, String> {
    #[derive(serde::Deserialize)]
    struct JColumn {
        key: String,
        name: String,
        #[serde(default)]
        unit: String,
    }
    #[derive(serde::Deserialize)]
    struct JLog {
        columns: Vec<JColumn>,
        rows: Vec<Vec<serde_json::Value>>,
    }
    let j: JLog = serde_json::from_slice(bytes)
        .map_err(|e| format!("не похоже на JSON-лог RecuScan: {e}"))?;
    let columns: Vec<LogColumn> = j
        .columns
        .into_iter()
        .enumerate()
        .map(|(i, c)| LogColumn {
            key: if c.key.is_empty() { format!("c{i}") } else { c.key },
            name: c.name,
            unit: c.unit,
        })
        .collect();
    let mut ts_ms = Vec::with_capacity(j.rows.len());
    let mut values: Vec<Vec<Option<f64>>> = vec![Vec::with_capacity(j.rows.len()); columns.len()];
    for row in &j.rows {
        let ts = row
            .first()
            .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
            .ok_or("строка без метки времени")?;
        ts_ms.push(ts);
        for (i, col) in values.iter_mut().enumerate() {
            col.push(row.get(i + 1).and_then(|v| v.as_f64()));
        }
    }
    if ts_ms.is_empty() {
        return Err("в логе нет строк данных".into());
    }
    Ok(ParsedLog {
        columns,
        ts_ms,
        values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ParsedLog {
        ParsedLog {
            columns: vec![
                LogColumn { key: "rpm".into(), name: "Обороты".into(), unit: "об/мин".into() },
                LogColumn { key: "speed".into(), name: "Скорость".into(), unit: "км/ч".into() },
            ],
            ts_ms: vec![1000, 2000],
            values: vec![vec![Some(800.5), None], vec![Some(0.0), Some(60.25)]],
        }
    }

    #[test]
    fn csv_roundtrip() {
        let log = sample();
        let bytes = render(ExportFormat::Csv, &log);
        let back = parse_csv(&bytes).unwrap();
        assert_eq!(back.ts_ms, log.ts_ms);
        assert_eq!(back.columns.len(), 2);
        assert_eq!(back.columns[0].name, "Обороты");
        assert_eq!(back.columns[0].unit, "об/мин");
        assert_eq!(back.values[0], log.values[0]);
        assert_eq!(back.values[1], log.values[1]);
    }

    #[test]
    fn json_roundtrip() {
        let log = sample();
        let bytes = render(ExportFormat::Json, &log);
        let back = parse_json(&bytes).unwrap();
        assert_eq!(back.ts_ms, log.ts_ms);
        assert_eq!(back.columns[1].key, "speed");
        assert_eq!(back.values[0][1], None);
        assert_eq!(back.values[1][1], Some(60.25));
    }

    #[test]
    fn xls_escapes_and_renders() {
        let log = sample();
        let bytes = render(ExportFormat::Xls, &log);
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("Excel.Sheet"));
        assert!(text.contains("Обороты [об/мин]"));
        assert!(text.contains("800.5"));
        assert!(text.contains("<Cell/>"));
    }

    #[test]
    fn csv_rejects_garbage() {
        assert!(parse_csv(b"hello,world").is_err());
        assert!(parse_csv(b"").is_err());
    }
}
