<script setup lang="ts">

import { logger } from '../../../platform/services/logger'
import { onMounted, onUnmounted, ref, computed, nextTick } from 'vue';
import { convertFileSrc } from '@tauri-apps/api/core';
import * as api from '../api';
import type { Annotation, Book } from '../types';
import { decodeText, parseChapters, type TxtChapter } from '../utils/txt';
import AnnotationBubble from './AnnotationBubble.vue';

const props = defineProps<{ book: Book }>();

type Chapter = TxtChapter;

interface HighlightSegment {
    text: string;
    color?: string;
    ids?: string[];
}

interface RenderedChapter {
    index: number;
    title: string;
    paragraphs: string[];
    offsets: number[];
    segments: HighlightSegment[][];
}

interface TxtAnnotation {
    id: string;
    color: string;
    chapter: number;
    start: number;
    end: number;
    text: string;
}

type PageMode = 'page' | 'scroll';

const PAGE_MODE_KEY = 'reader_txt_page_mode';

const loading = ref(true);
const errorMsg = ref('');
const chapters = ref<Chapter[]>([]);
const currentIndex = ref(0);
const contentEl = ref<HTMLDivElement | null>(null);
const chapterPanelEl = ref<HTMLElement | null>(null);
const showChapterList = ref(false);

// 翻页模式：page = 一次显示一章（左右翻页），scroll = 纵向连续滚动、触底自动加载下一章
const pageMode = ref<PageMode>(readPageMode());
// 滚动模式下已挂载到 DOM 的章节区间（闭区间）
const scrollStart = ref(0);
const scrollEnd = ref(0);

let appending = false;
let prepending = false;
let scrollRaf = 0;

const totalChapters = computed(() => chapters.value.length);
const currentChapter = computed(() => chapters.value[currentIndex.value] ?? null);

// 已加载的标注（仅包含带 chapter/start/end 的 TXT 标注）
const annotations = ref<TxtAnnotation[]>([]);

// 选中气泡
const selectionBubble = ref({
    visible: false,
    x: 0,
    y: 0,
    text: '',
    chapter: 0,
    start: 0,
    end: 0,
});

function readPageMode(): PageMode {
    try {
        return localStorage.getItem(PAGE_MODE_KEY) === 'scroll' ? 'scroll' : 'page';
    } catch {
        return 'page';
    }
}

function splitParagraphs(content: string): string[] {
    return content
        .split(/\n/)
        .map((l) => l.trim())
        .filter((l) => l.length > 0);
}

/** 每个段落在章节纯文本中的起始偏移（段落之间以 \n 分隔） */
function buildOffsets(paragraphs: string[]): number[] {
    const offsets: number[] = [];
    let acc = 0;
    for (const p of paragraphs) {
        offsets.push(acc);
        acc += p.length + 1;
    }
    return offsets;
}

function chapterTextLength(paragraphs: string[], offsets: number[]): number {
    if (paragraphs.length === 0) return 0;
    const last = paragraphs.length - 1;
    return offsets[last] + paragraphs[last].length;
}

/** 按标注位置把段落切成普通/高亮片段，重叠部分以先出现的标注为准 */
function buildSegments(
    paragraphs: string[],
    offsets: number[],
    chapterIndex: number,
): HighlightSegment[][] {
    const list = annotations.value
        .filter((a) => a.chapter === chapterIndex)
        .sort((a, b) => a.start - b.start);

    return paragraphs.map((text, i) => {
        if (list.length === 0) return [{ text }];
        const po = offsets[i];
        const pe = po + text.length;
        const segs: HighlightSegment[] = [];
        let cursor = po;
        for (const ann of list) {
            const s = Math.max(ann.start, po);
            const e = Math.min(ann.end, pe);
            if (e <= s) continue;
            if (s > cursor) segs.push({ text: text.slice(cursor - po, s - po) });
            const from = Math.max(s, cursor);
            if (from < e) {
                segs.push({ text: text.slice(from - po, e - po), color: ann.color, ids: [ann.id] });
                cursor = e;
            }
        }
        if (cursor < pe) segs.push({ text: text.slice(cursor - po) });
        return segs.length > 0 ? segs : [{ text }];
    });
}

