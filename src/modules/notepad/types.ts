export interface Note {
    id: string;
    title: string;
    content: string;
    tags: string[];
    folderId: string | null;
    isPinned: boolean;
    createdAt: number;
    updatedAt: number;
    deletedAt: number | null;
    /** 最后写入设备 ID */
    deviceId: string;
    /** 每次创建/更新/删除 +1 */
    version: number;
}

export interface Folder {
    id: string;
    name: string;
    parentId: string | null;
    sortOrder: number;
}

export interface CreateNoteInput {
    title: string;
    content?: string;
    folderId?: string | null;
    tags?: string[];
}

export interface UpdateNoteInput {
    id: string;
    title?: string;
    content?: string;
    folderId?: string | null;
    tags?: string[];
    isPinned?: boolean;
}

export interface CreateFolderInput {
    name: string;
    parentId?: string | null;
}

export interface UpdateFolderInput {
    id: string;
    name: string;
}

/** 笔记操作菜单的锚点：触发按钮在视口中的位置 */
export interface MenuAnchor {
    top: number;
    bottom: number;
    right: number;
}