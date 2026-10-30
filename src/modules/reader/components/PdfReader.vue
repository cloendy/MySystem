<script setup lang="ts">

import { logger } from '../../../platform/services/logger'
import { onMounted, onUnmounted, ref, nextTick, defineExpose } from 'vue';
import { convertFileSrc } from '@tauri-apps/api/core';
import * as pdfjsLib from 'pdfjs-dist';
import * as api from '../api';
import type { Annotation, Book } from '../types';
import AnnotationBubble from './AnnotationBubble.vue';

const props = defineProps<{ book: Book }>();

interface PdfRect {
    x: number;
    y: number;
    w: number;
    h: number;
}

interface PdfAnnotation {
    id: string;
    color: string;
    page: number;
    rects: PdfRect[];
    text: string;
}

const containerEl = ref<HTMLDivElement | null>(null);
const loading = ref(true);
const errorMsg = ref('');
const pageCount = ref(0);
const currentPage = ref(1);
const scale = ref(1.2);

// 选中气泡
const selectionBubble = ref({
    visible: false,
    x: 0,
    y: 0,
    text: '',
    page: 1,
    rects: [] as PdfRect[],
});

// 已加载的 PDF 标注（position 里带 rects 的记录）
const annotations = ref<PdfAnnotation[]>([]);

let pdfDoc: any = null;
let renderingTask: any = null;
let textLayerTask: any = null;
let saveTimer: number | null = null;

// 当前页的 DOM 引用（每次重渲染时重建）
let pageEl: HTMLDivElement | null = null;
let textLayerEl: HTMLDivElement | null = null;
let highlightLayerEl: HTMLDivElement | null = null;

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

