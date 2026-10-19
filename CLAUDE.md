# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

个人跨平台系统（MySystem）：基于 **Tauri 2 + Vue 3 + TypeScript + Vite** 的桌面/移动应用，目标是同一套代码运行在 Windows（PC 主机）和 Android（客户端），数据通过局域网同步（PC 为主机，无中心服务器）。

**`personal-system-design-v2.md` 是整个系统的设计依据（spec）**——涉及架构决策、数据结构、同步方案时先查阅该文档。注意文档是设计目标，部分实现已经偏离（见文末“实现与设计的已知偏差”），改代码时以实际代码为准并在必要时同步修正文档。

当前已实现两个模块：**记事本**（Markdown 编辑/预览、文件夹、标签、置顶、搜索）和 **阅读器**（EPUB/PDF/TXT 阅读、目录、标注高亮、阅读进度）；设置页已接入局域网设备发现与手动同步。

## 常用命令

包管理器是 **pnpm**（有 pnpm-lock.yaml，无 package-lock）。

```bash
pnpm dev            # 仅启动前端 Vite 开发服务器（端口 1420，strictPort）
pnpm tauri dev      # 完整开发：编译 Rust 并启动桌面应用（会先执行 pnpm dev）
pnpm build          # 类型检查（vue-tsc --noEmit）+ 前端构建
pnpm tauri build    # 打包发布版应用（会先执行 pnpm build）
cargo check         # 在 src-tauri/ 内单独检查 Rust 代码
```

未配置测试和 lint。`pnpm build` 中的 `vue-tsc --noEmit` 是唯一的静态检查手段，前端改动后应运行它验证。

## 架构

### 平台/模块（platform/modules）分层

- `src/platform/` — 平台外壳与基础服务，模块代码**不应反向依赖具体业务**：
  - `shell/` — `AppShell.vue`（Windows 端：顶栏 + 侧边导航 + 内容区）、`MobileShell.vue`（Android 端：顶栏 + 底部 Tab）、`HomeView.vue`、`SettingsView.vue`（同步开关、设备名、扫描局域网设备、连接/同步、同步日志、系统日志查看面板）
  - `services/logger.ts` — 平台统一日志入口：`logger.info/warn/error/debug` 写入 Rust 日志服务，另提供 `listLogFiles`/`readLogFile`/`getLogDir` 供设置页查看日志
  - `services/module-loader.ts` — 模块加载核心：读取 `public/modules.json` 获取模块注册表，用 `import.meta.glob('../../modules/*/index.ts')` 动态发现模块入口，按平台（`windows` / `android`）过滤 `enabled` 标志后加载
  - `services/module-types.ts` — 核心契约 `ModuleInterface`（`id`/`onLoad`/`getRoutes`/`onSearch` 等）和 `ModuleContext`（目前只注入 `platform` 与 `events`）
  - `services/event-bus.ts` — 简易事件总线，作为 `ModuleContext.events`
  - `router/index.ts` — 全局单例 vue-router（hash 模式）；`registerModuleRoutes()` 把模块路由统一加上 `/{moduleId}` 前缀注册，路由名加 `moduleId:` 前缀
  - `services/storage.ts` — 文档基础 CRUD 的 invoke 封装；`getDeviceId()` 用 localStorage 键 `mysystem_device_id` 持久化设备 ID（UUID v4），各模块 API 都从这里取设备 ID
  - `services/config.ts` — 配置读写 + 设备发现 + 同步 HTTP 客户端的前端 invoke 封装（`getConfig`/`setConfig`、`startDiscoveryListener`/`scanDevices`、`startSyncServer`、`checkRemoteDevice`/`exchangeSyncData`/`getSyncPayload`/`applyLocalPayload`）
  - `services/types.ts` — 与 Rust 模型对应的前端类型（camelCase）
- `src/modules/<id>/` — 功能模块（现有 `notepad`、`reader`），每个模块必须：
  1. 在 `public/modules.json` 中注册（含 `enabled: { windows, android }` 开关）
  2. 提供 `index.ts` 默认导出实现 `ModuleInterface`
  3. 视图放在模块自己的 `views/` 下

  模块内部约定结构：`index.ts`（模块契约与路由）、`api.ts`（invoke 封装）、`types.ts`（类型）、`stores/`（Pinia store）、`views/`、`components/`。模块通过自己的 `api.ts` 直接 invoke Rust 命令，**不经过** `platform/services/storage.ts` 的业务封装。

**新增功能模块的完整流程**：创建 `src/modules/<id>/index.ts` → 在 `public/modules.json` 注册 → 路由会自动挂到 `/<id>/...`；如需持久化，在 `platform/database_init.rs` 补充建表并在 Rust 端新增命令。

### Rust 后端（src-tauri/src）

