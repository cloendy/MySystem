use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// 书籍编号上限（十位补零）
const MAX_BOOK_ID: i64 = 9_999_999_999;
/// 标注编号上限（十位补零）
const MAX_ANNOTATION_ID: i64 = 9_999_999_999;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub id: String,
    pub title: String,
    pub author: String,
    pub cover_url: String,
    pub file_path: String,
    pub file_format: String,
    /// 源文件内容 SHA-256（用于导入去重，迁移的旧数据为空）
    pub hash: String,
    pub total_pages: i64,
    pub current_page: i64,
    pub reading_status: String,
    /// 本地时间字符串（UTC，`YYYY-MM-DD HH:MM:SS.mmm`）
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
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
    /// 本地时间字符串（UTC，`YYYY-MM-DD HH:MM:SS.mmm`）
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
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
    pub hash: Option<String>,
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
    pub total_pages: Option<i64>,
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

/// 阅读器表使用 DATETIME 文本存储（UTC），格式固定便于字符串排序。
const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

fn now_datetime() -> String {
    chrono::Utc::now().format(DATETIME_FORMAT).to_string()
}

/// 把旧表的毫秒时间戳转换成 DATETIME 文本。
fn ms_to_datetime(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|dt| dt.format(DATETIME_FORMAT).to_string())
        .unwrap_or_default()
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
        hash: row.get(6)?,
        total_pages: row.get(7)?,
        current_page: row.get(8)?,
        reading_status: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
        deleted_at: row.get(12)?,
        version: row.get(13)?,
        device_id: row.get(14)?,
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
    SELECT Book_ID, Book_Title, Book_Author, IFNULL(Cover_Url, ''), File_Path, File_Format,
           Book_Hash, Total_Pages, Current_Page, Reading_Status,
           Create_Time, Update_Time, Delete_Time, Version, Device_ID
    FROM Read_Books
    WHERE Delete_Time IS NULL
";

fn table_exists(conn: &Connection, name: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        params![name],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

/// 阅读器模块初始化：把旧的 `documents` / `books` / `annotations` 迁移到新表并删除旧表。
pub fn init(app: &tauri::AppHandle) -> Result<(), String> {
    let mut conn = open_conn(app)?;
    migrate_legacy_reader(&mut conn)
}

/// 旧阅读器表迁移：
/// - 按 `created_at` 重新编号为十位补零的 `Book_ID` / `Annotation_ID`；
/// - 旧数据没有内容哈希，`Book_Hash` 留空，不参与去重；
/// - 迁移成功后删除 `annotations`、`books`、`documents`（外键顺序）；失败整体回滚。
fn migrate_legacy_reader(conn: &mut Connection) -> Result<(), String> {
    if !table_exists(conn, "documents")? {
        return Ok(());
    }

    let has_books = table_exists(conn, "books")?;
    let has_annotations = table_exists(conn, "annotations")?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // 旧书籍：documents(type='book') JOIN books
    let legacy_books: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        i64,
        String,
        i64,
        i64,
        Option<i64>,
        i64,
        String,
    )> = if has_books {
        let mut stmt = tx
            .prepare(
                "SELECT d.id, d.title, IFNULL(b.author, ''), IFNULL(b.cover_url, ''),
                        b.file_path, b.file_format, IFNULL(b.total_pages, 0),
                        IFNULL(b.current_page, 0), IFNULL(b.reading_status, 'unread'),
                        d.created_at, d.updated_at, d.deleted_at, d.version, d.device_id
                 FROM documents d
                 INNER JOIN books b ON b.document_id = d.id
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
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    } else {
        Vec::new()
    };

    let mut id_map: HashMap<String, String> = HashMap::new();
    let mut book_count = 0usize;
    for (idx, b) in legacy_books.iter().enumerate() {
        let new_id = format!("{:010}", idx as i64 + 1);
        id_map.insert(b.0.clone(), new_id.clone());
        tx.execute(
            "INSERT INTO Read_Books
             (Book_ID, Book_Title, Book_Author, Book_Hash, File_Path, File_Format,
              Total_Pages, Current_Page, Reading_Status, Cover_Url,
              Create_Time, Update_Time, Delete_Time, Device_ID, Version)
             VALUES (?1, ?2, ?3, '', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                new_id,
                b.1,
                b.2,
                b.4,
                b.5,
                b.6,
                b.7,
                b.8,
                if b.3.is_empty() { None } else { Some(&b.3) },
                ms_to_datetime(b.9),
                ms_to_datetime(b.10),
                b.11.map(ms_to_datetime),
                b.13,
                b.12
            ],
        )
        .map_err(|e| format!("迁移书籍失败: {}", e))?;
        book_count += 1;
    }

    // 旧标注：documents(type='annotation') JOIN annotations
    let mut annotation_count = 0usize;
    let mut skipped_count = 0usize;
    if has_annotations {
        let legacy_annotations: Vec<(
            String,
            String,
            String,
            String,
            i64,
            String,
            String,
            i64,
            i64,
            Option<i64>,
            i64,
            String,
        )> = {
            let mut stmt = tx
                .prepare(
                    "SELECT d.id, a.book_id, a.text, IFNULL(a.note, ''),
                            IFNULL(a.page, 0), IFNULL(a.position, ''),
                            IFNULL(a.color, '#FFEB3B'),
                            d.created_at, d.updated_at, d.deleted_at, d.version, d.device_id
                     FROM documents d
                     INNER JOIN annotations a ON a.document_id = d.id
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
                        row.get(11)?,
                    ))
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        for (idx, a) in legacy_annotations.iter().enumerate() {
            let Some(new_book_id) = id_map.get(&a.1).cloned() else {
                skipped_count += 1;
                continue;
            };
            let new_id = format!("{:010}", idx as i64 + 1);
            tx.execute(
                "INSERT INTO Read_Annotation
                 (Annotation_ID, Book_ID, Annotation_Text, Annotation_Note, Page, Position, Color,
                  Create_Time, Update_Time, Delete_Time, Device_ID, Version)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    new_id,
                    new_book_id,
                    a.2,
                    a.3,
                    a.4,
                    a.5,
                    a.6,
                    ms_to_datetime(a.7),
                    ms_to_datetime(a.8),
                    a.9.map(ms_to_datetime),
                    a.11,
                    a.10
                ],
            )
            .map_err(|e| format!("迁移标注失败: {}", e))?;
            annotation_count += 1;
        }
    }

    // 先删子表再删 documents，避免外键检查失败
    if has_annotations {
        tx.execute("DROP TABLE annotations", [])
            .map_err(|e| format!("删除旧标注表失败: {}", e))?;
    }
    if has_books {
        tx.execute("DROP TABLE books", [])
            .map_err(|e| format!("删除旧书籍表失败: {}", e))?;
    }
    tx.execute("DROP TABLE documents", [])
        .map_err(|e| format!("删除旧文档表失败: {}", e))?;

    tx.commit().map_err(|e| e.to_string())?;

    crate::log_info!(
        "[Reader] 阅读器表迁移完成: {} 本书 / {} 条标注 -> Read_Books / Read_Annotation",
        book_count,
        annotation_count
    );
    if skipped_count > 0 {
        crate::log_warn!("[Reader] 迁移时跳过 {} 条找不到所属书籍的标注", skipped_count);
    }
    Ok(())
}

