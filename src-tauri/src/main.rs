// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod platform;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
