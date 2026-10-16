<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed, nextTick } from 'vue';
import { convertFileSrc } from '@tauri-apps/api/core';
import * as api from '../api';
import type { Book } from '../types';

const props = defineProps<{ book: Book }>();

interface Chapter {
    title: string;
    content: string;
    lineStart: number;
}

const loading = ref(true);
const errorMsg = ref('');
const chapters = ref<Chapter[]>([]);
const currentIndex = ref(0);
const contentEl = ref<HTMLDivElement | null>(null);
const showChapterList = ref(false);

const totalChapters = computed(() => chapters.value.length);
const currentChapter = computed(() => chapters.value[currentIndex.value] ?? null);

const paragraphs = computed(() => {
    const content = currentChapter.value?.content ?? '';
    return content
        .split(/\n/)
        .map((l) => l.trim())
        .filter((l) => l.length > 0);
});

/**
 * 按规则解析章节：
 * - 行首为"第"
 * - 上一行为空
 * - 该行长度合理（防止正文中带"第"的长句误判）
 */
function parseChapters(text: string): Chapter[] {
    const lines = text.split(/\r?\n/);
    const result: Chapter[] = [];
    let currentTitle = '序章';
    let currentLines: string[] = [];
    let currentStart = 0;

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const trimmed = line.trim();

        // 更宽松的章节标题识别：
        // 1. 以"第"开头
        // 2. 中间有"章""节""回"之一
        // 3. 长度不超过 40
        // 4. 不是纯数字或标点
        const isChapterStart =
            trimmed.startsWith('第') &&
            /第[一二三四五六七八九十百千万零〇\d]+[章节回]/.test(trimmed) &&
            trimmed.length > 0 &&
            trimmed.length < 40;

        if (isChapterStart) {
            const hasContent = currentLines.some((l) => l.trim().length > 0);
            if (hasContent) {
                result.push({
                    title: currentTitle,
                    content: currentLines.join('\n'),
                    lineStart: currentStart,
                });
            }
            currentTitle = trimmed;
            currentLines = [];
            currentStart = i;
        } else {
            currentLines.push(line);
        }
    }

    const hasContent = currentLines.some((l) => l.trim().length > 0);
    if (hasContent || result.length === 0) {
        result.push({
            title: currentTitle,
            content: currentLines.join('\n'),
            lineStart: currentStart,
        });
    }

    return result;
}

function decodeText(bytes: Uint8Array): string {
    // BOM 检测
    if (bytes[0] === 0xff && bytes[1] === 0xfe) {
        return new TextDecoder('utf-16le').decode(bytes).replace(/^\uFEFF/, '');
    }
    if (bytes[0] === 0xfe && bytes[1] === 0xff) {
        return new TextDecoder('utf-16be').decode(bytes).replace(/^\uFEFF/, '');
    }
    if (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
        return new TextDecoder('utf-8').decode(bytes).replace(/^\uFEFF/, '');
    }

    // 尝试严格 UTF-8
    try {
        return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    } catch {
        // 回退 GBK
        try {
            const text = new TextDecoder('gbk').decode(bytes);
            console.log('[TXT] 使用 GBK 解码');
            return text;
        } catch {
            return new TextDecoder('utf-8').decode(bytes);
        }
    }
}

async function load() {
    const url = convertFileSrc(props.book.filePath);
    console.log('[TXT] 加载:', url);

    const resp = await fetch(url);
    if (!resp.ok) throw new Error(`读取文件失败: HTTP ${resp.status}`);
    const bytes = new Uint8Array(await resp.arrayBuffer());
    console.log('[TXT] 读取完成，字节数:', bytes.length);

    const text = decodeText(bytes);
    chapters.value = parseChapters(text);
    console.log('[TXT] 解析完成，共', chapters.value.length, '章');

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

function scrollToTop() {
    nextTick(() => {
        if (contentEl.value) contentEl.value.scrollTop = 0;
    });
}

function goToChapter(index: number) {
    if (index < 0 || index >= chapters.value.length) return;
    currentIndex.value = index;
    scrollToTop();
    scheduleSave();
    showChapterList.value = false;
}

function prevChapter() {
    if (currentIndex.value > 0) goToChapter(currentIndex.value - 1);
}

function nextChapter() {
    if (currentIndex.value < chapters.value.length - 1) {
        goToChapter(currentIndex.value + 1);
    }
}

function toggleChapterList() {
    showChapterList.value = !showChapterList.value;
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
        console.log(`[TXT] 进度已保存: 第 ${page}/${totalChapters.value} 章`);
    } catch (err) {
        console.warn('[TXT] 保存进度失败:', err);
    }
}

function handleKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement | null;
    if (target?.tagName === 'INPUT') return;

    if (e.key === 'ArrowLeft' || e.key === 'PageUp') {
        e.preventDefault();
        prevChapter();
    } else if (e.key === 'ArrowRight' || e.key === 'PageDown') {
        e.preventDefault();
        nextChapter();
    }
}

onMounted(async () => {
    try {
        await load();
        loading.value = false;
        window.addEventListener('keydown', handleKeydown);
    } catch (err) {
        console.error('[TXT] 加载失败:', err);
        errorMsg.value = err instanceof Error ? err.message : String(err);
        loading.value = false;
    }
});

onUnmounted(async () => {
    window.removeEventListener('keydown', handleKeydown);
    await saveProgress();
});

defineExpose({
    prevChapter,
    nextChapter,
    goToChapter,
    toggleChapterList,
    totalChapters,
    currentIndex,
});
</script>

<template>
    <div class="txt-reader">
        <!-- 章节列表侧栏 -->
        <aside v-if="showChapterList" class="chapter-panel">
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
        <div ref="contentEl" class="txt-content-area">
            <div v-if="loading" class="overlay">加载中...</div>
            <div v-else-if="errorMsg" class="overlay error">{{ errorMsg }}</div>

            <template v-else-if="currentChapter">
                <h2 class="chapter-title">{{ currentChapter.title }}</h2>
                <div class="chapter-body">
                    <p v-for="(p, i) in paragraphs" :key="i">{{ p }}</p>
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
            </template>
        </div>
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
</style>