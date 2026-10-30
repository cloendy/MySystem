//! 数据库结构初始化。
//!
//! 建表统一由 Rust 在启动时完成，避免前端 SQL 插件把相对路径解析到 AppData。

use rusqlite::Connection;

/// 在统一 data 目录下的 `mydata.db` 中创建全部业务表。
pub fn init_database(app: &tauri::AppHandle) -> Result<(), String> {
    let path = crate::platform::paths::db_path(app)?;
    let conn = Connection::open(&path)
        .map_err(|e| format!("打开数据库失败 {}: {}", path.display(), e))?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS Read_Books (
          Book_ID VARCHAR(10) PRIMARY KEY NOT NULL,
          Book_Title TEXT NOT NULL DEFAULT '',
          Book_Author TEXT NOT NULL DEFAULT '',
          Book_Hash CHAR(64) NOT NULL DEFAULT '',
          File_Path TEXT NOT NULL,
          File_Format VARCHAR(8) NOT NULL,
          Total_Pages INTEGER NOT NULL DEFAULT 0,
          Current_Page INTEGER NOT NULL DEFAULT 0,
          Reading_Status VARCHAR(12) NOT NULL DEFAULT 'unread',
          Cover_Url TEXT,
          Create_Time DATETIME NOT NULL,
          Update_Time DATETIME NOT NULL,
          Delete_Time DATETIME,
          Device_ID TEXT NOT NULL DEFAULT '',
          Version INTEGER NOT NULL DEFAULT 1
        );

        CREATE INDEX IF NOT EXISTS idx_read_books_hash
          ON Read_Books (Book_Hash, Delete_Time);

        CREATE INDEX IF NOT EXISTS idx_read_books_list
          ON Read_Books (Delete_Time, Update_Time);

        CREATE TABLE IF NOT EXISTS Read_Annotation (
          Annotation_ID VARCHAR(10) PRIMARY KEY NOT NULL,
          Book_ID VARCHAR(10) NOT NULL,
          Annotation_Text TEXT NOT NULL,
          Annotation_Note TEXT NOT NULL DEFAULT '',
          Page INTEGER NOT NULL DEFAULT 0,
          Position TEXT NOT NULL DEFAULT '',
          Color VARCHAR(16) NOT NULL DEFAULT '#FFEB3B',
          Create_Time DATETIME NOT NULL,
          Update_Time DATETIME NOT NULL,
          Delete_Time DATETIME,
          Device_ID TEXT NOT NULL DEFAULT '',
          Version INTEGER NOT NULL DEFAULT 1
        );

        CREATE INDEX IF NOT EXISTS idx_read_annotation_book
          ON Read_Annotation (Book_ID, Delete_Time, Page);

        CREATE TABLE IF NOT EXISTS Note_Content (
          Note_ID VARCHAR(10) PRIMARY KEY NOT NULL,
          Note_Title VARCHAR(20) NOT NULL DEFAULT '',
          Note_Content TEXT NOT NULL DEFAULT '',
          Tags TEXT NOT NULL DEFAULT '[]',
          Create_Time INTEGER NOT NULL,
          Update_Time INTEGER NOT NULL,
          Delete_Time INTEGER,
          Device_ID TEXT NOT NULL DEFAULT '',
          Version INTEGER NOT NULL DEFAULT 1,
          Is_Folder CHAR(1) NOT NULL DEFAULT '0' CHECK (Is_Folder IN ('0', '1', '2', '3')),
          Folder1 CHAR(4),
          Folder2 CHAR(4),
          Folder3 CHAR(4),
          Is_Pinned CHAR(1) NOT NULL DEFAULT '0' CHECK (Is_Pinned IN ('0', '1'))
        );

        CREATE INDEX IF NOT EXISTS idx_note_content_folder
          ON Note_Content (Is_Folder, Folder1, Folder2, Folder3);

        CREATE INDEX IF NOT EXISTS idx_note_content_list
          ON Note_Content (Delete_Time, Is_Pinned, Update_Time);

        CREATE TABLE IF NOT EXISTS Note_Folders (
          Folder_ID CHAR(4) PRIMARY KEY NOT NULL,
          Folder_Name VARCHAR(20) NOT NULL,
          Folder_Level CHAR(1) NOT NULL CHECK (Folder_Level IN ('0', '1', '2', '3')),
          Folder_Parent_ID CHAR(4),
          Folder_Parent_Name VARCHAR(20),
          Folder_Order CHAR(4) NOT NULL,
          Folder_Flg1 CHAR(10),
          Folder_Flg2 CHAR(10),
          Folder_Flg3 CHAR(10)
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_note_folders_unique
          ON Note_Folders (Folder_Name, Folder_Level, IFNULL(Folder_Parent_ID, ''));

        CREATE INDEX IF NOT EXISTS idx_note_folders_parent_order
          ON Note_Folders (Folder_Parent_ID, Folder_Order);

        CREATE TABLE IF NOT EXISTS app_config (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL,
          updated_at INTEGER NOT NULL
        );
        "#,
    )
    .map_err(|e| format!("初始化数据库表失败: {}", e))?;

    crate::log_info!("[Data] 数据库已初始化: {}", path.display());
    Ok(())
}
