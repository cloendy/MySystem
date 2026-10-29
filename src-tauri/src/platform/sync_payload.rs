use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPayload {
    pub id: String,
    pub r#type: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    pub version: i64,
    pub device_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NotePayload {
    pub document_id: String,
    pub content: String,
    pub tags: String, // JSON 数组字符串
    pub folder_id: Option<String>,
    pub is_pinned: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SyncPayload {
    pub documents: Vec<DocumentPayload>,
    pub notes: Vec<NotePayload>,
    pub sender_device_id: String,
    pub sender_device_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    pub accepted: usize,
    pub rejected: usize,
    pub merged: usize,
    pub server_payload: SyncPayload,
}

use rusqlite::{Connection, params};

/// 从数据库读取全量文档和笔记
pub fn read_local_payload(
    conn: &Connection,
    device_id: &str,
    device_name: &str,
) -> Result<SyncPayload, String> {
    let mut doc_stmt = conn
        .prepare(
            "SELECT id, type, title, created_at, updated_at, deleted_at, version, device_id
             FROM documents
             WHERE type != 'note'",
        )
        .map_err(|e| e.to_string())?;
    let docs = doc_stmt
        .query_map([], |row| {
            Ok(DocumentPayload {
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
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(SyncPayload {
        documents: docs,
        // 笔记已改用独立的 Note_Content 表，跨设备 ID 会冲突，暂不参与同步；
        // 保留空数组以兼容旧版本的 payload 结构。
        notes: Vec::new(),
        sender_device_id: device_id.to_string(),
        sender_device_name: device_name.to_string(),
    })
}

/// 合并收到的数据到本地数据库
/// 返回 (accepted, rejected, merged)
pub fn merge_payload(
    conn: &mut Connection,
    incoming: &SyncPayload,
) -> Result<(usize, usize, usize), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut merged = 0usize;

    for doc in &incoming.documents {
        // 笔记已改用独立的 Note_Content 表，忽略旧客户端发来的笔记文档
        if doc.r#type == "note" {
            continue;
        }

        // 查询本地是否有该文档
        let local: Option<(i64, i64)> = tx
            .query_row(
                "SELECT updated_at, version FROM documents WHERE id = ?1",
                params![doc.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        match local {
            None => {
                // 本地没有，直接插入
                tx.execute(
                    "INSERT INTO documents (id, type, title, created_at, updated_at, deleted_at, version, device_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        doc.id, doc.r#type, doc.title, doc.created_at, doc.updated_at,
                        doc.deleted_at, doc.version, doc.device_id
                    ],
                )
                .map_err(|e| format!("插入文档失败: {}", e))?;

                accepted += 1;
            }
            Some((local_updated, _local_version)) => {
                if doc.updated_at > local_updated {
                    // 对方的更新，覆盖本地
                    tx.execute(
                        "UPDATE documents SET title = ?1, updated_at = ?2, deleted_at = ?3,
                                               version = ?4, device_id = ?5
                         WHERE id = ?6",
                        params![
                            doc.title,
                            doc.updated_at,
                            doc.deleted_at,
                            doc.version,
                            doc.device_id,
                            doc.id
                        ],
                    )
                    .map_err(|e| format!("更新文档失败: {}", e))?;

                    merged += 1;
                } else if doc.updated_at == local_updated {
                    // 时间相同，视为无变化
                    rejected += 1;
                } else {
                    // 本地更新，保留本地
                    rejected += 1;
                }
            }
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok((accepted, rejected, merged))
}

/// Tauri 命令：读取本机全量数据（用于发起方推送）
#[tauri::command]
pub fn get_sync_payload(
    app: tauri::AppHandle,
    device_id: String,
    device_name: String,
) -> Result<SyncPayload, String> {
    let path = crate::platform::paths::db_path(&app)?;
    let conn = Connection::open(path).map_err(|e| format!("打开数据库失败: {}", e))?;
    read_local_payload(&conn, &device_id, &device_name)
}

#[tauri::command]
pub fn apply_local_payload(
    app: tauri::AppHandle,
    payload: SyncPayload,
) -> Result<(usize, usize, usize), String> {
    let path = crate::platform::paths::db_path(&app)?;
    let mut conn = Connection::open(path).map_err(|e| format!("打开数据库失败: {}", e))?;
    merge_payload(&mut conn, &payload)
}
