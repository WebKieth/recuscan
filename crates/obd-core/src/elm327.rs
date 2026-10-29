//! Клиент ELM327: AT-инициализация, отправка OBD-запросов, разбор ответов.

use std::time::Duration;

use tokio::time::{sleep, timeout};

use crate::transport::Transport;
use crate::{ObdError, Result};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(4);

pub struct Elm327 {
    t: Box<dyn Transport>,
    timeout: Duration,
}

impl Elm327 {
    /// Стандартная инициализация адаптера и автовыбор протокола.
    /// Если авто-определение не нашло ЭБУ (частая болезнь дешёвых клонов),
    /// перебираем протоколы вручную в порядке убывания распространённости.
    pub async fn init(t: Box<dyn Transport>) -> Result<Self> {
        let mut elm = Self {
            t,
            timeout: DEFAULT_TIMEOUT,
        };
        // ATZ перезагружает адаптер: ответ может задержаться
        let _ = elm.at("ATZ").await;
        sleep(Duration::from_millis(800)).await;
        elm.at("ATE0").await?; // echo off
        elm.at("ATL0").await?; // linefeeds off
        elm.at("ATS0").await?; // spaces off
        elm.at("ATH0").await?; // headers off — упрощает разбор полезной нагрузки

        elm.at("ATSP0").await?; // автоматический выбор протокола
        if elm.query_pid(0x01, 0x00).await.is_ok() {
            return Ok(elm);
        }

        // Fallback: ручной перебор. 6-9 — CAN (ISO 15765-4), 3-5 — ISO/KWP/J1850
        for proto in [6u8, 7, 8, 9, 3, 4, 5] {
            elm.at(&format!("ATSP{proto}")).await?;
            if elm.query_pid(0x01, 0x00).await.is_ok() {
                // Фиксируем найденный протокол, чтобы адаптер не искал заново
                let _ = elm.at(&format!("ATTP{proto}")).await;
                return Ok(elm);
            }
        }
        Err(ObdError::ConnectionFailed)
    }

    /// AT-команда адаптера.
    pub async fn at(&mut self, cmd: &str) -> Result<String> {
        let resp = self.raw(cmd).await?;
        if resp.contains('?') {
            return Err(ObdError::Adapter(format!("команда {cmd} не поддержана")));
        }
        Ok(resp)
    }

    /// Человекочитаемое имя выбранного протокола (ATDP).
    pub async fn describe_protocol(&mut self) -> Result<String> {
        let resp = self.raw("ATDP").await?;
        Ok(resp.trim().trim_start_matches("AUTO,").trim().to_string())
    }

    /// Запрос Mode/PID (напр. mode=0x01, pid=0x0C → «010C»). Возвращает байты данных.
    pub async fn query_pid(&mut self, mode: u8, pid: u8) -> Result<Vec<u8>> {
        let raw = self.raw(&format!("{mode:02X}{pid:02X}")).await?;
        if raw.contains("NODATA") {
            return Err(ObdError::NoData);
        }
        let lines = parse_obd_lines(&raw);
        extract_payload(&lines, mode, pid)
    }

    /// Запрос режима без PID (03, 04, 07, 0A). Возвращает объединённые данные.
    pub async fn query_mode(&mut self, mode: u8) -> Result<Vec<u8>> {
        let raw = self.raw(&format!("{mode:02X}")).await?;
        if raw.contains("NODATA") {
            return Err(ObdError::NoData);
        }
        let lines = parse_obd_lines(&raw);
        extract_payload(&lines, mode, 0x00)
    }

    /// Отправить команду и прочитать ответ до приглашения '>'.
    async fn raw(&mut self, cmd: &str) -> Result<String> {
        self.t.send(cmd.as_bytes()).await?;
        self.t.send(b"\r").await?;
        let bytes = timeout(self.timeout, self.read_to_prompt())
            .await
            .map_err(|_| ObdError::Timeout)??;
        Ok(String::from_utf8_lossy(&bytes).replace('>', "").trim().to_string())
    }

    async fn read_to_prompt(&mut self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut buf = [0u8; 512];
        loop {
            let n = self.t.recv(&mut buf).await?;
            out.extend_from_slice(&buf[..n]);
            if out.contains(&b'>') || out.len() > 16 * 1024 {
                break;
            }
        }
        Ok(out)
    }
}

