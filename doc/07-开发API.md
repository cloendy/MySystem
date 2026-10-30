# 07 开发 API

本章列出前端可调用的全部 Tauri 命令、模块封装与事件接口。命令实现分散在 Rust 各模块，但统一在 `src-tauri/src/commands.rs` 注册。

## 7.1 调用约定

前端：

```ts
import { invoke } from '@tauri-apps/api/core';

const notes = await invoke<Note[]>('list_notes', { folderId: null });
```

- 命令名与 Rust 函数名一致（snake_case）。
- 参数对象使用 camelCase（`deviceId`、`folderId`、`filePath`）；Tauri 自动映射到 Rust 的 snake_case 参数。
- 返回值为 `Result<T, String>` 的 `T`；失败时 Promise reject，错误是中文 `String`。
- 业务模块不直接 `invoke`，而是通过自己的 `api.ts` 封装，并自动注入 `getDeviceId()`。
- 新增命令必须加入 `commands.rs` 的 `invoke_handler()`，否则报 “command not found”。

## 7.2 命令总览（28 个）

### 平台：日志（4）

| 命令 | 参数 | 返回 |
|---|---|---|
| `write_log` | `{ level, message }` | 无 |
| `list_log_files` | 无 | `LogFileInfo[]` |
| `read_log_file` | `{ name }` | `string` |
| `get_log_dir` | 无 | `string` |

### 平台：配置（2）

| 命令 | 参数 | 返回 |
|---|---|---|
| `get_config` | `{ key }` | `string \| null` |
| `set_config` | `{ key, value }` | 无 |

### 记事本：笔记（6）

| 命令 | 参数 | 返回 |
|---|---|---|
| `create_note` | `{ input: CreateNoteInput & { deviceId } }` | `Note` |
| `get_note` | `{ id }` | `Note \| null` |
| `list_notes` | `{ folderId: string \| null }` | `Note[]` |
| `update_note` | `{ input: UpdateNoteInput & { deviceId } }` | `Note` |
| `delete_note` | `{ id, deviceId }` | 无 |
| `search_notes` | `{ keyword }` | `Note[]` |

### 记事本：文件夹（4）

| 命令 | 参数 | 返回 |
|---|---|---|
| `create_folder` | `{ input: { name, parentId? } }` | `Folder` |
| `list_folders` | 无 | `Folder[]` |
| `update_folder` | `{ input: { id, name } }` | `Folder` |
| `delete_folder` | `{ id }` | 无 |

### 阅读器（10）

| 命令 | 参数 | 返回 |
|---|---|---|
| `create_book` | `{ input: CreateBookInput & { deviceId } }` | `Book` |
| `get_book` | `{ id }` | `Book \| null` |
| `list_books` | 无 | `Book[]` |
| `update_book` | `{ input: UpdateBookInput & { deviceId } }` | `Book` |
| `delete_book` | `{ id, deviceId }` | 无 |
| `create_annotation` | `{ input: CreateAnnotationInput & { deviceId } }` | `Annotation` |
| `list_annotations` | `{ bookId }` | `Annotation[]` |
| `delete_annotation` | `{ id, deviceId }` | 无 |
| `import_book_file` | `{ sourcePath }` | `{ filePath, fileFormat, title, hash, duplicate, existing }` |
| `delete_book_file` | `{ filePath }` | 无 |

### JSON 分析（1）

| 命令 | 参数 | 返回 |
|---|---|---|
| `read_json_file` | `{ path }` | `{ path, content, size }` |

### 角色图鉴（1）

| 命令 | 参数 | 返回 |
|---|---|---|
| `read_ys_file` | 无 | `{ path, content, size } \| null` |

## 7.3 平台命令详情

### 日志

```ts
interface LogFileInfo {
    name: string;
    size: number;
    modifiedAt: number;  // Unix ms
}
```

