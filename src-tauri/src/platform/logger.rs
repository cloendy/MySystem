//! 平台日志服务。
//!
//! 日志同时输出到终端和 `<data>/log/mysystem-YYYY-MM-DD.log`。
//! Rust 代码使用 `crate::log_info!` / `log_warn!` / `log_error!` / `log_debug!`，
//! 前端代码使用 `src/platform/services/logger.ts` 中的 `logger`。

use once_cell::sync::{Lazy, OnceCell};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

static LOG_DIR: OnceCell<PathBuf> = OnceCell::new();
static WRITE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// 单个日志文件最多读取的大小（超出时只返回尾部）
const MAX_READ_BYTES: usize = 512 * 1024;

/// 初始化日志目录，应在应用启动时尽早调用。
pub fn init(app: &tauri::AppHandle) -> Result<(), String> {
    let dir = crate::platform::paths::log_dir(app)?;
    let _ = LOG_DIR.set(dir);
    Ok(())
}

pub fn debug(message: impl AsRef<str>) {
    write("DEBUG", message.as_ref());
}

pub fn info(message: impl AsRef<str>) {
    write("INFO", message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    write("WARN", message.as_ref());
}

pub fn error(message: impl AsRef<str>) {
    write("ERROR", message.as_ref());
}

fn write(level: &str, message: &str) {
    let now = chrono::Local::now();
    let line = format!(
        "[{}] [{}] {}\n",
        now.format("%Y-%m-%d %H:%M:%S%.3f"),
        level,
        message
    );

    // 终端输出
    if level == "ERROR" {
        eprint!("{}", line);
    } else {
        print!("{}", line);
    }

    // 文件输出（初始化前只输出到终端）
    let Some(dir) = LOG_DIR.get() else {
        return;
    };
    let Ok(_guard) = WRITE_LOCK.lock() else {
        return;
    };
    let path = dir.join(format!("mysystem-{}.log", now.format("%Y-%m-%d")));
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(line.as_bytes());
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFileInfo {
    pub name: String,
    pub size: u64,
    pub modified_at: i64,
}

/// 前端写入日志。
#[tauri::command]
pub fn write_log(level: String, message: String) {
    match level.to_ascii_lowercase().as_str() {
        "debug" => debug(message),
        "warn" | "warning" => warn(message),
        "error" => error(message),
        _ => info(message),
    }
}

/// 列出日志文件（按修改时间倒序）。
#[tauri::command]
pub fn list_log_files(app: tauri::AppHandle) -> Result<Vec<LogFileInfo>, String> {
    let dir = crate::platform::paths::log_dir(&app)?;
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| format!("读取日志目录失败 {}: {}", dir.display(), e))?;

    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("log") {
            continue;
        }
        let meta = entry.metadata().ok();
        let modified_at = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        files.push(LogFileInfo {
            name: entry.file_name().to_string_lossy().to_string(),
            size: meta.map(|m| m.len()).unwrap_or(0),
            modified_at,
        });
    }
    files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(files)
}

/// 读取日志文件（最多返回最后 512 KB）。
#[tauri::command]
pub fn read_log_file(app: tauri::AppHandle, name: String) -> Result<String, String> {
    if name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || !name.ends_with(".log")
    {
        return Err("非法的日志文件名".to_string());
    }

    let dir = crate::platform::paths::log_dir(&app)?;
    let path = dir.join(&name);
    let bytes = std::fs::read(&path)
        .map_err(|e| format!("读取日志失败 {}: {}", path.display(), e))?;

    let (slice, truncated) = if bytes.len() > MAX_READ_BYTES {
        (&bytes[bytes.len() - MAX_READ_BYTES..], true)
    } else {
        (&bytes[..], false)
    };

    let mut text = String::from_utf8_lossy(slice).to_string();
    if truncated {
        text = format!(
            "...（日志过大，仅显示最后 {} KB）...\n{}",
            MAX_READ_BYTES / 1024,
            text
        );
    }
    Ok(text)
}

/// 返回日志目录的绝对路径（用于界面展示）。
#[tauri::command]
pub fn get_log_dir(app: tauri::AppHandle) -> Result<String, String> {
    Ok(crate::platform::paths::log_dir(&app)?
        .to_string_lossy()
        .to_string())
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::platform::logger::debug(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::platform::logger::info(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::platform::logger::warn(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::platform::logger::error(format!($($arg)*))
    };
}
