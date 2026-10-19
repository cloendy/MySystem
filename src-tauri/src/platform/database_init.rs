//! 数据库结构初始化。
//!
//! 建表统一由 Rust 在启动时完成，避免前端 SQL 插件把相对路径解析到 AppData。

use rusqlite::Connection;

/// 在统一 data 目录下的 `mydata.db` 中创建全部业务表。
pub fn init_database(app: &tauri::AppHandle) -> Result<(), String> {
    let path = crate::platform::paths::db_path(app)?;
    let conn = Connection::open(&path)
        .map_err(|e| format!("打开数据库失败 {}: {}", path.display(), e))?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS documents (
          id TEXT PRIMARY KEY,
          type TEXT NOT NULL,
          title TEXT NOT NULL,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL,
          deleted_at INTEGER,
          version INTEGER NOT NULL DEFAULT 1,
          device_id TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS notes (
          document_id TEXT PRIMARY KEY REFERENCES documents(id),
          content TEXT NOT NULL DEFAULT '',
          tags TEXT NOT NULL DEFAULT '[]',
          folder_id TEXT,
          is_pinned INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS folders (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          parent_id TEXT,
          sort_order INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS books (
          document_id TEXT PRIMARY KEY REFERENCES documents(id),
          author TEXT,
          cover_url TEXT,
          file_path TEXT NOT NULL,
          file_format TEXT NOT NULL,
          total_pages INTEGER DEFAULT 0,
          current_page INTEGER DEFAULT 0,
          reading_status TEXT DEFAULT 'unread'
        );

        CREATE TABLE IF NOT EXISTS annotations (
          document_id TEXT PRIMARY KEY REFERENCES documents(id),
          book_id TEXT NOT NULL,
          text TEXT NOT NULL,
          note TEXT DEFAULT '',
          page INTEGER,
          position TEXT,
          color TEXT DEFAULT '#FFEB3B'
        );

        CREATE TABLE IF NOT EXISTS sync_operations (
          id TEXT PRIMARY KEY,
          document_id TEXT NOT NULL,
          operation TEXT NOT NULL,
          field TEXT,
          value TEXT,
          timestamp INTEGER NOT NULL,
          device_id TEXT NOT NULL,
          synced INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS sync_state (
          id INTEGER PRIMARY KEY DEFAULT 1,
          last_sync_timestamp INTEGER,
          last_sync_device_id TEXT,
          pending_operations_count INTEGER DEFAULT 0,
          host_device_id TEXT,
          host_ip TEXT,
          host_port INTEGER
        );

        CREATE TABLE IF NOT EXISTS known_devices (
          device_id TEXT PRIMARY KEY,
          device_name TEXT,
          device_type TEXT,
          ip_address TEXT,
          port INTEGER,
          last_seen_at INTEGER,
          is_host INTEGER DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS app_config (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL,
          updated_at INTEGER NOT NULL
        );
        "#,
    )
    .map_err(|e| format!("初始化数据库表失败: {}", e))?;

    crate::log_info!("[Data] 数据库已初始化: {}", path.display());
    Ok(())
}
