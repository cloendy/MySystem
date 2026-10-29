<script setup lang="ts">

import { logger } from '../../../platform/services/logger'
import { onMounted, onUnmounted, ref, nextTick, defineExpose } from 'vue';
import { convertFileSrc } from '@tauri-apps/api/core';
import * as pdfjsLib from 'pdfjs-dist';
import * as api from '../api';
import type { Book } from '../types';

const props = defineProps<{ book: Book }>();

const containerEl = ref<HTMLDivElement | null>(null);
const loading = ref(true);
const errorMsg = ref('');
const pageCount = ref(0);
const currentPage = ref(1);
const scale = ref(1.2);

let pdfDoc: any = null;
let renderingTask: any = null;
let saveTimer: number | null = null;

// 配置 worker
import PdfWorker from 'pdfjs-dist/build/pdf.worker.min.mjs?url';
pdfjsLib.GlobalWorkerOptions.workerSrc = PdfWorker;

function withTimeout<T>(promise: Promise<T>, ms: number, label: string): Promise<T> {
    return Promise.race([
        promise,
        new Promise<T>((_, reject) =>
            setTimeout(() => reject(new Error(`${label} 超时（${ms}ms）`)), ms),
        ),
    ]);
}

/**
 * 防抖保存进度到数据库（500ms 后写入）
 */
function scheduleSave() {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = window.setTimeout(() => {
        saveProgress();
    }, 500);
}

/**
 * 立即保存进度到数据库
 */
async function saveProgress() {
    if (saveTimer) {
        clearTimeout(saveTimer);
        saveTimer = null;
    }
    if (!pageCount.value || currentPage.value <= 0) return;

    // 判断是否已读完
    const isFinished = currentPage.value >= pageCount.value;
    const newStatus: 'reading' | 'finished' = isFinished ? 'finished' : 'reading';

    // 无变化则不写
    if (
        props.book.currentPage === currentPage.value &&
        props.book.readingStatus === newStatus
    ) {
        return;
    }

    try {
        await api.updateBook({
            id: props.book.id,
            currentPage: currentPage.value,
            readingStatus: newStatus,
        });
        logger.info(
            `[PDF] 进度已保存: ${currentPage.value}/${pageCount.value} (${newStatus})`,
        );
    } catch (err) {
        logger.warn('[PDF] 保存进度失败:', err);
    }
}

async function load() {
    const url = convertFileSrc(props.book.filePath);
    logger.info('[PDF] 加载:', url);

    const loadingTask = pdfjsLib.getDocument({
        url,
        cMapUrl: 'https://unpkg.com/pdfjs-dist@4.0.379/cmaps/',
        cMapPacked: true,
    });

    pdfDoc = await withTimeout(loadingTask.promise, 20000, 'PDF 解析');
    pageCount.value = pdfDoc.numPages;
    logger.info('[PDF] 加载完成，共', pageCount.value, '页');

    // 恢复上次阅读页码
    const savedPage = props.book.currentPage;
    if (savedPage && savedPage > 0 && savedPage <= pageCount.value) {
        currentPage.value = savedPage;
        logger.info('[PDF] 恢复上次页码:', savedPage);
    }

    await renderCurrentPage();
}

async function renderCurrentPage() {
    if (!pdfDoc || !containerEl.value) return;

    if (renderingTask) {
        try {
            renderingTask.cancel();
        } catch { }
        renderingTask = null;
    }

    containerEl.value.innerHTML = '';

    const page = await pdfDoc.getPage(currentPage.value);
    const viewport = page.getViewport({ scale: scale.value });

    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('无法获取 canvas 上下文');

    // 用 devicePixelRatio 提升清晰度
    const dpr = window.devicePixelRatio || 1;
    canvas.width = viewport.width * dpr;
    canvas.height = viewport.height * dpr;
    canvas.style.width = `${viewport.width}px`;
    canvas.style.height = `${viewport.height}px`;
    canvas.style.display = 'block';
    canvas.style.margin = '0 auto';
    canvas.style.boxShadow = '0 2px 8px rgba(0,0,0,0.2)';

    containerEl.value.appendChild(canvas);

    renderingTask = page.render({
        canvasContext: ctx,
        viewport,
        transform: dpr !== 1 ? [dpr, 0, 0, dpr, 0, 0] : undefined,
    });

    try {
        await renderingTask.promise;
    } catch (err: any) {
        if (err?.name !== 'RenderingCancelledException') {
            throw err;
        }
    }
}

async function prevPage() {
    if (currentPage.value <= 1) return;
    currentPage.value -= 1;
    await renderCurrentPage();
    scheduleSave();
}

async function nextPage() {
    if (currentPage.value >= pageCount.value) return;
    currentPage.value += 1;
    await renderCurrentPage();
    scheduleSave();
}

async function goToPage(page: number) {
    if (page < 1 || page > pageCount.value) return;
    currentPage.value = page;
    await renderCurrentPage();
    scheduleSave();
}

async function zoomIn() {
    scale.value = Math.min(3, scale.value + 0.2);
    await renderCurrentPage();
}

async function zoomOut() {
    scale.value = Math.max(0.5, scale.value - 0.2);
    await renderCurrentPage();
}

// 键盘翻页和缩放统一由 BookReaderView 处理，避免重复触发

onMounted(async () => {
    try {
        await nextTick();
        await load();
        loading.value = false;
    } catch (err) {
        logger.error('[PDF] 加载失败:', err);
        errorMsg.value = err instanceof Error ? err.message : String(err);
        loading.value = false;
    }
});

onUnmounted(async () => {
    // 卸载时立即保存一次，避免防抖窗口内的翻页丢失
    await saveProgress();
    try {
        renderingTask?.cancel();
    } catch { }
    try {
        pdfDoc?.destroy();
    } catch { }
});

defineExpose({
    prevPage,
    nextPage,
    goToPage,
    zoomIn,
    zoomOut,
    pageCount,
    currentPage,
});
</script>

<template>
    <div class="pdf-reader">
        <div ref="containerEl" class="pdf-scroll"></div>
        <div v-if="loading" class="overlay dark">加载中...</div>
        <div v-if="errorMsg" class="overlay dark error">{{ errorMsg }}</div>
    </div>
</template>

<style scoped>
.pdf-reader {
    width: 100%;
    height: 100%;
    position: relative;
    background: #525659;
    overflow: hidden;
}

.pdf-scroll {
    width: 100%;
    height: 100%;
    overflow: auto;
    padding: 20px;
    display: flex;
    justify-content: center;
    align-items: flex-start;
}
</style>