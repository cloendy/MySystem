use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFolderInput {
    pub name: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFolderInput {
    pub id: String,
    pub name: Option<String>,
    pub parent_id: Option<String>,
    pub sort_order: Option<i64>,
}

fn open_conn(app: &tauri::AppHandle) -> Result<Connection, String> {
    let path = crate::platform::paths::db_path(app)?;
    Connection::open(path).map_err(|e| format!("打开数据库失败: {}", e))
}

fn row_to_folder(row: &rusqlite::Row) -> rusqlite::Result<Folder> {
    Ok(Folder {
        id: row.get(0)?,
        name: row.get(1)?,
        parent_id: row.get(2)?,
        sort_order: row.get(3)?,
    })
}

#[tauri::command]
pub fn create_folder(app: tauri::AppHandle, input: CreateFolderInput) -> Result<Folder, String> {
    let conn = open_conn(&app)?;
    let id = Uuid::new_v4().to_string();

    // 获取同级最大 sort_order
    let max_order: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) FROM folders WHERE parent_id IS ?1",
            params![input.parent_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO folders (id, name, parent_id, sort_order) VALUES (?1, ?2, ?3, ?4)",
        params![id, input.name, input.parent_id, max_order + 1],
    )
    .map_err(|e| format!("创建文件夹失败: {}", e))?;

    Ok(Folder {
        id,
        name: input.name,
        parent_id: input.parent_id,
        sort_order: max_order + 1,
    })
}

#[tauri::command]
pub fn list_folders(app: tauri::AppHandle) -> Result<Vec<Folder>, String> {
    let conn = open_conn(&app)?;
    let mut stmt = conn
        .prepare("SELECT id, name, parent_id, sort_order FROM folders ORDER BY sort_order ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], row_to_folder)
        .map_err(|e| e.to_string())?;

    let mut folders = Vec::new();
    for row in rows {
        folders.push(row.map_err(|e| e.to_string())?);
    }
    Ok(folders)
}

#[tauri::command]
pub fn update_folder(app: tauri::AppHandle, input: UpdateFolderInput) -> Result<Folder, String> {
    let conn = open_conn(&app)?;

    if let Some(name) = &input.name {
        conn.execute(
            "UPDATE folders SET name = ?1 WHERE id = ?2",
            params![name, input.id],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(parent_id) = &input.parent_id {
        conn.execute(
            "UPDATE folders SET parent_id = ?1 WHERE id = ?2",
            params![parent_id, input.id],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(sort_order) = input.sort_order {
        conn.execute(
            "UPDATE folders SET sort_order = ?1 WHERE id = ?2",
            params![sort_order, input.id],
        )
        .map_err(|e| e.to_string())?;
    }

    let folder = conn
        .query_row(
            "SELECT id, name, parent_id, sort_order FROM folders WHERE id = ?1",
            params![input.id],
            row_to_folder,
        )
        .map_err(|e| e.to_string())?;
    Ok(folder)
}

#[tauri::command]
pub fn delete_folder(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // 该文件夹下的笔记归到"未分类"（folder_id 置空）
    tx.execute(
        "UPDATE notes SET folder_id = NULL WHERE folder_id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;

    // 子文件夹的 parent_id 置空（提升到根级别）
    tx.execute(
        "UPDATE folders SET parent_id = NULL WHERE parent_id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;

    tx.execute("DELETE FROM folders WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
