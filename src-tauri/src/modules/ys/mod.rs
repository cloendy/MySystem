//! 角色图鉴模块：读取 `data/其他/ys.json`。

use serde::Serialize;

/// 允许读取的最大文件大小（20 MB）
const MAX_YS_FILE_SIZE: u64 = 20 * 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YsFileContent {
    pub path: String,
    pub content: String,
    pub size: u64,
}

/// 读取 `<data>/其他/ys.json`；文件不存在时返回 `None`（前端显示空状态）。
#[tauri::command]
pub fn read_ys_file(app: tauri::AppHandle) -> Result<Option<YsFileContent>, String> {
    let path = crate::platform::paths::data_dir(&app)?
        .join("其他")
        .join("ys.json");

    let metadata = match std::fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            crate::log_info!("[YS] 未找到数据文件: {}", path.display());
            return Ok(None);
        }
        Err(error) => {
            return Err(format!("读取 ys.json 失败：{}", error));
        }
    };

    if !metadata.is_file() {
        return Err(format!("数据路径不是文件：{}", path.display()));
    }
    if metadata.len() > MAX_YS_FILE_SIZE {
        return Err(format!(
            "ys.json 过大（{:.1} MB），最多支持 20 MB",
            metadata.len() as f64 / 1024.0 / 1024.0
        ));
    }

    let bytes = std::fs::read(&path).map_err(|e| format!("读取 ys.json 失败：{}", e))?;
    let text = String::from_utf8(bytes).map_err(|_| "ys.json 不是有效的 UTF-8 编码".to_string())?;
    let content = text.strip_prefix('\u{feff}').unwrap_or(&text).to_string();

    crate::log_info!(
        "[YS] 读取角色数据: {} ({} 字节)",
        path.display(),
        metadata.len()
    );

    Ok(Some(YsFileContent {
        path: path.display().to_string(),
        content,
        size: metadata.len(),
    }))
}
