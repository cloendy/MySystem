---
name: new-module
description: 在 MySystem 中新增功能模块（前端模块，可选 Rust 存储与命令）。当用户要求新增模块、新建功能页、添加业务模块时使用。
---

# 新增功能模块

开始前先确认（用户未说明时再问）：

- 模块 `id`：小写英文，同时用作目录名、路由前缀、Rust 模块名
- 显示名称、图标（emoji）、排序 `order`（现有模块用 10、20，新模块取 30、40……）
- 平台开关，默认 windows、android 都启用
- 是否需要持久化

## 1. 前端模块（必需）

创建 `src/modules/<id>/index.ts`：

```ts
import { logger } from '../../platform/services/logger';
import type { ModuleInterface } from '../../platform/services/module-types';

const <camelId>Module: ModuleInterface = {
    id: '<id>',
    name: '<名称>',
    version: '1.0.0',
    icon: '<emoji>',
    order: <order>,
    enabled: { windows: true, android: true },

    async onLoad(context) {
        logger.info('[<名称>] 模块已加载，平台:', context.platform);
    },

    getRoutes() {
        return [
            {
                path: '/',
                // 不要加模块前缀，注册时会自动变成 `<id>:index`
                name: 'index',
                component: () => import('./views/<Id>View.vue'),
            },
        ];
    },
};

export default <camelId>Module;
```

约定：

- **不需要修改任何注册文件**。loader 通过 `import.meta.glob('../../modules/*/index.ts')` 自动发现。
- **新增/删除模块目录后必须重启 `pnpm dev`**：glob 在开发服务器启动时展开。
- `id` 必须全库唯一，与目录名保持一致（Rust 侧目录也用同一个 id）。
- 路由 `path` 写相对形式（`/`、`/edit/:id`），会自动挂到 `/<id>/...`；`name` 不要加前缀。
- 模块内部结构：`views/`、`components/`、`stores/`、`api.ts`、`types.ts`。
- 模块自己的 `api.ts` 直接 invoke Rust 命令，不要经过 `platform/services/storage.ts`。
- 日志统一用 `../../platform/services/logger` 的 `logger.*`，不要用 `console.*`。

## 2. 持久化（可选）

五步缺一不可：

1. **建表**：在 `src-tauri/src/platform/database_init.rs` 的 `execute_batch` 中追加 `CREATE TABLE IF NOT EXISTS`。
   - 通用字段（id/title/created_at/updated_at/deleted_at/version/device_id）放 `documents`
   - 业务扩展表用 `document_id TEXT PRIMARY KEY REFERENCES documents(id)` 关联
2. **Rust 存储**：在 `src-tauri/src/modules/<id>/mod.rs` 实现 `#[tauri::command]`，多文件时拆子模块。
   - 数据库路径必须用 `crate::platform::paths::db_path(app)`，不要自己拼
   - 时间戳用 `chrono::Utc::now().timestamp_millis()`
   - 模型加 `#[serde(rename_all = "camelCase")]`
   - 日志用 `crate::log_info!` / `log_warn!` / `log_error!`
3. **声明模块**：在 `src-tauri/src/modules/mod.rs` 加 `pub mod <id>;`
4. **注册命令**：在 `src-tauri/src/commands.rs` 的 `invoke_handler!` 中追加每个命令。**漏注册会报 “command not found”**。
5. **前端封装**：在 `src/modules/<id>/api.ts` 用 `invoke<T>('command_name', { ... })` 封装，参数名 camelCase；设备 ID 从 `platform/services/device` 的 `getDeviceId()` 获取。

## 3. 验证

```bash
pnpm exec vue-tsc --noEmit          # 前端类型检查
cd src-tauri && cargo check          # Rust 编译检查
```

然后重启 `pnpm dev`，确认导航出现、路由可进入、日志有模块加载记录。

## 不要做

- 不要创建或恢复 `public/modules.json`
- 不要把业务存储放回 `src-tauri/src/platform/`（那里只放平台服务）
- 不要手工给路由名加模块前缀
- 不要用裸 `console.*` / `println!` 写日志