function buildRenderedChapter(index: number): RenderedChapter | null {
    const ch = chapters.value[index];
    if (!ch) return null;
    const paragraphs = splitParagraphs(ch.content);
    const offsets = buildOffsets(paragraphs);
    return {
        index,
        title: ch.title,
        paragraphs,
        offsets,
        segments: buildSegments(paragraphs, offsets, index),
    };
}

const currentRendered = computed<RenderedChapter | null>(() => {
    if (!currentChapter.value) return null;
    return buildRenderedChapter(currentIndex.value);
});

const renderedChapters = computed<RenderedChapter[]>(() => {
    const end = Math.min(scrollEnd.value, chapters.value.length - 1);
    const list: RenderedChapter[] = [];
    for (let i = scrollStart.value; i <= end; i++) {
        const ch = buildRenderedChapter(i);
        if (ch) list.push(ch);
    }
    return list;
});

async function load() {
    const url = convertFileSrc(props.book.filePath);
    logger.info('[TXT] 加载:', url);

    const resp = await fetch(url);
    if (!resp.ok) throw new Error(`读取文件失败: HTTP ${resp.status}`);
    const bytes = new Uint8Array(await resp.arrayBuffer());
    logger.info('[TXT] 读取完成，字节数:', bytes.length);

    const text = decodeText(bytes);
    chapters.value = parseChapters(text);
    logger.info('[TXT] 解析完成，共', chapters.value.length, '章');

    if (chapters.value.length === 0) {
        throw new Error('文件为空或无法解析章节');
    }

    // 恢复进度
    if (props.book.currentPage && props.book.currentPage > 0) {
        const idx = props.book.currentPage - 1;
        if (idx >= 0 && idx < chapters.value.length) {
            currentIndex.value = idx;
        }
    }
}

/** 加载已有标注，只保留带 chapter/start/end 的 TXT 标注 */
async function loadAnnotations() {
    try {
        const list = await api.listAnnotations(props.book.id);
        const parsed: TxtAnnotation[] = [];
        for (const ann of list) {
            try {
                const pos = JSON.parse(ann.position || '{}');
                if (
                    typeof pos.chapter !== 'number' ||
                    typeof pos.start !== 'number' ||
                    typeof pos.end !== 'number'
                ) {
                    continue;
                }
                parsed.push({
                    id: ann.id,
                    color: ann.color,
                    chapter: pos.chapter,
                    start: pos.start,
                    end: pos.end,
                    text: ann.text,
                });
            } catch {
                // 忽略解析失败的旧数据
            }
        }
        annotations.value = parsed;
    } catch (err) {
        logger.warn('[TXT] 加载已有标注失败:', err);
    }
}

/** 找到文本节点所在的段落 */
function closestParagraph(node: Node | null): HTMLElement | null {
    let el = node instanceof Element ? node : (node?.parentElement ?? null);
    while (el && !el.matches('p[data-offset]')) el = el.parentElement;
    return el as HTMLElement | null;
}

/** 计算选区端点相对段落开头的字符偏移 */
function offsetWithin(el: HTMLElement, container: Node, offset: number): number {
    const range = document.createRange();
    range.selectNodeContents(el);
    try {
        range.setEnd(container, offset);
    } catch {
        return 0;
    }
    return range.toString().length;
}

/** 选中文字后弹出气泡，并把选区换算成章节内字符偏移 */
function handleMouseUp() {
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed || sel.rangeCount === 0) return;

    const area = contentEl.value;
    if (!area) return;
    const range = sel.getRangeAt(0);
    if (!area.contains(range.startContainer) || !area.contains(range.endContainer)) return;

    const startP = closestParagraph(range.startContainer);
    if (!startP) return;
    const startSection = startP.closest<HTMLElement>('[data-chapter-index]');
    const chapterIndex = Number(startSection?.dataset.chapterIndex ?? currentIndex.value);
    const ch = chapters.value[chapterIndex];
    if (!ch) return;

    const paragraphs = splitParagraphs(ch.content);
    const offsets = buildOffsets(paragraphs);
    const chapterLength = chapterTextLength(paragraphs, offsets);
    const start = Number(startP.dataset.offset ?? 0) + offsetWithin(startP, range.startContainer, range.startOffset);

    // 跨章节时把终点收敛到当前章节末尾
    let end = chapterLength;
    const endP = closestParagraph(range.endContainer);
    const endSection = endP?.closest<HTMLElement>('[data-chapter-index]');
    if (endP && endSection === startSection) {
        end = Number(endP.dataset.offset ?? 0) + offsetWithin(endP, range.endContainer, range.endOffset);
    }
    if (end <= start) return;

    const text = sel.toString().trim();
    if (!text) return;

    const rect = range.getBoundingClientRect();
    selectionBubble.value = {
        visible: true,
        x: rect.left + rect.width / 2,
        y: rect.top - 8,
        text,
        chapter: chapterIndex,
        start,
        end,
    };
}

