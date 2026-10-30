import { invoke } from '@tauri-apps/api/core';
import type { YsFileContent } from './types';

/** 读取 `<data>/其他/ys.json`；文件不存在时返回 null */
export async function readYsFile(): Promise<YsFileContent | null> {
    return await invoke<YsFileContent | null>('read_ys_file');
}
