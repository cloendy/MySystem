<script setup lang="ts">
import { onMounted, onUnmounted, ref, nextTick, defineExpose } from 'vue';
import ePub, { Book as EpubBook, Rendition } from 'epubjs';
import { convertFileSrc } from '@tauri-apps/api/core';
import type { Book } from '../types';
import { useReaderStore } from '../stores/reader';
import * as api from '../api';

const props = defineProps<{ book: Book }>();

const store = useReaderStore();

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

async function load() {
    if (!viewerEl.value) throw new Error('viewer 容器未挂载');

    const assetUrl = convertFileSrc(props.book.filePath);
    console.log('[EPUB] 加载:', assetUrl);

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

    rendition.on('relocated', (location: any) => {
        const cfi = location?.start?.cfi;
        if (cfi) localStorage.setItem(`epub_pos_${props.book.id}`, cfi);
        const href = location?.start?.href;
        if (href) {
            const match = toc.value.find((t) => href.includes(t.href.split('#')[0]));
            currentChapter.value = match?.label ?? '';
        }
    });

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
            console.warn('[EPUB] 处理选中事件失败:', err);
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
function toggleToc() {
    // 由父组件控制显示
}

onMounted(async () => {
    try {
        await nextTick();
        await load();
        loading.value = false;
    } catch (err) {
        console.error('[EPUB] 加载失败:', err);
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
                console.warn('[EPUB] 渲染单个标注失败:', err);
            }
        }
    } catch (err) {
        console.warn('[EPUB] 加载已有标注失败:', err);
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
        console.log('[EPUB] 标注已保存:', ann.id);
    } catch (err) {
        console.error('[EPUB] 保存标注失败:', err);
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
        console.warn('[EPUB] 跳转失败:', err);
    }
}

/**
 * 删除标注（供父组件调用后刷新高亮）
 */
async function removeAnnotationHighlight(cfi: string) {
    if (!rendition) return;
    try {
        rendition.annotations.remove(cfi, 'highlight');
        appliedHighlights.delete(cfi);
    } catch (err) {
        console.warn('[EPUB] 移除高亮失败:', err);
    }
}

defineExpose({
    prevPage,
    nextPage,
    goToHref,
    goToCfi,
    removeAnnotationHighlight,
    toc,
    currentChapter,
});
</script>

<template>
    <div class="epub-reader">
        <div ref="viewerEl" class="epub-viewer"></div>
        <div v-if="loading" class="overlay">加载中...</div>
        <div v-if="errorMsg" class="overlay error">{{ errorMsg }}</div>

        <!-- 选中气泡 -->
        <div v-if="selectionBubble.visible" class="selection-bubble"
            :style="{ left: selectionBubble.x + 'px', top: selectionBubble.y + 'px' }" @click.stop>
            <span class="bubble-label">添加标注</span>
            <div class="color-row">
                <button v-for="c in ['#FFEB3B', '#A5D6A7', '#90CAF9', '#F48FB1']" :key="c" class="color-btn"
                    :style="{ background: c }" @click="saveAnnotation(c)"></button>
            </div>
        </div>
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



.selection-bubble {
    position: fixed;
    transform: translate(-50%, -100%);
    background: white;
    border: 1px solid #d0d0d0;
    border-radius: 6px;
    padding: 8px 10px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    z-index: 100;
    display: flex;
    flex-direction: column;
    gap: 6px;
    align-items: center;
}

.bubble-label {
    font-size: 11px;
    color: #666;
    white-space: nowrap;
}

.color-row {
    display: flex;
    gap: 6px;
}

.color-btn {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 1px solid rgba(0, 0, 0, 0.1);
    cursor: pointer;
    padding: 0;
}

.color-btn:hover {
    transform: scale(1.15);
    transition: transform 0.1s;
}
</style>