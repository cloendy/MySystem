use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};

/// 默认文件夹「未命名」：固定 id，层级 0，排序 0。
pub const DEFAULT_FOLDER_ID: &str = "0001";
pub const DEFAULT_FOLDER_NAME: &str = "未命名";
pub const DEFAULT_FOLDER_LEVEL: i64 = 0;
/// 文件夹最大嵌套层数：1 / 2 / 3，层级 3 之下不能再建文件夹。
pub const MAX_FOLDER_DEPTH: i64 = 3;
/// 文件夹名称最大长度（字符数）。
pub const MAX_FOLDER_NAME_LEN: usize = 20;
/// 旧版默认文件夹 id，仅用于迁移。
const LEGACY_UNCATEGORIZED_ID: &str = "uncategorized";

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
    pub name: String,
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

/// 校验文件夹名称：去空格后非空且不超过 20 个字符。
fn normalize_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("文件夹名称不能为空".to_string());
    }
    if name.chars().count() > MAX_FOLDER_NAME_LEN {
        return Err(format!("文件夹名称不能超过 {} 个字符", MAX_FOLDER_NAME_LEN));
    }
    Ok(name)
}

/// 读取文件夹层级；不存在时返回 None。
fn folder_level(conn: &Connection, id: &str) -> Result<Option<i64>, String> {
    conn.query_row(
        "SELECT CAST(Folder_Level AS INTEGER) FROM Note_Folders WHERE Folder_ID = ?1",
        params![id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn folder_name(conn: &Connection, id: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT Folder_Name FROM Note_Folders WHERE Folder_ID = ?1",
        params![id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn folder_parent(conn: &Connection, id: &str) -> Result<Option<String>, String> {
    let parent: Option<Option<String>> = conn
        .query_row(
            "SELECT Folder_Parent_ID FROM Note_Folders WHERE Folder_ID = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(parent.flatten())
}

/// 判断表是否存在，用于兼容旧表迁移。
fn table_exists(conn: &Connection, name: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        params![name],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

/// 笔记所在文件夹路径，对应 `Note_Content` 的 `Is_Folder` + `Folder1~3`。
///
/// `level = 0` 表示默认文件夹「未命名」，此时三个 Folder 列都为空。
#[derive(Debug, Clone, Default)]
pub struct FolderPath {
    pub level: i64,
    pub folder1: Option<String>,
    pub folder2: Option<String>,
    pub folder3: Option<String>,
}

impl FolderPath {
    /// 返回可直接写入 SQL 的四列值（层级字符串 + 三个可空编号）。
    pub fn columns(&self) -> (String, Option<String>, Option<String>, Option<String>) {
        (
            self.level.to_string(),
            self.folder1.clone(),
            self.folder2.clone(),
            self.folder3.clone(),
        )
    }
}

/// 解析文件夹的完整路径。
///
/// - `folder_id` 为 `None` 或 `0001`：返回默认路径（层级 0，全空）；
/// - 文件夹不存在：返回 `Ok(None)`，由调用方决定报错还是回退默认；
/// - 否则沿父链向上取最多三级，返回 `Folder1`（最上层）~`Folder3`（当前层）。
pub fn resolve_folder_path(
    conn: &Connection,
    folder_id: Option<&str>,
) -> Result<Option<FolderPath>, String> {
    let Some(folder_id) = folder_id else {
        return Ok(Some(FolderPath::default()));
    };
    if folder_id == DEFAULT_FOLDER_ID {
        return Ok(Some(FolderPath::default()));
    }

    let Some(level) = folder_level(conn, folder_id)? else {
        return Ok(None);
    };
    if level == 0 {
        return Ok(Some(FolderPath::default()));
    }

    // 当前文件夹在前，最多向上追溯三级
    let mut chain = vec![folder_id.to_string()];
    let mut parent = folder_parent(conn, folder_id)?;
    while let Some(parent_id) = parent {
        if chain.len() >= MAX_FOLDER_DEPTH as usize {
            break;
        }
        chain.push(parent_id.clone());
        parent = folder_parent(conn, &parent_id)?;
    }

    let mut path = FolderPath {
        level: chain.len() as i64,
        ..Default::default()
    };
    match chain.len() {
        1 => path.folder1 = chain.first().cloned(),
        2 => {
            path.folder2 = chain.first().cloned();
            path.folder1 = chain.get(1).cloned();
        }
        _ => {
            path.folder3 = chain.first().cloned();
            path.folder2 = chain.get(1).cloned();
            path.folder1 = chain.get(2).cloned();
        }
    }
    Ok(Some(path))
}

/// 把位于 `from_folder_id` 的笔记整体改到 `target` 路径。
///
/// 只匹配「当前层级对应列」等于该文件夹 ID 的行，避免误伤位于其后代文件夹中的笔记。
pub fn move_notes_to_folder(
    tx: &Transaction,
    from_folder_id: &str,
    target: &FolderPath,
) -> Result<(), String> {
    let Some(level) = folder_level(tx, from_folder_id)? else {
        return Ok(());
    };

    let (level_text, folder1, folder2, folder3) = target.columns();
    if level == 0 {
        tx.execute(
            "UPDATE Note_Content
             SET Is_Folder = ?1, Folder1 = ?2, Folder2 = ?3, Folder3 = ?4
             WHERE Is_Folder = '0'",
            params![level_text, folder1, folder2, folder3],
        )
        .map_err(|e| format!("更新笔记文件夹失败: {}", e))?;
        return Ok(());
    }

    let level = level.clamp(1, MAX_FOLDER_DEPTH);
    let column = format!("Folder{}", level);
    let sql = format!(
        "UPDATE Note_Content
         SET Is_Folder = ?1, Folder1 = ?2, Folder2 = ?3, Folder3 = ?4
         WHERE Is_Folder = ?5 AND {} = ?6",
        column
    );
    tx.execute(
        &sql,
        params![
            level_text,
            folder1,
            folder2,
            folder3,
            level.to_string(),
            from_folder_id
        ],
    )
    .map_err(|e| format!("更新笔记文件夹失败: {}", e))?;
    Ok(())
}

/// 按 `Note_Content` 中记录的当前文件夹重建 `Is_Folder` + `Folder1~3`。
///
/// 文件夹层级变化（例如删除文件夹导致子文件夹整体上移）后调用。
pub fn refresh_note_paths(tx: &Transaction) -> Result<(), String> {
    let rows: Vec<(String, String, Option<String>, Option<String>, Option<String>)> = {
        let mut stmt = tx
            .prepare(
                "SELECT Note_ID, Is_Folder, Folder1, Folder2, Folder3 FROM Note_Content",
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
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    for (note_id, is_folder, folder1, folder2, folder3) in rows {
        // 反推当前所在文件夹；找不到时归入默认文件夹
        let current = match is_folder.as_str() {
            "1" => folder1,
            "2" => folder2,
            "3" => folder3,
            _ => None,
        };
        let path = match current.as_deref() {
            Some(id) => resolve_folder_path(tx, Some(id))?.unwrap_or_default(),
            None => FolderPath::default(),
        };
        let (level_text, f1, f2, f3) = path.columns();
        tx.execute(
            "UPDATE Note_Content
             SET Is_Folder = ?1, Folder1 = ?2, Folder2 = ?3, Folder3 = ?4
             WHERE Note_ID = ?5",
            params![level_text, f1, f2, f3, note_id],
        )
        .map_err(|e| format!("刷新笔记文件夹路径失败: {}", e))?;
    }
    Ok(())
}

/// 生成下一个四位编号（0001 起自增）。
fn next_folder_id(conn: &Connection) -> Result<String, String> {
    let max: Option<i64> = conn
        .query_row(
            "SELECT MAX(CAST(Folder_ID AS INTEGER)) FROM Note_Folders",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let next = max.unwrap_or(0) + 1;
    if next > 9999 {
        return Err("文件夹编号已用完（最多 9999 个）".to_string());
    }
    Ok(format!("{:04}", next))
}

/// 同级下一个排序号（默认文件夹占用 0，其余从 1 开始）。
fn next_order(conn: &Connection, parent_id: Option<&str>) -> Result<i64, String> {
    let max: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(CAST(Folder_Order AS INTEGER)), 0)
             FROM Note_Folders WHERE Folder_Parent_ID IS ?1",
            params![parent_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(max + 1)
}

fn duplicate_exists(
    conn: &Connection,
    name: &str,
    level: i64,
    parent_id: Option<&str>,
    exclude_id: Option<&str>,
) -> Result<bool, String> {
    let level = level.to_string();
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM Note_Folders
            WHERE Folder_Name = ?1
              AND Folder_Level = ?2
              AND Folder_Parent_ID IS ?3
              AND (?4 IS NULL OR Folder_ID != ?4)
         )",
        params![name, level, parent_id, exclude_id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

/// 确保默认文件夹「未命名」存在。
///
/// 笔记的文件夹路径由 `Note_Content.Is_Folder` + `Folder1~3` 冗余保存，
/// 无归属/指向不存在文件夹的笔记由 `refresh_note_paths()` 统一归入「未命名」。
pub fn ensure_default_folder(conn: &Connection) -> Result<(), String> {
    let level = DEFAULT_FOLDER_LEVEL.to_string();
    conn.execute(
        "INSERT OR IGNORE INTO Note_Folders
         (Folder_ID, Folder_Name, Folder_Level, Folder_Parent_ID, Folder_Parent_Name,
          Folder_Order, Folder_Flg1, Folder_Flg2, Folder_Flg3)
         VALUES (?1, ?2, ?3, NULL, NULL, '0', NULL, NULL, NULL)",
        params![DEFAULT_FOLDER_ID, DEFAULT_FOLDER_NAME, level],
    )
    .map_err(|e| format!("创建默认文件夹失败: {}", e))?;

    Ok(())
}

#[derive(Debug)]
struct LegacyFolder {
    id: String,
    name: String,
    parent_id: Option<String>,
    sort_order: i64,
}

struct NewFolderRow {
    id: String,
    name: String,
    level: i64,
    parent_id: Option<String>,
    parent_name: Option<String>,
    order: i64,
}

/// 把旧 `folders` 表迁移到 `Note_Folders`，随后删除旧表。
///
/// - 旧的「未分类」映射为固定的 `0001 / 未命名 / 层级 0`；
/// - 其余文件夹按原层级与顺序重新编号（0002 起），并同步更新 `notes.folder_id`；
/// - 只有旧表存在时才执行，整个过程在一个事务内完成。
pub fn migrate_legacy_folders(conn: &mut Connection) -> Result<(), String> {
    let legacy_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'folders')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !legacy_exists {
        return Ok(());
    }
    // 旧表升级时 notes 仍存在；若已完成笔记迁移，则只刷新 Note_Content 的路径。
    let legacy_notes_exists = table_exists(conn, "notes")?;

    let legacy: Vec<LegacyFolder> = {
        let mut stmt = conn
            .prepare("SELECT id, name, parent_id, sort_order FROM folders")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(LegacyFolder {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    parent_id: row.get(2)?,
                    sort_order: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    let known_ids: HashSet<String> = legacy.iter().map(|folder| folder.id.clone()).collect();
    let mut children: HashMap<String, Vec<usize>> = HashMap::new();
    let mut roots: Vec<usize> = Vec::new();

    for (index, folder) in legacy.iter().enumerate() {
        if folder.id == LEGACY_UNCATEGORIZED_ID {
            continue;
        }
        match &folder.parent_id {
            Some(parent_id)
                if parent_id != LEGACY_UNCATEGORIZED_ID && known_ids.contains(parent_id) =>
            {
                children.entry(parent_id.clone()).or_default().push(index);
            }
            _ => roots.push(index),
        }
    }

    let by_legacy_order = |left: &usize, right: &usize| {
        legacy[*left]
            .sort_order
            .cmp(&legacy[*right].sort_order)
            .then_with(|| legacy[*left].name.cmp(&legacy[*right].name))
    };
    roots.sort_by(by_legacy_order);
    for siblings in children.values_mut() {
        siblings.sort_by(by_legacy_order);
    }

    let mut mapping: HashMap<String, String> = HashMap::new();
    mapping.insert(
        LEGACY_UNCATEGORIZED_ID.to_string(),
        DEFAULT_FOLDER_ID.to_string(),
    );

    let mut new_rows: Vec<NewFolderRow> = Vec::new();
    let mut next_number: i64 = 2;

    // 显式栈做深度优先遍历，保证父文件夹先于子文件夹分配编号。
    let mut stack: Vec<(usize, i64, Option<String>, Option<String>, i64)> = roots
        .iter()
        .enumerate()
        .map(|(order, &index)| (index, 1, None, None, order as i64 + 1))
        .collect();
    stack.reverse();

    while let Some((index, level, parent_id, parent_name, order)) = stack.pop() {
        if level > MAX_FOLDER_DEPTH {
            return Err("历史文件夹嵌套超过 3 层，无法迁移".to_string());
        }
        if next_number > 9999 {
            return Err("文件夹编号超过 9999，无法迁移".to_string());
        }

        let old = &legacy[index];
        let new_id = format!("{:04}", next_number);
        next_number += 1;
        mapping.insert(old.id.clone(), new_id.clone());
        new_rows.push(NewFolderRow {
            id: new_id.clone(),
            name: old.name.clone(),
            level,
            parent_id: parent_id.clone(),
            parent_name: parent_name.clone(),
            order,
        });

        if let Some(siblings) = children.get(&old.id) {
            for (child_order, &child) in siblings.iter().enumerate().rev() {
                stack.push((
                    child,
                    level + 1,
                    Some(new_id.clone()),
                    Some(old.name.clone()),
                    child_order as i64 + 1,
                ));
            }
        }
    }

    let expected = legacy
        .iter()
        .filter(|folder| folder.id != LEGACY_UNCATEGORIZED_ID)
        .count();
    if new_rows.len() != expected {
        return Err("历史文件夹存在循环引用，无法迁移".to_string());
    }

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    for row in &new_rows {
        let level = row.level.to_string();
        let order = row.order.to_string();
        tx.execute(
            "INSERT INTO Note_Folders
             (Folder_ID, Folder_Name, Folder_Level, Folder_Parent_ID, Folder_Parent_Name,
              Folder_Order, Folder_Flg1, Folder_Flg2, Folder_Flg3)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL, NULL)",
            params![
                row.id,
                row.name,
                level,
                row.parent_id,
                row.parent_name,
                order
            ],
        )
        .map_err(|e| format!("迁移文件夹失败: {}", e))?;
    }

    if legacy_notes_exists {
        for (old_id, new_id) in &mapping {
            if old_id == LEGACY_UNCATEGORIZED_ID {
                continue;
            }
            tx.execute(
                "UPDATE notes SET folder_id = ?1 WHERE folder_id = ?2",
                params![new_id, old_id],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.execute(
            "UPDATE notes SET folder_id = ?1 WHERE folder_id = ?2",
            params![DEFAULT_FOLDER_ID, LEGACY_UNCATEGORIZED_ID],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.execute("DROP TABLE folders", [])
        .map_err(|e| format!("删除旧文件夹表失败: {}", e))?;

    // 若笔记已迁移到 Note_Content，按新的文件夹编号重建路径
    refresh_note_paths(&tx)?;

    tx.commit().map_err(|e| e.to_string())?;

    crate::log_info!(
        "[Notepad] 文件夹表迁移完成: {} 个文件夹 -> Note_Folders",
        new_rows.len()
    );
    Ok(())
}

#[tauri::command]
pub fn create_folder(app: tauri::AppHandle, input: CreateFolderInput) -> Result<Folder, String> {
    let name = normalize_name(&input.name)?;
    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let (level, parent_name) = match &input.parent_id {
        None => (1, None),
        Some(parent_id) => {
            if parent_id == DEFAULT_FOLDER_ID {
                return Err("「未命名」下不能创建子文件夹".to_string());
            }
            let parent_level =
                folder_level(&tx, parent_id)?.ok_or_else(|| "父文件夹不存在".to_string())?;
            if parent_level >= MAX_FOLDER_DEPTH {
                return Err(format!("文件夹最多嵌套 {} 层", MAX_FOLDER_DEPTH));
            }
            let parent_name =
                folder_name(&tx, parent_id)?.ok_or_else(|| "父文件夹不存在".to_string())?;
            (parent_level + 1, Some(parent_name))
        }
    };

    if duplicate_exists(&tx, name, level, input.parent_id.as_deref(), None)? {
        return Err("同级已存在同名文件夹".to_string());
    }

    let id = next_folder_id(&tx)?;
    let order = next_order(&tx, input.parent_id.as_deref())?;
    let level_text = level.to_string();
    let order_text = order.to_string();

    tx.execute(
        "INSERT INTO Note_Folders
         (Folder_ID, Folder_Name, Folder_Level, Folder_Parent_ID, Folder_Parent_Name,
          Folder_Order, Folder_Flg1, Folder_Flg2, Folder_Flg3)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL, NULL)",
        params![
            id,
            name,
            level_text,
            input.parent_id,
            parent_name,
            order_text
        ],
    )
    .map_err(|e| format!("创建文件夹失败: {}", e))?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(Folder {
        id,
        name: name.to_string(),
        parent_id: input.parent_id,
        sort_order: order,
    })
}

#[tauri::command]
pub fn list_folders(app: tauri::AppHandle) -> Result<Vec<Folder>, String> {
    let conn = open_conn(&app)?;
    let mut stmt = conn
        .prepare(
            "SELECT Folder_ID, Folder_Name, Folder_Parent_ID, CAST(Folder_Order AS INTEGER)
             FROM Note_Folders
             ORDER BY CAST(Folder_Order AS INTEGER) ASC, Folder_Name ASC",
        )
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
    if input.id == DEFAULT_FOLDER_ID {
        return Err("「未命名」是默认文件夹，不能修改".to_string());
    }

    let name = normalize_name(&input.name)?;
    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let (parent_id, level): (Option<String>, i64) = tx
        .query_row(
            "SELECT Folder_Parent_ID, CAST(Folder_Level AS INTEGER)
             FROM Note_Folders WHERE Folder_ID = ?1",
            params![input.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "文件夹不存在".to_string())?;

    if duplicate_exists(&tx, name, level, parent_id.as_deref(), Some(&input.id))? {
        return Err("同级已存在同名文件夹".to_string());
    }

    tx.execute(
        "UPDATE Note_Folders SET Folder_Name = ?1 WHERE Folder_ID = ?2",
        params![name, input.id],
    )
    .map_err(|e| e.to_string())?;

    // 子文件夹冗余保存了父文件夹名称，重命名时需要同步
    tx.execute(
        "UPDATE Note_Folders SET Folder_Parent_Name = ?1 WHERE Folder_Parent_ID = ?2",
        params![name, input.id],
    )
    .map_err(|e| e.to_string())?;

    let folder = tx
        .query_row(
            "SELECT Folder_ID, Folder_Name, Folder_Parent_ID, CAST(Folder_Order AS INTEGER)
             FROM Note_Folders WHERE Folder_ID = ?1",
            params![input.id],
            row_to_folder,
        )
        .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(folder)
}

/// 删除文件夹后，其后代层级整体上移一级。
fn shift_descendant_levels(
    tx: &Transaction,
    parent_id: &str,
    delta: i64,
) -> Result<(), String> {
    let children: Vec<String> = {
        let mut stmt = tx
            .prepare("SELECT Folder_ID FROM Note_Folders WHERE Folder_Parent_ID = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![parent_id], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    for child in children {
        tx.execute(
            "UPDATE Note_Folders
             SET Folder_Level = CAST(Folder_Level AS INTEGER) + ?1
             WHERE Folder_ID = ?2",
            params![delta, child],
        )
        .map_err(|e| e.to_string())?;
        shift_descendant_levels(tx, &child, delta)?;
    }
    Ok(())
}

/// 上移后与同级文件夹重名时，改名为 `原名_folder_被删文件夹名`；仍冲突则追加序号。
fn moved_folder_name(
    tx: &Transaction,
    name: &str,
    origin_name: &str,
    level: i64,
    parent_id: Option<&str>,
    folder_id: &str,
) -> Result<String, String> {
    if !duplicate_exists(tx, name, level, parent_id, Some(folder_id))? {
        return Ok(name.to_string());
    }

    let combined = format!("{}_folder_{}", name, origin_name);
    let base: String = combined.chars().take(MAX_FOLDER_NAME_LEN).collect();
    if !duplicate_exists(tx, &base, level, parent_id, Some(folder_id))? {
        return Ok(base);
    }

    for suffix_number in 2..1000 {
        let suffix = format!("({})", suffix_number);
        let keep = MAX_FOLDER_NAME_LEN.saturating_sub(suffix.chars().count());
        let stem: String = base.chars().take(keep).collect();
        let candidate = format!("{}{}", stem, suffix);
        if !duplicate_exists(tx, &candidate, level, parent_id, Some(folder_id))? {
            return Ok(candidate);
        }
    }
    Err("无法为移动的文件夹生成唯一名称".to_string())
}

#[tauri::command]
pub fn delete_folder(app: tauri::AppHandle, id: String) -> Result<(), String> {
    if id == DEFAULT_FOLDER_ID {
        return Err("「未命名」是默认文件夹，不能删除".to_string());
    }

    let mut conn = open_conn(&app)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let (parent_id, level, deleted_name): (Option<String>, i64, String) = tx
        .query_row(
            "SELECT Folder_Parent_ID, CAST(Folder_Level AS INTEGER), Folder_Name
             FROM Note_Folders WHERE Folder_ID = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "文件夹不存在".to_string())?;

    let parent_name = match &parent_id {
        Some(parent_id) => folder_name(&tx, parent_id)?,
        None => None,
    };

    let children: Vec<String> = {
        let mut stmt = tx
            .prepare(
                "SELECT Folder_ID FROM Note_Folders
                 WHERE Folder_Parent_ID = ?1
                 ORDER BY CAST(Folder_Order AS INTEGER) ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![id], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    // 文件夹下的笔记移动到其父文件夹；被删的是根级文件夹时归入「未命名」
    let note_target = parent_id
        .clone()
        .unwrap_or_else(|| DEFAULT_FOLDER_ID.to_string());
    let target_path = resolve_folder_path(&tx, Some(&note_target))?
        .unwrap_or_default();
    move_notes_to_folder(&tx, &id, &target_path)?;

    // 子文件夹上移一级，追加到新父级的末尾
    let base_order: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(CAST(Folder_Order AS INTEGER)), 0)
             FROM Note_Folders
             WHERE Folder_Parent_ID IS ?1 AND Folder_ID != ?2",
            params![parent_id, id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // 子文件夹上移到被删文件夹所在层级（删除根级文件夹时，子文件夹成为根级）
    let new_level = level;
    for (index, child) in children.iter().enumerate() {
        let child_name = folder_name(&tx, child)?.ok_or_else(|| "子文件夹不存在".to_string())?;
        let name = moved_folder_name(
            &tx,
            &child_name,
            &deleted_name,
            new_level,
            parent_id.as_deref(),
            child,
        )?;
        let level_text = new_level.to_string();
        let order_text = (base_order + index as i64 + 1).to_string();

        tx.execute(
            "UPDATE Note_Folders
             SET Folder_Parent_ID = ?1,
                 Folder_Parent_Name = ?2,
                 Folder_Level = ?3,
                 Folder_Order = ?4,
                 Folder_Name = ?5
             WHERE Folder_ID = ?6",
            params![parent_id, parent_name, level_text, order_text, name, child],
        )
        .map_err(|e| e.to_string())?;

        // 若子文件夹改名，其下一级的父文件夹名称同步更新
        tx.execute(
            "UPDATE Note_Folders SET Folder_Parent_Name = ?1 WHERE Folder_Parent_ID = ?2",
            params![name, child],
        )
        .map_err(|e| e.to_string())?;

        shift_descendant_levels(&tx, child, -1)?;
    }

    tx.execute("DELETE FROM Note_Folders WHERE Folder_ID = ?1", params![id])
        .map_err(|e| e.to_string())?;

    // 子文件夹层级整体上移后，其内部笔记的 Is_Folder/Folder1~3 需要重建
    refresh_note_paths(&tx)?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
