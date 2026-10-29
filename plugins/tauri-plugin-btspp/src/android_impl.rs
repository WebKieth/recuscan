//! Android-реализация: мост к Kotlin-плагину через PluginHandle.

use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::models::*;
use crate::Result;

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> Result<BtSpp<R>> {
    let handle = api.register_android_plugin("com.recuscan.btspp", "BtSppPlugin")?;
    Ok(BtSpp(std::sync::Arc::new(handle)))
}

pub struct BtSpp<R: Runtime>(std::sync::Arc<PluginHandle<R>>);

impl<R: Runtime> Clone for BtSpp<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<R: Runtime> BtSpp<R> {
    /// Сопряжённые устройства Bluetooth.
    pub fn list_paired(&self) -> Result<Vec<BtDevice>> {
        let resp: ListPairedResponse = self.0.run_mobile_plugin("listPaired", ())?;
        Ok(resp.devices)
    }

    /// Открыть RFCOMM-сокет (SPP) к устройству.
    pub fn connect(&self, address: String) -> Result<()> {
        self.0
            .run_mobile_plugin::<()>("connect", ConnectArgs { address })?;
        Ok(())
    }

    /// Отправить байты (base64).
    pub fn write(&self, data_base64: String) -> Result<()> {
        self.0
            .run_mobile_plugin::<()>("write", WriteArgs { data: data_base64 })?;
        Ok(())
    }

    /// Прочитать доступные байты (base64). Пустая строка — данных пока нет.
    pub fn read(&self) -> Result<ReadResponse> {
        Ok(self.0.run_mobile_plugin::<ReadResponse>("read", ())?)
    }

    pub fn disconnect(&self) -> Result<()> {
        self.0.run_mobile_plugin::<()>("disconnect", ())?;
        Ok(())
    }
}
