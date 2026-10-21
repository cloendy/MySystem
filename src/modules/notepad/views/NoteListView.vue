<script setup lang="ts">
import { onMounted, onUnmounted, computed, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { useNotepadStore } from '../stores/notepad';
import FolderTree from '../components/FolderTree.vue';
import TagFilter from '../components/TagFilter.vue';
import NoteMenu from '../components/NoteMenu.vue';
import { DEFAULT_FOLDER_ID } from '../constants';
import type { MenuAnchor, Note } from '../types';
import { detectPlatform } from '../../../platform/services/module-loader';
import { confirmAction } from '../../../platform/services/dialog';

const router = useRouter();
const store = useNotepadStore();
const isMobile = detectPlatform() === 'android';

interface MenuState {
    note: Note;
    anchor: MenuAnchor;
}

const menuState = ref<MenuState | null>(null);

const searchInput = ref('');
let searchTimer: number | null = null;

watch(searchInput, (val) => {
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = window.setTimeout(() => {
        store.search(val);
    }, 250);
});

onMounted(async () => {
    await Promise.all([
        store.fetchNotes(store.currentFolderId),
        // 移动菜单需要文件夹列表；移动端没有 FolderTree，也在这里加载
        store.fetchFolders(),
    ]);
    window.addEventListener('mysystem:sync-completed', handleSyncCompleted);
});

onUnmounted(() => {
    window.removeEventListener('mysystem:sync-completed', handleSyncCompleted);
});

async function handleSyncCompleted() {
    await store.fetchNotes(store.currentFolderId);
    await store.fetchFolders();
}

async function handleCreate() {
    const note = await store.addNote({
        title: '新笔记',
        content: '',
        // 在「全部笔记」下新建时放入默认文件夹「未命名」
        folderId: store.currentFolderId ?? DEFAULT_FOLDER_ID,
    });
    router.push(`/notepad/edit/${note.id}`);
}

function openNoteMenu(note: Note, event: MouseEvent) {
    event.stopPropagation();
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    menuState.value = {
        note,
        anchor: { top: rect.top, bottom: rect.bottom, right: rect.right },
    };
}

async function handleMenuMove(folderId: string) {
    const note = menuState.value?.note;
    menuState.value = null;
    if (!note) return;
    await store.moveNoteToFolder(note.id, folderId);
    // 当前在具体文件夹或搜索结果下时，刷新列表以反映移动后的归属
    if (store.isSearching) {
        await store.search(store.searchKeyword);
    } else if (store.currentFolderId) {
        await store.fetchNotes(store.currentFolderId);
    }
}

async function handleMenuDelete() {
    const note = menuState.value?.note;
    menuState.value = null;
    if (!note) return;
    if (!(await confirmAction('确定删除这条笔记？', '删除笔记'))) return;
    await store.removeNote(note.id);
}

function formatTime(ms: number): string {
    const d = new Date(ms);
    const now = new Date();
    const isToday =
        d.getFullYear() === now.getFullYear() &&
        d.getMonth() === now.getMonth() &&
        d.getDate() === now.getDate();
    if (isToday) {
        return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
    }
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

function preview(content: string): string {
    const line = content.split('\n').find((l) => l.trim().length > 0) ?? '';
    return line.length > 50 ? line.slice(0, 50) + '...' : line;
}

const currentFolderName = computed(() => {
    if (store.isSearching) {
        return `搜索：${store.searchKeyword}`;
    }
    const folderName = store.currentFolderId
        ? store.folders.find((f) => f.id === store.currentFolderId)?.name ?? '全部笔记'
        : '全部笔记';
    if (store.selectedTag) {
        return `${folderName} · #${store.selectedTag}`;
    }
    return folderName;
});

const sortedNotes = computed(() => {
    if (store.isSearching) {
        return store.searchResults;
    }
    let list = store.notes;
    if (store.selectedTag) {
        list = list.filter((n) => n.tags.includes(store.selectedTag!));
    }
    return [...list].sort((a, b) => {
        if (a.isPinned !== b.isPinned) return a.isPinned ? -1 : 1;
        return b.updatedAt - a.updatedAt;
    });
});
</script>

<template>
    <div class="note-list-view">
        <FolderTree v-if="!isMobile" />
        <div class="content-area">
            <div class="toolbar">
                <div class="toolbar-left">
                    <input v-model="searchInput" class="text-input search-input" type="text"
                        placeholder="搜索笔记标题或内容..." />
                    <button v-if="store.isSearching" class="btn btn-ghost"
                        @click="searchInput = ''; store.clearSearch()">
                        清空
                    </button>
                </div>
                <button class="btn btn-primary" @click="handleCreate">+ 新建笔记</button>
            </div>

            <TagFilter v-if="!isMobile" />

            <h2 class="section-title">{{ currentFolderName }}</h2>

            <div v-if="!store.loading && sortedNotes.length === 0" class="empty-state">
                <template v-if="store.isSearching">
                    <div>没有匹配的笔记</div>
                    <div class="hint">试试其他关键词</div>
                </template>
                <template v-else>
                    <div>还没有笔记</div>
                    <div class="hint">点击右上角“新建笔记”开始记录</div>
                </template>
            </div>

            <ul v-else class="note-list">
                <li v-for="note in sortedNotes" :key="note.id" class="note-item"
                    @click="router.push(`/notepad/edit/${note.id}`)">
                    <div class="note-header">
                        <span class="note-title">
                            <span v-if="note.isPinned" class="pin">📌</span>
                            {{ note.title || '无标题' }}
                        </span>
                        <button class="note-menu-btn" title="更多操作"
                            @click="openNoteMenu(note, $event)">⋯</button>
                    </div>
                    <div class="note-preview">{{ preview(note.content) || '（无内容）' }}</div>
                    <div class="note-footer">
                        <div class="tags">
                            <span v-for="tag in note.tags" :key="tag" class="tag">#{{ tag }}</span>
                        </div>
                        <span class="note-time">{{ formatTime(note.updatedAt) }}</span>
                    </div>
                </li>
            </ul>
        </div>

        <NoteMenu v-if="menuState" :note="menuState.note" :anchor="menuState.anchor"
            @close="menuState = null" @move="handleMenuMove" @delete="handleMenuDelete" />
    </div>
</template>

<style scoped>
.note-list-view {
    display: flex;
    height: 100%;
    overflow: hidden;
}

.content-area {
    flex: 1;
    padding: 20px;
    overflow-y: auto;
    min-width: 0;
}

.toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
    gap: 12px;
    padding: 0;
    border-bottom: none;
    background: transparent;
    min-height: auto;
}

.toolbar-left {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
}

.search-input {
    width: 100%;
    max-width: 400px;
}

.section-title {
    font-size: 16px;
    font-weight: 500;
    color: #666;
    margin-bottom: 12px;
}

.note-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 0;
    margin: 0;
}

.note-item {
    padding: 12px 16px;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    cursor: pointer;
    background: #fff;
    transition: background 0.15s, border-color 0.15s;
}

.note-item:hover {
    background: #f8f8f8;
    border-color: #c8c8c8;
}

.note-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 6px;
}

.note-title {
    font-weight: 600;
    font-size: 15px;
    color: #333;
}

.pin {
    font-size: 12px;
    margin-right: 4px;
}

.note-time {
    font-size: 12px;
    color: #999;
    flex-shrink: 0;
}

.note-menu-btn {
    border: none;
    background: transparent;
    color: #999;
    font-size: 18px;
    line-height: 1;
    padding: 0 4px;
    cursor: pointer;
    border-radius: 3px;
}

.note-menu-btn:hover {
    color: #2c3e50;
    background: #ececec;
}

.note-preview {
    font-size: 13px;
    color: #666;
    margin-bottom: 8px;
    line-height: 1.4;
    min-height: 18px;
}

.note-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.tags {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
}

.tag {
    font-size: 11px;
    color: #5a7fa0;
    background: #eaf1f8;
    padding: 2px 6px;
    border-radius: 3px;
}
</style>