/// Разбирает сырой ответ адаптера на строки байт, отбрасывая служебный вывод.
pub fn parse_obd_lines(raw: &str) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for line in raw.split(['\r', '\n']) {
        let line = line.trim();
        if line.is_empty()
            || line.contains("SEARCHING")
            || line.contains("STOPPED")
            || line.contains("NODATA")
            || line.contains("ELM327")
            || line == "OK"
            || line == "?"
            || line.starts_with("AT")
        {
            continue;
        }
        // ISO-TP многострочный формат: «0: 49 02 01 ...» — берём после префикса
        let payload = match line.find(':') {
            Some(idx)
                if line[..idx].len() <= 2
                    && line[..idx].chars().all(|c| c.is_ascii_hexdigit()) =>
            {
                &line[idx + 1..]
            }
            _ => line,
        };
        let hex: String = payload.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        // Ответ Mode 04 («44») — всего 2 символа, не отбрасываем короткие строки
        if hex.len() < 2 || hex.len() % 2 != 0 {
            continue;
        }
        if let Ok(bytes) = decode_hex(&hex) {
            out.push(bytes);
        }
    }
    out
}

/// Извлекает полезную нагрузку ответа на mode/pid из распарсенных строк.
pub fn extract_payload(lines: &[Vec<u8>], mode: u8, pid: u8) -> Result<Vec<u8>> {
    let positive = mode.wrapping_add(0x40);

    if mode == 0x09 {
        // Mode 09: [0x49, pid, count, data...] + возможные строки-продолжения ISO-TP
        let mut data = Vec::new();
        let mut found = false;
        for line in lines {
            if line.len() >= 3 && line[0] == 0x49 && line[1] == pid {
                found = true;
                data.extend_from_slice(&line[3..]);
            } else if found {
                data.extend_from_slice(line);
            }
        }
        if found {
            return Ok(data);
        }
    } else if pid == 0x00 && matches!(mode, 0x03 | 0x04 | 0x07 | 0x0A) {
        // Режимы без PID: данные после байта положительного ответа
        let mut data = Vec::new();
        let mut found = false;
        for line in lines {
            if !line.is_empty() && line[0] == positive {
                found = true;
                data.extend_from_slice(&line[1..]);
            }
        }
        if found {
            return Ok(data);
        }
    } else {
        for line in lines {
            if line.len() >= 2 && line[0] == positive && line[1] == pid {
                return Ok(line[2..].to_vec());
            }
        }
    }
    Err(ObdError::Parse(format!(
        "в ответе нет данных на запрос {mode:02X}{pid:02X}"
    )))
}

fn decode_hex(s: &str) -> std::result::Result<Vec<u8>, ()> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_line_response() {
        let raw = "41 0C 1A F8\r\r>";
        let lines = parse_obd_lines(raw);
        assert_eq!(lines, vec![vec![0x41, 0x0C, 0x1A, 0xF8]]);
        let payload = extract_payload(&lines, 0x01, 0x0C).unwrap();
        assert_eq!(payload, vec![0x1A, 0xF8]);
    }

    #[test]
    fn skips_service_lines() {
        let raw = "SEARCHING...\r41 00 BE 1F A8 13\rOK\r>";
        let payload = extract_payload(&parse_obd_lines(raw), 0x01, 0x00).unwrap();
        assert_eq!(payload, vec![0xBE, 0x1F, 0xA8, 0x13]);
    }

    #[test]
    fn parses_isotp_multiline_vin() {
        let raw = "014\r0: 49 02 01 57 50 30\r1: 5A 5A 5A 39 39 5A\r2: 54 53 33 39 32 31\r3: 32 34\r>";
        let payload = extract_payload(&parse_obd_lines(raw), 0x09, 0x02).unwrap();
        assert_eq!(payload.len(), 17);
        assert_eq!(&payload[..3], b"WP0");
    }

    #[test]
    fn parses_dtc_mode03() {
        // У Mode 03 нет байта-счётчика (в отличие от Mode 09): сразу пары кодов
        let raw = "43 01 33 01 52 00 00\r>";
        let payload = extract_payload(&parse_obd_lines(raw), 0x03, 0x00).unwrap();
        assert_eq!(payload, vec![0x01, 0x33, 0x01, 0x52, 0x00, 0x00]);
        assert_eq!(crate::dtc::decode_list(&payload), vec!["P0133", "P0152"]);
    }

    #[test]
    fn parses_clear_dtc_response() {
        let raw = "44\r>";
        let payload = extract_payload(&parse_obd_lines(raw), 0x04, 0x00).unwrap();
        assert!(payload.is_empty());
    }

    #[test]
    fn missing_response_is_error() {
        let lines = parse_obd_lines("NO DATA\r>");
        assert!(extract_payload(&lines, 0x01, 0x0C).is_err());
    }
}
