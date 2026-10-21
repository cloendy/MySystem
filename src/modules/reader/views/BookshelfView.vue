<script setup lang="ts">

import { logger } from '../../../platform/services/logger'
import { confirmAction } from '../../../platform/services/dialog'
import { onMounted, onUnmounted, computed, ref } from 'vue';
import { useRouter } from 'vue-router';
import { useReaderStore } from '../stores/reader';
import type { Book } from '../types';
import { open } from '@tauri-apps/plugin-dialog';
// import { readFile } from '@tauri-apps/plugin-fs';
import { convertFileSrc } from '@tauri-apps/api/core';
import ePub from 'epubjs';
import * as pdfjsLib from 'pdfjs-dist';
import * as api from '../api';

import PdfWorker from 'pdfjs-dist/build/pdf.worker.min.mjs?url';
pdfjsLib.GlobalWorkerOptions.workerSrc = PdfWorker;

const router = useRouter();
const store = useReaderStore();
const filter = ref<'all' | 'reading' | 'finished' | 'unread'>('all');
const importing = ref(false);

onMounted(async () => {
    await store.fetchBooks();
    window.addEventListener('mysystem:sync-completed', handleSyncCompleted);
});

onUnmounted(() => {
    window.removeEventListener('mysystem:sync-completed', handleSyncCompleted);
});

async function handleSyncCompleted() {
    await store.fetchBooks();
}

const filteredBooks = computed(() => {
    if (filter.value === 'all') return store.books;
    return store.books.filter((b) => b.readingStatus === filter.value);
});

function openBook(book: Book) {
    router.push(`/reader/book/${book.id}`);
}

async function handleDelete(book: Book, event: MouseEvent) {
    event.stopPropagation();
    if (!(await confirmAction(`确定删除《${book.title}》？`, '删除书籍'))) return;
    await store.removeBook(book.id);
}

function progressPercent(book: Book): number {
    if (!book.totalPages || book.totalPages <= 0) return 0;
    return Math.min(100, Math.round((book.currentPage / book.totalPages) * 100));
}

function formatIcon(format: string): string {
    switch (format) {
        case 'epub': return '📗';
        case 'pdf': return '📕';
        case 'mobi': return '📘';
        case 'txt': return '📄';
        default: return '📖';
    }
}

function statusLabel(book: Book): string {
    switch (book.readingStatus) {
        case 'reading': return '在读';
        case 'finished': return '已读完';
        default: return '未读';
    }
}

function progressLabel(book: Book): string {
    if (!book.totalPages || book.totalPages <= 0) return '未开始';
    if (book.fileFormat === 'txt') {
        return `第 ${book.currentPage} / ${book.totalPages} 章`;
    }
    return `${book.currentPage} / ${book.totalPages} 页`;
}

/**
 * 按文件格式解析出总页数/章节数
 */
async function detectTotalPages(
    filePath: string,
    format: string,
): Promise<number> {
    try {
        const url = convertFileSrc(filePath);

        if (format === 'pdf') {
            const loadingTask = pdfjsLib.getDocument({ url });
            const doc = await loadingTask.promise;
            const n = doc.numPages;
            // pdfjs-dist 4.x 用 cleanup 代替 destroy
            await doc.cleanup();
            logger.info('[Import] PDF 页数:', n);
            return n;
        }

        if (format === 'epub') {
            const book = ePub(url);
            await Promise.race([
                book.ready,
                new Promise((_, reject) =>
                    setTimeout(() => reject(new Error('EPUB 解析超时')), 8000),
                ),
            ]);
            const n = (book.spine as any)?.length ?? 0;
            book.destroy();
            logger.info('[Import] EPUB 章节数:', n);
            return n;
        }

        if (format === 'txt') {
            const resp = await fetch(url);
            const bytes = new Uint8Array(await resp.arrayBuffer());
            const text = decodeTextForCount(bytes);
            const lines = text.split(/\r?\n/);
            let count = 0;
            for (const raw of lines) {
                const trimmed = raw.trim();
                if (
                    trimmed.startsWith('第') &&
                    /第[一二三四五六七八九十百千万零〇\d]+[章节回]/.test(trimmed) &&
                    trimmed.length < 40
                ) {
                    count++;
                }
            }
            const result = count > 0 ? count : 1;
            logger.info('[Import] TXT 章节数:', result);
            return result;
        }

        return 0;
    } catch (err) {
        logger.warn('[Import] 解析页数失败:', err);
        return 0;
    }
}

/**
 * 轻量解码（用于章节计数）
 */
function decodeTextForCount(bytes: Uint8Array): string {
    if (bytes[0] === 0xff && bytes[1] === 0xfe) {
        return new TextDecoder('utf-16le').decode(bytes).replace(/^\uFEFF/, '');
    }
    if (bytes[0] === 0xfe && bytes[1] === 0xff) {
        return new TextDecoder('utf-16be').decode(bytes).replace(/^\uFEFF/, '');
    }
    if (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
        return new TextDecoder('utf-8').decode(bytes).replace(/^\uFEFF/, '');
    }
    try {
        return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    } catch {
        try {
            return new TextDecoder('gbk').decode(bytes);
        } catch {
            return new TextDecoder('utf-8').decode(bytes);
        }
    }
}