- **`main.rs` 是真正的入口**：创建 `tauri::Builder`、注册 `tauri_plugin_fs` / `tauri_plugin_dialog` 插件，并在 `generate_handler![]` 中注册**全部** Tauri 命令。新增命令必须同时在这里注册，否则前端 invoke 会报 “command not found”。
- `lib.rs` 是模板遗留的 `run()`（仅 `greet`），目前**没有被 `main.rs` 调用**。不要在 `lib.rs` 里找命令注册；若将来启用 Android 的 `mobile_entry_point`，需要把 `main.rs` 的插件与命令注册迁移过去。
- `models/document.rs` — `BaseDocument`、`CreateDocumentInput`、`UpdateDocumentInput`，统一 `#[serde(rename_all = "camelCase")]`
- `platform/` — 按功能拆分的后端服务，均直接使用 `rusqlite` 打开 SQLite 文件：

  | 文件 | 职责 |
  |---|---|
  | `paths.rs` | 统一数据目录解析（`data_dir`/`db_path`/`books_dir`/`log_dir`） |
  | `database_init.rs` | 启动时在统一数据库中创建全部业务表 |
  | `logger.rs` | 平台日志服务：写 `<data>/log/mysystem-YYYY-MM-DD.log`，提供 `log_*!` 宏与日志查看命令 |
  | `storage.rs` | `documents` 基础 CRUD（软删除 + 写 `sync_operations`） |
  | `notepad_storage.rs` | `create_note`/`get_note`/`list_notes`/`update_note`/`delete_note`/`search_notes`（documents JOIN notes） |
  | `folder_storage.rs` | 文件夹 CRUD；删除时把笔记移到未分类、子文件夹提升到根级 |
  | `reader_storage.rs` | 书籍与标注 CRUD、`read_book_file`、`import_book_file`、`delete_book_file` |
  | `config_storage.rs` | `app_config` 键值配置的 get/set/delete |
  | `discovery.rs` | UDP 局域网设备发现（端口 54321，magic `MYSYSTEM_V1`，`discover`/`announce`） |
  | `http_server.rs` | 基于 `tiny_http` 的同步服务端（端口 54322）：`GET /api/info`、`POST /api/sync/exchange` |
  | `http_client.rs` | 基于 `reqwest` 的客户端：握手与数据交换 |
  | `sync_payload.rs` | `SyncPayload`/`SyncResponse` 定义、全量读取与按 `updated_at` 合并 |

- Rust 模型使用 `#[serde(rename_all = "camelCase")]`；前端对应类型在 `src/platform/services/types.ts` 与各模块 `types.ts`。注意 `Note.tags` 在 Rust 是 `Vec<String>`、数据库存 JSON 字符串，模块 API 返回给前端时已是数组。

### 数据层：统一 data 目录

- 建表职责在 Rust `platform/database_init.rs`（`main.rs` 的 `setup` 中执行全部 `CREATE TABLE IF NOT EXISTS`）。
- 业务数据读写走 Rust `rusqlite`；前端不直接执行 SQL，也不再使用 `@tauri-apps/plugin-sql` 加载数据库（该插件会把相对路径固定拼到 AppData，已移除注册以避免误写）。
- **所有数据统一放在 `<应用根目录>/data/` 下**，路径解析集中在 `platform/paths.rs`：
  - `data_dir()`：开发模式（`tauri dev`）取 `CARGO_MANIFEST_DIR/../data`（即项目根 `data`）；发布模式取 **exe 所在目录/data**；目录不可写或 Android 回退到 `app_data_dir()/data`；可用环境变量 `MYSYSTEM_DATA_DIR` 覆盖。
  - `db_path()`：`<data>/mydata.db`
  - `books_dir()`：`<data>/books`
  - `log_dir()`：`<data>/log`
- 新增存储代码必须复用 `paths.rs`，不要再自行拼 `app_data_dir()` 或 exe 目录。
- 书籍文件：`import_book_file` 复制到 `<data>/books/`；`delete_book_file` 只允许删除 canonicalize 后位于 `<data>/books/` 内的文件。
- 统一基表为 `documents`（id/type/title/created_at/updated_at/deleted_at/version/device_id），扩展表通过 `document_id` 外键关联：`notes`、`books`、`annotations`。`annotations` 的主键就是 `document_id`（一条标注 = 一条 `type='annotation'` 的 document）。`sync_state`、`known_devices` 表已建但当前未使用。

### 日志

