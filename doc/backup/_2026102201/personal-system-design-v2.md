# 个人跨平台系统设计文档

**文档版本**：v2.0（修订版）
**目标读者**：开发者本人、后续参与开发的 LLM
**文档目的**：作为系统开发的唯一依据，可直接据此生成开发步骤和代码
**修订说明**：本版去掉中间服务器，改为局域网 PC 主机 + Android 客户端同步；暂不考虑 Web 端；开发工具使用 VS Code。


## 目录

1. [需求概述](#1-需求概述)
2. [系统架构设计](#2-系统架构设计)
3. [技术选型](#3-技术选型)
4. [Tauri Rust 后端入门介绍](#4-tauri-rust-后端入门介绍)
5. [数据结构设计](#5-数据结构设计)
6. [页面设计](#6-页面设计)
7. [模块化实现机制](#7-模块化实现机制)
8. [局域网同步方案](#8-局域网同步方案)
9. [项目目录结构](#9-项目目录结构)
10. [开发环境配置（VS Code）](#10-开发环境配置vs-code)
11. [开发路线图](#11-开发路线图)
12. [补充设计建议](#12-补充设计建议)
13. [附录：给 LLM 的开发步骤生成指令](#13-附录给-llm-的开发步骤生成指令)


## 1. 需求概述

### 1.1 核心需求

| 编号 | 需求 | 说明 |
|---|---|---|
| R1 | 多端运行 | 可安装在 Windows 和 Android 上，暂不考虑 Web 端 |
| R2 | 数据互通 | 多端数据通过局域网同步保持一致 |
| R3 | 模块化外壳 | 系统有平台外壳，内部功能模块化；PC 端加载全部模块，Android 端可按需启用 |
| R4 | 首批模块 | 记事本、阅读器 |
| R5 | 无中间服务器 | 去掉中心服务器，由 PC 端作为主机，Android 端作为客户端定时同步 |
| R6 | 开发工具 | 使用 VS Code |

### 1.2 设计目标

- **一次开发、多端部署**：同一套前端代码编译为 Windows 和 Android 应用
- **按需加载**：Android 端可自由启用/禁用功能模块
- **本地优先**：数据先写本地，离线可用，局域网内自动同步
- **低维护成本**：无服务器运维，适合个人使用


## 2. 系统架构设计

### 2.1 架构风格：平台/插件风格

本系统采用 **平台/插件（Platform/Plug-in）软件体系结构风格**，简称 P/P 风格。

- **Platform（平台外壳）** ：负责窗口渲染、路由管理、数据存储、同步引擎、模块生命周期管理、跨模块通信总线、主题与配置管理。
- **Plug-in（功能模块）** ：每个功能模块是独立单元，包含自身 UI、业务逻辑和数据模型，通过平台提供的标准接口接入。

该风格的核心优势：插件可动态插入、删除和替换，功能模块之间解耦，便于并行开发和增量升级。

### 2.2 分层架构

```
┌─────────────────────────────────────────────┐
│           模块层（Plug-in Layer）              │
│  ┌──────────┐ ┌──────────┐ ┌──────────────┐  │
│  │  记事本   │ │  阅读器   │ │  未来模块...  │  │
│  └──────────┘ └──────────┘ └──────────────┘  │
├─────────────────────────────────────────────┤
│         平台服务层（Platform Services）        │
│  模块管理器 │ 数据同步引擎 │ 事件总线 │ 配置中心 │
│  文件系统  │ 网络通信    │ 搜索服务 │ 通知服务  │
├─────────────────────────────────────────────┤
│          适配层（Adapter Layer）               │
│  ┌───────────────────────────────────────┐   │
│  │  存储适配器（SQLite）                   │   │
│  │  网络适配器（HTTP / UDP）               │   │
│  │  平台适配器（Windows / Android）         │   │
│  └───────────────────────────────────────┘   │
├─────────────────────────────────────────────┤
│           运行时层（Runtime Layer）            │
│       Tauri Runtime（Windows / Android）      │
└─────────────────────────────────────────────┘
```

同层之间无依赖关系，上层可调用下层，下层不可反向依赖上层。

### 2.3 多端部署策略

| 端 | 运行时方案 | 加载模块 | 说明 |
|---|---|---|---|
| Windows 桌面端 | Tauri 2.0（Windows） | 全部模块 | 完整功能，可作为同步主机 |
| Android 移动端 | Tauri 2.0（Android） | 按需选择 | 触屏优化 UI，作为同步客户端 |


## 3. 技术选型

### 3.1 编程语言

| 层次 | 语言 | 理由 |
|---|---|---|
| 前端 UI | TypeScript | 类型安全，Vue 3 原生支持 |
| 后端/系统层 | Rust | Tauri 原生语言，性能高，内存安全 |
| 数据库查询 | SQL | 结构化查询，SQLite 标准 |

### 3.2 前端框架：Vue 3 + TypeScript

- 使用 Composition API，每个功能模块封装为独立的组合式函数
- UI 组件库推荐：Element Plus（PC 端）、Vant（Android 端）
- 状态管理：Pinia
- 路由：Vue Router（动态路由，由模块注册）

### 3.3 系统外壳：Tauri 2.0

| 对比项 | Tauri 2.0 | Electron | Flutter |
|---|---|---|---|
| 包体积 | 极小（~3MB） | 大（~50MB+） | 中等 |
| 内存占用 | 低 | 高 | 中等 |
| Windows 支持 | ✅ | ✅ | ✅ |
| Android 支持 | ✅（2.0 起） | ❌ | ✅ |
| Rust 后端 | ✅ | ❌ | ❌ |
| 学习成本 | 中等 | 低 | 中等 |

Tauri 2.0 原生支持 Windows 和 Android，使用相同 Rust 后端和前端代码，是个人项目实现桌面+移动跨平台的最优选择。

### 3.4 数据存储

| 端 | 本地存储方案 | 说明 |
|---|---|---|
| Windows | SQLite（通过 sqlx 或 rusqlite） | 结构化数据，性能好 |
| Android | SQLite | 同上 |
| 文件存储 | 系统文件系统 | 阅读器的电子书文件 |

### 3.5 局域网同步技术

| 功能 | 方案 | 说明 |
|---|---|---|
| 设备发现 | UDP 广播 | Rust 端用 `std::net::UdpSocket` 实现 |
| HTTP 服务 | tiny_http 或 axum | PC 主机端启动，处理同步请求 |
| 数据合并 | CRDT（abyo-crdt） | 纯 Rust 实现，避免 WASM 往返开销 |
| 同步协议 | HTTP + JSON | 轻量级，易于调试 |

### 3.6 开发工具：VS Code

必需插件：
- **rust-analyzer**：Rust 代码智能提示、跳转、错误检查
- **Tauri**：快速运行和打包 Tauri 应用
- **Volar**：Vue 3 语法支持和类型提示


## 4. Tauri Rust 后端入门介绍

### 4.1 什么是 Tauri Rust 后端

Tauri 应用由两部分组成：

- **前端（HTML/CSS/JavaScript/TypeScript）** ：运行在 WebView 中，负责用户界面和交互。
- **Rust 后端**：运行在操作系统原生环境中，负责文件读写、数据库操作、网络通信等系统级任务。

前端和后端通过 **IPC（Inter-Process Communication，进程间通信）** 机制通信。前端通过 `invoke()` 函数调用 Rust 端的命令（Command），Rust 端通过 `#[tauri::command]` 宏定义这些命令。

**为什么需要 Rust 后端？**
浏览器环境（WebView）出于安全考虑，不能直接操作硬盘文件、不能直接访问 SQLite 数据库、不能监听局域网端口。Rust 后端作为唯一拥有完全操作系统访问权限的组件，替前端完成这些任务。

**你需要写多少 Rust？**
对于本项目，Rust 后端主要写三类代码：
1. **数据库操作**：用 `sqlx` 或 `rusqlite` 写 SQL 查询。
2. **文件读写**：读写笔记文件、电子书文件。
3. **局域网同步**：启动 HTTP 服务器、UDP 广播、CRDT 合并。

这些代码一旦写好，后续基本不需要频繁改动。日常开发 90% 的时间都在写 TypeScript 前端。

### 4.2 Tauri Command 示例

**Rust 端定义命令**：

```rust
// src-tauri/src/commands.rs
use tauri::command;

#[command]
pub async fn save_note(id: String, title: String, content: String) -> Result<(), String> {
    // 调用数据库服务保存笔记
    crate::platform::storage::save_note(&id, &title, &content)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_all_notes() -> Result<Vec<Note>, String> {
    crate::platform::storage::get_all_notes()
        .await
        .map_err(|e| e.to_string())
}
```

**注册命令**：

```rust
// src-tauri/src/main.rs
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::save_note,
            commands::get_all_notes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**前端调用**：

```typescript
// src/platform/services/storage.ts
import { invoke } from '@tauri-apps/api/core';

export async function saveNote(id: string, title: string, content: string): Promise<void> {
  await invoke('save_note', { id, title, content });
}

export async function getAllNotes(): Promise<Note[]> {
  return await invoke<Note[]>('get_all_notes');
}
```


## 5. 数据结构设计

### 5.1 TypeScript 接口定义

```typescript
// ========== 模块注册表 ==========
interface ModuleRegistration {
  id: string;              // 模块唯一标识，如 "notepad", "reader"
  name: string;            // 显示名称
  version: string;         // 语义化版本
  description: string;
  icon: string;            // 图标资源路径
  platforms: Platform[];   // 支持的平台: 'windows' | 'android'
  entryPoint: string;      // 模块入口文件路径
  dependencies?: string[]; // 依赖的其他模块ID
  status: 'active' | 'disabled' | 'error';
}

type Platform = 'windows' | 'android';

// ========== 基础文档模型 ==========
interface BaseDocument {
  id: string;              // UUID v7（时间有序，便于同步）
  type: string;            // 'note' | 'book' | 'annotation' | ...
  title: string;
  createdAt: number;       // Unix时间戳（毫秒）
  updatedAt: number;
  deletedAt: number | null; // 软删除标记（用于同步）
  version: number;         // 乐观锁版本号
  deviceId: string;        // 最后修改的设备ID
}

// ========== 记事本模块 ==========
interface Note extends BaseDocument {
  type: 'note';
  content: string;         // Markdown格式
  tags: string[];
  folderId: string | null; // 所属文件夹
  isPinned: boolean;
}

interface Folder {
  id: string;
  name: string;
  parentId: string | null; // 支持嵌套
  sortOrder: number;
}

// ========== 阅读器模块 ==========
interface Book extends BaseDocument {
  type: 'book';
  author: string;
  coverUrl: string;
  filePath: string;        // 本地文件路径
  fileFormat: 'epub' | 'pdf' | 'mobi' | 'txt';
  totalPages: number;
  currentPage: number;     // 阅读进度
  readingStatus: 'unread' | 'reading' | 'finished';
}

interface Annotation extends BaseDocument {
  type: 'annotation';
  bookId: string;
  text: string;            // 标注的原文
  note: string;            // 用户批注
  page: number;
  position: { x: number; y: number; width: number; height: number };
  color: string;
}

// ========== 同步相关 ==========
interface SyncOperation {
  id: string;
  documentId: string;
  operation: 'create' | 'update' | 'delete';
  field: string;
  value: unknown;
  timestamp: number;
  deviceId: string;
  synced: boolean;
}

interface KnownDevice {
  deviceId: string;
  deviceName: string;
  deviceType: 'pc' | 'android';
  ipAddress: string;
  port: number;
  lastSeenAt: number;
  isHost: boolean;
}
```

### 5.2 SQLite 表结构

```sql
-- 基础文档表（所有类型共享）
CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL,
    title TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,
    version INTEGER NOT NULL DEFAULT 1,
    device_id TEXT NOT NULL
);

-- 笔记扩展表
CREATE TABLE IF NOT EXISTS notes (
    document_id TEXT PRIMARY KEY REFERENCES documents(id),
    content TEXT NOT NULL DEFAULT '',
    tags TEXT NOT NULL DEFAULT '[]',      -- JSON 数组
    folder_id TEXT,
    is_pinned INTEGER NOT NULL DEFAULT 0
);

-- 文件夹表
CREATE TABLE IF NOT EXISTS folders (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    parent_id TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0
);

-- 书籍扩展表
CREATE TABLE IF NOT EXISTS books (
    document_id TEXT PRIMARY KEY REFERENCES documents(id),
    author TEXT,
    cover_url TEXT,
    file_path TEXT NOT NULL,
    file_format TEXT NOT NULL,
    total_pages INTEGER DEFAULT 0,
    current_page INTEGER DEFAULT 0,
    reading_status TEXT DEFAULT 'unread'
);

-- 标注表
CREATE TABLE IF NOT EXISTS annotations (
    document_id TEXT PRIMARY KEY REFERENCES documents(id),
    book_id TEXT NOT NULL,
    text TEXT NOT NULL,
    note TEXT DEFAULT '',
    page INTEGER,
    position TEXT,                         -- JSON 对象
    color TEXT DEFAULT '#FFEB3B'
);

-- 同步操作日志表
CREATE TABLE IF NOT EXISTS sync_operations (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    field TEXT,
    value TEXT,
    timestamp INTEGER NOT NULL,
    device_id TEXT NOT NULL,
    synced INTEGER NOT NULL DEFAULT 0
);

-- 同步状态表
CREATE TABLE IF NOT EXISTS sync_state (
    id INTEGER PRIMARY KEY DEFAULT 1,
    last_sync_timestamp INTEGER,
    last_sync_device_id TEXT,
    pending_operations_count INTEGER DEFAULT 0,
    host_device_id TEXT,
    host_ip TEXT,
    host_port INTEGER
);

-- 已知设备表
CREATE TABLE IF NOT EXISTS known_devices (
    device_id TEXT PRIMARY KEY,
    device_name TEXT,
    device_type TEXT,
    ip_address TEXT,
    port INTEGER,
    last_seen_at INTEGER,
    is_host INTEGER DEFAULT 0
);
```

### 5.3 同步策略

- **文档级同步**：每个文档有独立的 `version` 和 `updatedAt`，同步时比较版本决定是否需要传输。
- **字段级合并**：使用 CRDT 自动合并并发编辑冲突。
- **软删除**：删除操作只标记 `deletedAt`，同步完成后定期清理。
- **操作日志**：所有修改先写入本地操作日志，同步引擎按序推送。


## 6. 页面设计

### 6.1 PC 端布局（Windows）

```
┌──────────────────────────────────────────────────┐
│  ☰  Logo           [搜索框]        [同步状态] [设置] │
├────────┬─────────────────────────────────────────┤
│        │                                         │
│ 📝记事本│         主内容区域                       │
│ 📖阅读器│         （根据所选模块渲染）               │
│ ➕添加  │                                         │
│ 模块   │                                         │
│        │                                         │
├────────┴─────────────────────────────────────────┤
│  状态栏：同步状态 | 存储使用 | 当前模块版本          │
└──────────────────────────────────────────────────┘
```

### 6.2 Android 端布局

```
┌──────────────────────┐
│  [页面标题]    [搜索]  │
├──────────────────────┤
│                      │
│     主内容区域         │
│                      │
│                      │
├──────────────────────┤
│ 📝  📖  ➕  ⚙️        │
│ 笔记 阅读 模块 设置     │
└──────────────────────┘
```

### 6.3 模块管理页面

```
┌─────────────────────────────────────────┐
│  模块管理                                │
│                                         │
│  ┌─────────────────────────────────┐    │
│  │ 📝 记事本          v1.2.0  [已启用]│    │
│  │   支持Markdown、标签、搜索        │    │
│  │   适用平台: Win / Android        │    │
│  └─────────────────────────────────┘    │
│  ┌─────────────────────────────────┐    │
│  │ 📖 阅读器          v1.0.0  [已启用]│    │
│  │   支持EPUB/PDF/TXT，标注、进度同步 │    │
│  │   适用平台: Win / Android        │    │
│  └─────────────────────────────────┘    │
│  ┌─────────────────────────────────┐    │
│  │ ➕ 安装新模块                     │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

### 6.4 同步设置页面

```
┌─────────────────────────────────────────┐
│  同步设置                                │
│                                         │
│  设备名称: [我的PC]              [修改]   │
│  角色:     ● 主机    ○ 客户端           │
│                                         │
│  ─── 主机模式 ───                        │
│  监听端口: [54321]                       │
│  服务状态: ● 运行中                      │
│                                         │
│  ─── 客户端模式 ───                      │
│  自动同步间隔: [5分钟 ▼]                  │
│  上次同步: 2026-10-12 14:30:00           │
│  同步状态: ✅ 已同步 (3条变更)            │
│                                         │
│  [立即同步]  [查看同步日志]               │
└─────────────────────────────────────────┘
```

### 6.5 记事本模块页面

- **列表视图**：卡片/列表展示笔记，支持按文件夹、标签筛选，支持搜索。
- **编辑视图**：Markdown 编辑器（推荐 CodeMirror 6 或 Milkdown），左侧编辑、右侧实时预览。
- **文件夹管理**：树形结构，支持拖拽排序。
- **标签系统**：多标签筛选，标签云视图。

### 6.6 阅读器模块页面

- **书架视图**：网格/列表展示书籍封面和阅读进度。
- **阅读视图**：电子书渲染引擎（EPUB 用 epub.js，PDF 用 pdf.js），支持字号调整、主题切换。
- **标注面板**：侧边栏展示当前书籍的所有标注和笔记，点击可跳转到原文位置。
- **导入功能**：从本地文件系统导入电子书。


## 7. 模块化实现机制

### 7.1 模块接口定义

每个功能模块必须实现以下接口：

```typescript
interface ModuleInterface {
  // 生命周期
  onLoad(context: ModuleContext): Promise<void>;
  onUnload(): Promise<void>;
  onActivate(): void;      // 模块被切换到前台
  onDeactivate(): void;    // 模块被切到后台

  // 路由
  getRoutes(): ModuleRoute[];

  // 数据
  getDataSchemas(): DataSchema[];

  // 搜索
  onSearch(query: string): Promise<SearchResult[]>;
}

interface ModuleContext {
  storage: StorageService;      // 数据存储接口
  sync: SyncService;            // 数据同步接口
  events: EventBus;             // 跨模块事件总线
  config: ConfigService;        // 配置读取
  ui: UIRegistry;               // UI组件注册
}

interface ModuleRoute {
  path: string;
  name: string;
  component: () => Promise<Component>;
  meta?: Record<string, unknown>;
}
```

### 7.2 模块加载流程

1. 平台启动时读取 `modules.json` 配置文件，获取已安装模块列表。
2. 根据当前平台的启用配置，筛选需要加载的模块。
3. 依次调用各模块的 `onLoad()` 方法，传入平台服务上下文。
4. 模块向平台注册自己的路由、UI 入口和数据 schema。
5. 平台根据注册信息生成导航菜单和页面路由。

### 7.3 模块配置文件 `modules.json`

```json
{
  "modules": [
    {
      "id": "notepad",
      "name": "记事本",
      "version": "1.2.0",
      "entryPoint": "src/modules/notepad/index.ts",
      "enabled": { "windows": true, "android": true }
    },
    {
      "id": "reader",
      "name": "阅读器",
      "version": "1.0.0",
      "entryPoint": "src/modules/reader/index.ts",
      "enabled": { "windows": true, "android": true }
    }
  ]
}
```

### 7.4 模块按平台启用

移动端和 PC 端都可通过设置页面随时启用或禁用模块。配置持久化在 SQLite 的 `app_config` 表中（键值对存储）。


## 8. 局域网同步方案

### 8.1 同步架构：主机-客户端模型

```
┌─────────────────────────────────────────────────────────────┐
│                     家庭局域网                                │
│                                                             │
│   ┌──────────────┐         ┌──────────────┐                 │
│   │  PC (主机)    │◄──UDP──►│ Android 客户端 │                 │
│   │              │  广播发现  │              │                 │
│   │ ┌──────────┐ │         │ ┌──────────┐ │                 │
│   │ │ SQLite   │ │◄──HTTP──│ │ SQLite   │ │                 │
│   │ │ 主数据库  │ │  同步请求 │ │ 本地副本  │ │                 │
│   │ └──────────┘ │         │ └──────────┘ │                 │
│   │ ┌──────────┐ │         │              │                 │
│   │ │ 轻量HTTP  │ │         │              │                 │
│   │ │ 服务器    │ │         │              │                 │
│   │ └──────────┘ │         │              │                 │
│   └──────────────┘         └──────────────┘                 │
│                                                             │
│   PC 也可以同时作为客户端，从其他 PC 主机拉取                   │
└─────────────────────────────────────────────────────────────┘
```

### 8.2 工作流程

1. **设备发现**：每台设备定时（如每 30 秒）向局域网广播自己的信息（主机名、IP、角色、端口号），同时监听广播，维护“附近设备列表”。超时未响应的节点自动移除。
2. **PC 主机启动 HTTP 服务**：PC 端在 Rust 后端启动轻量 HTTP 服务器，暴露以下接口：
   - `GET /api/info`：返回主机版本信息，供客户端确认连通性。
   - `GET /api/snapshot`：返回全量数据快照（首次同步或数据差异过大时使用）。
   - `POST /api/push`：接收客户端的增量推送。
   - `GET /api/pull?since={timestamp}`：返回指定时间后的增量变更。
3. **客户端定时同步**：Android 端定时（如每 5 分钟，可配置）向 PC 主机发起 HTTP 请求，拉取增量更新并合并到本地。
4. **冲突解决**：使用 CRDT（abyo-crdt）自动合并多端修改。

### 8.3 同步触发机制

- **定时同步**：Android 端每隔 N 分钟自动向 PC 主机发起一次同步请求（N 可在设置中调整）。
- **手动触发**：在 PC 或 Android 端点击“立即同步”按钮，强制发起一次同步。
- **推送通知**：PC 主机在数据变更后，通过 UDP 广播发送“通知”，Android 端收到后立即发起拉取请求。

### 8.4 HTTP 接口详细定义

```typescript
// GET /api/info
interface InfoResponse {
  deviceId: string;
  deviceName: string;
  version: string;
  protocolVersion: number;
}

// GET /api/snapshot
interface SnapshotResponse {
  documents: BaseDocument[];
  notes: Note[];
  folders: Folder[];
  books: Book[];
  annotations: Annotation[];
  serverTimestamp: number;
}

// POST /api/push
interface PushRequest {
  deviceId: string;
  operations: SyncOperation[];
}
interface PushResponse {
  accepted: number;
  conflicts: SyncOperation[];
  serverTimestamp: number;
}

// GET /api/pull?since={timestamp}
interface PullResponse {
  operations: SyncOperation[];
  serverTimestamp: number;
}
```

### 8.5 安全防护

局域网同步虽不经过外部服务器，但仍需基本防护：

- PC 主机端设置**配对密钥（pair secret）** 。
- Android 客户端首次连接时输入该密钥。
- 后续请求通过 **HMAC 签名**验证合法性。
- 防止同一局域网内其他设备误触或恶意访问。

### 8.6 离线场景

如果 Android 设备离开局域网，同步会暂时失败。此时：

- 数据先写入本地 SQLite，标记为“待同步”。
- 回到局域网后，同步引擎自动重试并推送积压的变更。


## 9. 项目目录结构

```
personal-system/
├── src-tauri/                    # Tauri Rust后端
│   ├── src/
│   │   ├── main.rs              # 入口
│   │   ├── commands.rs          # Tauri命令暴露给前端
│   │   ├── platform/            # 平台核心服务
│   │   │   ├── mod.rs
│   │   │   ├── module_manager.rs # 模块管理器
│   │   │   ├── storage.rs        # 存储服务
│   │   │   ├── sync_engine.rs    # 同步引擎
│   │   │   ├── discovery.rs      # UDP设备发现
│   │   │   ├── http_server.rs    # HTTP服务
│   │   │   ├── crdt.rs           # CRDT合并逻辑
│   │   │   └── event_bus.rs      # 事件总线
│   │   └── models/              # Rust数据模型
│   │       ├── mod.rs
│   │       ├── document.rs
│   │       ├── note.rs
│   │       └── sync.rs
│   ├── Cargo.toml
│   └── tauri.conf.json          # Tauri配置
│
├── src/                          # 前端（TypeScript + Vue）
│   ├── platform/                 # 平台外壳
│   │   ├── shell/               # 外壳组件（导航、布局）
│   │   ├── router/              # 动态路由管理
│   │   ├── store/               # 全局状态（Pinia）
│   │   └── services/            # 前端平台服务
│   │       ├── module-loader.ts  # 模块加载器
│   │       ├── sync-client.ts    # 同步客户端
│   │       └── storage.ts        # 存储抽象层
│   ├── modules/                  # 功能模块
│   │   ├── notepad/             # 记事本模块
│   │   │   ├── index.ts          # 模块入口
│   │   │   ├── views/
│   │   │   ├── components/
│   │   │   ├── stores/
│   │   │   └── types.ts
│   │   └── reader/              # 阅读器模块
│   │       ├── index.ts
│   │       ├── views/
│   │       ├── components/
│   │       ├── stores/
│   │       └── types.ts
│   ├── assets/
│   └── main.ts
│
├── modules.json                  # 模块注册配置
├── package.json
├── vite.config.ts
├── tsconfig.json
└── .vscode/                      # VS Code配置
    ├── settings.json
    └── launch.json
```


## 10. 开发环境配置（VS Code）

### 10.1 必需插件

| 插件 | 用途 |
|---|---|
| rust-analyzer | Rust 代码智能提示、跳转、错误检查 |
| Tauri | 快速运行和打包 Tauri 应用 |
| Volar | Vue 3 语法支持和类型提示 |

### 10.2 环境变量配置

**Windows**：
- 确保 `C:\Users\{用户名}\.cargo\bin` 已加入系统 PATH。
- 在 VS Code 中关闭所有终端并重新打开窗口。
- 验证：在 VS Code 内置终端执行 `cargo --version` 和 `rustc --version`。

**Android 构建链路**：
- JDK 必须为 17（非 21 或 8）。
- 设置 `ANDROID_HOME` 和 `ANDROID_NDK_ROOT` 环境变量。
- 在 `tauri.conf.json` 中启用 `"build": { "mobile": true }`。
- 使用 `tauri android dev` 命令预览。

### 10.3 项目初始化命令

```bash
# 创建 Tauri + Vue 3 + TypeScript 项目
pnpm create tauri-app

# 选择 Vue 3 + TypeScript 模板
# 然后在项目根目录安装依赖
pnpm install

# 启动开发模式
pnpm tauri dev

# Android 开发模式
pnpm tauri android dev
```

### 10.4 VS Code 配置示例

`.vscode/settings.json`：

```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave": true,
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "esbenp.prettier-vscode",
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "[vue]": {
    "editor.defaultFormatter": "Vue.volar"
  }
}
```

`.vscode/launch.json`：

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Tauri Development Debug",
      "cargo": {
        "args": ["build", "--manifest-path=./src-tauri/Cargo.toml", "--no-default-features"]
      },
      "preLaunchTask": "rust: cargo build"
    }
  ]
}
```


## 11. 开发路线图

| 阶段 | 内容 | 关键任务 | 预估时间 |
|---|---|---|---|
| 第一阶段 | 项目初始化 | 创建 Tauri + Vue 3 + TS 项目；配置 VS Code；验证 `pnpm tauri dev` 可运行 | 1周 |
| 第二阶段 | 平台外壳 | 实现导航布局、动态路由、SQLite 存储服务、模块加载器、事件总线 | 2-3周 |
| 第三阶段 | 记事本模块 | 笔记列表、Markdown 编辑、文件夹树、标签系统、搜索 | 2-3周 |
| 第四阶段 | 局域网同步 | UDP 设备发现、HTTP 服务端、HTTP 客户端、CRDT 合并、同步设置页面 | 2-3周 |
| 第五阶段 | 阅读器模块 | 书架视图、EPUB/PDF 渲染、标注面板、导入功能 | 3周 |
| 第六阶段 | Android 适配 | 触屏 UI 优化、文件访问权限、后台同步、真机测试 | 2周 |

### 11.1 各阶段验收标准

**第一阶段**：
- `pnpm tauri dev` 能在 Windows 上启动窗口。
- VS Code 中 rust-analyzer 和 Volar 正常工作。

**第二阶段**：
- 侧边栏可切换模块。
- SQLite 数据库自动创建，表结构正确。
- `modules.json` 中的模块能被动态加载。

**第三阶段**：
- 可创建、编辑、删除笔记。
- Markdown 实时预览正常。
- 文件夹和标签筛选正常。

**第四阶段**：
- PC 主机启动后，Android 客户端能在 30 秒内发现。
- Android 端能推送和拉取数据。
- 两端同时编辑同一笔记，CRDT 能自动合并。

**第五阶段**：
- EPUB 和 PDF 能正常渲染。
- 标注能保存并同步。
- 阅读进度能同步。

**第六阶段**：
- Android APK 能安装到真机。
- 触屏操作流畅。
- 离开局域网后数据本地保存，回到局域网后自动同步。


## 12. 补充设计建议

### 12.1 主题与外观

使用 CSS 变量实现主题系统，支持日间/夜间/护眼模式，所有模块共享主题配置。

### 12.2 搜索功能

平台提供全局搜索服务，各模块通过 `onSearch()` 接口参与搜索，搜索结果统一展示。

### 12.3 备份与恢复

提供数据导出/导入功能（JSON 格式），支持定时自动备份到用户指定目录。

### 12.4 快捷键系统

PC 端提供全局快捷键注册机制，各模块可声明自己的快捷键。

### 12.5 通知系统

平台提供统一的通知接口，阅读器可用于阅读提醒，记事本可用于待办提醒。

### 12.6 未来可扩展模块

待办清单、日历、密码管理、书签管理、RSS 阅读器等。

### 12.7 主机切换

如果希望有备用主机，可在 `known_devices` 表中设置多个 `is_host=1` 的设备，同步时按优先级尝试连接。个人使用单主机方案通常够用。

### 12.8 Android 真机测试注意事项

- UDP 广播在模拟器中通常无法正常工作，局域网同步需在真机测试。
- 确保手机和 PC 连接同一 WiFi。
- 路由器不能开启“AP 隔离”（AP isolation）功能。


## 13. 附录：给 LLM 的开发步骤生成指令

> **使用说明**：将本设计文档完整提供给 LLM，并附加以下指令，即可让 LLM 生成具体的开发步骤和代码。

### 13.1 给 LLM 的系统提示词

```
你是一位资深全栈工程师，熟悉 Tauri 2.0、Rust、Vue 3、TypeScript、SQLite 和 CRDT。
你将根据以下设计文档，为开发者生成详细的开发步骤和代码。

要求：
1. 严格按照设计文档的技术选型和架构执行。
2. 每个开发步骤必须包含：目标、具体操作、验证方式。
3. 代码示例使用 TypeScript 和 Rust，确保可运行。
4. 优先实现平台外壳，再实现功能模块。
5. 同步功能使用局域网主机-客户端模型，不引入中间服务器。
6. 暂不考虑 Web 端。
7. 开发工具为 VS Code。
```

### 13.2 给 LLM 的任务模板

```
请根据设计文档，生成【第 X 阶段：阶段名称】的详细开发步骤。

输出格式：
## 阶段目标
## 前置条件
## 任务清单
### 任务 1：任务名称
- 目标：
- 操作步骤：
- 代码示例：
- 验证方式：
### 任务 2：...
## 常见问题与解决
## 验收标准
```

### 13.3 建议的 LLM 对话流程

1. **第一步**：将完整设计文档粘贴给 LLM，要求它总结项目结构和关键技术点。
2. **第二步**：要求 LLM 生成第一阶段（项目初始化）的详细步骤。
3. **第三步**：完成第一阶段后，要求 LLM 生成第二阶段（平台外壳）的详细步骤。
4. **第四步**：依次推进，每完成一个阶段，将实际遇到的问题反馈给 LLM，让它调整后续步骤。
5. **第五步**：在第四阶段（局域网同步）时，特别要求 LLM 提供 UDP 广播和 HTTP 服务的完整 Rust 代码示例。

### 13.4 关键提醒

- 让 LLM 生成代码时，要求它注明文件路径。
- 要求 LLM 对每个 Rust command 提供前端调用示例。
- 要求 LLM 对 CRDT 合并逻辑提供单元测试示例。
- 如果 LLM 生成的代码无法运行，将错误信息完整反馈给它，要求修复。

---

**文档结束**

> 本文档为完整设计文档，可直接保存为 `personal-system-design-v2.md`。后续开发中，任何偏离本文档的决策都应记录在单独的“决策日志”中，以便追溯。