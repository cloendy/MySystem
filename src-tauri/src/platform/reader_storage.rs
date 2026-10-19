use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub id: String,
    pub title: String,
    pub author: String,
    pub cover_url: String,
    pub file_path: String,
    pub file_format: String,
    pub total_pages: i64,
    pub current_page: i64,
    pub reading_status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    pub version: i64,
    pub device_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    pub id: String,
    pub book_id: String,
    pub text: String,
    pub note: String,
    pub page: i64,
    pub position: String,
    pub color: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    pub version: i64,
    pub device_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBookInput {
    pub title: String,
    pub author: Option<String>,
    pub cover_url: Option<String>,
    pub file_path: String,
    pub file_format: String,
    pub total_pages: Option<i64>,
    pub device_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBookInput {
    pub id: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub current_page: Option<i64>,
    pub reading_status: Option<String>,
    pub device_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAnnotationInput {
    pub book_id: String,
    pub text: String,
    pub note: Option<String>,
    pub page: i64,
    pub position: Option<String>,
    pub color: Option<String>,
    pub device_id: String,
}

fn open_conn(app: &tauri::AppHandle) -> Result<Connection, String> {
    let path = crate::platform::paths::db_path(app)?;
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

fn row_to_book(row: &rusqlite::Row) -> rusqlite::Result<Book> {
    Ok(Book {
        id: row.get(0)?,
        title: row.get(1)?,
        author: row.get(2)?,
        cover_url: row.get(3)?,
        file_path: row.get(4)?,
        file_format: row.get(5)?,
        total_pages: row.get(6)?,
        current_page: row.get(7)?,
        reading_status: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        deleted_at: row.get(11)?,
        version: row.get(12)?,
        device_id: row.get(13)?,
    })
}

fn row_to_annotation(row: &rusqlite::Row) -> rusqlite::Result<Annotation> {
    Ok(Annotation {
        id: row.get(0)?,
        book_id: row.get(1)?,
        text: row.get(2)?,
        note: row.get(3)?,
        page: row.get(4)?,
        position: row.get(5)?,
        color: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        deleted_at: row.get(9)?,
        version: row.get(10)?,
        device_id: row.get(11)?,
    })
}

const SELECT_BOOK: &str = "
    SELECT d.id, d.title, b.author, b.cover_url, b.file_path, b.file_format,
           b.total_pages, b.current_page, b.reading_status,
           d.created_at, d.updated_at, d.deleted_at, d.version, d.device_id
    FROM documents d
    INNER JOIN books b ON b.document_id = d.id
    WHERE d.deleted_at IS NULL
";

#[tauri::command]
pub fn create_book(app: tauri::AppHandle, input: CreateBookInput) -> Result<Book, String> {
    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let id = Uuid::new_v4().to_string();
    let ts = now_ms();
    let author = input.author.unwrap_or_default();
    let cover_url = input.cover_url.unwrap_or_default();
    let total_pages = input.total_pages.unwrap_or(0);

    tx.execute(
        "INSERT INTO documents (id, type, title, created_at, updated_at, deleted_at, version, device_id)
         VALUES (?1, 'book', ?2, ?3, ?3, NULL, 1, ?4)",
        params![id, input.title, ts, input.device_id],
    )
    .map_err(|e| format!("插入文档失败: {}", e))?;

    tx.execute(
        "INSERT INTO books (document_id, author, cover_url, file_path, file_format, total_pages, current_page, reading_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 'unread')",
        params![id, author, cover_url, input.file_path, input.file_format, total_pages],
    )
    .map_err(|e| format!("插入书籍失败: {}", e))?;

    log_sync_op(&tx, &id, "create", "title", &input.title, &input.device_id)?;
    tx.commit().map_err(|e| e.to_string())?;

    Ok(Book {
        id,
        title: input.title,
        author,
        cover_url,
        file_path: input.file_path,
        file_format: input.file_format,
        total_pages,
        current_page: 0,
        reading_status: "unread".to_string(),
        created_at: ts,
        updated_at: ts,
        deleted_at: None,
        version: 1,
        device_id: input.device_id,
    })
}

#[tauri::command]
pub fn get_book(app: tauri::AppHandle, id: String) -> Result<Option<Book>, String> {
    let conn = open_conn(&app)?;
    let sql = format!("{} AND d.id = ?1", SELECT_BOOK);
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let result = stmt
        .query_row(params![id], row_to_book)
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
pub fn list_books(app: tauri::AppHandle) -> Result<Vec<Book>, String> {
    let conn = open_conn(&app)?;
    let sql = format!("{} ORDER BY d.updated_at DESC", SELECT_BOOK);
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_book).map_err(|e| e.to_string())?;

    let mut books = Vec::new();
    for row in rows {
        books.push(row.map_err(|e| e.to_string())?);
    }
    Ok(books)
}

#[tauri::command]
pub fn update_book(app: tauri::AppHandle, input: UpdateBookInput) -> Result<Book, String> {
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

    if let Some(author) = &input.author {
        tx.execute(
            "UPDATE books SET author = ?1 WHERE document_id = ?2",
            params![author, input.id],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(current_page) = input.current_page {
        tx.execute(
            "UPDATE books SET current_page = ?1 WHERE document_id = ?2",
            params![current_page, input.id],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(status) = &input.reading_status {
        tx.execute(
            "UPDATE books SET reading_status = ?1 WHERE document_id = ?2",
            params![status, input.id],
        )
        .map_err(|e| e.to_string())?;
    }

    log_sync_op(&tx, &input.id, "update", "", "", &input.device_id)?;
    tx.commit().map_err(|e| e.to_string())?;

    let book = get_book(app, input.id)?.ok_or_else(|| "书籍不存在".to_string())?;
    Ok(book)
}

#[tauri::command]
pub fn delete_book(app: tauri::AppHandle, id: String, device_id: String) -> Result<(), String> {
    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let ts = now_ms();
    tx.execute(
        "UPDATE documents SET deleted_at = ?1, updated_at = ?1, version = version + 1, device_id = ?2
         WHERE id = ?3 AND deleted_at IS NULL",
        params![ts, device_id, id],
    )
    .map_err(|e| format!("删除书籍失败: {}", e))?;

    log_sync_op(&tx, &id, "delete", "", "", &device_id)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

// ============= 标注 =============

#[tauri::command]
pub fn create_annotation(
    app: tauri::AppHandle,
    input: CreateAnnotationInput,
) -> Result<Annotation, String> {
    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let id = Uuid::new_v4().to_string();
    let ts = now_ms();
    let note = input.note.unwrap_or_default();
    let position = input.position.unwrap_or_else(|| "{}".to_string());
    let color = input.color.unwrap_or_else(|| "#FFEB3B".to_string());

    tx.execute(
        "INSERT INTO documents (id, type, title, created_at, updated_at, deleted_at, version, device_id)
         VALUES (?1, 'annotation', ?2, ?3, ?3, NULL, 1, ?4)",
        params![id, input.text, ts, input.device_id],
    )
    .map_err(|e| format!("插入标注文档失败: {}", e))?;

    tx.execute(
        "INSERT INTO annotations (document_id, book_id, text, note, page, position, color)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            input.book_id,
            input.text,
            note,
            input.page,
            position,
            color
        ],
    )
    .map_err(|e| format!("插入标注失败: {}", e))?;

    log_sync_op(&tx, &id, "create", "text", &input.text, &input.device_id)?;
    tx.commit().map_err(|e| e.to_string())?;

    Ok(Annotation {
        id,
        book_id: input.book_id,
        text: input.text,
        note,
        page: input.page,
        position,
        color,
        created_at: ts,
        updated_at: ts,
        deleted_at: None,
        version: 1,
        device_id: input.device_id,
    })
}

#[tauri::command]
pub fn list_annotations(app: tauri::AppHandle, book_id: String) -> Result<Vec<Annotation>, String> {
    let conn = open_conn(&app)?;
    let mut stmt = conn
        .prepare(
            "SELECT d.id, a.book_id, a.text, a.note, a.page, a.position, a.color,
                    d.created_at, d.updated_at, d.deleted_at, d.version, d.device_id
             FROM documents d
             INNER JOIN annotations a ON a.document_id = d.id
             WHERE d.deleted_at IS NULL AND a.book_id = ?1
             ORDER BY a.page ASC, d.created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![book_id], row_to_annotation)
        .map_err(|e| e.to_string())?;
    let mut anns = Vec::new();
    for row in rows {
        anns.push(row.map_err(|e| e.to_string())?);
    }
    Ok(anns)
}

#[tauri::command]
pub fn delete_annotation(
    app: tauri::AppHandle,
    id: String,
    device_id: String,
) -> Result<(), String> {
    let conn = open_conn(&app)?;
    let ts = now_ms();
    conn.execute(
        "UPDATE documents SET deleted_at = ?1, updated_at = ?1, version = version + 1, device_id = ?2
         WHERE id = ?3",
        params![ts, device_id, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn read_book_file(file_path: String) -> Result<Vec<u8>, String> {
    std::fs::read(&file_path).map_err(|e| format!("读取文件失败: {}", e))
}

use std::path::Path;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedBookResult {
    pub file_path: String,
    pub file_format: String,
    pub title: String,
}

/// 复制外部文件到统一数据目录下的 books 子目录，返回新路径
#[tauri::command]
pub fn import_book_file(
    app: tauri::AppHandle,
    source_path: String,
) -> Result<ImportedBookResult, String> {
    let src = Path::new(&source_path);
    if !src.exists() {
        return Err(format!("源文件不存在: {}", source_path));
    }

    // 取文件扩展名
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .ok_or_else(|| "无法识别文件扩展名".to_string())?;

    let format = match ext.as_str() {
        "epub" => "epub",
        "pdf" => "pdf",
        "mobi" => "mobi",
        "txt" => "txt",
        other => return Err(format!("不支持的文件格式: {}", other)),
    };

    // 文件名（不含扩展名）作为默认标题
    let title = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("未命名")
        .to_string();

    // 统一数据目录下的 books 子目录
    let books_dir = get_books_dir(&app)?;
    std::fs::create_dir_all(&books_dir).map_err(|e| format!("创建目录失败: {}", e))?;

    // 生成唯一文件名，避免冲突
    let file_id = Uuid::new_v4().to_string();
    let dst = books_dir.join(format!("{}.{}", file_id, ext));

    std::fs::copy(src, &dst).map_err(|e| format!("复制文件失败: {}", e))?;

    crate::log_info!("[Import] 已导入 {} -> {}", source_path, dst.display());

    Ok(ImportedBookResult {
        file_path: dst.to_string_lossy().to_string(),
        file_format: format.to_string(),
        title,
    })
}

/// 删除已导入的书籍文件（仅允许删除 data/books 下的文件）
#[tauri::command]
pub fn delete_book_file(app: tauri::AppHandle, file_path: String) -> Result<(), String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Ok(());
    }

    let books_dir = crate::platform::paths::books_dir(&app)?;
    let canonical_file = path
        .canonicalize()
        .map_err(|e| format!("无法解析文件路径: {}", e))?;
    let canonical_books = books_dir
        .canonicalize()
        .map_err(|e| format!("无法解析 books 目录: {}", e))?;

    if !canonical_file.starts_with(&canonical_books) {
        return Err("拒绝删除 data/books 目录之外的文件".to_string());
    }

    std::fs::remove_file(path).map_err(|e| format!("删除文件失败: {}", e))?;
    crate::log_info!("[Import] 已删除文件: {}", file_path);
    Ok(())
}

fn get_books_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    crate::platform::paths::books_dir(app)
}
