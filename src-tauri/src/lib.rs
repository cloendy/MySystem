mod commands;
mod models;
mod modules;
mod platform;

/// 构建并运行 Tauri 应用。
///
/// 桌面端由 `main.rs` 调用；`mobile_entry_point` 供 Android 入口使用。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            use tauri::Manager;

            // 日志服务需要最先初始化，后续启动日志才能写入 data/log
            if let Err(e) = platform::logger::init(app.handle()) {
                crate::log_error!("[Data] 日志初始化失败: {}", e);
            }
            crate::log_info!("应用启动");

            // 建表统一在 Rust 端完成，所有数据都位于 <应用根>/data/mydata.db
            if let Err(e) = platform::database_init::init_database(app.handle()) {
                crate::log_error!("[Data] 数据库初始化失败: {}", e);
            }

            // 允许 asset 协议读取 data 目录，用于书籍文件预览
            if let Ok(data_dir) = platform::paths::data_dir(app.handle()) {
                if let Err(e) = app.asset_protocol_scope().allow_directory(&data_dir, true) {
                    crate::log_warn!("[Data] 无法授权 asset 协议访问数据目录: {}", e);
                }
            }
            Ok(())
        })
        .invoke_handler(commands::invoke_handler())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                crate::log_info!("应用退出");
            }
        });
}
