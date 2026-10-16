import Database from '@tauri-apps/plugin-sql';

let db: Database | null = null;

export async function getDb(): Promise<Database> {
  if (!db) {
    db = await Database.load('sqlite:mydata.db');
  }
  return db;
}

export async function initDatabase(): Promise<void> {
  const database = await getDb();

  // 基础文档表
  await database.execute(`
    CREATE TABLE IF NOT EXISTS documents (
      id TEXT PRIMARY KEY,
      type TEXT NOT NULL,
      title TEXT NOT NULL,
      created_at INTEGER NOT NULL,
      updated_at INTEGER NOT NULL,
      deleted_at INTEGER,
      version INTEGER NOT NULL DEFAULT 1,
      device_id TEXT NOT NULL
    )
  `);

  // 笔记扩展表
  await database.execute(`
    CREATE TABLE IF NOT EXISTS notes (
      document_id TEXT PRIMARY KEY REFERENCES documents(id),
      content TEXT NOT NULL DEFAULT '',
      tags TEXT NOT NULL DEFAULT '[]',
      folder_id TEXT,
      is_pinned INTEGER NOT NULL DEFAULT 0
    )
  `);

  // 文件夹表
  await database.execute(`
    CREATE TABLE IF NOT EXISTS folders (
      id TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      parent_id TEXT,
      sort_order INTEGER NOT NULL DEFAULT 0
    )
  `);

  // 书籍扩展表
  await database.execute(`
    CREATE TABLE IF NOT EXISTS books (
      document_id TEXT PRIMARY KEY REFERENCES documents(id),
      author TEXT,
      cover_url TEXT,
      file_path TEXT NOT NULL,
      file_format TEXT NOT NULL,
      total_pages INTEGER DEFAULT 0,
      current_page INTEGER DEFAULT 0,
      reading_status TEXT DEFAULT 'unread'
    )
  `);

  // 标注表
  await database.execute(`
    CREATE TABLE IF NOT EXISTS annotations (
      document_id TEXT PRIMARY KEY REFERENCES documents(id),
      book_id TEXT NOT NULL,
      text TEXT NOT NULL,
      note TEXT DEFAULT '',
      page INTEGER,
      position TEXT,
      color TEXT DEFAULT '#FFEB3B'
    )
  `);

  // 同步操作日志表
  await database.execute(`
    CREATE TABLE IF NOT EXISTS sync_operations (
      id TEXT PRIMARY KEY,
      document_id TEXT NOT NULL,
      operation TEXT NOT NULL,
      field TEXT,
      value TEXT,
      timestamp INTEGER NOT NULL,
      device_id TEXT NOT NULL,
      synced INTEGER NOT NULL DEFAULT 0
    )
  `);

  // 同步状态表
  await database.execute(`
    CREATE TABLE IF NOT EXISTS sync_state (
      id INTEGER PRIMARY KEY DEFAULT 1,
      last_sync_timestamp INTEGER,
      last_sync_device_id TEXT,
      pending_operations_count INTEGER DEFAULT 0,
      host_device_id TEXT,
      host_ip TEXT,
      host_port INTEGER
    )
  `);

  // 已知设备表
  await database.execute(`
    CREATE TABLE IF NOT EXISTS known_devices (
      device_id TEXT PRIMARY KEY,
      device_name TEXT,
      device_type TEXT,
      ip_address TEXT,
      port INTEGER,
      last_seen_at INTEGER,
      is_host INTEGER DEFAULT 0
    )
  `);

  // 通用配置表（键值对）
  await database.execute(`
    CREATE TABLE IF NOT EXISTS app_config (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL,
      updated_at INTEGER NOT NULL
    )
  `);

  console.log('[DB] All tables initialized.');
}