/// 生成下一个十位补零的书籍编号。
fn next_book_id(conn: &Connection) -> Result<String, String> {
    let max: Option<i64> = conn
        .query_row(
            "SELECT MAX(CAST(Book_ID AS INTEGER)) FROM Read_Books",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let next = max.unwrap_or(0) + 1;
    if next > MAX_BOOK_ID {
        return Err("书籍编号已用完（最多 9999999999 本）".to_string());
    }
    Ok(format!("{:010}", next))
}

/// 生成下一个十位补零的标注编号。
fn next_annotation_id(conn: &Connection) -> Result<String, String> {
    let max: Option<i64> = conn
        .query_row(
            "SELECT MAX(CAST(Annotation_ID AS INTEGER)) FROM Read_Annotation",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let next = max.unwrap_or(0) + 1;
    if next > MAX_ANNOTATION_ID {
        return Err("标注编号已用完（最多 9999999999 条）".to_string());
    }
    Ok(format!("{:010}", next))
}

#[tauri::command]
pub fn create_book(app: tauri::AppHandle, input: CreateBookInput) -> Result<Book, String> {
    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let id = next_book_id(&tx)?;
    let ts = now_datetime();
    let author = input.author.unwrap_or_default();
    let cover_url = input.cover_url.unwrap_or_default();
    let hash = input.hash.unwrap_or_default();
    let total_pages = input.total_pages.unwrap_or(0);

    tx.execute(
        "INSERT INTO Read_Books
         (Book_ID, Book_Title, Book_Author, Book_Hash, File_Path, File_Format,
          Total_Pages, Current_Page, Reading_Status, Cover_Url,
          Create_Time, Update_Time, Delete_Time, Device_ID, Version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 'unread', ?8, ?9, ?9, NULL, ?10, 1)",
        params![
            id,
            input.title,
            author,
            hash,
            input.file_path,
            input.file_format,
            total_pages,
            cover_url,
            ts,
            input.device_id
        ],
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
        hash,
        total_pages,
        current_page: 0,
        reading_status: "unread".to_string(),
        created_at: ts.clone(),
        updated_at: ts,
        deleted_at: None,
        version: 1,
        device_id: input.device_id,
    })
}

#[tauri::command]
pub fn get_book(app: tauri::AppHandle, id: String) -> Result<Option<Book>, String> {
    let conn = open_conn(&app)?;
    let sql = format!("{} AND Book_ID = ?1", SELECT_BOOK);
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
    let sql = format!("{} ORDER BY Update_Time DESC", SELECT_BOOK);
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

    let ts = now_datetime();
    let affected = tx
        .execute(
            "UPDATE Read_Books
             SET Book_Title = COALESCE(?1, Book_Title),
                 Book_Author = COALESCE(?2, Book_Author),
                 Current_Page = COALESCE(?3, Current_Page),
                 Total_Pages = COALESCE(?4, Total_Pages),
                 Reading_Status = COALESCE(?5, Reading_Status),
                 Update_Time = ?6,
                 Version = Version + 1,
                 Device_ID = ?7
             WHERE Book_ID = ?8 AND Delete_Time IS NULL",
            params![
                input.title,
                input.author,
                input.current_page,
                input.total_pages,
                input.reading_status,
                ts,
                input.device_id,
                input.id
            ],
        )
        .map_err(|e| e.to_string())?;

    if affected == 0 {
        return Err("书籍不存在".to_string());
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

    let ts = now_datetime();
    let affected = tx
        .execute(
            "UPDATE Read_Books
             SET Delete_Time = ?1, Update_Time = ?1, Version = Version + 1, Device_ID = ?2
             WHERE Book_ID = ?3 AND Delete_Time IS NULL",
            params![ts, device_id, id],
        )
        .map_err(|e| format!("删除书籍失败: {}", e))?;

    if affected == 0 {
        return Err("书籍不存在".to_string());
    }

    // 级联软删除该书的标注
    tx.execute(
        "UPDATE Read_Annotation
         SET Delete_Time = ?1, Update_Time = ?1, Version = Version + 1, Device_ID = ?2
         WHERE Book_ID = ?3 AND Delete_Time IS NULL",
        params![ts, device_id, id],
    )
    .map_err(|e| format!("删除书籍标注失败: {}", e))?;

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

    let id = next_annotation_id(&tx)?;
    let ts = now_datetime();
    let note = input.note.unwrap_or_default();
    let position = input.position.unwrap_or_else(|| "{}".to_string());
    let color = input.color.unwrap_or_else(|| "#FFEB3B".to_string());

    tx.execute(
        "INSERT INTO Read_Annotation
         (Annotation_ID, Book_ID, Annotation_Text, Annotation_Note, Page, Position, Color,
          Create_Time, Update_Time, Delete_Time, Device_ID, Version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, NULL, ?9, 1)",
        params![
            id,
            input.book_id,
            input.text,
            note,
            input.page,
            position,
            color,
            ts,
            input.device_id
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
        created_at: ts.clone(),
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
            "SELECT Annotation_ID, Book_ID, Annotation_Text, Annotation_Note, Page, Position, Color,
                    Create_Time, Update_Time, Delete_Time, Version, Device_ID
             FROM Read_Annotation
             WHERE Book_ID = ?1 AND Delete_Time IS NULL
             ORDER BY Page ASC, Create_Time ASC",
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
    let ts = now_datetime();
    let affected = conn
        .execute(
            "UPDATE Read_Annotation
             SET Delete_Time = ?1, Update_Time = ?1, Version = Version + 1, Device_ID = ?2
             WHERE Annotation_ID = ?3 AND Delete_Time IS NULL",
            params![ts, device_id, id],
        )
        .map_err(|e| e.to_string())?;
    if affected == 0 {
        return Err("标注不存在".to_string());
    }
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedBookResult {
    pub file_path: String,
    pub file_format: String,
    pub title: String,
    /// 源文件 SHA-256
    pub hash: String,
    /// 是否命中已存在的书籍（命中时不会复制文件）
    pub duplicate: bool,
    /// 命中时返回已有书籍
    pub existing: Option<Book>,
}

/// 流式计算文件 SHA-256
fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("读取源文件失败 {}: {}", path.display(), e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("读取源文件失败: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// 复制外部文件到统一数据目录下的 books 子目录；内容哈希命中已有书籍时不复制。
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

    let hash = sha256_file(src)?;

    // 命中同内容书籍：不复制文件，交给前端让用户选择
    let conn = open_conn(&app)?;
    let existing: Option<Book> = {
        let sql = format!("{} AND Book_Hash = ?1", SELECT_BOOK);
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        stmt.query_row(params![hash], row_to_book)
            .optional()
            .map_err(|e| e.to_string())?
    };

    if let Some(existing) = existing {
        crate::log_info!("[Import] 命中已有书籍: {} ({})", existing.title, existing.id);
        return Ok(ImportedBookResult {
            file_path: existing.file_path.clone(),
            file_format: existing.file_format.clone(),
            title,
            hash,
            duplicate: true,
            existing: Some(existing),
        });
    }

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
        hash,
        duplicate: false,
        existing: None,
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
