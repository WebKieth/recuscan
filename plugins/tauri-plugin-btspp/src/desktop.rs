//! Заглушка для десктопа: Bluetooth SPP пока доступен только на Android.

use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;
use crate::{Error, Result};

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> Result<BtSpp<R>> {
    Ok(BtSpp(std::marker::PhantomData))
}

// PhantomData<fn() -> R>: структура остаётся Send + Sync независимо от R.
pub struct BtSpp<R: Runtime>(std::marker::PhantomData<fn() -> R>);

impl<R: Runtime> Clone for BtSpp<R> {
    fn clone(&self) -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<R: Runtime> BtSpp<R> {
    pub fn list_paired(&self) -> Result<Vec<BtDevice>> {
        Err(Error::Unsupported)
    }

    pub fn connect(&self, _address: String) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn write(&self, _data_base64: String) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn read(&self) -> Result<ReadResponse> {
        Err(Error::Unsupported)
    }

    pub fn disconnect(&self) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn pick_folder(&self) -> Result<PickedUri> {
        Err(Error::Unsupported)
    }

    pub fn pick_log_file(&self) -> Result<PickedUri> {
        Err(Error::Unsupported)
    }

    pub fn save_export(&self, _args: SaveExportArgs) -> Result<SaveExportResponse> {
        Err(Error::Unsupported)
    }

    pub fn list_exports(&self, _tree_uri: Option<String>) -> Result<Vec<ExportFileEntry>> {
        Err(Error::Unsupported)
    }

    pub fn read_export(&self, _uri: String) -> Result<ReadResponse> {
        Err(Error::Unsupported)
    }
}
