pub mod folders;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub folder_id: Option<String>,
    pub is_pinned: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    pub version: i64,
    pub device_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteInput {
    pub title: String,
    pub content: Option<String>,
    pub folder_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub device_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteInput {
    pub id: String,
    pub title: Option<String>,
    pub content: Option<String>,
    pub folder_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_pinned: Option<bool>,
    pub device_id: String,
}

fn open_conn(app: &tauri::AppHandle) -> Result<Connection, String> {
    let path = crate::platform::paths::db_path(app)?;
    Connection::open(path).map_err(|e| format!("打开数据库失败: {}", e))
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// 初始化记事本模块数据：迁移旧文件夹表，并确保默认文件夹「未命名」存在。
pub fn init(app: &tauri::AppHandle) -> Result<(), String> {
    let mut conn = open_conn(app)?;
    folders::migrate_legacy_folders(&mut conn)?;
    folders::ensure_default_folder(&conn)
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

fn row_to_note(row: &rusqlite::Row) -> rusqlite::Result<Note> {
    let tags_json: String = row.get(3)?;
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    Ok(Note {
        id: row.get(0)?,
        title: row.get(1)?,
        content: row.get(2)?,
        tags,
        folder_id: row.get(4)?,
        is_pinned: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        deleted_at: row.get(8)?,
        version: row.get(9)?,
        device_id: row.get(10)?,
    })
}

const SELECT_NOTE: &str = "
    SELECT d.id, d.title, n.content, n.tags, n.folder_id, n.is_pinned,
           d.created_at, d.updated_at, d.deleted_at, d.version, d.device_id
    FROM documents d
    INNER JOIN notes n ON n.document_id = d.id
    WHERE d.deleted_at IS NULL
";

#[tauri::command]
pub fn create_note(app: tauri::AppHandle, input: CreateNoteInput) -> Result<Note, String> {
    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let id = Uuid::new_v4().to_string();
    let ts = now_ms();
    let content = input.content.unwrap_or_default();
    let tags = input.tags.unwrap_or_default();
    let tags_json = serde_json::to_string(&tags).map_err(|e| e.to_string())?;
    // 未指定文件夹时统一放入默认文件夹「未命名」
    let folder_id = input
        .folder_id
        .clone()
        .or_else(|| Some(folders::DEFAULT_FOLDER_ID.to_string()));

    tx.execute(
        "INSERT INTO documents (id, type, title, created_at, updated_at, deleted_at, version, device_id)
         VALUES (?1, 'note', ?2, ?3, ?3, NULL, 1, ?4)",
        params![id, input.title, ts, input.device_id],
    )
    .map_err(|e| format!("插入文档失败: {}", e))?;

    tx.execute(
        "INSERT INTO notes (document_id, content, tags, folder_id, is_pinned)
         VALUES (?1, ?2, ?3, ?4, 0)",
        params![id, content, tags_json, folder_id],
    )
    .map_err(|e| format!("插入笔记失败: {}", e))?;

    log_sync_op(&tx, &id, "create", "title", &input.title, &input.device_id)?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(Note {
        id,
        title: input.title,
        content,
        tags,
        folder_id,
        is_pinned: false,
        created_at: ts,
        updated_at: ts,
        deleted_at: None,
        version: 1,
        device_id: input.device_id,
    })
}

#[tauri::command]
pub fn get_note(app: tauri::AppHandle, id: String) -> Result<Option<Note>, String> {
    let conn = open_conn(&app)?;
    let sql = format!("{} AND d.id = ?1", SELECT_NOTE);
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let result = stmt
        .query_row(params![id], row_to_note)
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
pub fn list_notes(app: tauri::AppHandle, folder_id: Option<String>) -> Result<Vec<Note>, String> {
    let conn = open_conn(&app)?;

    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::ToSql>>) = match folder_id {
        Some(fid) => (
            format!(
                "{} AND n.folder_id = ?1 ORDER BY n.is_pinned DESC, d.updated_at DESC",
                SELECT_NOTE
            ),
            vec![Box::new(fid)],
        ),
        None => (
            format!(
                "{} ORDER BY n.is_pinned DESC, d.updated_at DESC",
                SELECT_NOTE
            ),
            vec![],
        ),
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_ref: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

    let rows = stmt
        .query_map(params_ref.as_slice(), row_to_note)
        .map_err(|e| e.to_string())?;
    let mut notes = Vec::new();
    for row in rows {
        notes.push(row.map_err(|e| e.to_string())?);
    }
    Ok(notes)
}

#[tauri::command]
pub fn update_note(app: tauri::AppHandle, input: UpdateNoteInput) -> Result<Note, String> {
    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let ts = now_ms();

    if let Some(title) = &input.title {
        tx.execute(
            "UPDATE documents SET title = ?1, updated_at = ?2, version = version + 1, device_id = ?3
             WHERE id = ?4 AND deleted_at IS NULL",
            params![title, ts, input.device_id, input.id],
        )
        .map_err(|e| e.to_string())?;
    } else {
        tx.execute(
            "UPDATE documents SET updated_at = ?1, version = version + 1, device_id = ?2 WHERE id = ?3",
            params![ts, input.device_id, input.id],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Some(content) = &input.content {
        tx.execute(
            "UPDATE notes SET content = ?1 WHERE document_id = ?2",
            params![content, input.id],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Some(tags) = &input.tags {
        let tags_json = serde_json::to_string(tags).map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE notes SET tags = ?1 WHERE document_id = ?2",
            params![tags_json, input.id],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Some(folder_id) = &input.folder_id {
        tx.execute(
            "UPDATE notes SET folder_id = ?1 WHERE document_id = ?2",
            params![folder_id, input.id],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Some(is_pinned) = input.is_pinned {
        tx.execute(
            "UPDATE notes SET is_pinned = ?1 WHERE document_id = ?2",
            params![is_pinned as i64, input.id],
        )
        .map_err(|e| e.to_string())?;
    }

    log_sync_op(&tx, &input.id, "update", "", "", &input.device_id)?;
    tx.commit().map_err(|e| e.to_string())?;

    let note = get_note(app, input.id)?.ok_or_else(|| "笔记不存在".to_string())?;
    Ok(note)
}

#[tauri::command]
pub fn delete_note(app: tauri::AppHandle, id: String, device_id: String) -> Result<(), String> {
    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let ts = now_ms();
    let affected = tx
        .execute(
            "UPDATE documents SET deleted_at = ?1, updated_at = ?1, version = version + 1, device_id = ?2
             WHERE id = ?3 AND deleted_at IS NULL",
            params![ts, device_id, id],
        )
        .map_err(|e| format!("删除笔记失败: {}", e))?;

    if affected == 0 {
        return Err("笔记不存在或已删除".to_string());
    }

    log_sync_op(&tx, &id, "delete", "", "", &device_id)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn search_notes(app: tauri::AppHandle, keyword: String) -> Result<Vec<Note>, String> {
    let conn = open_conn(&app)?;

    if keyword.trim().is_empty() {
        return Ok(Vec::new());
    }

    // SQL LIKE 通配符：两端加 %
    let pattern = format!("%{}%", keyword);

    let sql = format!(
        "{} AND (d.title LIKE ?1 OR n.content LIKE ?1)
         ORDER BY n.is_pinned DESC, d.updated_at DESC",
        SELECT_NOTE
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![pattern], row_to_note)
        .map_err(|e| e.to_string())?;

    let mut notes = Vec::new();
    for row in rows {
        notes.push(row.map_err(|e| e.to_string())?);
    }
    Ok(notes)
}
