//! Bluetooth SPP транспорт до ELM327 (только Android).
//! Канал идёт через Kotlin-плагин tauri-plugin-btspp; байты передаются в base64.

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use obd_core::transport::Transport;
use obd_core::{ObdError, Result};
use tauri_plugin_btspp::BtSpp;

pub struct BtTransport {
    plugin: BtSpp<tauri::Wry>,
}

impl BtTransport {
    pub fn new(plugin: BtSpp<tauri::Wry>) -> Self {
        Self { plugin }
    }
}

fn to_err(e: impl std::fmt::Display) -> ObdError {
    ObdError::Adapter(e.to_string())
}

#[async_trait]
impl Transport for BtTransport {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        let plugin = self.plugin.clone();
        let payload = B64.encode(data);
        // JNI-вызов синхронный — выносим из async-контекста
        tokio::task::spawn_blocking(move || plugin.write(payload))
            .await
            .map_err(to_err)?
            .map_err(to_err)
    }

    async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        let plugin = self.plugin.clone();
        let resp = tokio::task::spawn_blocking(move || plugin.read())
            .await
            .map_err(to_err)?
            .map_err(to_err)?;
        if resp.data.is_empty() {
            return Ok(0);
        }
        let bytes = B64
            .decode(&resp.data)
            .map_err(|e| ObdError::Parse(e.to_string()))?;
        let n = bytes.len().min(buf.len());
        buf[..n].copy_from_slice(&bytes[..n]);
        Ok(n)
    }
}
