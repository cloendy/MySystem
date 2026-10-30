<script setup lang="ts">

import { logger } from '../../../platform/services/logger'
import { onMounted, onUnmounted, ref, nextTick, defineExpose } from 'vue';
import ePub, { Book as EpubBook, Rendition } from 'epubjs';
import { convertFileSrc } from '@tauri-apps/api/core';
import type { Annotation, Book } from '../types';
import * as api from '../api';
import AnnotationBubble from './AnnotationBubble.vue';

const props = defineProps<{ book: Book }>();

// 选中气泡
const selectionBubble = ref<{
    visible: boolean;
    x: number;
    y: number;
    text: string;
    cfi: string;
}>({
    visible: false,
    x: 0,
    y: 0,
    text: '',
    cfi: '',
});

// 已应用的标注 cfi 集合（用于避免重复高亮）
const appliedHighlights = new Set<string>();

const viewerEl = ref<HTMLDivElement | null>(null);
const toc = ref<Array<{ label: string; href: string; level: number }>>([]);
const currentChapter = ref('');
const currentTocIndex = ref(-1);
const loading = ref(true);
const errorMsg = ref('');

let epubBook: EpubBook | null = null;
let rendition: Rendition | null = null;

function withTimeout<T>(promise: Promise<T>, ms: number, label: string): Promise<T> {
    return Promise.race([
        promise,
        new Promise<T>((_, reject) =>
            setTimeout(() => reject(new Error(`${label} 超时（${ms}ms）`)), ms),
        ),
    ]);
}

function flattenToc(
    items: any[],
    level: number,
): Array<{ label: string; href: string; level: number }> {
    const result: Array<{ label: string; href: string; level: number }> = [];
    for (const item of items) {
        result.push({ label: item.label?.trim() ?? '', href: item.href, level });
        if (item.subitems && item.subitems.length > 0) {
            result.push(...flattenToc(item.subitems, level + 1));
        }
    }
    return result;
}

/** 记录当前位置，并同步目录中高亮的章节 */
function updateLocation(location: any) {
    const cfi = location?.start?.cfi;
    if (cfi) localStorage.setItem(`epub_pos_${props.book.id}`, cfi);

    const href = location?.start?.href;
    if (!href) return;

    const target = String(href).split('#')[0];
    const idx = toc.value.findIndex((item) => {
        const itemHref = item.href.split('#')[0];
        return target === itemHref || target.endsWith(itemHref) || itemHref.endsWith(target);
    });
    currentTocIndex.value = idx;
    currentChapter.value = idx >= 0 ? toc.value[idx].label : '';
}

async function load() {
    if (!viewerEl.value) throw new Error('viewer 容器未挂载');

    const assetUrl = convertFileSrc(props.book.filePath);
    logger.info('[EPUB] 加载:', assetUrl);

    epubBook = ePub(assetUrl);
    await withTimeout(epubBook.ready, 15000, 'book.ready');

    const spine = epubBook.spine as any;
    if (!spine || !spine.length || spine.length === 0) {
        throw new Error('EPUB 没有可渲染的章节');
    }

    const rect = viewerEl.value.getBoundingClientRect();
    rendition = epubBook.renderTo(viewerEl.value, {
        width: rect.width,
        height: rect.height,
        flow: 'scrolled-doc',
        spread: 'none',
    });

    const savedCfi = localStorage.getItem(`epub_pos_${props.book.id}`);
    let displayed = false;
    if (savedCfi) {
        try {
            await withTimeout(rendition.display(savedCfi), 8000, '恢复位置');
            displayed = true;
        } catch {
            localStorage.removeItem(`epub_pos_${props.book.id}`);
        }
    }
    if (!displayed) {
        await withTimeout(rendition.display(), 15000, 'display');
    }

    try {
        const navigation = await withTimeout(
            epubBook.loaded.navigation as Promise<any>,
            5000,
            '目录',
        );
        toc.value = flattenToc(navigation?.toc ?? [], 0);
    } catch {
        toc.value = [];
    }

    rendition.on('relocated', updateLocation);

    // 首次打开时主动同步一次位置，保证目录能定位到当前章节
    try {
        const location = await (rendition as any).currentLocation?.();
        if (location) updateLocation(location);
    } catch {
        // 忽略：部分版本可能不支持 currentLocation
    }

    // 监听选中事件
    rendition.on('selected', (cfiRange: string, contents: any) => {
        try {
            const text = contents?.window?.getSelection()?.toString()?.trim() ?? '';
            if (!text) return;

            // 计算气泡位置：基于窗口范围
            const range = contents.range(cfiRange);
            if (range) {
                const rect = range.getBoundingClientRect();
                const iframe = viewerEl.value?.querySelector('iframe');
                const iframeRect = iframe?.getBoundingClientRect() ?? { top: 0, left: 0 };
                selectionBubble.value = {
                    visible: true,
                    x: iframeRect.left + rect.left + rect.width / 2,
                    y: iframeRect.top + rect.top - 8,
                    text,
                    cfi: cfiRange,
                };
            } else {
                selectionBubble.value = {
                    visible: true,
                    x: window.innerWidth / 2,
                    y: 100,
                    text,
                    cfi: cfiRange,
                };
            }
        } catch (err) {
            logger.warn('[EPUB] 处理选中事件失败:', err);
        }
    });

    // 点击其他地方关闭气泡
    const closeBubble = () => {
        selectionBubble.value.visible = false;
    };
    window.addEventListener('click', closeBubble);
    // 保存引用以便卸载时移除
    (window as any).__epubCloseBubble = closeBubble;

    // 渲染已有的标注
    await renderExistingAnnotations();
}

