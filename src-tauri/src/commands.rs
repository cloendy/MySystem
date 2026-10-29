use tauri::ipc::Invoke;

/// 全部 Tauri 命令的集中注册点（应用组装层）。
///
/// 新增命令时在对应模块实现 `#[tauri::command]` 后，务必在此处追加，
/// 否则前端 invoke 会报 “command not found”。
pub fn invoke_handler() -> impl Fn(Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        // 平台：日志
        crate::platform::logger::write_log,
        crate::platform::logger::list_log_files,
        crate::platform::logger::read_log_file,
        crate::platform::logger::get_log_dir,
        // 平台：配置
        crate::platform::config_storage::get_config,
        crate::platform::config_storage::set_config,
        // 平台：局域网发现与同步
        crate::platform::discovery::start_discovery_listener,
        crate::platform::discovery::stop_discovery_listener,
        crate::platform::discovery::scan_devices,
        crate::platform::http_server::start_sync_server,
        crate::platform::http_server::stop_sync_server,
        crate::platform::http_client::check_remote_device,
        crate::platform::http_client::exchange_sync_data,
        crate::platform::sync_payload::get_sync_payload,
        crate::platform::sync_payload::apply_local_payload,
        // 记事本模块
        crate::modules::notepad::create_note,
        crate::modules::notepad::get_note,
        crate::modules::notepad::list_notes,
        crate::modules::notepad::update_note,
        crate::modules::notepad::delete_note,
        crate::modules::notepad::search_notes,
        crate::modules::notepad::folders::create_folder,
        crate::modules::notepad::folders::list_folders,
        crate::modules::notepad::folders::update_folder,
        crate::modules::notepad::folders::delete_folder,
        // 阅读器模块
        crate::modules::reader::create_book,
        crate::modules::reader::get_book,
        crate::modules::reader::list_books,
        crate::modules::reader::update_book,
        crate::modules::reader::delete_book,
        crate::modules::reader::create_annotation,
        crate::modules::reader::list_annotations,
        crate::modules::reader::delete_annotation,
        crate::modules::reader::import_book_file,
        crate::modules::reader::delete_book_file,
    ]
}
