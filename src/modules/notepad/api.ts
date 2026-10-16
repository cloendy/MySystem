import { invoke } from '@tauri-apps/api/core';
import { getDeviceId } from '../../platform/services/storage';
import type {
    Folder, CreateFolderInput, UpdateFolderInput,
    Note, CreateNoteInput, UpdateNoteInput
} from './types';

export async function createNote(input: CreateNoteInput): Promise<Note> {
    return await invoke<Note>('create_note', {
        input: {
            ...input,
            deviceId: getDeviceId(),
        },
    });
}

export async function getNote(id: string): Promise<Note | null> {
    return await invoke<Note | null>('get_note', { id });
}

export async function listNotes(folderId?: string | null): Promise<Note[]> {
    return await invoke<Note[]>('list_notes', {
        folderId: folderId ?? null,
    });
}

export async function updateNote(input: UpdateNoteInput): Promise<Note> {
    return await invoke<Note>('update_note', {
        input: {
            ...input,
            deviceId: getDeviceId(),
        },
    });
}

export async function deleteNote(id: string): Promise<void> {
    await invoke('delete_note', {
        id,
        deviceId: getDeviceId(),
    });
}

export async function createFolder(input: CreateFolderInput): Promise<Folder> {
    return await invoke<Folder>('create_folder', { input });
}

export async function listFolders(): Promise<Folder[]> {
    return await invoke<Folder[]>('list_folders');
}

export async function updateFolder(input: UpdateFolderInput): Promise<Folder> {
    return await invoke<Folder>('update_folder', { input });
}

export async function deleteFolder(id: string): Promise<void> {
    await invoke('delete_folder', { id });
}

export async function searchNotes(keyword: string): Promise<Note[]> {
    return await invoke<Note[]>('search_notes', { keyword });
}