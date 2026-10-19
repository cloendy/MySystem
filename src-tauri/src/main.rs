// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod platform;

fn main() {
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
        .invoke_handler(tauri::generate_handler![
            platform::paths::get_data_dir,
            platform::logger::write_log,
            platform::logger::list_log_files,
            platform::logger::read_log_file,
            platform::logger::get_log_dir,
            platform::storage::create_document,
            platform::storage::get_document,
            platform::storage::list_documents,
            platform::storage::update_document,
            platform::storage::delete_document,
            platform::notepad_storage::create_note,
            platform::notepad_storage::get_note,
            platform::notepad_storage::list_notes,
            platform::notepad_storage::update_note,
            platform::notepad_storage::delete_note,
            platform::notepad_storage::search_notes,
            platform::folder_storage::create_folder,
            platform::folder_storage::list_folders,
            platform::folder_storage::update_folder,
            platform::folder_storage::delete_folder,
            platform::config_storage::get_config,
            platform::config_storage::set_config,
            platform::config_storage::delete_config,
            platform::discovery::start_discovery_listener,
            platform::discovery::stop_discovery_listener,
            platform::discovery::scan_devices,
            platform::http_server::start_sync_server,
            platform::http_server::stop_sync_server,
            platform::http_client::check_remote_device,
            platform::http_client::exchange_sync_data,
            platform::sync_payload::get_sync_payload,
            platform::sync_payload::apply_local_payload,
            platform::reader_storage::create_book,
            platform::reader_storage::get_book,
            platform::reader_storage::list_books,
            platform::reader_storage::update_book,
            platform::reader_storage::delete_book,
            platform::reader_storage::create_annotation,
            platform::reader_storage::list_annotations,
            platform::reader_storage::delete_annotation,
            platform::reader_storage::read_book_file,
            platform::reader_storage::import_book_file,
            platform::reader_storage::delete_book_file,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                crate::log_info!("应用退出");
            }
        });
}
