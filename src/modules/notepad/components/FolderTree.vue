<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { useNotepadStore } from '../stores/notepad';
import { buildFolderTree, isDescendantOf, type FolderNode } from '../folders';
import { DEFAULT_FOLDER_ID, DEFAULT_FOLDER_NAME, MAX_FOLDER_DEPTH } from '../constants';
import { confirmAction } from '../../../platform/services/dialog';
import type { Folder } from '../types';

const props = defineProps<{
    /** 在编辑页使用时，点击文件夹后返回列表页 */
    navigateToList?: boolean;
    /** 离开当前页面前的回调（编辑页用它先保存笔记） */
    beforeNavigate?: () => Promise<void> | void;
}>();

const store = useNotepadStore();
const router = useRouter();
const expanded = ref<Set<string>>(new Set());

const tree = computed<FolderNode[]>(() => buildFolderTree(store.folders));

const visibleFolders = computed(() => {
    const result: FolderNode[] = [];
    const walk = (nodes: FolderNode[]) => {
        for (const node of nodes) {
            result.push(node);
            if (expanded.value.has(node.id)) walk(node.children);
        }
    };
    walk(tree.value);
    return result;
});

type TreeRow =
    | { kind: 'folder'; folder: FolderNode }
    | { kind: 'input'; level: number };

/** 内联新建输入框的数据 */
const creating = ref<{ parentId: string | null; level: number } | null>(null);
const newFolderName = ref('');
const createError = ref('');
const creatingBusy = ref(false);
const createInputEl = ref<HTMLInputElement | null>(null);

/** 内联重命名输入框的数据 */
const renaming = ref<{ id: string; originalName: string } | null>(null);
const renameName = ref('');
const renameError = ref('');
const renamingBusy = ref(false);
const renameInputEl = ref<HTMLInputElement | null>(null);

/** 把新建输入框插到新文件夹将要出现的位置 */
const rows = computed<TreeRow[]>(() => {
    const list: TreeRow[] = visibleFolders.value.map((folder) => ({ kind: 'folder', folder }));
    const state = creating.value;
    if (!state) return list;

    if (state.parentId === null) {
        // 根级新文件夹的排序号最大，排在所有根级文件夹之后
        list.push({ kind: 'input', level: 1 });
        return list;
    }

    const parentIndex = list.findIndex(
        (row) => row.kind === 'folder' && row.folder.id === state.parentId
    );
    if (parentIndex < 0) return list;

    // 插到父文件夹当前可见子孙的末尾
    let insertAt = parentIndex + 1;
    while (insertAt < list.length) {
        const row = list[insertAt];
        if (row.kind !== 'folder' || !isDescendantOf(store.folders, row.folder.id, state.parentId)) break;
        insertAt++;
    }
    list.splice(insertAt, 0, { kind: 'input', level: state.level });
    return list;
});

function rowKey(row: TreeRow): string {
    return row.kind === 'input' ? '__new_folder__' : row.folder.id;
}

onMounted(async () => {
    await store.fetchFolders();
});

watch(creating, async (state) => {
    if (!state) return;
    await nextTick();
    createInputEl.value?.focus();
    createInputEl.value?.select();
});

function setCreateInput(el: unknown) {
    createInputEl.value = el instanceof HTMLInputElement ? el : null;
}

watch(renaming, async (state) => {
    if (!state) return;
    await nextTick();
    renameInputEl.value?.focus();
    renameInputEl.value?.select();
});

function setRenameInput(el: unknown) {
    renameInputEl.value = el instanceof HTMLInputElement ? el : null;
}

function isDefaultFolder(folder: Folder): boolean {
    return folder.id === DEFAULT_FOLDER_ID;
}

/** 达到最大层级后不再允许创建子文件夹 */
function canAddChild(folder: FolderNode): boolean {
    return !isDefaultFolder(folder) && folder.level < MAX_FOLDER_DEPTH;
}

