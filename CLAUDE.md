# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

个人跨平台系统（MySystem）：基于 **Tauri 2 + Vue 3 + TypeScript + Vite** 的桌面/移动应用，目标是同一套代码运行在 Windows（PC 主机）和 Android（客户端）。局域网同步功能已整体移除，当前为各设备独立使用本地数据。

**`doc/` 下的 01~12 号文档是当前系统的参考文档**（索引见 `doc/README.md`）——涉及架构、数据结构时先查阅这些文档与 `doc/11-已知偏差与路线图.md`。文档以当前代码实现为准；原始设计文档 `personal-system-design-v2.md` 已归档到 `doc/backup/_2026102201/`，不再作为参考。改代码时以实际代码为准并在必要时同步修正文档。

完整项目文档位于 `doc/`，索引见 `doc/README.md`：系统架构、功能设计、UI、数据库、数据结构、开发 API、开发指南、部署配置、测试排错等。

当前已实现四个模块：**记事本**（Markdown / 纯文本编辑、嵌套文件夹、标签、置顶、搜索）、**阅读器**（EPUB/PDF/TXT 阅读、目录、标注高亮、阅读进度）、**JSON 分析**（导入文件或粘贴文本，展示结构与统计）和 **角色图鉴**（读取 `data/其他/ys.json`，列表展示、快捷筛选与自定义条件）。

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
  - `shell/` — `AppShell.vue`（Windows 端：顶栏 + 侧边导航 + 内容区）、`MobileShell.vue`（Android 端：顶栏 + 底部 Tab）、`SettingsView.vue`（设置页骨架；日志查看拆到 `settings/components/LogViewer.vue`）
  - `services/logger.ts` — 平台统一日志入口：`logger.info/warn/error/debug` 写入 Rust 日志服务，另提供 `listLogFiles`/`readLogFile`/`getLogDir` 供设置页查看日志
  - `services/module-loader.ts` — 模块加载核心：用 `import.meta.glob('../../modules/*/index.ts')` 自动发现 `src/modules/` 下的模块入口，从模块实例读取 `id`/`name`/`version`/`icon`/`order`/`enabled` 后加载（没有注册表文件）
  - `services/module-types.ts` — 核心契约 `ModuleInterface`（`id`/`name`/`version`/`onLoad`/`getRoutes`/`onSearch`，以及可选的 `icon`/`order`/`enabled`）和 `ModuleContext`（目前只注入 `platform` 与 `events`）
  - `services/event-bus.ts` — 简易事件总线，作为 `ModuleContext.events`
  - `router/index.ts` — 全局单例 vue-router（hash 模式）；`registerModuleRoutes()` 把模块路由统一加上 `/{moduleId}` 前缀注册，路由名加 `moduleId:` 前缀
  - `services/device.ts` — `getDeviceId()` 用 localStorage 键 `mysystem_device_id` 持久化设备 ID（UUID v4），各模块 API 与设置页从这里取设备 ID
  - `services/config.ts` — `app_config` 键值配置的前端 invoke 封装（`getConfig`/`setConfig`）
- `src/modules/<id>/` — 功能模块（现有 `notepad`、`reader`、`jsonview`、`ys`），模块自包含：
  1. 提供 `index.ts` 默认导出实现 `ModuleInterface`，在实例上声明元数据（`id` 必填；`icon`/`order`/`enabled` 可选，`enabled` 未声明的平台默认启用）
  2. 视图放在模块自己的 `views/` 下
  3. 不需要任何注册文件；放进 `src/modules/` 即被自动发现

  模块内部约定结构：`index.ts`（模块契约与路由）、`api.ts`（invoke 封装）、`types.ts`（类型）、`stores/`（Pinia store）、`views/`、`components/`。模块通过自己的 `api.ts` 直接 invoke Rust 命令。

**新增功能模块的完整流程**：在 `src/modules/<id>/` 下创建模块文件夹（入口 `index.ts`）→ 重启 `pnpm dev`（Vite 的 glob 在启动时展开）→ 路由自动挂到 `/<id>/...`，导航自动出现；如需持久化，在 `platform/database_init.rs` 补充建表，存储实现放进 `src-tauri/src/modules/<id>/`，并在 `commands.rs` 注册命令。

### Rust 后端（src-tauri/src）