/** 保存标注（与 EPUB/PDF 共用同一张表） */
async function saveAnnotation(color: string) {
    const bubble = selectionBubble.value;
    if (!bubble.text) return;

    try {
        const ann = await api.createAnnotation({
            bookId: props.book.id,
            text: bubble.text,
            note: '',
            page: bubble.chapter + 1,
            position: JSON.stringify({
                chapter: bubble.chapter,
                start: bubble.start,
                end: bubble.end,
                text: bubble.text,
            }),
            color,
        });

        annotations.value = [
            ...annotations.value,
            {
                id: ann.id,
                color,
                chapter: bubble.chapter,
                start: bubble.start,
                end: bubble.end,
                text: bubble.text,
            },
        ];
        window.dispatchEvent(new CustomEvent('mysystem:annotation-added', { detail: ann }));

        selectionBubble.value.visible = false;
        window.getSelection()?.removeAllRanges();
        logger.info('[TXT] 标注已保存:', ann.id);
    } catch (err) {
        logger.error('[TXT] 保存标注失败:', err);
    }
}

/** 跳转到标注位置（供父组件调用） */
async function jumpToAnnotation(ann: Annotation) {
    try {
        const pos = JSON.parse(ann.position || '{}');
        const chapter = Number(pos.chapter);
        if (!Number.isFinite(chapter)) return;
        await goToChapter(chapter);
        await nextTick();
        const target = contentEl.value?.querySelector<HTMLElement>(
            `[data-ann-ids~="${ann.id}"]`,
        );
        target?.scrollIntoView({ block: 'center', behavior: 'smooth' });
    } catch (err) {
        logger.warn('[TXT] 标注跳转失败:', err);
    }
}

/** 删除标注后移除高亮（供父组件调用） */
function removeAnnotationHighlight(ann: Annotation) {
    annotations.value = annotations.value.filter((a) => a.id !== ann.id);
}

function scrollToTop() {
    nextTick(() => {
        if (contentEl.value) contentEl.value.scrollTop = 0;
    });
}

async function goToChapter(index: number) {
    if (index < 0 || index >= chapters.value.length) return;
    currentIndex.value = index;

    if (pageMode.value === 'scroll') {
        // 重新从目标章节开始构建滚动内容
        scrollStart.value = index;
        scrollEnd.value = index;
        await nextTick();
        if (contentEl.value) contentEl.value.scrollTop = 0;
        await fillViewport();
    } else {
        scrollToTop();
    }

    scheduleSave();
    showChapterList.value = false;
}

async function prevChapter() {
    if (currentIndex.value > 0) await goToChapter(currentIndex.value - 1);
}

async function nextChapter() {
    if (currentIndex.value < chapters.value.length - 1) {
        await goToChapter(currentIndex.value + 1);
    }
}

/** 上一页：翻页模式切上一章，滚动模式向上滚一屏 */
function prevPage() {
    if (pageMode.value === 'scroll') {
        contentEl.value?.scrollBy({
            top: -contentEl.value.clientHeight * 0.9,
            behavior: 'smooth',
        });
    } else {
        void prevChapter();
    }
}

/** 下一页：翻页模式切下一章，滚动模式向下滚一屏 */
function nextPage() {
    if (pageMode.value === 'scroll') {
        contentEl.value?.scrollBy({
            top: contentEl.value.clientHeight * 0.9,
            behavior: 'smooth',
        });
    } else {
        void nextChapter();
    }
}

