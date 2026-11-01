use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BtDevice {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPairedResponse {
    pub devices: Vec<BtDevice>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectArgs {
    pub address: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteArgs {
    /// Данные в base64 (бинарный канал через JSON).
    pub data: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResponse {
    /// Прочитанные байты в base64; пустая строка — данных нет.
    pub data: String,
}

// ---------- Хранилище экспорта (SAF + MediaStore, Android) ----------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PickedUri {
    /// content:// URI; None — пользователь отменил выбор.
    pub uri: Option<String>,
    /// Человекочитаемое имя папки/файла.
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveExportArgs {
    /// SAF tree URI; None — папка по умолчанию (MediaStore Downloads/recuscan).
    pub tree_uri: Option<String>,
    pub file_name: String,
    pub mime: String,
    /// Содержимое файла в base64 (бинарный канал через JSON).
    pub data: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveExportResponse {
    pub uri: String,
    /// Отображаемый путь вида "Загрузки/recuscan/file.csv".
    pub label: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListExportsArgs {
    pub tree_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFileEntry {
    /// content:// или file:// URI для последующего чтения.
    pub uri: String,
    pub name: String,
    pub size: i64,
    /// Время изменения, мс с эпохи.
    pub mtime: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListExportsResponse {
    pub files: Vec<ExportFileEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadExportArgs {
    pub uri: String,
}