- **`lib.rs` 是真正的入口**：`run()` 创建 `tauri::Builder`、注册 `tauri_plugin_dialog` 插件、执行启动初始化（日志、建表、asset 目录授权），并挂载 `commands::invoke_handler()`。`main.rs` 只保留 `windows_subsystem` 属性并调用 `mysystem_lib::run()`；`#[cfg_attr(mobile, tauri::mobile_entry_point)]` 也在 `lib.rs` 上，Android 入口复用同一份配置。
- **命令注册集中在 `commands.rs` 的 `invoke_handler()`**：这是唯一的应用组装层，同时引用平台命令与各业务模块命令。新增命令必须在这里追加，否则前端 invoke 会报 “command not found”。
- `modules/` — 业务模块的 Rust 侧实现，目录 id 与前端 `src/modules/` 对齐（模块只依赖 `platform/`，不反向被平台依赖）：

  | 文件 | 职责 |
  |---|---|
  | `notepad/mod.rs` | `create_note`/`get_note`/`list_notes`/`update_note`/`delete_note`/`search_notes`；表为独立的 `Note_Content`（`Note_ID` 十位补零自增主键、`Note_Title` 最多 20 字符、`Delete_Time` 软删除、`Is_Pinned` 置顶、`Device_ID`/`Version` 写入记录、`Tags` 存 JSON；`Is_Folder` + `Folder1~3` 冗余文件夹路径） |
  | `notepad/folders.rs` | 文件夹 CRUD，表为 `Note_Folders`（`Folder_ID` 四位补零自增、`Folder_Level` 0/1/2/3、`Folder_Parent_ID`/`Folder_Parent_Name` 冗余父级信息、`Folder_Order` 同级顺序）；最多嵌套 3 层（全部笔记/A/B/C），同级不允许重名；删除时其中笔记和子文件夹移动到被删文件夹的父级（删除根级文件夹时，笔记归入「未命名」、子文件夹保留在根级），子文件夹重名时改名为 `原名_folder_被删文件夹名`；`migrate_legacy_folders` 在启动时把旧 `folders` 表迁移过来并删除旧表，`ensure_default_folder` 创建固定 id 为 `0001`、层级 0、排序 0 的「未命名」；`refresh_note_paths` 按 `Note_Folders` 父链重建 `Note_Content` 的四列路径 |
  | `reader/mod.rs` | 书籍与标注 CRUD、`import_book_file`、`delete_book_file`；表为独立的 `Read_Books` / `Read_Annotation`（`Book_ID` / `Annotation_ID` 十位补零自增主键、PascalCase 字段、`Delete_Time` 软删除、`Device_ID`/`Version`）；`Book_Hash` 保存源文件 SHA-256 用于导入去重，`Read_Annotation.Position` 按格式存 CFI / 归一化矩形 / 章节偏移 JSON；`init` 在启动时把旧 `documents`/`books`/`annotations` 迁移到新表并删除旧表；`delete_book` 级联软删除标注 |
  | `jsonview/mod.rs` | `read_json_file`：读取本地 UTF-8 JSON 文件（最大 10 MB、去掉 BOM），供前端分析结构；不建表、不持久化 |
  | `ys/mod.rs` | `read_ys_file`：读取 `<data>/其他/ys.json`（最大 20 MB、去 BOM；文件不存在返回 `null`），前端每次进入页面重新解析与展示；不建表、不持久化 |

- `platform/` — 平台基础服务（不包含业务逻辑），均直接使用 `rusqlite` 打开 SQLite 文件：

  | 文件 | 职责 |
  |---|---|
  | `paths.rs` | 统一数据目录解析（`data_dir`/`db_path`/`books_dir`/`log_dir`） |
  | `database_init.rs` | 启动时在统一数据库中创建全部业务表 |
  | `logger.rs` | 平台日志服务：写 `<data>/log/mysystem-YYYY-MM-DD.log`，提供 `log_*!` 宏与日志查看命令 |
  | `config_storage.rs` | `app_config` 键值配置的 get/set |

- Rust 模型使用 `#[serde(rename_all = "camelCase")]`；前端对应类型在各模块 `types.ts`。注意 `Note.tags` 在 Rust 是 `Vec<String>`、数据库存 JSON 字符串，模块 API 返回给前端时已是数组。

### 数据层：统一 data 目录

- 建表职责在 Rust `platform/database_init.rs`（`lib.rs` 的 `setup` 中执行全部 `CREATE TABLE IF NOT EXISTS`）。
- 业务数据读写走 Rust `rusqlite`；前端不直接执行 SQL，也不再使用 `@tauri-apps/plugin-sql` 加载数据库（该插件会把相对路径固定拼到 AppData，已移除注册以避免误写）。
- **所有数据统一放在 `<应用根目录>/data/` 下**，路径解析集中在 `platform/paths.rs`：
  - `data_dir()`：开发模式（`tauri dev`）取 `CARGO_MANIFEST_DIR/../data`（即项目根 `data`）；发布模式取 **exe 所在目录/data**；目录不可写或 Android 回退到 `app_data_dir()/data`；可用环境变量 `MYSYSTEM_DATA_DIR` 覆盖。
  - `db_path()`：`<data>/mydata.db`
  - `books_dir()`：`<data>/books`
  - `log_dir()`：`<data>/log`