/** 加载已有标注，只保留带页面矩形的记录 */
async function loadAnnotations() {
    try {
        const list = await api.listAnnotations(props.book.id);
        const parsed: PdfAnnotation[] = [];
        for (const ann of list) {
            try {
                const pos = JSON.parse(ann.position || '{}');
                if (typeof pos.page !== 'number' || !Array.isArray(pos.rects) || pos.rects.length === 0) {
                    continue;
                }
                parsed.push({
                    id: ann.id,
                    color: ann.color,
                    page: pos.page,
                    rects: pos.rects,
                    text: ann.text,
                });
            } catch {
                // 忽略解析失败的旧数据
            }
        }
        annotations.value = parsed;
    } catch (err) {
        logger.warn('[PDF] 加载已有标注失败:', err);
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

    await loadAnnotations();
    await renderCurrentPage();
}

/** 渲染当前页的高亮矩形（归一化坐标还原到页面尺寸） */
function renderHighlights() {
    if (!highlightLayerEl) return;
    highlightLayerEl.innerHTML = '';

    const list = annotations.value.filter((a) => a.page === currentPage.value);
    for (const ann of list) {
        for (const r of ann.rects) {
            const div = document.createElement('div');
            div.className = 'pdf-highlight';
            div.style.left = `${r.x * 100}%`;
            div.style.top = `${r.y * 100}%`;
            div.style.width = `${r.w * 100}%`;
            div.style.height = `${r.h * 100}%`;
            div.style.background = `${ann.color}59`;
            div.dataset.annIds = ann.id;
            highlightLayerEl.appendChild(div);
        }
    }
}

async function renderCurrentPage() {
    if (!pdfDoc || !containerEl.value) return;

    if (renderingTask) {
        try {
            renderingTask.cancel();
        } catch { }
        renderingTask = null;
    }
    if (textLayerTask) {
        try {
            textLayerTask.cancel();
        } catch { }
        textLayerTask = null;
    }

    containerEl.value.innerHTML = '';
    pageEl = null;
    textLayerEl = null;
    highlightLayerEl = null;

    const page = await pdfDoc.getPage(currentPage.value);
    const viewport = page.getViewport({ scale: scale.value });

    // 页面容器：canvas / 高亮层 / 文本层叠在一起
    const wrapper = document.createElement('div');
    wrapper.className = 'pdf-page';
    wrapper.style.width = `${viewport.width}px`;
    wrapper.style.height = `${viewport.height}px`;
    pageEl = wrapper;

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
    wrapper.appendChild(canvas);

    const highlights = document.createElement('div');
    highlights.className = 'pdf-highlight-layer';
    highlightLayerEl = highlights;
    wrapper.appendChild(highlights);

    const textLayerDiv = document.createElement('div');
    textLayerDiv.className = 'pdf-text-layer';
    textLayerEl = textLayerDiv;
    wrapper.appendChild(textLayerDiv);

    containerEl.value.appendChild(wrapper);

    renderingTask = page.render({
        canvasContext: ctx,
        viewport,
        transform: dpr !== 1 ? [dpr, 0, 0, dpr, 0, 0] : undefined,
    });

    // 文本层（pdfjs 6 的 TextLayer 按百分比定位，需要自行设置缩放变量）
    const textLayer = new pdfjsLib.TextLayer({
        textContentSource: page.streamTextContent(),
        container: textLayerDiv,
        viewport,
    });
    textLayerTask = textLayer;
    textLayerDiv.style.setProperty('--total-scale-factor', String(viewport.scale));
    textLayerDiv.style.setProperty('--scale-round-x', '1px');
    textLayerDiv.style.setProperty('--scale-round-y', '1px');
    textLayerDiv.style.width = `${viewport.width}px`;
    textLayerDiv.style.height = `${viewport.height}px`;

    textLayer.render().catch((err: any) => {
        if (err?.name !== 'AbortException') {
            logger.warn('[PDF] 文本层渲染失败:', err);
        }
    });

    try {
        await renderingTask.promise;
    } catch (err: any) {
        if (err?.name !== 'RenderingCancelledException') {
            throw err;
        }
    }

    renderHighlights();
}

/** 选中文字后弹出气泡，并把选区换算成归一化矩形 */
function handleMouseUp() {
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed || sel.rangeCount === 0) return;

    const range = sel.getRangeAt(0);
    if (!pageEl || !textLayerEl) return;
    if (!textLayerEl.contains(range.startContainer) || !textLayerEl.contains(range.endContainer)) return;

    const pageRect = pageEl.getBoundingClientRect();
    if (pageRect.width === 0 || pageRect.height === 0) return;

    const rects: PdfRect[] = [];
    let firstRect: DOMRect | null = null;
    for (const r of Array.from(range.getClientRects())) {
        if (r.width < 1 || r.height < 1) continue;
        const left = Math.max(r.left, pageRect.left);
        const top = Math.max(r.top, pageRect.top);
        const right = Math.min(r.right, pageRect.right);
        const bottom = Math.min(r.bottom, pageRect.bottom);
        if (right - left < 1 || bottom - top < 1) continue;
        if (!firstRect) firstRect = r;
        rects.push({
            x: (left - pageRect.left) / pageRect.width,
            y: (top - pageRect.top) / pageRect.height,
            w: (right - left) / pageRect.width,
            h: (bottom - top) / pageRect.height,
        });
    }

    const text = sel.toString().trim();
    if (!text || rects.length === 0 || !firstRect) return;

    selectionBubble.value = {
        visible: true,
        x: firstRect.left + firstRect.width / 2,
        y: firstRect.top - 8,
        text,
        page: currentPage.value,
        rects,
    };
}

/** 保存标注（与 EPUB/TXT 共用同一张表） */
async function saveAnnotation(color: string) {
    const bubble = selectionBubble.value;
    if (!bubble.text || bubble.rects.length === 0) return;

    try {
        const ann = await api.createAnnotation({
            bookId: props.book.id,
            text: bubble.text,
            note: '',
            page: bubble.page,
            position: JSON.stringify({
                page: bubble.page,
                rects: bubble.rects,
                text: bubble.text,
            }),
            color,
        });

        annotations.value = [
            ...annotations.value,
            {
                id: ann.id,
                color,
                page: bubble.page,
                rects: bubble.rects,
                text: bubble.text,
            },
        ];
        renderHighlights();
        window.dispatchEvent(new CustomEvent('mysystem:annotation-added', { detail: ann }));

        selectionBubble.value.visible = false;
        window.getSelection()?.removeAllRanges();
        logger.info('[PDF] 标注已保存:', ann.id);
    } catch (err) {
        logger.error('[PDF] 保存标注失败:', err);
    }
}

