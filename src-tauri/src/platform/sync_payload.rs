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

use rusqlite::Connection;

/// 读取本机全量文档和笔记。
///
/// 笔记与阅读器都已改用各设备独立自增编号的独立表，暂不参与同步；
/// 保留空数组以兼容旧版本的 payload 结构。
pub fn read_local_payload(
    _conn: &Connection,
    device_id: &str,
    device_name: &str,
) -> Result<SyncPayload, String> {
    Ok(SyncPayload {
        documents: Vec::new(),
        notes: Vec::new(),
        sender_device_id: device_id.to_string(),
        sender_device_name: device_name.to_string(),
    })
}

/// 合并收到的数据到本地数据库
/// 返回 (accepted, rejected, merged)
pub fn merge_payload(
    _conn: &mut Connection,
    incoming: &SyncPayload,
) -> Result<(usize, usize, usize), String> {
    // 旧客户端发来的 documents 一律忽略（旧表已迁移并删除）
    Ok((0, incoming.documents.len(), 0))
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