function toggleChapterList() {
    showChapterList.value = !showChapterList.value;
    if (showChapterList.value) centerActiveChapter();
}

/** 打开目录时把当前章节滚动到面板垂直中心 */
function centerActiveChapter() {
    nextTick(() => {
        const panel = chapterPanelEl.value;
        if (!panel) return;
        const active = panel.querySelector<HTMLElement>('.chapter-item.active');
        if (!active) return;

        const panelRect = panel.getBoundingClientRect();
        const itemRect = active.getBoundingClientRect();
        panel.scrollTop +=
            itemRect.top - panelRect.top - panelRect.height / 2 + itemRect.height / 2;
    });
}

/** 切换翻页 / 滚动模式 */
function togglePageMode() {
    void setPageMode(pageMode.value === 'scroll' ? 'page' : 'scroll');
}

async function setPageMode(mode: PageMode) {
    if (mode === pageMode.value) return;
    pageMode.value = mode;
    try {
        localStorage.setItem(PAGE_MODE_KEY, mode);
    } catch {
        // localStorage 不可用时忽略，仅本次会话生效
    }

    if (mode === 'scroll') {
        scrollStart.value = currentIndex.value;
        scrollEnd.value = currentIndex.value;
        await nextTick();
        if (contentEl.value) contentEl.value.scrollTop = 0;
        await fillViewport();
        logger.info('[TXT] 已切换为滚动模式');
    } else {
        await nextTick();
        if (contentEl.value) contentEl.value.scrollTop = 0;
        logger.info('[TXT] 已切换为翻页模式');
    }
}

/** 滚动模式下章节过短时，继续追加后续章节直到填满视口 */
async function fillViewport() {
    if (pageMode.value !== 'scroll') return;
    await nextTick();
    const el = contentEl.value;
    if (!el) return;

    let guard = 0;
    while (
        scrollEnd.value < chapters.value.length - 1 &&
        el.scrollHeight <= el.clientHeight + 40 &&
        guard < 20
    ) {
        scrollEnd.value += 1;
        guard += 1;
        await nextTick();
    }
}

/** 触底时追加下一批章节 */
async function appendNextChapters() {
    if (appending) return;
    const el = contentEl.value;
    if (!el) return;

    appending = true;
    let guard = 0;
    while (scrollEnd.value < chapters.value.length - 1 && guard < 20) {
        scrollEnd.value += 1;
        guard += 1;
        await nextTick();
        if (el.scrollTop + el.clientHeight < el.scrollHeight - 320) break;
    }
    appending = false;
}

/** 接近顶部时向前补一页，避免用户无法向上回看 */
async function prependPrevChapter() {
    if (prepending || scrollStart.value <= 0) return;
    const el = contentEl.value;
    if (!el) return;

    prepending = true;
    const oldHeight = el.scrollHeight;
    const oldTop = el.scrollTop;
    scrollStart.value -= 1;
    await nextTick();
    // 保持当前阅读位置不跳动
    el.scrollTop = oldTop + (el.scrollHeight - oldHeight);
    prepending = false;
}

function updateActiveFromScroll(el: HTMLDivElement) {
    const headings = el.querySelectorAll<HTMLElement>('[data-chapter-index]');
    if (headings.length === 0) return;

    const anchor = el.getBoundingClientRect().top + 80;
    let active = Number(headings[0].dataset.chapterIndex ?? scrollStart.value);
    for (const heading of headings) {
        if (heading.getBoundingClientRect().top <= anchor) {
            active = Number(heading.dataset.chapterIndex ?? active);
        } else {
            break;
        }
    }

    // 已经滚到最底部时，进度直接记为当前已加载的最后一章
    if (el.scrollTop + el.clientHeight >= el.scrollHeight - 4) {
        active = scrollEnd.value;
    }

    if (active >= 0 && active !== currentIndex.value) {
        currentIndex.value = active;
        scheduleSave();
    }
}