- `list_log_files`：只列出 `.log` 文件，按修改时间倒序。
- `read_log_file`：拒绝包含 `/`、`\`、`..` 或非 `.log` 的文件名；超过 512KB 时只返回尾部并加提示。
- `write_log`：级别支持 `debug` / `warn` / `warning` / `error`，其余按 `info`。

### 配置

`get_config` 不存在时返回 `null`；`set_config` 使用 upsert。

## 7.4 记事本命令详情

### `create_note`

```json
{
  "input": {
    "title": "新笔记",
    "content": "",
    "folderId": "0001",
    "tags": [],
    "deviceId": "uuid"
  }
}
```

- `folderId` 省略或为 `null` 时，后端自动设为 `0001`（未命名）；传入不存在的文件夹返回「文件夹不存在」。
- `title` 超过 20 个字符时返回「笔记标题不能超过 20 个字符」。
- 写入 `Note_Content`（`Note_ID` 取 `MAX+1` 补零到十位，`Version = 1`，`Device_ID` 取输入设备，并按目标文件夹解析 `Is_Folder` + `Folder1~3`）。
- 返回完整 `Note`。

### `list_notes`

参数 `folderId`：

- `null`：返回全部未删除笔记；
- `"0001"`：只返回 `Is_Folder = '0'` 的笔记；
- 其他字符串：按「当前层级对应的列」筛选（`Is_Folder='1' AND Folder1=?` 或 `'2' AND Folder2=?` 或 `'3' AND Folder3=?`），避免把后代文件夹中的笔记误算进来。

排序：`Is_Pinned DESC, Update_Time DESC`。

### `update_note`

- 任意字段可选；`None` 不更新。
- 只要调用，`Note_Content.Update_Time` 会更新、`Version + 1`、`Device_ID` 改为当前设备（`Note_ID` 不变）。
- `title` 传空字符串会清空标题；超过 20 个字符返回错误。
- `content` 传空字符串会清空正文。
- `folderId` 有值时重新解析 `Is_Folder` + `Folder1~3`；不能通过 `null` 清空（Rust `Option` 语义），移动请传具体文件夹 ID。
- `isPinned` 传 `false` 可以取消置顶（`Option<bool>`）。
- 返回更新后的完整 `Note`。

### `delete_note`

软删除（`Delete_Time = Update_Time = now`，`Version + 1`，`Device_ID` 改为当前设备）；`affected == 0` 时返回「笔记不存在或已删除」。

### `search_notes`

- 关键词为空（trim 后）返回空数组；
- 使用 `LIKE '%keyword%'` 同时匹配 `Note_Title` 与 `Note_Content`，区分大小写行为由 SQLite 默认决定（ASCII 不区分，中文按字节匹配）。

### `create_folder`

- `parentId` 省略/null 表示根级；
- 不允许在 `0001` 下创建；
- 父级必须存在，且层级 < 3；
- 名称去空格后 1–20 字符；
- 同级重名返回「同级已存在同名文件夹」；
- 编号取当前最大值 +1，排序取同级最大值 +1。

### `list_folders`

返回全部文件夹（含「未命名」），排序 `CAST(Folder_Order AS INTEGER) ASC, Folder_Name ASC`。不返回层级。

### `update_folder`

- `0001` 不可修改；
- 只更新名称，并同步所有直接子级的 `Folder_Parent_Name`；
- 同级重名校验排除自身。

### `delete_folder`

- `0001` 不可删除；
- 笔记移动到父文件夹；根级删除时移到 `0001`；
- 子文件夹上移一级、追加排序、重名自动改名；
- 后代层级递归减 1；
- 物理删除文件夹记录。

## 7.5 阅读器命令详情

### `create_book`

```json
{
  "input": {
    "title": "书名",
    "author": "",
    "coverUrl": "",
    "filePath": "D:/.../data/books/<uuid>.epub",
    "fileFormat": "epub",
    "totalPages": 12,
    "hash": "9f86d081884c7d659a2feaa0c55ad015...",
    "deviceId": "uuid"
  }
}
```

- 写入 `Read_Books`（`Book_ID` 取 `MAX+1` 补零到十位，`Version = 1`，`Device_ID` 取输入设备）；
- `hash` 可选，通常由 `import_book_file` 返回后透传；
- 初始 `currentPage = 0`、`readingStatus = 'unread'`；
- 不校验文件是否存在；允许同一 `Book_Hash` 存在多条记录（用户选择「仍然导入」时）。

### `list_books` / `get_book`

按 `Update_Time DESC` 返回；只返回 `Delete_Time IS NULL` 的书。

### `update_book`

- 可选更新 `title`、`author`、`totalPages`、`currentPage`、`readingStatus`；
- 每次调用都会更新 `Read_Books.Update_Time`、`Version + 1`、`Device_ID`；
- `affected == 0` 时返回「书籍不存在或已删除」；返回更新后的 `Book`。

### `delete_book`

软删除书籍（`Delete_Time = Update_Time = now, Version + 1`），并在同事务内级联软删除其全部标注；不删除物理文件。前端 store 会再调用 `delete_book_file`（若文件仍被其他书籍记录引用则保留）。

### `create_annotation`

- 写入 `Read_Annotation`（`Annotation_ID` 取 `MAX+1` 补零到十位）；
- `bookId` 直接存书籍编号；`note` 默认 `''`，`position` 默认 `'{}'`，`color` 默认 `#FFEB3B`。

