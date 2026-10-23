pub mod folders;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 笔记标题最大长度（字符数），与 `Note_Content.Note_Title VARCHAR(20)` 对应。
pub const MAX_NOTE_TITLE_LEN: usize = 20;
/// 笔记编号上限：`Note_ID` 为 10 位补零字符串。
const MAX_NOTE_ID: i64 = 9_999_999_999;

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
    pub device_id: String,
    pub version: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteInput {
    pub title: String,
    pub content: Option<String>,
    pub folder_id: Option<String>,
    pub tags: Option<Vec<String>>,
    /// 笔记最后写入设备，创建/更新/删除时记录，同时写入同步操作日志。
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

fn validate_title(title: &str) -> Result<(), String> {
    if title.chars().count() > MAX_NOTE_TITLE_LEN {
        return Err(format!(
            "笔记标题不能超过 {} 个字符",
            MAX_NOTE_TITLE_LEN
        ));
    }
    Ok(())
}

/// 超长标题截断到 `MAX_NOTE_TITLE_LEN`，返回 (标题, 是否截断)。
fn truncate_title(title: &str) -> (String, bool) {
    if title.chars().count() <= MAX_NOTE_TITLE_LEN {
        (title.to_string(), false)
    } else {
        (
            title.chars().take(MAX_NOTE_TITLE_LEN).collect(),
            true,
        )
    }
}

/// 初始化记事本模块数据：迁移旧文件夹/笔记表，确保默认文件夹存在，并重建笔记路径。
pub fn init(app: &tauri::AppHandle) -> Result<(), String> {
    let mut conn = open_conn(app)?;
    ensure_note_content_columns(&conn)?;
    folders::migrate_legacy_folders(&mut conn)?;
    folders::ensure_default_folder(&conn)?;
    migrate_legacy_notes(&mut conn)?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    folders::refresh_note_paths(&tx)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// 老版本建出的 `Note_Content` 可能缺少后加的列，启动时补齐。
fn ensure_note_content_columns(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(Note_Content)")
        .map_err(|e| e.to_string())?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    if !columns.iter().any(|c| c == "Device_ID") {
        conn.execute(
            "ALTER TABLE Note_Content ADD COLUMN Device_ID TEXT NOT NULL DEFAULT ''",
            [],
        )
        .map_err(|e| format!("为 Note_Content 添加 Device_ID 失败: {}", e))?;
        crate::log_info!("[Notepad] Note_Content 已补充 Device_ID 列");
    }
    if !columns.iter().any(|c| c == "Version") {
        conn.execute(
            "ALTER TABLE Note_Content ADD COLUMN Version INTEGER NOT NULL DEFAULT 1",
            [],
        )
        .map_err(|e| format!("为 Note_Content 添加 Version 失败: {}", e))?;
        crate::log_info!("[Notepad] Note_Content 已补充 Version 列");
    }
    Ok(())
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        params![name],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

/// 把旧 `documents(type='note')` + `notes` 迁移到 `Note_Content`。
///
/// - 按 `created_at` 重新分配 10 位补零编号；
/// - 按当前 `folder_id` 计算 `Is_Folder` + `Folder1~3`；
/// - 超长标题截断并记录日志；
/// - 完成后删除 `type='note'` 的 documents 记录并 `DROP TABLE notes`。
fn migrate_legacy_notes(conn: &mut Connection) -> Result<(), String> {
    if !table_exists(conn, "notes")? {
        return Ok(());
    }

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let legacy_notes: Vec<(
        String,
        String,
        String,
        String,
        Option<String>,
        i64,
        i64,
        i64,
        Option<i64>,
        String,
        i64,
    )> = {
        let mut stmt = tx
            .prepare(
                "SELECT d.id, d.title, n.content, n.tags, n.folder_id, n.is_pinned,
                        d.created_at, d.updated_at, d.deleted_at, d.device_id, d.version
                 FROM documents d
                 INNER JOIN notes n ON n.document_id = d.id
                 ORDER BY d.created_at ASC, d.id ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    let mut next_id: i64 = 1;
    let mut truncated_count = 0usize;

    for (
        _document_id,
        title,
        content,
        tags,
        folder_id,
        is_pinned,
        created_at,
        updated_at,
        deleted_at,
        device_id,
        version,
    ) in legacy_notes
    {
        if next_id > MAX_NOTE_ID {
            return Err("笔记编号已用完（最多 9999999999 条）".to_string());
        }
        let note_id = format!("{:010}", next_id);
        next_id += 1;

        let (title, truncated) = truncate_title(&title);
        if truncated {
            truncated_count += 1;
        }

        let path = folders::resolve_folder_path(&tx, folder_id.as_deref())?
            .unwrap_or_default();
        let (is_folder, folder1, folder2, folder3) = path.columns();
        let pinned = if is_pinned != 0 { "1" } else { "0" };

        tx.execute(
            "INSERT INTO Note_Content
             (Note_ID, Note_Title, Note_Content, Tags, Create_Time, Update_Time, Delete_Time,
              Device_ID, Version, Is_Folder, Folder1, Folder2, Folder3, Is_Pinned)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                note_id,
                title,
                content,
                tags,
                created_at,
                updated_at,
                deleted_at,
                device_id,
                version,
                is_folder,
                folder1,
                folder2,
                folder3,
                pinned
            ],
        )
        .map_err(|e| format!("迁移笔记失败: {}", e))?;
    }

    // `notes` 通过外键引用 `documents(id)`，且 rusqlite 默认开启外键检查：
    // 必须先删除旧表（或先删 notes 行），再删除 documents 中 type='note' 的记录。
    tx.execute("DROP TABLE notes", [])
        .map_err(|e| format!("删除旧笔记表失败: {}", e))?;
    tx.execute("DELETE FROM documents WHERE type = 'note'", [])
        .map_err(|e| format!("删除旧笔记文档失败: {}", e))?;
    tx.commit().map_err(|e| e.to_string())?;

    crate::log_info!("[Notepad] 笔记表迁移完成: {} 条 -> Note_Content", next_id - 1);
    if truncated_count > 0 {
        crate::log_warn!(
            "[Notepad] 迁移时有 {} 条笔记标题超过 {} 字符，已截断",
            truncated_count,
            MAX_NOTE_TITLE_LEN
        );
    }
    Ok(())
}

fn log_sync_op(
    conn: &Connection,
    note_id: &str,
    operation: &str,
    field: &str,
    value: &str,
    device_id: &str,
) -> Result<(), String> {
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO sync_operations (id, document_id, operation, field, value, timestamp, device_id, synced)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
        params![id, note_id, operation, field, value, now_ms(), device_id],
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
        is_pinned: row.get::<_, String>(5)? == "1",
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        deleted_at: row.get(8)?,
        device_id: row.get(9)?,
        version: row.get(10)?,
    })
}

const SELECT_NOTE: &str = "
    SELECT Note_ID, Note_Title, Note_Content, Tags,
           CASE Is_Folder
               WHEN '0' THEN '0001'
               WHEN '1' THEN Folder1
               WHEN '2' THEN Folder2
               WHEN '3' THEN Folder3
           END,
           Is_Pinned, Create_Time, Update_Time, Delete_Time, Device_ID, Version
    FROM Note_Content
    WHERE Delete_Time IS NULL
";

/// 生成下一个 10 位补零编号（从 0000000001 开始）。
fn next_note_id(conn: &Connection) -> Result<String, String> {
    let max: Option<i64> = conn
        .query_row(
            "SELECT MAX(CAST(Note_ID AS INTEGER)) FROM Note_Content",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let next = max.unwrap_or(0) + 1;
    if next > MAX_NOTE_ID {
        return Err("笔记编号已用完（最多 9999999999 条）".to_string());
    }
    Ok(format!("{:010}", next))
}

#[tauri::command]
pub fn create_note(app: tauri::AppHandle, input: CreateNoteInput) -> Result<Note, String> {
    validate_title(&input.title)?;

    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let note_id = next_note_id(&tx)?;
    let ts = now_ms();
    let content = input.content.unwrap_or_default();
    let tags = input.tags.unwrap_or_default();
    let tags_json = serde_json::to_string(&tags).map_err(|e| e.to_string())?;
    // 未指定文件夹时统一放入默认文件夹「未命名」
    let folder_id = input
        .folder_id
        .clone()
        .unwrap_or_else(|| folders::DEFAULT_FOLDER_ID.to_string());
    let path = folders::resolve_folder_path(&tx, Some(&folder_id))?
        .ok_or_else(|| "文件夹不存在".to_string())?;
    let (is_folder, folder1, folder2, folder3) = path.columns();

    tx.execute(
        "INSERT INTO Note_Content
         (Note_ID, Note_Title, Note_Content, Tags, Create_Time, Update_Time, Delete_Time,
          Device_ID, Version, Is_Folder, Folder1, Folder2, Folder3, Is_Pinned)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, NULL, ?6, 1, ?7, ?8, ?9, ?10, '0')",
        params![
            &note_id,
            input.title,
            content,
            tags_json,
            ts,
            input.device_id,
            is_folder,
            folder1,
            folder2,
            folder3
        ],
    )
    .map_err(|e| format!("插入笔记失败: {}", e))?;

    log_sync_op(&tx, &note_id, "create", "title", &input.title, &input.device_id)?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(Note {
        id: note_id,
        title: input.title,
        content,
        tags,
        folder_id: Some(folder_id),
        is_pinned: false,
        created_at: ts,
        updated_at: ts,
        deleted_at: None,
        device_id: input.device_id,
        version: 1,
    })
}

#[tauri::command]
pub fn get_note(app: tauri::AppHandle, id: String) -> Result<Option<Note>, String> {
    let conn = open_conn(&app)?;
    let sql = format!("{} AND Note_ID = ?1", SELECT_NOTE);
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
        Some(fid) if fid == folders::DEFAULT_FOLDER_ID => (
            format!(
                "{} AND Is_Folder = '0' ORDER BY Is_Pinned DESC, Update_Time DESC",
                SELECT_NOTE
            ),
            vec![],
        ),
        Some(fid) => (
            format!(
                "{} AND ((Is_Folder = '1' AND Folder1 = ?1)
                       OR (Is_Folder = '2' AND Folder2 = ?1)
                       OR (Is_Folder = '3' AND Folder3 = ?1))
                 ORDER BY Is_Pinned DESC, Update_Time DESC",
                SELECT_NOTE
            ),
            vec![Box::new(fid)],
        ),
        None => (
            format!("{} ORDER BY Is_Pinned DESC, Update_Time DESC", SELECT_NOTE),
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
    if let Some(title) = &input.title {
        validate_title(title)?;
    }

    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let ts = now_ms();
    let affected = tx
        .execute(
            "UPDATE Note_Content
             SET Update_Time = ?1, Version = Version + 1, Device_ID = ?3
             WHERE Note_ID = ?2 AND Delete_Time IS NULL",
            params![ts, input.id, input.device_id],
        )
        .map_err(|e| e.to_string())?;
    if affected == 0 {
        return Err("笔记不存在或已删除".to_string());
    }

    if let Some(title) = &input.title {
        tx.execute(
            "UPDATE Note_Content SET Note_Title = ?1 WHERE Note_ID = ?2",
            params![title, input.id],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Some(content) = &input.content {
        tx.execute(
            "UPDATE Note_Content SET Note_Content = ?1 WHERE Note_ID = ?2",
            params![content, input.id],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Some(tags) = &input.tags {
        let tags_json = serde_json::to_string(tags).map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE Note_Content SET Tags = ?1 WHERE Note_ID = ?2",
            params![tags_json, input.id],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Some(folder_id) = &input.folder_id {
        let path = folders::resolve_folder_path(&tx, Some(folder_id))?
            .ok_or_else(|| "文件夹不存在".to_string())?;
        let (is_folder, folder1, folder2, folder3) = path.columns();
        tx.execute(
            "UPDATE Note_Content
             SET Is_Folder = ?1, Folder1 = ?2, Folder2 = ?3, Folder3 = ?4
             WHERE Note_ID = ?5",
            params![is_folder, folder1, folder2, folder3, input.id],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Some(is_pinned) = input.is_pinned {
        tx.execute(
            "UPDATE Note_Content SET Is_Pinned = ?1 WHERE Note_ID = ?2",
            params![if is_pinned { "1" } else { "0" }, input.id],
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
            "UPDATE Note_Content
             SET Delete_Time = ?1, Update_Time = ?1, Version = Version + 1, Device_ID = ?3
             WHERE Note_ID = ?2 AND Delete_Time IS NULL",
            params![ts, id, device_id],
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
        "{} AND (Note_Title LIKE ?1 OR Note_Content LIKE ?1)
         ORDER BY Is_Pinned DESC, Update_Time DESC",
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
