//! Транспортный уровень до ELM327-адаптера.
//!
//! Wi-Fi адаптеры выставляют TCP-сокет (обычно 192.168.0.10:35000),
//! USB — последовательный порт, Bluetooth Classic — RFCOMM (SPP).

use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::{ObdError, Result};

/// Асинхронный байтовый канал до адаптера.
#[async_trait]
pub trait Transport: Send {
    /// Отправить байты адаптеру.
    async fn send(&mut self, data: &[u8]) -> Result<()>;
    /// Прочитать доступные байты. 0 — данных пока нет.
    async fn recv(&mut self, buf: &mut [u8]) -> Result<usize>;
}

/// Wi-Fi ELM327 (TCP-сокет).
pub struct TcpTransport {
    stream: TcpStream,
}

impl TcpTransport {
    pub async fn connect(host: &str, port: u16) -> Result<Self> {
        let stream = tokio::time::timeout(
            Duration::from_secs(5),
            TcpStream::connect((host, port)),
        )
        .await
        .map_err(|_| ObdError::Timeout)??;
        let _ = stream.set_nodelay(true);
        Ok(Self { stream })
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.stream.write_all(data).await?;
        Ok(())
    }

    async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.stream.read(buf).await?;
        Ok(n)
    }
}

/// USB / COM-порт ELM327 (feature `serial`).
#[cfg(feature = "serial")]
pub struct SerialTransport {
    port: Box<dyn serialport::SerialPort>,
}

#[cfg(feature = "serial")]
impl SerialTransport {
    pub fn open(path: &str, baud: u32) -> Result<Self> {
        let port = serialport::new(path, baud)
            .timeout(Duration::from_millis(100))
            .open()?;
        Ok(Self { port })
    }
}

#[cfg(feature = "serial")]
#[async_trait]
impl Transport for SerialTransport {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        use std::io::Write;
        self.port.write_all(data)?;
        self.port.flush()?;
        Ok(())
    }

    async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        use std::io::Read;
        match self.port.read(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(0),
            Err(e) => Err(e.into()),
        }
    }
}

// TODO(Phase 5): Bluetooth Classic SPP.
// Android — собственный Tauri-плагин на Kotlin (BluetoothSocket/RFCOMM);
// десктоп — BlueZ (Linux), IOBluetooth (macOS), Win32 SPP.
