<script setup lang="ts">

import { logger } from '../../../platform/services/logger'
import { onMounted, onUnmounted, ref, computed, watch, nextTick } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useReaderStore } from '../stores/reader';
import EpubReader from '../components/EpubReader.vue';
import PdfReader from '../components/PdfReader.vue';
import TxtReader from '../components/TxtReader.vue';
import AnnotationPanel from '../components/AnnotationPanel.vue';
import type { Annotation } from '../types';

const route = useRoute();
const router = useRouter();
const store = useReaderStore();

const loading = ref(true);
const errorMsg = ref('');
const tocVisible = ref(false);
const annotationVisible = ref(false);

const epubRef = ref<InstanceType<typeof EpubReader> | null>(null);
const pdfRef = ref<InstanceType<typeof PdfReader> | null>(null);
const txtRef = ref<InstanceType<typeof TxtReader> | null>(null);
const tocPanelEl = ref<HTMLElement | null>(null);

const jumpInput = ref('');

const fileFormat = computed(() => store.currentBook?.fileFormat ?? '');
const toc = computed(() => epubRef.value?.toc ?? []);
const tocActiveIndex = computed(() => epubRef.value?.currentTocIndex ?? -1);
const currentChapter = computed(() => epubRef.value?.currentChapter ?? '');
const pdfPageCount = computed(() => pdfRef.value?.pageCount ?? 0);
const pdfCurrentPage = computed(() => pdfRef.value?.currentPage ?? 0);
const txtTotalChapters = computed(() => txtRef.value?.totalChapters ?? 0);
const txtCurrentIndex = computed(() => txtRef.value?.currentIndex ?? 0);
const txtPageMode = computed(() => txtRef.value?.pageMode ?? 'page');

/** 当前格式是否支持目录面板（EPUB 侧栏 / TXT 章节列表） */
const hasToc = computed(() => fileFormat.value === 'epub' || fileFormat.value === 'txt');
const jumpPlaceholder = computed(() => (fileFormat.value === 'pdf' ? '页码' : '章节号'));

function goBack() {
    router.push('/reader/');
}

function prevPage() {
    if (fileFormat.value === 'epub') epubRef.value?.prevPage();
    else if (fileFormat.value === 'pdf') pdfRef.value?.prevPage();
    else if (fileFormat.value === 'txt') txtRef.value?.prevPage();
}

function nextPage() {
    if (fileFormat.value === 'epub') epubRef.value?.nextPage();
    else if (fileFormat.value === 'pdf') pdfRef.value?.nextPage();
    else if (fileFormat.value === 'txt') txtRef.value?.nextPage();
}

function toggleToc() {
    if (fileFormat.value === 'epub') tocVisible.value = !tocVisible.value;
    else if (fileFormat.value === 'txt') txtRef.value?.toggleChapterList();
}

function handleTocClick(href: string) {
    epubRef.value?.goToHref(href);
    tocVisible.value = false;
}

function handleJump() {
    const val = jumpInput.value.trim();
    if (!val) return;
    const num = parseInt(val, 10);
    if (isNaN(num)) return;

    if (fileFormat.value === 'pdf') {
        const total = pdfPageCount.value;
        if (num < 1 || num > total) {
            alert(`请输入 1-${total} 之间的页码`);
            return;
        }
        pdfRef.value?.goToPage(num);
    } else if (fileFormat.value === 'txt') {
        const total = txtTotalChapters.value;
        if (num < 1 || num > total) {
            alert(`请输入 1-${total} 之间的章节号`);
            return;
        }
        txtRef.value?.goToChapter(num - 1);
    } else {
        return;
    }
    jumpInput.value = '';
}

/** 标注跳转：按格式转发给对应阅读器 */
function handleAnnotationJump(ann: Annotation) {
    if (fileFormat.value === 'epub') epubRef.value?.jumpToAnnotation(ann);
    else if (fileFormat.value === 'pdf') pdfRef.value?.jumpToAnnotation(ann);
    else if (fileFormat.value === 'txt') txtRef.value?.jumpToAnnotation(ann);
}