async function handleSelect(id: string | null) {
    if (props.beforeNavigate) await props.beforeNavigate();
    await store.selectFolder(id);
    if (props.navigateToList) {
        router.push('/notepad/');
    }
}

function startCreate(parent: FolderNode | null) {
    if (parent && !canAddChild(parent)) return;
    cancelRename();
    createError.value = '';
    newFolderName.value = '';
    creating.value = {
        parentId: parent?.id ?? null,
        level: parent ? parent.level + 1 : 1,
    };
    if (parent) {
        expanded.value.add(parent.id);
        expanded.value = new Set(expanded.value);
    }
}

function cancelCreate() {
    creating.value = null;
    newFolderName.value = '';
    createError.value = '';
    creatingBusy.value = false;
}

async function commitCreate() {
    const state = creating.value;
    if (!state || creatingBusy.value) return;

    const name = newFolderName.value.trim();
    if (!name) {
        cancelCreate();
        return;
    }

    const duplicate = store.folders.some(
        (folder) => (folder.parentId ?? null) === state.parentId && folder.name === name
    );
    if (duplicate) {
        createError.value = '同级已存在同名文件夹';
        return;
    }

    creatingBusy.value = true;
    try {
        await store.addFolder(name, state.parentId);
        // 等待期间用户可能已切换到另一处新建，只有仍是同一输入框时才关闭
        if (creating.value === state) cancelCreate();
    } catch (err) {
        if (creating.value === state) {
            createError.value = err instanceof Error ? err.message : String(err);
        }
    } finally {
        creatingBusy.value = false;
    }
}

function handleInputKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
        event.preventDefault();
        commitCreate();
    } else if (event.key === 'Escape') {
        event.preventDefault();
        cancelCreate();
    }
}

async function handleRename(folder: Folder) {
    if (isDefaultFolder(folder)) return;
    cancelCreate();
    renaming.value = { id: folder.id, originalName: folder.name };
    renameName.value = folder.name;
    renameError.value = '';
    renamingBusy.value = false;
}

function cancelRename() {
    renaming.value = null;
    renameName.value = '';
    renameError.value = '';
    renamingBusy.value = false;
}

async function commitRename() {
    const state = renaming.value;
    if (!state || renamingBusy.value) return;

    const name = renameName.value.trim();
    // 名字为空或没有变化，视为取消
    if (!name || name === state.originalName) {
        cancelRename();
        return;
    }

    const folder = store.folders.find((item) => item.id === state.id);
    const duplicate = store.folders.some(
        (item) =>
            item.id !== state.id &&
            (item.parentId ?? null) === (folder?.parentId ?? null) &&
            item.name === name
    );
    if (duplicate) {
        renameError.value = '同级已存在同名文件夹';
        return;
    }

    renamingBusy.value = true;
    try {
        await store.renameFolder(state.id, name);
        if (renaming.value === state) cancelRename();
    } catch (err) {
        if (renaming.value === state) {
            renameError.value = err instanceof Error ? err.message : String(err);
        }
    } finally {
        renamingBusy.value = false;
    }
}

function handleRenameKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
        event.preventDefault();
        commitRename();
    } else if (event.key === 'Escape') {
        event.preventDefault();
        cancelRename();
    }
}

