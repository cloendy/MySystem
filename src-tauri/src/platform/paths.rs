//! 统一的数据目录解析。
//!
//! 所有业务数据（数据库、书籍文件等）都放在 `<应用根目录>/data` 下：
//! - 开发模式（tauri dev）：项目根目录/data，即 src-tauri 的上一级目录
//! - 发布模式：exe 所在目录/data（exe 直接放在项目根目录时与开发模式一致）
//! - Android 或目录不可写时：应用私有数据目录/data
//!
//! 可通过环境变量 `MYSYSTEM_DATA_DIR` 覆盖数据目录。

use std::path::{Path, PathBuf};

/// 获取数据根目录，并确保目录存在。
pub fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = resolve_data_dir(app)?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("创建数据目录失败 {}: {}", dir.display(), e))?;
    Ok(dir)
}

/// 数据库文件路径：`<data>/mydata.db`
pub fn db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("mydata.db"))
}

/// 书籍文件目录：`<data>/books`
pub fn books_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = data_dir(app)?.join("books");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("创建 books 目录失败 {}: {}", dir.display(), e))?;
    Ok(dir)
}

/// 日志目录：`<data>/log`
pub fn log_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = data_dir(app)?.join("log");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("创建 log 目录失败 {}: {}", dir.display(), e))?;
    Ok(dir)
}

fn resolve_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // 1. 环境变量优先，便于特殊部署
    if let Some(dir) = std::env::var_os("MYSYSTEM_DATA_DIR") {
        return Ok(PathBuf::from(dir));
    }

    platform_data_dir(app)
}

/// 开发模式：数据固定放在项目根目录/data，保证与放在根目录的 exe 共用同一份数据。
#[cfg(all(debug_assertions, not(target_os = "android")))]
fn platform_data_dir(_app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir
        .parent()
        .ok_or_else(|| "无法确定项目根目录".to_string())?;
    Ok(project_root.join("data"))
}

/// 发布模式：exe 所在目录/data；目录不可写时回退应用私有目录。
#[cfg(all(not(debug_assertions), not(target_os = "android")))]
fn platform_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let dir = exe_dir.join("data");
            if is_writable(&dir) {
                return Ok(dir);
            }
            crate::log_warn!("[Data] 数据目录不可写，回退到应用数据目录: {}", dir.display());
        }
    }
    fallback_data_dir(app)
}

/// Android 等移动端：使用应用私有数据目录。
#[cfg(target_os = "android")]
fn platform_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    fallback_data_dir(app)
}

#[allow(dead_code)]
fn fallback_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用数据目录: {}", e))?;
    Ok(base.join("data"))
}

#[allow(dead_code)]
fn is_writable(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let test = dir.join(".write_test");
    let ok = std::fs::write(&test, b"test").is_ok();
    if ok {
        std::fs::remove_file(&test).ok();
    }
    ok
}