/** 删除标注后移除对应高亮 */
function handleAnnotationRemove(ann: Annotation) {
    if (fileFormat.value === 'epub') epubRef.value?.removeAnnotationHighlight(ann);
    else if (fileFormat.value === 'pdf') pdfRef.value?.removeAnnotationHighlight(ann);
    else if (fileFormat.value === 'txt') txtRef.value?.removeAnnotationHighlight(ann);
}

function handleKeydown(e: KeyboardEvent) {
    // 输入框 / 可编辑区域内不拦截按键
    const target = e.target as HTMLElement | null;
    if (target && (
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable
    )) {
        return;
    }

    if (e.key === 'ArrowLeft' || e.key === 'PageUp') {
        e.preventDefault();
        prevPage();
    } else if (e.key === 'ArrowRight' || e.key === 'PageDown') {
        e.preventDefault();
        nextPage();
    } else if (fileFormat.value === 'pdf' && (e.key === '+' || e.key === '=')) {
        e.preventDefault();
        pdfRef.value?.zoomIn();
    } else if (fileFormat.value === 'pdf' && e.key === '-') {
        e.preventDefault();
        pdfRef.value?.zoomOut();
    }
}

/** 打开目录时把当前章节滚动到面板垂直中心 */
function centerActiveToc() {
    const panel = tocPanelEl.value;
    if (!panel) return;
    const active = panel.querySelector<HTMLElement>('.toc-item.active');
    if (!active) return;

    const panelRect = panel.getBoundingClientRect();
    const itemRect = active.getBoundingClientRect();
    panel.scrollTop +=
        itemRect.top - panelRect.top - panelRect.height / 2 + itemRect.height / 2;
}

watch(tocVisible, async (visible) => {
    if (!visible) return;
    await nextTick();
    centerActiveToc();
});

// 目录异步加载完成后再定位一次
watch(
    () => toc.value.length,
    async () => {
        if (!tocVisible.value) return;
        await nextTick();
        centerActiveToc();
    },
);

// TXT 打开后按实际章节数回写 totalPages，顺带修正历史数据
watch(
    txtTotalChapters,
    async (total) => {
        if (fileFormat.value !== 'txt' || !total) return;
        const book = store.currentBook;
        if (!book || book.totalPages === total) return;
        try {
            await store.editBook({ id: book.id, totalPages: total });
            logger.info(`[TXT] 章节数已修正: ${book.totalPages} -> ${total}`);
        } catch (err) {
            logger.warn('[TXT] 修正章节数失败:', err);
        }
    },
    { immediate: true },
);

onMounted(async () => {
    try {
        const id = route.params.id as string;
        await store.selectBook(id);
        if (!store.currentBook) {
            errorMsg.value = '书籍不存在';
            loading.value = false;
            return;
        }

        const fmt = store.currentBook.fileFormat;
        if (fmt !== 'epub' && fmt !== 'pdf' && fmt !== 'txt') {
            errorMsg.value = `暂不支持格式: ${fmt}`;
            loading.value = false;
            return;
        }

        loading.value = false;
        window.addEventListener('keydown', handleKeydown);
    } catch (err) {
        logger.error('[Reader] 出错:', err);
        errorMsg.value = err instanceof Error ? err.message : String(err);
        loading.value = false;
    }
});

onUnmounted(async () => {
    window.removeEventListener('keydown', handleKeydown);
    // 返回书架时刷新书籍列表，让进度和状态同步显示
    try {
        await store.fetchBooks();
        logger.info('[Reader] 书架数据已刷新');
    } catch (err) {
        logger.warn('[Reader] 刷新书架失败:', err);
    }
});
</script>