function handleScroll() {
    if (pageMode.value !== 'scroll' || scrollRaf) return;
    scrollRaf = requestAnimationFrame(() => {
        scrollRaf = 0;
        const el = contentEl.value;
        if (!el) return;

        updateActiveFromScroll(el);
        if (el.scrollTop + el.clientHeight >= el.scrollHeight - 240) {
            void appendNextChapters();
        }
        if (el.scrollTop < 120) {
            void prependPrevChapter();
        }
    });
}

// 保存进度（章节号作为页码）
let saveTimer: number | null = null;
function scheduleSave() {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = window.setTimeout(saveProgress, 500);
}

async function saveProgress() {
    if (saveTimer) {
        clearTimeout(saveTimer);
        saveTimer = null;
    }
    if (!totalChapters.value) return;

    const page = currentIndex.value + 1;
    const isFinished = currentIndex.value >= totalChapters.value - 1;
    const status: 'reading' | 'finished' = isFinished ? 'finished' : 'reading';

    if (props.book.currentPage === page && props.book.readingStatus === status) {
        return;
    }

    try {
        await api.updateBook({
            id: props.book.id,
            currentPage: page,
            readingStatus: status,
        });
        logger.info(`[TXT] 进度已保存: 第 ${page}/${totalChapters.value} 章`);
    } catch (err) {
        logger.warn('[TXT] 保存进度失败:', err);
    }
}

function closeBubble() {
    selectionBubble.value.visible = false;
}

onMounted(async () => {
    try {
        await load();
        await loadAnnotations();
        loading.value = false;

        if (pageMode.value === 'scroll') {
            // 从上次阅读的章节开始构建滚动内容
            scrollStart.value = currentIndex.value;
            scrollEnd.value = currentIndex.value;
            await fillViewport();
        }
        window.addEventListener('mousedown', closeBubble);
    } catch (err) {
        logger.error('[TXT] 加载失败:', err);
        errorMsg.value = err instanceof Error ? err.message : String(err);
        loading.value = false;
    }
});

onUnmounted(async () => {
    if (scrollRaf) cancelAnimationFrame(scrollRaf);
    window.removeEventListener('mousedown', closeBubble);
    await saveProgress();
});

defineExpose({
    prevPage,
    nextPage,
    prevChapter,
    nextChapter,
    goToChapter,
    toggleChapterList,
    togglePageMode,
    pageMode,
    totalChapters,
    currentIndex,
    currentChapter,
    jumpToAnnotation,
    removeAnnotationHighlight,
});
</script>

<template>
    <div class="txt-reader">
        <!-- 章节列表侧栏 -->
        <aside v-if="showChapterList" ref="chapterPanelEl" class="chapter-panel">
            <div class="chapter-panel-header">
                <span>目录 ({{ totalChapters }})</span>
                <button class="btn btn-ghost btn-icon" @click="toggleChapterList">×</button>
            </div>
            <ul class="chapter-list">
                <li v-for="(ch, idx) in chapters" :key="idx" class="chapter-item"
                    :class="{ active: idx === currentIndex }" @click="goToChapter(idx)">
                    {{ ch.title }}
                </li>
            </ul>
        </aside>

        <!-- 正文 -->
        <div ref="contentEl" class="txt-content-area" @scroll.passive="handleScroll" @mouseup="handleMouseUp">
            <div v-if="loading" class="overlay">加载中...</div>
            <div v-else-if="errorMsg" class="overlay error">{{ errorMsg }}</div>

            <!-- 翻页模式：一次显示一章 -->
            <template v-else-if="pageMode === 'page' && currentRendered">
                <section :data-chapter-index="currentRendered.index">
                    <h2 class="chapter-title">{{ currentRendered.title }}</h2>
                    <div class="chapter-body">
                        <p v-for="(segments, i) in currentRendered.segments" :key="i"
                            :data-offset="currentRendered.offsets[i]">
                            <span v-for="(seg, si) in segments" :key="si"
                                :class="{ 'txt-highlight': seg.color }"
                                :style="seg.color ? { background: seg.color + '4d' } : undefined"
                                :data-ann-ids="seg.ids ? seg.ids.join(' ') : undefined">{{ seg.text }}</span>
                        </p>
                    </div>

                    <div class="chapter-end">
                        <button v-if="currentIndex > 0" class="btn" @click="prevChapter">
                            ← 上一章
                        </button>
                        <button v-if="currentIndex < totalChapters - 1" class="btn btn-primary" @click="nextChapter">
                            下一章 →
                        </button>
                        <div v-else class="no-more">— 全文完 —</div>
                    </div>
                </section>
            </template>

            <!-- 滚动模式：纵向连续加载 -->
            <template v-else-if="pageMode === 'scroll'">
                <section v-for="ch in renderedChapters" :key="ch.index" class="scroll-chapter"
                    :data-chapter-index="ch.index">
                    <h2 class="chapter-title">{{ ch.title }}</h2>
                    <div class="chapter-body">
                        <p v-for="(segments, i) in ch.segments" :key="i" :data-offset="ch.offsets[i]">
                            <span v-for="(seg, si) in segments" :key="si"
                                :class="{ 'txt-highlight': seg.color }"
                                :style="seg.color ? { background: seg.color + '4d' } : undefined"
                                :data-ann-ids="seg.ids ? seg.ids.join(' ') : undefined">{{ seg.text }}</span>
                        </p>
                    </div>
                </section>

                <div class="scroll-footer">
                    <span v-if="scrollEnd < totalChapters - 1">继续下滑加载下一章…</span>
                    <span v-else class="no-more">— 全文完 —</span>
                </div>
            </template>
        </div>

        <!-- 选中气泡 -->
        <AnnotationBubble v-if="selectionBubble.visible" :x="selectionBubble.x" :y="selectionBubble.y"
            @select="saveAnnotation" />
    </div>