async function handleImport() {
    try {
        const selected = await open({
            multiple: false,
            filters: [
                {
                    name: '电子书',
                    extensions: ['epub', 'pdf', 'mobi', 'txt'],
                },
            ],
        });

        if (!selected || typeof selected !== 'string') return;

        importing.value = true;

        logger.info('[Import] 复制文件:', selected);
        const imported = await api.importBookFile(selected);
        logger.info('[Import] 文件已复制:', imported);

        // 自动识别页数/章节数
        const totalPages = await detectTotalPages(imported.filePath, imported.fileFormat);
        logger.info('[Import] 识别结果 totalPages =', totalPages);

        await store.addBook({
            title: imported.title,
            author: '',
            filePath: imported.filePath,
            fileFormat: imported.fileFormat as 'epub' | 'pdf' | 'mobi' | 'txt',
            totalPages,
        });
    } catch (err) {
        logger.error('[Import] 导入失败:', err);
        alert(`导入失败: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
        importing.value = false;
    }
}
</script>

<template>
    <div class="bookshelf-view">
        <div class="toolbar">
            <h2 class="page-title">我的书架</h2>
            <div class="toolbar-right">
                <select v-model="filter" class="filter-select">
                    <option value="all">全部</option>
                    <option value="reading">在读</option>
                    <option value="unread">未读</option>
                    <option value="finished">已读完</option>
                </select>
                <button class="btn btn-primary" :disabled="importing" @click="handleImport">
                    {{ importing ? '导入中...' : '+ 导入书籍' }}
                </button>
            </div>
        </div>

        <div v-if="!store.loading && filteredBooks.length === 0" class="empty-state">
            <div>书架还是空的</div>
            <div class="hint">点击右上角“导入书籍”开始</div>
        </div>

        <div v-else class="book-grid">
            <div v-for="book in filteredBooks" :key="book.id" class="book-card" @click="openBook(book)">
                <div class="book-cover">
                    <img v-if="book.coverUrl" :src="book.coverUrl" :alt="book.title" />
                    <div v-else class="cover-placeholder">
                        <span class="format-icon">{{ formatIcon(book.fileFormat) }}</span>
                        <span class="format-label">{{ book.fileFormat.toUpperCase() }}</span>
                    </div>
                    <span class="status-badge" :class="book.readingStatus">
                        {{ statusLabel(book) }}
                    </span>
                </div>

                <div class="book-info">
                    <div class="book-title" :title="book.title">{{ book.title }}</div>
                    <div class="book-author">{{ book.author || '未知作者' }}</div>

                    <div v-if="book.totalPages > 0" class="progress-bar">
                        <div class="progress-fill" :style="{ width: progressPercent(book) + '%' }"></div>
                    </div>
                    <div class="progress-text">
                        {{ progressLabel(book) }}
                        <template v-if="book.totalPages > 0"> · {{ progressPercent(book) }}%</template>
                    </div>

                    <button class="btn btn-danger btn-icon" @click="handleDelete(book, $event)">删除</button>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.bookshelf-view {
    padding: 20px;
    height: 100%;
    overflow-y: auto;
}

.toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
    padding: 0;
    border: none;
    background: transparent;
    min-height: auto;
}

.page-title {
    font-size: 20px;
    font-weight: 600;
}

.toolbar-right {
    display: flex;
    gap: 8px;
    align-items: center;
}

.filter-select {
    padding: 6px 10px;
    border: 1px solid #d0d0d0;
    border-radius: 4px;
    background: white;
    font-size: 13px;
    cursor: pointer;
    outline: none;
}

.book-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 16px;
}

.book-card {
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    overflow: hidden;
    background: #fff;
    cursor: pointer;
    transition: transform 0.15s, box-shadow 0.15s;
    display: flex;
    flex-direction: column;
}

.book-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

.book-cover {
    position: relative;
    height: 200px;
    background: #f0f0f0;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
}

.book-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
}

.cover-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: #888;
}

.format-icon {
    font-size: 48px;
}

.format-label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 1px;
    color: #999;
}

.status-badge {
    position: absolute;
    top: 8px;
    right: 8px;
    padding: 3px 8px;
    border-radius: 3px;
    font-size: 11px;
    font-weight: 500;
    background: rgba(255, 255, 255, 0.9);
    color: #666;
}

.status-badge.reading {
    background: #2c3e50;
    color: white;
}

.status-badge.finished {
    background: #27ae60;
    color: white;
}

.book-info {
    padding: 12px;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.book-title {
    font-size: 14px;
    font-weight: 600;
    color: #333;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.book-author {
    font-size: 12px;
    color: #888;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.progress-bar {
    height: 3px;
    background: #e0e0e0;
    border-radius: 2px;
    overflow: hidden;
    margin-top: 6px;
}

.progress-fill {
    height: 100%;
    background: #2c3e50;
    transition: width 0.3s;
}

.progress-text {
    font-size: 11px;
    color: #999;
}

.book-info .btn-danger {
    margin-top: 6px;
    align-self: flex-start;
}
</style>