import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import * as api from '../api';
import type {
    Note, CreateNoteInput, UpdateNoteInput, Folder
} from '../types';

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
    }

    async function addNote(input: CreateNoteInput) {
        const note = await api.createNote(input);
        notes.value.unshift(note);
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
        await api.deleteNote(id);
        notes.value = notes.value.filter((n) => n.id !== id);
        if (currentNote.value?.id === id) currentNote.value = null;
    }

    async function fetchFolders() {
        folders.value = await api.listFolders();
    }

    async function addFolder(name: string, parentId: string | null = null) {
        const folder = await api.createFolder({ name, parentId });
        folders.value.push(folder);
        return folder;
    }

    async function renameFolder(id: string, name: string) {
        const updated = await api.updateFolder({ id, name });
        const idx = folders.value.findIndex((f) => f.id === id);
        if (idx >= 0) folders.value[idx] = updated;
    }

    async function removeFolder(id: string) {
        await api.deleteFolder(id);
        folders.value = folders.value.filter((f) => f.id !== id);
        if (currentFolderId.value === id) {
            currentFolderId.value = null;
            await fetchNotes(null);
        }
    }

    async function selectFolder(id: string | null) {
        currentFolderId.value = id;
        selectedTag.value = null;   // 切换文件夹时清空标签筛选
        await fetchNotes(id);
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
    }

    async function addTagToNote(noteId: string, tag: string) {
        const note = notes.value.find((n) => n.id === noteId);
        if (!note || note.tags.includes(tag)) return;
        const newTags = [...note.tags, tag];
        await editNote({ id: noteId, tags: newTags });
    }

    async function removeTagFromNote(noteId: string, tag: string) {
        const note = notes.value.find((n) => n.id === noteId);
        if (!note) return;
        const newTags = note.tags.filter((t) => t !== tag);
        await editNote({ id: noteId, tags: newTags });
    }

    async function search(keyword: string) {
        searchKeyword.value = keyword;
        if (!keyword.trim()) {
            searchResults.value = [];
            return;
        }
        searchResults.value = await api.searchNotes(keyword.trim());
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