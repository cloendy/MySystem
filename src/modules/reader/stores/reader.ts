import { logger } from '../../../platform/services/logger'
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

/** 标注文本预览，避免日志里出现整段内容 */
function previewText(text: string, max = 20): string {
    const flat = text.replace(/\s+/g, ' ').trim();
    return flat.length > max ? `${flat.slice(0, max)}…` : flat;
}

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
            logger.info(
                `[Reader] 打开书籍: ${currentBook.value.title}`
                + ` (${currentBook.value.fileFormat}, ${annotations.value.length} 条标注)`,
            );
        }
    }

    async function addBook(input: CreateBookInput) {
        const book = await api.createBook(input);
        books.value.unshift(book);
        logger.info(`[Reader] 导入书籍: ${book.title} (${book.fileFormat})`);
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
            logger.warn('[Reader] 删除文件失败（可能是外部路径）:', err);
        }

        books.value = books.value.filter((b) => b.id !== id);
        if (currentBook.value?.id === id) currentBook.value = null;
        logger.info(`[Reader] 删除书籍: ${book.title}`);
    }

    async function addAnnotation(input: CreateAnnotationInput) {
        const ann = await api.createAnnotation(input);
        annotations.value.push(ann);
        const bookTitle = currentBook.value?.title
            ?? books.value.find((b) => b.id === input.bookId)?.title
            ?? input.bookId;
        logger.info(
            `[Reader] 添加标注: ${bookTitle} p${input.page}`
            + ` 「${previewText(input.text)}」`,
        );
        return ann;
    }

    async function removeAnnotation(id: string) {
        const target = annotations.value.find((a) => a.id === id);
        await api.deleteAnnotation(id);
        annotations.value = annotations.value.filter((a) => a.id !== id);
        const bookTitle = currentBook.value?.title ?? target?.bookId ?? id;
        logger.info(
            `[Reader] 删除标注: ${bookTitle}`
            + (target ? ` p${target.page} 「${previewText(target.text)}」` : ` (${id})`),
        );
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