async function prevPage() {
    if (rendition) await rendition.prev().catch(() => { });
}
async function nextPage() {
    if (rendition) await rendition.next().catch(() => { });
}
async function goToHref(href: string) {
    if (rendition) await rendition.display(href);
}

onMounted(async () => {
    try {
        await nextTick();
        await load();
        loading.value = false;
    } catch (err) {
        logger.error('[EPUB] 加载失败:', err);
        errorMsg.value = err instanceof Error ? err.message : String(err);
        loading.value = false;
    }
});

onUnmounted(() => {
    try { rendition?.destroy(); } catch { }
    try { epubBook?.destroy(); } catch { }
    const closeBubble = (window as any).__epubCloseBubble;
    if (closeBubble) {
        window.removeEventListener('click', closeBubble);
        delete (window as any).__epubCloseBubble;
    }
});

/**
 * 把已有标注渲染到 epub 页面
 */
async function renderExistingAnnotations() {
    if (!rendition) return;
    try {
        const anns = await api.listAnnotations(props.book.id);
        for (const ann of anns) {
            try {
                const position = JSON.parse(ann.position || '{}');
                const cfi = position.cfi;
                if (!cfi || appliedHighlights.has(cfi)) continue;

                rendition.annotations.highlight(
                    cfi,
                    {},
                    () => {
                        // 点击高亮时，可以在侧边面板滚动定位（暂时不做）
                    },
                    'epub-highlight',
                    { fill: ann.color || '#FFEB3B', 'fill-opacity': '0.3' },
                );
                appliedHighlights.add(cfi);
            } catch (err) {
                logger.warn('[EPUB] 渲染单个标注失败:', err);
            }
        }
    } catch (err) {
        logger.warn('[EPUB] 加载已有标注失败:', err);
    }
}

/**
 * 保存标注
 */
async function saveAnnotation(color: string) {
    if (!selectionBubble.value.text || !selectionBubble.value.cfi) return;

    try {
        const ann = await api.createAnnotation({
            bookId: props.book.id,
            text: selectionBubble.value.text,
            note: '',
            page: 0,
            position: JSON.stringify({ cfi: selectionBubble.value.cfi }),
            color,
        });

        // 立即高亮
        rendition?.annotations.highlight(
            selectionBubble.value.cfi,
            {},
            () => { },
            'epub-highlight',
            { fill: color, 'fill-opacity': '0.3' },
        );
        appliedHighlights.add(selectionBubble.value.cfi);

        // 通知父组件刷新标注列表
        window.dispatchEvent(new CustomEvent('mysystem:annotation-added', { detail: ann }));

        selectionBubble.value.visible = false;
        logger.info('[EPUB] 标注已保存:', ann.id);
    } catch (err) {
        logger.error('[EPUB] 保存标注失败:', err);
    }
}

/**
 * 跳转到指定 CFI（供父组件调用）
 */
async function goToCfi(cfi: string) {
    if (!rendition) return;
    try {
        await rendition.display(cfi);
    } catch (err) {
        logger.warn('[EPUB] 跳转失败:', err);
    }
}

/** 跳转到标注位置（供父组件调用，与其他格式协议一致） */
async function jumpToAnnotation(ann: Annotation) {
    try {
        const position = JSON.parse(ann.position || '{}');
        if (position.cfi) await goToCfi(position.cfi);
    } catch (err) {
        logger.warn('[EPUB] 标注跳转失败:', err);
    }
}

/**
 * 删除标注后移除高亮（供父组件调用）
 */
async function removeAnnotationHighlight(ann: Annotation) {
    if (!rendition) return;
    try {
        const position = JSON.parse(ann.position || '{}');
        const cfi = position.cfi;
        if (!cfi) return;
        rendition.annotations.remove(cfi, 'highlight');
        appliedHighlights.delete(cfi);
    } catch (err) {
        logger.warn('[EPUB] 移除高亮失败:', err);
    }
}

defineExpose({
    prevPage,
    nextPage,
    goToHref,
    goToCfi,
    jumpToAnnotation,
    removeAnnotationHighlight,
    toc,
    currentChapter,
    currentTocIndex,
});
</script>

<template>
    <div class="epub-reader">
        <div ref="viewerEl" class="epub-viewer"></div>
        <div v-if="loading" class="overlay">加载中...</div>
        <div v-if="errorMsg" class="overlay error">{{ errorMsg }}</div>

        <!-- 选中气泡 -->
        <AnnotationBubble v-if="selectionBubble.visible" :x="selectionBubble.x" :y="selectionBubble.y"
            @select="saveAnnotation" />
    </div>
</template>

<style scoped>
.epub-reader {
    width: 100%;
    height: 100%;
    position: relative;
    overflow: hidden;
}

.epub-viewer {
    width: 100%;
    height: 100%;
}
</style>