/** 跳转到标注位置（供父组件调用） */
async function jumpToAnnotation(ann: Annotation) {
    try {
        const pos = JSON.parse(ann.position || '{}');
        const page = Number(pos.page);
        if (!Number.isFinite(page)) return;
        await goToPage(page);
        const target = containerEl.value?.querySelector<HTMLElement>(
            `[data-ann-ids~="${ann.id}"]`,
        );
        target?.scrollIntoView({ block: 'center', behavior: 'smooth' });
    } catch (err) {
        logger.warn('[PDF] 标注跳转失败:', err);
    }
}

/** 删除标注后移除高亮（供父组件调用） */
function removeAnnotationHighlight(ann: Annotation) {
    annotations.value = annotations.value.filter((a) => a.id !== ann.id);
    renderHighlights();
}

function closeBubble() {
    selectionBubble.value.visible = false;
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
        window.addEventListener('mousedown', closeBubble);
    } catch (err) {
        logger.error('[PDF] 加载失败:', err);
        errorMsg.value = err instanceof Error ? err.message : String(err);
        loading.value = false;
    }
});

onUnmounted(async () => {
    // 卸载时立即保存一次，避免防抖窗口内的翻页丢失
    window.removeEventListener('mousedown', closeBubble);
    await saveProgress();
    try {
        renderingTask?.cancel();
    } catch { }
    try {
        textLayerTask?.cancel();
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
    jumpToAnnotation,
    removeAnnotationHighlight,
    pageCount,
    currentPage,
});
</script>

<template>
    <div class="pdf-reader">
        <div ref="containerEl" class="pdf-scroll" @mouseup="handleMouseUp"></div>
        <div v-if="loading" class="overlay dark">加载中...</div>
        <div v-if="errorMsg" class="overlay dark error">{{ errorMsg }}</div>

        <!-- 选中气泡 -->
        <AnnotationBubble v-if="selectionBubble.visible" :x="selectionBubble.x" :y="selectionBubble.y"
            @select="saveAnnotation" />
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

.pdf-page {
    position: relative;
    flex-shrink: 0;
    background: #fff;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
}

.pdf-page canvas {
    display: block;
}

.pdf-highlight-layer {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 1;
}

.pdf-highlight {
    position: absolute;
    border-radius: 1px;
}

/* pdfjs 6 文本层的最小样式（包内无现成 CSS 可直接引入） */
.pdf-text-layer {
    position: absolute;
    inset: 0;
    overflow: clip;
    line-height: 1;
    letter-spacing: normal;
    word-spacing: normal;
    text-size-adjust: none;
    forced-color-adjust: none;
    transform-origin: 0 0;
    caret-color: CanvasText;
    color-scheme: only light;
    z-index: 2;
    --min-font-size: 1;
    --text-scale-factor: calc(var(--total-scale-factor) * var(--min-font-size));
    --min-font-size-inv: calc(1 / var(--min-font-size));
}

.pdf-text-layer :is(span, br) {
    color: transparent;
    position: absolute;
    white-space: pre;
    cursor: text;
    transform-origin: 0% 0%;
    user-select: text;
}

.pdf-text-layer span {
    z-index: 1;
    --font-height: 0;
    font-size: calc(var(--text-scale-factor) * var(--font-height));
    --scale-x: 1;
    --rotate: 0deg;
    transform: rotate(var(--rotate)) scaleX(var(--scale-x)) scale(var(--min-font-size-inv));
}

.pdf-text-layer .markedContent {
    display: contents;
}

.pdf-text-layer ::selection {
    background: rgba(0, 120, 255, 0.3);
}
</style>
