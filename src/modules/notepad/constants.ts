/** 默认文件夹「未命名」的固定编号，与 Rust 端 `modules/notepad/folders.rs` 保持一致 */
export const DEFAULT_FOLDER_ID = '0001';

/** 默认文件夹名称 */
export const DEFAULT_FOLDER_NAME = '未命名';

/** 文件夹最大嵌套层数：全部笔记 / A / B / C，C 之下不能再建文件夹 */
export const MAX_FOLDER_DEPTH = 3;

/** 编辑器模式在 localStorage 中的键 */
export const EDITOR_MODE_STORAGE_KEY = 'mysystem_notepad_editor_mode';
