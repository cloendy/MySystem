<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useNotepadStore } from '../stores/notepad';
import type { Folder } from '../types';

const store = useNotepadStore();
const expanded = ref<Set<string>>(new Set());

interface FolderNode extends Folder {
    children: FolderNode[];
    depth: number;
}

const tree = computed<FolderNode[]>(() => {
    const map = new Map<string, FolderNode>();
    const roots: FolderNode[] = [];

    for (const f of store.folders) {
        map.set(f.id, { ...f, children: [], depth: 0 });
    }
    for (const f of store.folders) {
        const node = map.get(f.id)!;
        if (f.parentId && map.has(f.parentId)) {
            const parent = map.get(f.parentId)!;
            node.depth = parent.depth + 1;
            parent.children.push(node);
        } else {
            roots.push(node);
        }
    }
    return roots;
});

function flatten(nodes: FolderNode[]): FolderNode[] {
    const result: FolderNode[] = [];
    for (const n of nodes) {
        result.push(n);
        if (expanded.value.has(n.id)) {
            result.push(...flatten(n.children));
        }
    }
    return result;
}

const visibleFolders = computed(() => flatten(tree.value));

onMounted(async () => {
    await store.fetchFolders();
});

async function handleCreate() {
    const name = prompt('文件夹名称：');
    if (!name?.trim()) return;
    const folder = await store.addFolder(name.trim(), null);
    expanded.value.add(folder.id);
}

async function handleRename(folder: Folder) {
    const name = prompt('新名称：', folder.name);
    if (!name?.trim() || name === folder.name) return;
    await store.renameFolder(folder.id, name.trim());
}

async function handleDelete(folder: Folder) {
    if (!confirm(`确定删除文件夹「${folder.name}」？其中的笔记会移动到「未分类」。`)) return;
    await store.removeFolder(folder.id);
}

function handleSelect(folder: Folder) {
    store.selectFolder(folder.id);
}

function toggleExpand(id: string, event: MouseEvent) {
    event.stopPropagation();
    if (expanded.value.has(id)) {
        expanded.value.delete(id);
    } else {
        expanded.value.add(id);
    }
    expanded.value = new Set(expanded.value);
}

function hasChildren(id: string): boolean {
    return store.folders.some((f) => f.parentId === id);
}
</script>

<template>
    <div class="side-panel folder-tree">
        <div class="side-panel-header">
            <span>文件夹</span>
            <button class="btn-add" @click="handleCreate" title="新建文件夹">+</button>
        </div>

        <ul class="tree-list">
            <li class="tree-item all-notes" :class="{ active: store.currentFolderId === null }"
                @click="store.selectFolder(null)">
                全部笔记
            </li>

            <li v-for="folder in visibleFolders" :key="folder.id" class="tree-item"
                :class="{ active: store.currentFolderId === folder.id }"
                :style="{ paddingLeft: `${8 + folder.depth * 16}px` }" @click="handleSelect(folder)">
                <span v-if="hasChildren(folder.id)" class="arrow" @click="toggleExpand(folder.id, $event)">
                    {{ expanded.has(folder.id) ? '▼' : '▶' }}
                </span>
                <span v-else class="arrow-placeholder"></span>
                <span class="folder-name">{{ folder.name }}</span>
                <span class="actions">
                    <button class="action-btn" @click.stop="handleRename(folder)" title="重命名">✎</button>
                    <button class="action-btn" @click.stop="handleDelete(folder)" title="删除">✕</button>
                </span>
            </li>
        </ul>
    </div>
</template>

<style scoped>
.folder-tree {
    width: 200px;
    border-right: 1px solid #e0e0e0;
    background: #fafafa;
    height: 100%;
    overflow-y: auto;
    flex-shrink: 0;
}

.tree-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 12px;
    font-size: 12px;
    color: #666;
    text-transform: uppercase;
    font-weight: 600;
}

.btn-add {
    background: transparent;
    border: none;
    font-size: 16px;
    cursor: pointer;
    color: #666;
    padding: 0 4px;
}

.btn-add:hover {
    color: #2c3e50;
}

.tree-list {
    list-style: none;
    padding: 0;
    margin: 0;
}

.tree-item {
    display: flex;
    align-items: center;
    padding: 6px 8px;
    cursor: pointer;
    font-size: 13px;
    color: #333;
    position: relative;
}

.tree-item:hover {
    background: #ececec;
}

.tree-item.active {
    background: #e0e8f0;
    color: #2c3e50;
    font-weight: 500;
}

.all-notes {
    padding-left: 12px;
    font-weight: 500;
}

.arrow,
.arrow-placeholder {
    width: 14px;
    display: inline-block;
    font-size: 9px;
    color: #999;
    text-align: center;
}

.folder-name {
    flex: 1;
    margin-left: 4px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.actions {
    display: none;
    gap: 2px;
}

.tree-item:hover .actions {
    display: flex;
}

.action-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 11px;
    color: #999;
    padding: 0 3px;
}

.action-btn:hover {
    color: #2c3e50;
}
</style>