</template>

<style scoped>
.txt-reader {
    width: 100%;
    height: 100%;
    display: flex;
    overflow: hidden;
    position: relative;
    background: #fff;
}

.chapter-panel {
    width: 260px;
    border-right: 1px solid #e0e0e0;
    background: #fafafa;
    overflow-y: auto;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
}

.chapter-panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    font-size: 12px;
    font-weight: 600;
    color: #666;
    text-transform: uppercase;
    border-bottom: 1px solid #e0e0e0;
}

.chapter-list {
    list-style: none;
    padding: 0;
    margin: 0;
}

.chapter-item {
    padding: 8px 16px;
    font-size: 13px;
    color: #333;
    cursor: pointer;
    border-bottom: 1px solid #f0f0f0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.chapter-item:hover {
    background: #ececec;
}

.chapter-item.active {
    background: #e0e8f0;
    color: #2c3e50;
    font-weight: 500;
}

.txt-content-area {
    flex: 1;
    overflow-y: auto;
    padding: 32px 48px 80px;
    background: #fff;
    min-width: 0;
}

.chapter-title {
    font-size: 22px;
    font-weight: 600;
    color: #333;
    text-align: center;
    margin: 0 0 28px;
    padding-bottom: 14px;
    border-bottom: 1px solid #eee;
}

.chapter-body {
    font-size: 16px;
    color: #333;
    font-family: -apple-system, BlinkMacSystemFont, 'PingFang SC', 'Microsoft YaHei', sans-serif;
    max-width: 780px;
    margin: 0 auto;
}

.chapter-body p {
    margin: 0 0 0.9em;
    line-height: 1.9;
    text-indent: 2em;
    word-break: break-word;
}

.txt-highlight {
    border-radius: 2px;
    cursor: pointer;
}

.chapter-end {
    margin-top: 48px;
    padding-top: 24px;
    border-top: 1px dashed #e0e0e0;
    display: flex;
    justify-content: center;
    gap: 12px;
    max-width: 780px;
    margin-left: auto;
    margin-right: auto;
}

.no-more {
    color: #999;
    font-size: 14px;
    font-style: italic;
}

/* 滚动模式 */
.scroll-chapter {
    max-width: 780px;
    margin: 0 auto;
}

.scroll-chapter + .scroll-chapter {
    margin-top: 40px;
    padding-top: 32px;
    border-top: 1px solid #f0f0f0;
}

.scroll-footer {
    max-width: 780px;
    margin: 32px auto 0;
    text-align: center;
    color: #999;
    font-size: 13px;
}
</style>