<template>
    <div class="reader-view">
        <div class="toolbar reader-toolbar">
            <button class="btn" @click="goBack">← 返回书架</button>

            <button v-if="hasToc" class="btn" @click="toggleToc">目录</button>
            <button class="btn" @click="annotationVisible = !annotationVisible">标注</button>

            <span class="book-title">{{ store.currentBook?.title }}</span>

            <span v-if="fileFormat === 'epub'" class="chapter">{{ currentChapter }}</span>
            <span v-if="fileFormat === 'pdf' && pdfPageCount > 0" class="chapter">
                {{ pdfCurrentPage }} / {{ pdfPageCount }} 页
            </span>
            <span v-if="fileFormat === 'txt' && txtTotalChapters > 0" class="chapter">
                第 {{ txtCurrentIndex + 1 }} / {{ txtTotalChapters }} 章
            </span>

            <span class="spacer"></span>

            <!-- PDF / TXT 位置跳转 -->
            <template v-if="(fileFormat === 'txt' && txtTotalChapters > 0) || (fileFormat === 'pdf' && pdfPageCount > 0)">
                <input v-model="jumpInput" class="text-input jump-input" type="text"
                    :placeholder="jumpPlaceholder" @keydown.enter="handleJump" />
                <button class="btn" @click="handleJump">跳转</button>
            </template>

            <!-- TXT 翻页 / 滚动模式 -->
            <button v-if="fileFormat === 'txt'" class="btn"
                :title="`当前：${txtPageMode === 'scroll' ? '滚动模式' : '翻页模式'}，点击切换`"
                @click="txtRef?.togglePageMode()">
                {{ txtPageMode === 'scroll' ? '📜 滚动' : '📄 翻页' }}
            </button>

            <!-- PDF 缩放 -->
            <template v-if="fileFormat === 'pdf'">
                <button class="btn btn-icon" @click="pdfRef?.zoomOut()">−</button>
                <button class="btn btn-icon" @click="pdfRef?.zoomIn()">+</button>
            </template>

            <button class="btn btn-icon" @click="prevPage" title="上一页">‹</button>
            <button class="btn btn-icon" @click="nextPage" title="下一页">›</button>
        </div>

        <div class="reader-body">
            <aside v-if="tocVisible && fileFormat === 'epub'" ref="tocPanelEl" class="side-panel toc-panel">
                <div class="side-panel-header">
                    <span>目录</span>
                </div>
                <ul class="toc-list">
                    <li v-for="(item, idx) in toc" :key="idx" class="toc-item"
                        :class="{ active: idx === tocActiveIndex }"
                        :style="{ paddingLeft: `${12 + item.level * 12}px` }" @click="handleTocClick(item.href)">
                        {{ item.label }}
                    </li>
                </ul>
            </aside>

            <div class="reader-main">
                <div v-if="loading" class="overlay">加载中...</div>
                <div v-else-if="errorMsg" class="overlay error">{{ errorMsg }}</div>
                <EpubReader v-else-if="fileFormat === 'epub' && store.currentBook" ref="epubRef"
                    :book="store.currentBook" />
                <PdfReader v-else-if="fileFormat === 'pdf' && store.currentBook" ref="pdfRef"
                    :book="store.currentBook" />
                <TxtReader v-else-if="fileFormat === 'txt' && store.currentBook" ref="txtRef"
                    :book="store.currentBook" />
            </div>

            <AnnotationPanel v-if="annotationVisible && store.currentBook"
                :book-id="store.currentBook.id" @jump="handleAnnotationJump" @remove="handleAnnotationRemove" />
        </div>
    </div>
</template>

<style scoped>
.reader-view {
    display: flex;
    flex-direction: column;
    height: 100%;
}

.reader-toolbar {
    gap: 8px;
}

.book-title {
    font-size: 14px;
    font-weight: 500;
    color: #333;
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.chapter {
    font-size: 12px;
    color: #888;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.jump-input {
    width: 70px;
    padding: 5px 8px;
    font-size: 12px;
    text-align: center;
}

.reader-body {
    display: flex;
    flex: 1;
    overflow: hidden;
    min-height: 0;
}

.toc-list {
    list-style: none;
    padding: 0;
    margin: 0;
}

.toc-item {
    padding: 8px 12px;
    font-size: 13px;
    color: #333;
    cursor: pointer;
    border-bottom: 1px solid #f0f0f0;
}

.toc-item:hover {
    background: #ececec;
    color: #2c3e50;
}

.toc-item.active {
    background: #e0e8f0;
    color: #2c3e50;
    font-weight: 500;
}

.reader-main {
    flex: 1;
    position: relative;
    overflow: hidden;
    background: #fff;
    min-height: 0;
    min-width: 0;
}
</style>