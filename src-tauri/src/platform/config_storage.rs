use rusqlite::{Connection, OptionalExtension, params};

fn open_conn(app: &tauri::AppHandle) -> Result<Connection, String> {
    let path = crate::platform::paths::db_path(app)?;
    Connection::open(path).map_err(|e| format!("打开数据库失败: {}", e))
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[tauri::command]
pub fn get_config(app: tauri::AppHandle, key: String) -> Result<Option<String>, String> {
    let conn = open_conn(&app)?;
    let result = conn
        .query_row(
            "SELECT value FROM app_config WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
pub fn set_config(app: tauri::AppHandle, key: String, value: String) -> Result<(), String> {
    let conn = open_conn(&app)?;
    conn.execute(
        "INSERT INTO app_config (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, value, now_ms()],
    )
    .map_err(|e| format!("写入配置失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn delete_config(app: tauri::AppHandle, key: String) -> Result<(), String> {
    let conn = open_conn(&app)?;
    conn.execute("DELETE FROM app_config WHERE key = ?1", params![key])
        .map_err(|e| e.to_string())?;
    Ok(())
}
