import type { Folder } from './types';

export interface FolderNode extends Folder {
    children: FolderNode[];
    /** 层级，根级文件夹（全部笔记下的 A）为 1 */
    level: number;
}

/**
 * 把扁平文件夹列表组装成树。
 *
 * 层级通过向上递归计算，不依赖后端返回顺序；父节点不存在或出现环时按根级处理。
 */
export function buildFolderTree(folders: Folder[]): FolderNode[] {
    const map = new Map<string, FolderNode>();
    for (const folder of folders) {
        map.set(folder.id, { ...folder, children: [], level: 1 });
    }

    const resolving = new Set<string>();

    function resolveLevel(node: FolderNode): number {
        const parent = node.parentId ? map.get(node.parentId) : undefined;
        if (!parent || parent.id === node.id || resolving.has(node.id)) {
            node.level = 1;
            return node.level;
        }
        resolving.add(node.id);
        node.level = resolveLevel(parent) + 1;
        resolving.delete(node.id);
        return node.level;
    }

    const roots: FolderNode[] = [];
    for (const node of map.values()) resolveLevel(node);
    for (const node of map.values()) {
        const parent = node.parentId ? map.get(node.parentId) : undefined;
        if (parent && parent.id !== node.id) {
            parent.children.push(node);
        } else {
            roots.push(node);
        }
    }

    // 「未命名」的排序为 0，同级其他文件夹从 1 开始
    const bySortOrder = (a: FolderNode, b: FolderNode) =>
        a.sortOrder - b.sortOrder || a.name.localeCompare(b.name);
    const sortTree = (nodes: FolderNode[]) => {
        nodes.sort(bySortOrder);
        nodes.forEach((node) => sortTree(node.children));
    };
    sortTree(roots);

    return roots;
}

/** 按树顺序展开全部文件夹（用于移动笔记时的文件夹选择列表） */
export function flattenFolderTree(nodes: FolderNode[]): FolderNode[] {
    const result: FolderNode[] = [];
    for (const node of nodes) {
        result.push(node);
        result.push(...flattenFolderTree(node.children));
    }
    return result;
}

/** 判断 `id` 是否为 `ancestorId` 的子孙（不包含自身） */
export function isDescendantOf(folders: Folder[], id: string, ancestorId: string): boolean {
    const byId = new Map(folders.map((folder) => [folder.id, folder]));
    const visited = new Set<string>();
    let parentId = byId.get(id)?.parentId ?? null;

    while (parentId) {
        if (parentId === ancestorId) return true;
        if (visited.has(parentId)) return false;
        visited.add(parentId);
        parentId = byId.get(parentId)?.parentId ?? null;
    }
    return false;
}
