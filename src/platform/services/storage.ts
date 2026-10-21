import { invoke } from '@tauri-apps/api/core';
import type {
    BaseDocument,
    CreateDocumentInput,
    UpdateDocumentInput,
} from './types';
import { getDeviceId } from './device';

/**
 * 文档基础 CRUD 的 invoke 封装。
 *
 * 注意：各业务模块目前通过自己的 `api.ts` 直接 invoke Rust 命令，
 * 本文件暂未被业务代码使用；设备标识已拆分到 `device.ts`。
 */

/**
 * 创建文档
 */
export async function createDocument(
    type: string,
    title: string,
): Promise<BaseDocument> {
    const input: CreateDocumentInput = {
        type,
        title,
        deviceId: getDeviceId(),
    };
    return await invoke<BaseDocument>('create_document', { input });
}

/**
 * 获取单个文档
 */
export async function getDocument(id: string): Promise<BaseDocument | null> {
    return await invoke<BaseDocument | null>('get_document', { id });
}

/**
 * 列出文档（可按 type 过滤）
 */
export async function listDocuments(type?: string): Promise<BaseDocument[]> {
    return await invoke<BaseDocument[]>('list_documents', { type: type ?? null });
}

/**
 * 更新文档标题
 */
export async function updateDocument(
    id: string,
    title: string,
): Promise<BaseDocument> {
    const input: UpdateDocumentInput = {
        id,
        title,
        deviceId: getDeviceId(),
    };
    return await invoke<BaseDocument>('update_document', { input });
}

/**
 * 删除文档（软删除）
 */
export async function deleteDocument(id: string): Promise<void> {
    await invoke('delete_document', {
        id,
        deviceId: getDeviceId(),
    });
}