- 新增存储代码必须复用 `paths.rs`，不要再自行拼 `app_data_dir()` 或 exe 目录。
- 书籍文件：`import_book_file` 复制到 `<data>/books/`；`delete_book_file` 只允许删除 canonicalize 后位于 `<data>/books/` 内的文件。
- 角色图鉴读取**外部数据文件** `<data>/其他/ys.json`（字段说明见 `data/其他/设定.txt`），只读、不入库；文件缺失时模块显示空状态，每次进入页面重新读取，因此修改 JSON 后重新打开即可生效。远程角色图标依赖 CSP 中的 `https://act-webstatic.mihoyo.com`。
- 阅读器使用**独立表 `Read_Books` / `Read_Annotation`**，与 `documents`/`books`/`annotations` 旧表无关：主键 `Book_ID`/`Annotation_ID` 为十位补零字符串（`0000000001` 起自增），字段 PascalCase，`Book_Hash` 保存源文件 SHA-256（导入去重，命中时前端三选一：打开已有/仍然导入/取消），`Read_Annotation.Book_ID` 逻辑关联书籍，`Position` 按格式存 `{"cfi"}`（EPUB）/ `{"page",rects}`（PDF 归一化矩形）/ `{"chapter",start,end}`（TXT 章节内偏移）；`Delete_Time` 软删除、`Device_ID`/`Version` 记录最后写入；`Create_Time`/`Update_Time`/`Delete_Time` 为 `DATETIME` UTC 文本（`YYYY-MM-DD HH:MM:SS.mmm`）；启动时 `modules::reader::init()` 迁移旧表数据并删除 `annotations`/`books`/`documents`。
- 记事本笔记使用**独立表 `Note_Content`**，不再写入 `documents`：主键 `Note_ID` 为十位补零字符串（`0000000001` 起自增），标题 `Note_Title` 最多 20 字符（前后端都校验/限制），`Delete_Time` 软删除，`Is_Pinned` 为 `'0'`/`'1'`，`Device_ID` / `Version` 记录最后写入设备与版本（创建 1，更新/删除 +1）；文件夹路径冗余在 `Is_Folder` + `Folder1~3`（`0` 表示「未命名」且三列全空，`N` 表示当前文件夹 ID 放在 `FolderN`，父链依次向前填）。启动时 `ensure_note_content_columns` 补齐后加列，`migrate_legacy_notes` 把旧 `notes` + `documents(type='note')` 迁移过来并删除旧数据/旧表（需先删 `notes` 再删 `documents`，否则外键检查失败）。
- 记事本文件夹：表为 `Note_Folders`，`Folder_ID` 为四位补零字符串（`0001` 起自增，主键），`Folder_Level` 取 0/1/2/3（0 仅用于默认文件夹，1~3 为嵌套层级，3 之下不可再建），`Folder_Order` 为同级顺序（默认文件夹为 0，其余从 1 开始），`Folder_Flg1~3` 为备用字段。`Folder_Name + Folder_Level + Folder_Parent_ID` 唯一（同级不允许重名，前后端都校验），`Folder_Parent_ID`/`Folder_Parent_Name` 在重命名和移动子文件夹时同步维护。默认文件夹「未命名」固定 `0001 / 层级 0 / 排序 0`，由 `modules::notepad::init` 在启动时创建，新建笔记未指定文件夹时归入其中。旧版 `folders` 表由 `migrate_legacy_folders` 在启动时迁移：`uncategorized` → `0001 未命名`，其余按原层级与顺序重新编号为 `0002` 起，随后删除旧表。文件夹层级/父级变化后由 `refresh_note_paths` 重建 `Note_Content` 的 `Is_Folder` + `Folder1~3`。

### 日志

