//! JSON 分析模块：读取本地 JSON 文本文件。

use serde::Serialize;

/// 单个 JSON 文件允许的最大字节数（10 MB）
const MAX_JSON_FILE_SIZE: u64 = 10 * 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonFileContent {
    pub path: String,
    pub content: String,
    pub size: u64,
}

/// 读取本地 JSON 文本文件（UTF-8），去掉开头的 BOM。
#[tauri::command]
pub fn read_json_file(path: String) -> Result<JsonFileContent, String> {
    let file_path = std::path::Path::new(&path);
    let metadata = std::fs::metadata(file_path).map_err(|e| format!("无法读取文件：{}", e))?;
    if !metadata.is_file() {
        return Err("所选路径不是文件".to_string());
    }
    if metadata.len() > MAX_JSON_FILE_SIZE {
        return Err(format!(
            "文件过大（{:.1} MB），最多支持 10 MB",
            metadata.len() as f64 / 1024.0 / 1024.0
        ));
    }

    let bytes = std::fs::read(file_path).map_err(|e| format!("读取文件失败：{}", e))?;
    let text = String::from_utf8(bytes).map_err(|_| "文件不是有效的 UTF-8 编码".to_string())?;
    let content = text.strip_prefix('\u{feff}').unwrap_or(&text).to_string();

    crate::log_info!("[JsonView] 读取文件: {} ({} 字节)", path, metadata.len());

    Ok(JsonFileContent {
        path,
        content,
        size: metadata.len(),
    })
}