### `list_annotations`

按 `Page ASC, Create_Time ASC` 返回指定书籍的未删除标注。

### `delete_annotation`

软删除（`Delete_Time = Update_Time = now, Version + 1`）；`affected == 0` 时返回「标注不存在或已删除」。

### `import_book_file`

- 校验源文件存在、扩展名属于 `epub/pdf/mobi/txt`；
- 流式计算源文件 SHA-256；
- 查 `Read_Books` 中是否存在相同 `Book_Hash` 且未删除的书：
  - 命中：**不复制文件**，返回 `duplicate: true`、`existing: 已有 Book`、`filePath: 已有书籍路径`（前端据此弹出「打开已有 / 仍然导入 / 取消」）；
  - 未命中：复制到 `<data>/books/<uuid>.<ext>`，返回 `duplicate: false`、`existing: null`；
- 标题取文件名（不含扩展名）；
- 返回 `{ filePath, fileFormat, title, hash, duplicate, existing }`；
- 不写入数据库，入库由前端随后调用 `create_book`。

### `delete_book_file`

- 文件不存在时直接成功；
- 规范化路径后必须位于 `<data>/books` 内，否则返回「拒绝删除 data/books 目录之外的文件」；
- 删除失败返回错误。

## 7.6 JSON 分析命令详情

### `read_json_file`

- 校验路径存在且为文件，大小不超过 10 MB；
- 按 UTF-8 读取，失败返回「文件不是有效的 UTF-8 编码」；
- 去掉开头的 BOM 后返回 `{ path, content, size }`（`size` 为文件字节数）；
- 不建表、不持久化；分析本身在前端 `utils/analyzer.ts` 完成。

## 7.7 角色图鉴命令详情

### `read_ys_file`

- 固定读取 `<data>/其他/ys.json`，不接受路径参数；
- 文件不存在时返回 `null`，前端显示「没有找到 ys.json」空状态；
- 文件大小上限 20 MB，按 UTF-8 读取并去掉开头 BOM；
- 返回 `{ path, content, size }`；解析、归一化与筛选都在前端完成，不写数据库。

## 7.8 前端 API 封装

### 记事本 `src/modules/notepad/api.ts`

```ts
createNote(input)      // invoke create_note，注入 deviceId
getNote(id)
listNotes(folderId?)
updateNote(input)      // 注入 deviceId
deleteNote(id)         // 注入 deviceId
searchNotes(keyword)
createFolder(input)
listFolders()
updateFolder(input)
deleteFolder(id)
```

### 阅读器 `src/modules/reader/api.ts`

```ts
createBook(input)          // 注入 deviceId
getBook(id)
listBooks()
updateBook(input)          // 注入 deviceId
deleteBook(id)             // 注入 deviceId
createAnnotation(input)    // 注入 deviceId
listAnnotations(bookId)
deleteAnnotation(id)       // 注入 deviceId
importBookFile(sourcePath)
deleteBookFile(filePath)
```

### JSON 分析 `src/modules/jsonview/api.ts`

```ts
readJsonFile(path)
```

### 角色图鉴 `src/modules/ys/api.ts`

```ts
readYsFile()   // 读取 data/其他/ys.json，缺失时返回 null
```

### 平台 `src/platform/services/config.ts`

```ts
getConfig(key)     // invoke get_config
setConfig(key, value)  // invoke set_config
```

`app_config` 作为通用键值配置能力保留，当前应用没有内置配置键。

### 平台 `src/platform/services/logger.ts`

```ts
logger.debug/info/warn/error(message, ...args)
listLogFiles()
readLogFile(name)
getLogDir()
```

### 平台 `src/platform/services/device.ts`

```ts
getDeviceId(): string  // localStorage 'mysystem_device_id'，首次生成 UUID v4
```

## 7.9 错误信息约定

- 所有命令返回 `Result<_, String>`，错误为可直接展示的中文文案；
- 前端通常让 Promise reject 向上传递，由视图 `try/catch` 或 store 处理；
- 常见错误文案：
  - `打开数据库失败: ...`
  - `笔记不存在或已删除`
  - `同级已存在同名文件夹`
  - `文件夹名称不能超过 20 个字符`
  - `文件夹最多嵌套 3 层`
  - `「未命名」是默认文件夹，不能删除`
  - `不支持的文件格式: xxx`
  - `拒绝删除 data/books 目录之外的文件`