async function handleDelete(folder: Folder) {
    const parentName = folder.parentId
        ? store.folders.find((item) => item.id === folder.parentId)?.name
        : null;
    const message = parentName
        ? `确定删除文件夹「${folder.name}」？其中的笔记和子文件夹会移动到「${parentName}」。`
        : `确定删除文件夹「${folder.name}」？其中的笔记会移动到「${DEFAULT_FOLDER_NAME}」，子文件夹会保留在「全部笔记」下。`;
    if (!(await confirmAction(message, '删除文件夹'))) return;
    await store.removeFolder(folder.id);
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
</script>

<template>
    <div class="side-panel folder-tree">
        <ul class="tree-list">
            <li class="tree-item all-notes" :class="{ active: store.currentFolderId === null }"
                @click="handleSelect(null)">
                <span class="folder-name">全部笔记</span>
                <button class="action-btn all-notes-add" @click.stop="startCreate(null)"
                    title="新建文件夹">＋</button>
            </li>

            <template v-for="row in rows" :key="rowKey(row)">
                <li v-if="row.kind === 'input'" class="tree-item creating-row"
                    :style="{ paddingLeft: `${8 + row.level * 16}px` }">
                    <span class="arrow-placeholder"></span>
                    <input :ref="setCreateInput" v-model="newFolderName" class="folder-name-input"
                        :class="{ invalid: createError }" :title="createError || '文件夹名称'"
                        placeholder="文件夹名称" @keydown="handleInputKeydown" @blur="commitCreate" />
                    <span v-if="createError" class="create-error">{{ createError }}</span>
                </li>
                <li v-else class="tree-item"
                    :class="{ active: store.currentFolderId === row.folder.id }"
                    :style="{ paddingLeft: `${8 + row.folder.level * 16}px` }"
                    @click="handleSelect(row.folder.id)">
                    <span v-if="row.folder.children.length > 0" class="arrow"
                        @click="toggleExpand(row.folder.id, $event)">
                        {{ expanded.has(row.folder.id) ? '▾' : '▸' }}
                    </span>
                    <span v-else class="arrow-placeholder"></span>
                    <input v-if="renaming?.id === row.folder.id" :ref="setRenameInput"
                        v-model="renameName" class="folder-name-input"
                        :class="{ invalid: renameError }" :title="renameError || '文件夹名称'"
                        @click.stop @keydown="handleRenameKeydown" @blur="commitRename" />
                    <template v-else>
                        <span class="folder-name">{{ row.folder.name }}</span>
                        <span v-if="!isDefaultFolder(row.folder)" class="actions">
                            <button v-if="canAddChild(row.folder)" class="action-btn"
                                @click.stop="startCreate(row.folder)" title="新建子文件夹">＋</button>
                            <button class="action-btn" @click.stop="handleRename(row.folder)" title="重命名">✎</button>
                            <button class="action-btn" @click.stop="handleDelete(row.folder)" title="删除">✕</button>
                        </span>
                    </template>
                    <span v-if="renaming?.id === row.folder.id && renameError" class="create-error">{{ renameError }}</span>
                </li>
            </template>
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

.tree-list {
    list-style: none;
    padding: 8px 0;
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

.all-notes-add {
    margin-left: auto;
}

.arrow,
.arrow-placeholder {
    width: 16px;
    display: inline-block;
    font-size: 12px;
    color: #888;
    text-align: center;
    flex-shrink: 0;
}

.arrow:hover {
    color: #2c3e50;
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
    flex-shrink: 0;
}

/* 悬停或选中时都显示右侧图标 */
.tree-item:hover .actions,
.tree-item.active .actions {
    display: flex;
}

.action-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 11px;
    color: #999;
    padding: 0 3px;
    flex-shrink: 0;
}

.action-btn:hover {
    color: #2c3e50;
}

.all-notes-add {
    font-size: 13px;
}

/* 内联新建输入框 */
.creating-row {
    cursor: default;
}

.creating-row:hover {
    background: transparent;
}

.folder-name-input {
    flex: 1;
    min-width: 0;
    margin-left: 4px;
    padding: 1px 4px;
    font-size: 13px;
    font-family: inherit;
    color: #333;
    border: 1px solid #2c3e50;
    border-radius: 3px;
    outline: none;
}

.folder-name-input.invalid {
    border-color: #c0392b;
}

.create-error {
    font-size: 11px;
    color: #c0392b;
    margin-left: 4px;
    max-width: 72px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex-shrink: 0;
}
</style>
