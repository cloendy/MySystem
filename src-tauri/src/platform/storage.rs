use crate::models::document::{BaseDocument, CreateDocumentInput, UpdateDocumentInput};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::PathBuf;
use uuid::Uuid;

pub fn db_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;

    // ===== 开发模式：走用户数据目录 =====
    #[cfg(debug_assertions)]
    {
        let dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("无法获取数据目录: {}", e))?;
        std::fs::create_dir_all(&dir).ok();
        println!("[Storage][DEV] 数据目录: {}", dir.display());
        return Ok(dir.join("mydata.db"));
    }

    // ===== 生产模式：走 exe 所在目录 =====
    #[cfg(not(debug_assertions))]
    {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(install_dir) = exe.parent() {
                // 可写性检测
                let test = install_dir.join(".write_test");
                if std::fs::write(&test, b"test").is_ok() {
                    std::fs::remove_file(&test).ok();
                    println!("[Storage][PROD] 数据目录: {}", install_dir.display());
                    return Ok(install_dir.join("mydata.db"));
                } else {
                    println!("[Storage][PROD] 安装目录不可写，回退");
                }
            }
        }

        // 回退：Program Files 不可写时使用用户目录
        let dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("无法获取数据目录: {}", e))?;
        std::fs::create_dir_all(&dir).ok();
        Ok(dir.join("mydata.db"))
    }
}

pub fn get_install_dir() -> Result<PathBuf, String> {
    let exe_path = std::env::current_exe().map_err(|e| format!("无法获取可执行文件路径: {}", e))?;
    let install_dir = exe_path
        .parent()
        .ok_or_else(|| "无法获取安装目录".to_string())?
        .to_path_buf();
    Ok(install_dir)
}

fn open_conn(app_handle: &tauri::AppHandle) -> Result<Connection, String> {
    let path = db_path(app_handle)?;
    Connection::open(path).map_err(|e| format!("打开数据库失败: {}", e))
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn log_sync_op(
    conn: &Connection,
    document_id: &str,
    operation: &str,
    field: &str,
    value: &str,
    device_id: &str,
) -> Result<(), String> {
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO sync_operations (id, document_id, operation, field, value, timestamp, device_id, synced)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
        params![id, document_id, operation, field, value, now_ms(), device_id],
    )
    .map_err(|e| format!("写入同步日志失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn create_document(
    app: tauri::AppHandle,
    input: CreateDocumentInput,
) -> Result<BaseDocument, String> {
    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let id = Uuid::new_v4().to_string();
    let ts = now_ms();

    tx.execute(
        "INSERT INTO documents (id, type, title, created_at, updated_at, deleted_at, version, device_id)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, 1, ?6)",
        params![id, input.r#type, input.title, ts, ts, input.device_id],
    )
    .map_err(|e| format!("插入文档失败: {}", e))?;

    log_sync_op(&tx, &id, "create", "title", &input.title, &input.device_id)?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(BaseDocument {
        id,
        r#type: input.r#type,
        title: input.title,
        created_at: ts,
        updated_at: ts,
        deleted_at: None,
        version: 1,
        device_id: input.device_id,
    })
}

#[tauri::command]
pub fn get_document(app: tauri::AppHandle, id: String) -> Result<Option<BaseDocument>, String> {
    let conn = open_conn(&app)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, type, title, created_at, updated_at, deleted_at, version, device_id
             FROM documents WHERE id = ?1 AND deleted_at IS NULL",
        )
        .map_err(|e| e.to_string())?;

    let result = stmt
        .query_row(params![id], |row| {
            Ok(BaseDocument {
                id: row.get(0)?,
                r#type: row.get(1)?,
                title: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                deleted_at: row.get(5)?,
                version: row.get(6)?,
                device_id: row.get(7)?,
            })
        })
        .optional()
        .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub fn list_documents(
    app: tauri::AppHandle,
    r#type: Option<String>,
) -> Result<Vec<BaseDocument>, String> {
    let conn = open_conn(&app)?;

    let sql = if r#type.is_some() {
        "SELECT id, type, title, created_at, updated_at, deleted_at, version, device_id
         FROM documents WHERE deleted_at IS NULL AND type = ?1 ORDER BY updated_at DESC"
    } else {
        "SELECT id, type, title, created_at, updated_at, deleted_at, version, device_id
         FROM documents WHERE deleted_at IS NULL ORDER BY updated_at DESC"
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let map_row = |row: &rusqlite::Row| -> rusqlite::Result<BaseDocument> {
        Ok(BaseDocument {
            id: row.get(0)?,
            r#type: row.get(1)?,
            title: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
            version: row.get(6)?,
            device_id: row.get(7)?,
        })
    };

    let mut docs = Vec::new();
    if let Some(t) = r#type {
        let rows = stmt
            .query_map(params![t], map_row)
            .map_err(|e| e.to_string())?;
        for row in rows {
            docs.push(row.map_err(|e| e.to_string())?);
        }
    } else {
        let rows = stmt.query_map([], map_row).map_err(|e| e.to_string())?;
        for row in rows {
            docs.push(row.map_err(|e| e.to_string())?);
        }
    }

    Ok(docs)
}

#[tauri::command]
pub fn update_document(
    app: tauri::AppHandle,
    input: UpdateDocumentInput,
) -> Result<BaseDocument, String> {
    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let ts = now_ms();
    let title = input.title.clone().unwrap_or_default();

    let affected = tx
        .execute(
            "UPDATE documents SET title = ?1, updated_at = ?2, version = version + 1, device_id = ?3
             WHERE id = ?4 AND deleted_at IS NULL",
            params![title, ts, input.device_id, input.id],
        )
        .map_err(|e| format!("更新文档失败: {}", e))?;

    if affected == 0 {
        return Err("文档不存在或已被删除".to_string());
    }

    log_sync_op(&tx, &input.id, "update", "title", &title, &input.device_id)?;

    tx.commit().map_err(|e| e.to_string())?;

    let doc = get_document(app, input.id)?.ok_or_else(|| "文档不存在".to_string())?;
    Ok(doc)
}

#[tauri::command]
pub fn delete_document(app: tauri::AppHandle, id: String, device_id: String) -> Result<(), String> {
    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let ts = now_ms();
    let affected = tx
        .execute(
            "UPDATE documents SET deleted_at = ?1, updated_at = ?1, version = version + 1, device_id = ?2
             WHERE id = ?3 AND deleted_at IS NULL",
            params![ts, device_id, id],
        )
        .map_err(|e| format!("删除文档失败: {}", e))?;

    if affected == 0 {
        return Err("文档不存在或已删除".to_string());
    }

    log_sync_op(&tx, &id, "delete", "", "", &device_id)?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
