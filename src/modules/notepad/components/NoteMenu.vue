<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { useNotepadStore } from '../stores/notepad';
import { buildFolderTree, flattenFolderTree } from '../folders';
import type { MenuAnchor, Note } from '../types';
import { useFloatingMenu } from '../composables/useFloatingMenu';

const props = defineProps<{
    note: Note;
    /** 触发按钮在视口中的位置，用于让菜单跟随按钮显示 */
    anchor: MenuAnchor;
}>();

const emit = defineEmits<{
    close: [];
    pin: [];
    move: [folderId: string];
    delete: [];
}>();

const store = useNotepadStore();
const menuEl = ref<HTMLElement | null>(null);
const view = ref<'root' | 'folders'>('root');
const { position, updatePosition } = useFloatingMenu(
    menuEl,
    () => props.anchor,
    () => emit('close')
);

const folderOptions = computed(() => flattenFolderTree(buildFolderTree(store.folders)));

// 切换到文件夹列表后菜单高度会变化，需要重新定位
watch(view, async () => {
    await nextTick();
    updatePosition();
});

function handleMove(folderId: string) {
    if (folderId === props.note.folderId) return;
    emit('move', folderId);
}
</script>

<template>
    <div ref="menuEl" class="note-menu" :style="{
        left: `${position.left}px`,
        top: `${position.top}px`,
        visibility: position.ready ? 'visible' : 'hidden',
    }" @mousedown.stop @click.stop>
        <template v-if="view === 'root'">
            <button class="menu-item" @click="emit('pin')">
                {{ note.isPinned ? '取消置顶' : '置顶' }}
            </button>
            <button class="menu-item" @click="view = 'folders'">
                移动
                <span class="chevron">›</span>
            </button>
            <button class="menu-item danger" @click="emit('delete')">删除</button>
        </template>

        <template v-else>
            <div class="menu-header">
                <button class="menu-back" @click="view = 'root'">←</button>
                <span>移动到文件夹</span>
            </div>
            <div class="menu-folders">
                <button v-for="folder in folderOptions" :key="folder.id" class="menu-item folder-item"
                    :class="{ current: folder.id === note.folderId }"
                    :style="{ paddingLeft: `${10 + (folder.level - 1) * 14}px` }"
                    :disabled="folder.id === note.folderId" @click="handleMove(folder.id)">
                    {{ folder.name }}
                    <span v-if="folder.id === note.folderId" class="current-mark">当前</span>
                </button>
                <div v-if="folderOptions.length === 0" class="menu-empty">暂无文件夹</div>
            </div>
        </template>
    </div>
</template>

<style scoped>
.note-menu {
    position: fixed;
    z-index: 1000;
    min-width: 168px;
    max-height: calc(100vh - 16px);
    padding: 4px;
    background: #fff;
    border: 1px solid #dcdcdc;
    border-radius: 6px;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.14);
    display: flex;
    flex-direction: column;
    font-size: 13px;
}

.menu-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    padding: 7px 10px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: #333;
    font-family: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
}

.menu-item:hover {
    background: #f0f2f4;
}

.menu-item.danger {
    color: #c0392b;
}

.menu-item.danger:hover {
    background: #fdecea;
}

.menu-item:disabled {
    color: #aaa;
    cursor: default;
}

.menu-item:disabled:hover {
    background: transparent;
}

.chevron {
    color: #999;
    font-size: 14px;
    line-height: 1;
}

.menu-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 6px 6px;
    color: #888;
    font-size: 12px;
    border-bottom: 1px solid #f0f0f0;
    margin-bottom: 4px;
}

.menu-back {
    border: none;
    background: transparent;
    color: #666;
    cursor: pointer;
    font-size: 14px;
    padding: 0 2px;
}

.menu-back:hover {
    color: #2c3e50;
}

.menu-folders {
    overflow-y: auto;
    max-height: 240px;
}

.folder-item {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.folder-item.current {
    color: #999;
}

.current-mark {
    font-size: 11px;
    color: #aaa;
    flex-shrink: 0;
}

.menu-empty {
    padding: 10px;
    color: #999;
    font-size: 12px;
    text-align: center;
}
</style>
