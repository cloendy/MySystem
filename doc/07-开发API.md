# 07 开发 API

本章列出前端可调用的全部 Tauri 命令、局域网 HTTP/UDP 协议、模块封装与事件接口。命令实现分散在 Rust 各模块，但统一在 `src-tauri/src/commands.rs` 注册。

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

## 7.2 命令总览（43 个）

### 平台：路径与日志（5）

| 命令 | 参数 | 返回 |
|---|---|---|
| `get_data_dir` | 无 | `string` |
| `write_log` | `{ level, message }` | 无 |
| `list_log_files` | 无 | `LogFileInfo[]` |
| `read_log_file` | `{ name }` | `string` |
| `get_log_dir` | 无 | `string` |

### 平台：文档基础 CRUD（5）

| 命令 | 参数 | 返回 |
|---|---|---|
| `create_document` | `{ input: { type, title, deviceId } }` | `BaseDocument` |
| `get_document` | `{ id }` | `BaseDocument \| null` |
| `list_documents` | `{ type?: string }` | `BaseDocument[]` |
| `update_document` | `{ input: { id, title?, deviceId } }` | `BaseDocument` |
| `delete_document` | `{ id, deviceId }` | 无 |

> 目前未被业务代码使用；模块都使用自己的命令。

### 平台：配置（3）

| 命令 | 参数 | 返回 |
|---|---|---|
| `get_config` | `{ key }` | `string \| null` |
| `set_config` | `{ key, value }` | 无 |
| `delete_config` | `{ key }` | 无 |

### 平台：发现与同步（9）

| 命令 | 参数 | 返回 |
|---|---|---|
| `start_discovery_listener` | `{ deviceId, deviceName, platform }` | 无 |
| `stop_discovery_listener` | 无 | 无 |
| `scan_devices` | `{ timeoutMs?: number, selfDeviceId }` | `DeviceInfo[]` |
| `start_sync_server` | `{ deviceId, deviceName, platform }` | 无 |
| `stop_sync_server` | 无 | 无 |
| `check_remote_device` | `{ ipAddress }` | `RemoteDeviceInfo` |
| `exchange_sync_data` | `{ ipAddress, payload: SyncPayload }` | `SyncResponse` |
| `get_sync_payload` | `{ deviceId, deviceName }` | `SyncPayload` |
| `apply_local_payload` | `{ payload: SyncPayload }` | `[accepted, rejected, merged]` |

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

### 阅读器（11）

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
| `read_book_file` | `{ filePath }` | `number[]`（字节数组） |
| `import_book_file` | `{ sourcePath }` | `{ filePath, fileFormat, title }` |
| `delete_book_file` | `{ filePath }` | 无 |

## 7.3 平台命令详情

### `get_data_dir`

返回 `<data>` 的绝对路径，并确保目录存在。

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

`get_config` 不存在时返回 `null`；`set_config` 使用 upsert；`delete_config` 幂等。

### 发现

```ts
interface DeviceInfo {
    deviceId: string;
    deviceName: string;
    platform: string;   // 'windows' | 'android'
    ipAddress: string;  // 来源 IP
}
```

- `start_discovery_listener`：绑定 `0.0.0.0:54321`，收到 `discover` 后回复 `announce`；重复调用直接返回。
- `stop_discovery_listener`：设置停止标志，线程最多 500ms 后退出。
- `scan_devices(timeoutMs = 2000, selfDeviceId)`：广播两次 `discover`，在超时内收集 `announce`，过滤自己和重复设备。

### 同步

```ts
get_sync_payload(deviceId, deviceName): Promise<SyncPayload>
apply_local_payload(payload): Promise<[number, number, number]>  // accepted, rejected, merged
check_remote_device(ipAddress): Promise<RemoteDeviceInfo>        // 3s 超时
exchange_sync_data(ipAddress, payload): Promise<SyncResponse>    // 30s 超时
```

- `start_sync_server`：在 `0.0.0.0:54322` 启动 tiny_http 服务；重复调用直接返回。
- `stop_sync_server`：发送停止信号并释放服务。
- `apply_local_payload` 返回数组顺序是 `(accepted, rejected, merged)`，与 `SyncResponse` 的字段顺序不同，使用时注意。

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
- 写入 `Note_Content`（`Note_ID` 取 `MAX+1` 补零到十位，`Version = 1`，`Device_ID` 取输入设备，并按目标文件夹解析 `Is_Folder` + `Folder1~3`），同时写一条 `sync_operations(create)`，`document_id` 使用 `Note_ID`。
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
    "deviceId": "uuid"
  }
}
```

- 写入 `documents(type='book')` 与 `books`；
- 初始 `currentPage = 0`、`readingStatus = 'unread'`；
- 不校验文件是否存在。

### `list_books` / `get_book`

按 `updated_at DESC` 返回；只返回 `deleted_at IS NULL` 的书。

### `update_book`

- 可选更新 `title`、`author`、`currentPage`、`readingStatus`；
- 每次调用都会更新 `documents.updated_at`、`version + 1`、`device_id`；
- 返回更新后的 `Book`。

### `delete_book`

软删除书籍文档；不删除物理文件、不级联标注。前端 store 会再调用 `delete_book_file`。

### `create_annotation`

- 写入 `documents(type='annotation', title=标注文本)` 与 `annotations`；
- `note` 默认 `''`，`position` 默认 `'{}'`，`color` 默认 `#FFEB3B`；
- 同时写 `sync_operations(create)`。