- **Rust 端**：`platform/logger.rs` 是唯一日志出口，初始化后写入 `<data>/log/mysystem-YYYY-MM-DD.log`（按天分文件），同时输出到终端。业务代码统一使用 `crate::log_info!` / `log_warn!` / `log_error!` / `log_debug!`，不要再使用 `println!`/`eprintln!`。
- **前端与模块**：统一使用 `src/platform/services/logger.ts` 的 `logger.info/warn/error/debug`（底层 invoke `write_log`），模块可以直接引用，不需要经过 `ModuleContext`。业务代码不要直接调用 `console.*`。
- **自动日志**：`platform/services/ui-logging.ts` 在 `main.ts` 中安装，通过捕获阶段点击委托自动记录按钮/链接点击（`[UI] 点击「...」 @ 路由`，同一路由同一标签 300ms 内去重；给元素加 `data-log-ignore` 可跳过）。页面切换由 `platform/router/index.ts` 的 `router.afterEach` 自动记录（`[Router] 页面切换: A -> B`）。
- **业务日志**：集中在各模块 store（新建/删除/重命名/标签/搜索、打开书籍/标注等）和同步流程（`SettingsView` 的同步日志会同时写入系统日志）；高频操作做节流（搜索 1s、笔记自动保存 5s），不要逐字符或逐页码写日志。
- **查看**：设置页“系统日志 → 查看日志”打开覆盖式日志面板（文件选择、级别筛选、刷新、日志目录展示）。后端命令为 `write_log`、`list_log_files`、`read_log_file`（单文件最多读取最后 512 KB）、`get_log_dir`。
- 日志目录初始化失败不应阻塞启动；初始化前只输出到终端。

### 局域网同步（现状）

- **发现**：UDP 54321 广播 `discover` 消息，监听方回复 `announce`；设备 ID/名称/平台随消息携带。
- **交换**：HTTP 54322，`GET /api/info` 握手，`POST /api/sync/exchange` 交换数据。
- **流程**（设置页手动触发）：本机 `get_sync_payload` 读出全量 → `exchange_sync_data` 发给对方 → 对方 `merge_payload` 并按 `updated_at` 合并（新覆盖旧，相等或本地更新则拒绝）→ 对方返回自己的全量 `server_payload` → 本机 `apply_local_payload` 再合并一次。
- **范围**：payload 只包含 `documents` + `notes`，**不含** `folders`、`books`、`annotations`；目前是全量交换，未使用 `sync_operations` 增量日志。
- 端口目前硬编码（54321/54322）；`config.ts` 中的 `CONFIG_KEYS.SYNC_PORT` 尚未生效。同步相关配置存在 `app_config`：`sync.receive.enabled`、`device.name`。
- 全局搜索尚未接入：`ModuleInterface.onSearch` 已定义，但 `AppShell` 的搜索框仍是 disabled。

### Tauri 配置要点

- `tauri.conf.json`：dev 端口固定 1420（strictPort，Vite 配置不可改）；CSP 允许 `asset:` 与内联样式/脚本；`assetProtocol.scope` 含硬编码路径 `D:\wgl\**` 与 `$APPDATA`（运行时还会动态授权 `<应用根>/data`）。
- 权限在 `src-tauri/capabilities/default.json` 中声明（当前含 core/dialog/fs）；新增 Tauri 插件/API 时需同步更新。
- Rust lib 名为 `mysystem_lib`（与 bin 名冲突规避，勿改名）。
- 打包目标仅 `nsis`（Windows，`perMachine`，使用自定义 `src-tauri/windows/installer.nsi`）；Android 工程（`src-tauri/gen/android`）尚未生成。

## 约定

- 代码注释、日志、UI 文案均使用中文；标识符用英文
- 前端日志统一用 `logger`（`platform/services/logger.ts`），Rust 日志统一用 `log_*!` 宏；不要用裸 `console.*` / `println!`
- 平台类型只有 `'windows' | 'android'`（`module-types.ts`），平台检测在 `module-loader.ts` 的 `detectPlatform()`（目前基于 userAgent 粗略判断）
- 时间戳统一为 Unix 毫秒（`chrono::Utc::now().timestamp_millis()`）
- 新增 Rust 命令：在对应 `platform/*.rs` 实现 `#[tauri::command]` → 在 `main.rs` 的 `generate_handler![]` 注册 → 前端在对应 `api.ts` 封装
- 新增数据表：在 `platform/database_init.rs` 添加建表语句，并同时在 Rust 侧实现读写

## 实现与设计的已知偏差

改动前请知悉，必要时先与用户确认是修实现还是改文档：

- 设计文档要求 UUID v7（时间有序），实际使用 `uuid::Uuid::new_v4()`
- 设计文档描述操作日志增量同步，实际为全量 payload 交换；`sync_operations` 只写不读
- 同步未覆盖文件夹、书籍、标注
- 数据库路径已统一到 `<应用根>/data/mydata.db`（见上“统一 data 目录”）
- `lib.rs` 移动端入口未接入实际命令注册
