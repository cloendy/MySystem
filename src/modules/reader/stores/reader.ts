import { defineStore } from 'pinia';
import { ref } from 'vue';
import * as api from '../api';
import type {
    Book,
    Annotation,
    CreateBookInput,
    UpdateBookInput,
    CreateAnnotationInput,
} from '../types';

export const useReaderStore = defineStore('reader', () => {
    const books = ref<Book[]>([]);
    const currentBook = ref<Book | null>(null);
    const annotations = ref<Annotation[]>([]);
    const loading = ref(false);

    async function fetchBooks() {
        loading.value = true;
        try {
            books.value = await api.listBooks();
        } finally {
            loading.value = false;
        }
    }

    async function selectBook(id: string) {
        currentBook.value = await api.getBook(id);
        if (currentBook.value) {
            annotations.value = await api.listAnnotations(id);
        }
    }

    async function addBook(input: CreateBookInput) {
        const book = await api.createBook(input);
        books.value.unshift(book);
        return book;
    }

    async function editBook(input: UpdateBookInput) {
        const book = await api.updateBook(input);
        const idx = books.value.findIndex((b) => b.id === book.id);
        if (idx >= 0) books.value[idx] = book;
        if (currentBook.value?.id === book.id) currentBook.value = book;
        return book;
    }

    async function removeBook(id: string) {
        const book = books.value.find((b) => b.id === id);
        if (!book) return;

        // 先删除数据库记录
        await api.deleteBook(id);

        // 尝试删除物理文件（失败也不影响）
        try {
            await api.deleteBookFile(book.filePath);
        } catch (err) {
            console.warn('[Reader] 删除文件失败（可能是外部路径）:', err);
        }

        books.value = books.value.filter((b) => b.id !== id);
        if (currentBook.value?.id === id) currentBook.value = null;
    }

    async function addAnnotation(input: CreateAnnotationInput) {
        const ann = await api.createAnnotation(input);
        annotations.value.push(ann);
        return ann;
    }

    async function removeAnnotation(id: string) {
        await api.deleteAnnotation(id);
        annotations.value = annotations.value.filter((a) => a.id !== id);
    }

    async function refreshAnnotations(bookId: string) {
        annotations.value = await api.listAnnotations(bookId);
    }

    return {
        books,
        currentBook,
        annotations,
        loading,
        fetchBooks,
        selectBook,
        addBook,
        editBook,
        removeBook,
        addAnnotation,
        removeAnnotation,
        refreshAnnotations,
    };
});