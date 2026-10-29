<script setup lang="ts">

import { logger } from '../../../platform/services/logger'
import { onMounted, onUnmounted, ref, computed, watch, nextTick } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useReaderStore } from '../stores/reader';
import EpubReader from '../components/EpubReader.vue';
import PdfReader from '../components/PdfReader.vue';
import TxtReader from '../components/TxtReader.vue';
import AnnotationPanel from '../components/AnnotationPanel.vue';

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

function handleTocClick(href: string) {
    epubRef.value?.goToHref(href);
    tocVisible.value = false;
}

function handleJump() {
    const val = jumpInput.value.trim();
    if (!val) return;
    const num = parseInt(val, 10);
    const total = txtTotalChapters.value;
    if (isNaN(num) || num < 1 || num > total) {
        alert(`请输入 1-${total} 之间的章节号`);
        return;
    }
    txtRef.value?.goToChapter(num - 1);
    jumpInput.value = '';
}

function handleAnnotationJump(cfi: string) {
    epubRef.value?.goToCfi(cfi);
}

function handleAnnotationRemove(cfi: string) {
    epubRef.value?.removeAnnotationHighlight(cfi);
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

            <button v-if="fileFormat === 'epub'" class="btn" @click="tocVisible = !tocVisible">
                目录
            </button>
            <button v-if="fileFormat === 'epub'" class="btn" @click="annotationVisible = !annotationVisible">
                标注
            </button>
            <button v-if="fileFormat === 'txt'" class="btn" @click="txtRef?.toggleChapterList()">
                目录
            </button>
            <button v-if="fileFormat === 'txt'" class="btn" :title="`当前：${txtPageMode === 'scroll' ? '滚动模式' : '翻页模式'}，点击切换`"
                @click="txtRef?.togglePageMode()">
                {{ txtPageMode === 'scroll' ? '📜 滚动' : '📄 翻页' }}
            </button>

            <span class="book-title">{{ store.currentBook?.title }}</span>

            <span v-if="fileFormat === 'epub'" class="chapter">{{ currentChapter }}</span>
            <span v-if="fileFormat === 'pdf' && pdfPageCount > 0" class="chapter">
                {{ pdfCurrentPage }} / {{ pdfPageCount }} 页
            </span>
            <span v-if="fileFormat === 'txt' && txtTotalChapters > 0" class="chapter">
                第 {{ txtCurrentIndex + 1 }} / {{ txtTotalChapters }} 章
            </span>

            <span class="spacer"></span>

            <!-- TXT 跳章 -->
            <template v-if="fileFormat === 'txt' && txtTotalChapters > 0">
                <input v-model="jumpInput" class="text-input jump-input" type="text" placeholder="章节号"
                    @keydown.enter="handleJump" />
                <button class="btn" @click="handleJump">跳转</button>
            </template>

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

            <AnnotationPanel v-if="annotationVisible && fileFormat === 'epub' && store.currentBook"
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