import { invoke } from '@tauri-apps/api/core';
import { getDeviceId } from '../../platform/services/device';
import type {
    Book,
    Annotation,
    CreateBookInput,
    UpdateBookInput,
    CreateAnnotationInput,
} from './types';

export async function createBook(input: CreateBookInput): Promise<Book> {
    return await invoke<Book>('create_book', {
        input: { ...input, deviceId: getDeviceId() },
    });
}

export async function getBook(id: string): Promise<Book | null> {
    return await invoke<Book | null>('get_book', { id });
}

export async function listBooks(): Promise<Book[]> {
    return await invoke<Book[]>('list_books');
}

export async function updateBook(input: UpdateBookInput): Promise<Book> {
    return await invoke<Book>('update_book', {
        input: { ...input, deviceId: getDeviceId() },
    });
}

export async function deleteBook(id: string): Promise<void> {
    await invoke('delete_book', { id, deviceId: getDeviceId() });
}

export async function createAnnotation(input: CreateAnnotationInput): Promise<Annotation> {
    return await invoke<Annotation>('create_annotation', {
        input: { ...input, deviceId: getDeviceId() },
    });
}

export async function listAnnotations(bookId: string): Promise<Annotation[]> {
    return await invoke<Annotation[]>('list_annotations', { bookId });
}

export async function deleteAnnotation(id: string): Promise<void> {
    await invoke('delete_annotation', { id, deviceId: getDeviceId() });
}

export interface ImportedBookResult {
    filePath: string;
    fileFormat: string;
    title: string;
    /** 源文件 SHA-256 */
    hash: string;
    /** 是否命中已存在的书籍（命中时不会复制文件） */
    duplicate: boolean;
    /** 命中时返回已有书籍记录 */
    existing: Book | null;
}

export async function importBookFile(sourcePath: string): Promise<ImportedBookResult> {
    return await invoke<ImportedBookResult>('import_book_file', { sourcePath });
}

export async function deleteBookFile(filePath: string): Promise<void> {
    await invoke('delete_book_file', { filePath });
}