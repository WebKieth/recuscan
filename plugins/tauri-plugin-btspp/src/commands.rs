use tauri::{command, AppHandle, Manager, Runtime};

use crate::models::BtDevice;
use crate::{BtSpp, Result};

/// Список сопряжённых Bluetooth-устройств (из фронтенда: `plugin:btspp|list_paired`).
#[command]
pub(crate) async fn list_paired<R: Runtime>(app: AppHandle<R>) -> Result<Vec<BtDevice>> {
    app.state::<BtSpp<R>>().list_paired()
}