### `list_annotations`

按 `page ASC, created_at ASC` 返回指定书籍的未删除标注。

### `delete_annotation`

软删除，不检查是否已删除。

### `read_book_file`

直接读取任意路径文件并返回字节数组。前端封装为 `Uint8Array`。注意这是同步命令，读取大文件会阻塞 IPC。

### `import_book_file`

- 校验源文件存在、扩展名属于 `epub/pdf/mobi/txt`；
- 标题取文件名（不含扩展名）；
- 复制到 `<data>/books/<uuid>.<ext>`；
- 返回 `{ filePath, fileFormat, title }`；
- 不写入数据库，入库由前端随后调用 `create_book`。

### `delete_book_file`

- 文件不存在时直接成功；
- 规范化路径后必须位于 `<data>/books` 内，否则返回「拒绝删除 data/books 目录之外的文件」；
- 删除失败返回错误。

## 7.6 局域网协议

### UDP 发现（端口 54321）

广播地址：`255.255.255.255:54321`，JSON，单包最大 2048 字节。

`discover`（扫描方 → 广播）：

```json
{
  "magic": "MYSYSTEM_V1",
  "msgType": "discover",
  "deviceId": "<本机 id>",
  "deviceName": null,
  "platform": null,
  "replyPort": 54321
}
```

`replyPort` 是扫描方临时 UDP 套接字的本地端口，应答直接发往 `源 IP:replyPort`。

`announce`（监听方 → 扫描方）：

```json
{
  "magic": "MYSYSTEM_V1",
  "msgType": "announce",
  "deviceId": "<设备 id>",
  "deviceName": "我的电脑",
  "platform": "windows",
  "replyPort": null
}
```

规则：

- `magic` 或 `msgType` 不匹配忽略；
- 监听方忽略自己发出的 `discover`；
- 扫描方忽略自己、按 `deviceId` 去重；
- 广播发送两次，间隔 50ms；
- 监听读超时 500ms，扫描读超时 200ms。

### HTTP 同步（端口 54322）

#### `GET /api/info`

响应 200：

```json
{
  "deviceId": "uuid",
  "deviceName": "我的电脑",
  "platform": "windows",
  "protocolVersion": 1
}
```

#### `POST /api/sync/exchange`

请求体：`SyncPayload`

```json
{
  "documents": [
    {
      "id": "uuid",
      "type": "book",
      "title": "标题",
      "createdAt": 1730000000000,
      "updatedAt": 1730000000000,
      "deletedAt": null,
      "version": 1,
      "deviceId": "uuid"
    }
  ],
  "notes": [],
  "senderDeviceId": "uuid",
  "senderDeviceName": "发起方"
}
```

说明：笔记已迁至独立的 `Note_Content` 表，本次不参与同步——`documents` 只包含非 `note` 类型，`notes` 始终为空；服务端合并时跳过 `type = "note"` 的文档并忽略收到的 `notes`。字段保留仅为新旧客户端兼容。

响应 200：`SyncResponse`

```json
{
  "accepted": 2,
  "rejected": 1,
  "merged": 3,
  "serverPayload": { "documents": [], "notes": [], "senderDeviceId": "...", "senderDeviceName": "..." }
}
```

错误：

| 状态码 | 场景 |
|---|---|
| 400 | 读取请求体失败、JSON 解析失败 |
| 404 | 其他路径 |
| 500 | 数据库路径/打开/合并/读取失败 |

服务端行为：合并请求数据 → 读取合并后的本地全量 → 返回。发起方收到后需再调用 `apply_local_payload` 合并 `serverPayload`，完成双向同步。

## 7.7 前端 API 封装

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
readBookFile(filePath)     // number[] → Uint8Array
importBookFile(sourcePath)
deleteBookFile(filePath)
```

### 平台 `src/platform/services/config.ts`

```ts
getConfig / setConfig / deleteConfig
getSyncReceiveEnabled / setSyncReceiveEnabled
getDeviceName / setDeviceName
startDiscoveryListener / stopDiscoveryListener
scanDevices(selfDeviceId, timeoutMs?)
startSyncServer / stopSyncServer
checkRemoteDevice(ipAddress)
getSyncPayload(deviceId, deviceName)
exchangeSyncData(ipAddress, payload)
applyLocalPayload(payload)
```

`CONFIG_KEYS`：`sync.receive.enabled`、`sync.port`、`device.name`。

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

## 7.8 错误信息约定

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
  - `对方返回错误状态码: ...`
