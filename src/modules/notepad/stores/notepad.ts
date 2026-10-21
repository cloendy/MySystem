import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { logger } from '../../../platform/services/logger';
import * as api from '../api';
import type {
    Note, CreateNoteInput, UpdateNoteInput, Folder
} from '../types';

/** 搜索日志最小间隔，避免逐字输入刷屏 */
const SEARCH_LOG_INTERVAL_MS = 1000;

export const useNotepadStore = defineStore('notepad', () => {
    const notes = ref<Note[]>([]);
    const currentNote = ref<Note | null>(null);
    const loading = ref(false);
    const folders = ref<Folder[]>([]);
    const currentFolderId = ref<string | null>(null);
    const selectedTag = ref<string | null>(null);
    const searchKeyword = ref('');
    const searchResults = ref<Note[]>([]);
    const isSearching = computed(() => searchKeyword.value.trim().length > 0);


    async function fetchNotes(folderId?: string | null) {
        loading.value = true;
        try {
            notes.value = await api.listNotes(folderId);
        } finally {
            loading.value = false;
        }
    }

    async function selectNote(id: string) {
        currentNote.value = await api.getNote(id);
        if (currentNote.value) {
            logger.info(`[Notepad] 打开笔记: ${currentNote.value.title || '(无标题)'}`);
        }
    }

    async function addNote(input: CreateNoteInput) {
        const note = await api.createNote(input);
        notes.value.unshift(note);
        logger.info(`[Notepad] 新建笔记: ${note.title || '(无标题)'} (${note.id})`);
        return note;
    }

    async function editNote(input: UpdateNoteInput) {
        const note = await api.updateNote(input);
        const idx = notes.value.findIndex((n) => n.id === note.id);
        if (idx >= 0) notes.value[idx] = note;
        if (currentNote.value?.id === note.id) currentNote.value = note;
        return note;
    }

    async function removeNote(id: string) {
        const target = notes.value.find((n) => n.id === id)
            ?? (currentNote.value?.id === id ? currentNote.value : null);
        await api.deleteNote(id);
        notes.value = notes.value.filter((n) => n.id !== id);
        if (currentNote.value?.id === id) currentNote.value = null;
        logger.info(`[Notepad] 删除笔记: ${target?.title || id}`);
    }

    async function fetchFolders() {
        folders.value = await api.listFolders();
    }

    async function addFolder(name: string, parentId: string | null = null) {
        const folder = await api.createFolder({ name, parentId });
        folders.value.push(folder);
        logger.info(`[Notepad] 新建文件夹: ${name}`);
        return folder;
    }

    async function renameFolder(id: string, name: string) {
        const oldName = folders.value.find((f) => f.id === id)?.name;
        const updated = await api.updateFolder({ id, name });
        const idx = folders.value.findIndex((f) => f.id === id);
        if (idx >= 0) folders.value[idx] = updated;
        logger.info(`[Notepad] 重命名文件夹: ${oldName || id} -> ${name}`);
    }

    async function removeFolder(id: string) {
        const name = folders.value.find((f) => f.id === id)?.name;
        await api.deleteFolder(id);
        // 子文件夹会被移动到上级，必须重新拉取才能拿到新的 parentId
        await fetchFolders();
        if (currentFolderId.value === id) {
            currentFolderId.value = null;
            await fetchNotes(null);
        }
        logger.info(`[Notepad] 删除文件夹: ${name || id}`);
    }

    async function selectFolder(id: string | null) {
        currentFolderId.value = id;
        selectedTag.value = null;   // 切换文件夹时清空标签筛选
        await fetchNotes(id);
        const name = id ? folders.value.find((f) => f.id === id)?.name : null;
        logger.info(`[Notepad] 切换文件夹: ${name || '全部笔记'}`);
    }

    async function moveNoteToFolder(noteId: string, folderId: string | null) {
        await editNote({ id: noteId, folderId });
    }

    const allTags = computed(() => {
        const tagCount = new Map<string, number>();
        for (const note of notes.value) {
            for (const tag of note.tags) {
                tagCount.set(tag, (tagCount.get(tag) ?? 0) + 1);
            }
        }
        return Array.from(tagCount.entries())
            .map(([name, count]) => ({ name, count }))
            .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name));
    });

    function selectTag(tag: string | null) {
        selectedTag.value = tag;
        logger.info(`[Notepad] 按标签筛选: ${tag || '清除筛选'}`);
    }

    async function addTagToNote(noteId: string, tag: string) {
        const note = notes.value.find((n) => n.id === noteId);
        if (!note || note.tags.includes(tag)) return;
        const newTags = [...note.tags, tag];
        await editNote({ id: noteId, tags: newTags });
        logger.info(`[Notepad] 添加标签: ${tag} -> ${note.title || noteId}`);
    }

    async function removeTagFromNote(noteId: string, tag: string) {
        const note = notes.value.find((n) => n.id === noteId);
        if (!note) return;
        const newTags = note.tags.filter((t) => t !== tag);
        await editNote({ id: noteId, tags: newTags });
        logger.info(`[Notepad] 移除标签: ${tag} <- ${note.title || noteId}`);
    }

    let lastSearchLogAt = 0;

    async function search(keyword: string) {
        searchKeyword.value = keyword;
        if (!keyword.trim()) {
            searchResults.value = [];
            return;
        }
        searchResults.value = await api.searchNotes(keyword.trim());
        const now = Date.now();
        if (now - lastSearchLogAt >= SEARCH_LOG_INTERVAL_MS) {
            lastSearchLogAt = now;
            logger.info(`[Notepad] 搜索 "${keyword.trim()}"：${searchResults.value.length} 条结果`);
        }
    }

    function clearSearch() {
        searchKeyword.value = '';
        searchResults.value = [];
    }

    return {
        notes,
        folders,
        currentFolderId,
        selectedTag,
        allTags,
        currentNote,
        loading,
        fetchNotes,
        fetchFolders,
        selectNote,
        addNote,
        editNote,
        removeNote,
        addFolder,
        renameFolder,
        removeFolder,
        selectFolder,
        moveNoteToFolder,
        selectTag,
        addTagToNote,
        removeTagFromNote,
        searchKeyword,
        searchResults,
        isSearching,
        search,
        clearSearch,
    };
});