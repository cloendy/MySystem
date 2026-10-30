import { invoke } from '@tauri-apps/api/core';
import type { JsonFileContent } from './types';

/** 读取本地 JSON 文本文件（UTF-8，最大 10 MB） */
export async function readJsonFile(path: string): Promise<JsonFileContent> {
    return await invoke<JsonFileContent>('read_json_file', { path });
}