- **Rust 端**：`platform/logger.rs` 是唯一日志出口，初始化后写入 `<data>/log/mysystem-YYYY-MM-DD.log`（按天分文件），同时输出到终端。业务代码统一使用 `crate::log_info!` / `log_warn!` / `log_error!` / `log_debug!`，不要再使用 `println!`/`eprintln!`。
- **前端与模块**：统一使用 `src/platform/services/logger.ts` 的 `logger.info/warn/error/debug`（底层 invoke `write_log`），模块可以直接引用，不需要经过 `ModuleContext`。业务代码不要直接调用 `console.*`。
- **自动日志**：`platform/services/ui-logging.ts` 在 `main.ts` 中安装，通过捕获阶段点击委托自动记录按钮/链接点击（`[UI] 点击「...」 @ 路由`，同一路由同一标签 300ms 内去重；给元素加 `data-log-ignore` 可跳过）。页面切换由 `platform/router/index.ts` 的 `router.afterEach` 自动记录（`[Router] 页面切换: A -> B`）。
- **业务日志**：集中在各模块 store（新建/删除/重命名/标签/搜索、打开书籍/标注等）；高频操作做节流（搜索 1s、笔记自动保存 5s），不要逐字符或逐页码写日志。
- **查看**：设置页“系统日志 → 查看日志”打开覆盖式日志面板（文件选择、级别筛选、刷新、日志目录展示）。后端命令为 `write_log`、`list_log_files`、`read_log_file`（单文件最多读取最后 512 KB）、`get_log_dir`。
- 日志目录初始化失败不应阻塞启动；初始化前只输出到终端。

### 局域网同步（已移除）

- 原 UDP 设备发现、HTTP 同步服务端/客户端、`SyncPayload` 全量交换与设置页同步界面均已删除（2026-10-30）。
- 同步相关的 Rust 文件 `platform/discovery.rs`、`http_server.rs`、`http_client.rs`、`sync_payload.rs` 及 `tiny_http`/`reqwest` 依赖已移除；`sync_operations` 表不再创建、也不再写入。
- 设备 ID（`getDeviceId()`）保留，仅用于各业务表 `Device_ID` 字段。`app_config` 与 `config_storage` 作为通用配置能力保留。
- 全局搜索尚未接入：`ModuleInterface.onSearch` 已定义，但 `AppShell` 的搜索框仍是 disabled。

### Tauri 配置要点

- `tauri.conf.json`：dev 端口固定 1420（strictPort，Vite 配置不可改）；CSP 允许 `asset:` 与内联样式/脚本；`assetProtocol.scope` 含硬编码路径 `D:\wgl\**` 与 `$APPDATA`（运行时还会动态授权 `<应用根>/data`）。
- 权限在 `src-tauri/capabilities/default.json` 中声明（当前含 core/dialog）；新增 Tauri 插件/API 时需同步更新。
- Rust lib 名为 `mysystem_lib`（与 bin 名冲突规避，勿改名）。
- 打包目标仅 `nsis`（Windows，`perMachine`，使用自定义 `src-tauri/windows/installer.nsi`）；Android 工程（`src-tauri/gen/android`）尚未生成。

## 约定

- 代码注释、日志、UI 文案均使用中文；标识符用英文
- 前端日志统一用 `logger`（`platform/services/logger.ts`），Rust 日志统一用 `log_*!` 宏；不要用裸 `console.*` / `println!`
- 平台类型只有 `'windows' | 'android'`（`module-types.ts`），平台检测在 `module-loader.ts` 的 `detectPlatform()`（目前基于 userAgent 粗略判断）
- 时间戳统一为 Unix 毫秒（`chrono::Utc::now().timestamp_millis()`）；例外：阅读器两表 `Read_Books` / `Read_Annotation` 的 `Create_Time` / `Update_Time` / `Delete_Time` 使用 `DATETIME` UTC 文本（`YYYY-MM-DD HH:MM:SS.mmm`），Rust/TS 类型为 `String` / `string`
- 笔记标题最多 20 个字符：编辑页 `maxlength=20`，Rust 侧 `chars().count()` 校验，超出返回错误
- 新增 Rust 命令：在对应 `platform/*.rs` 或 `modules/<id>/*.rs` 实现 `#[tauri::command]` → 在 `commands.rs` 的 `invoke_handler()` 中注册 → 前端在对应 `api.ts` 封装
- 新增数据表：在 `platform/database_init.rs` 添加建表语句，并同时在 Rust 侧实现读写

## 实现与历史设计的已知偏差

以下是当前实现与已归档的 `personal-system-design-v2.md` 之间的差异，仅用于追溯设计意图。改动前请知悉，必要时先与用户确认是修实现还是改文档：

- 设计文档要求 UUID v7（时间有序），实际使用 `uuid::Uuid::new_v4()`
- 设计文档中的局域网同步方案未实现，同步子系统已移除（2026-10-30）
- 数据库路径已统一到 `<应用根>/data/mydata.db`（见上“统一 data 目录”）
- 设计文档要求用 `public/modules.json` 注册模块（含 `entryPoint`/`enabled`），实际改为 `src/modules/` 自动发现；模块元数据（`icon`/`order`/`enabled`）由 `ModuleInterface` 实例声明
