<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { EditorView, basicSetup } from 'codemirror';
import { markdown } from '@codemirror/lang-markdown';
import { oneDark } from '@codemirror/theme-one-dark';
import { marked } from 'marked';
import { useNotepadStore } from '../stores/notepad';
import { logger } from '../../../platform/services/logger';
import { detectPlatform } from '../../../platform/services/module-loader';
import { confirmAction } from '../../../platform/services/dialog';
import TagEditor from '../components/TagEditor.vue';
import FolderTree from '../components/FolderTree.vue';
import { EDITOR_MODE_STORAGE_KEY } from '../constants';

/** 自动保存日志最小间隔，避免每次输入停顿都写一条 */
const SAVE_LOG_INTERVAL_MS = 5000;

let lastSaveLogAt = 0;

type EditorMode = 'markdown' | 'text';

function loadEditorMode(): EditorMode {
    return localStorage.getItem(EDITOR_MODE_STORAGE_KEY) === 'text' ? 'text' : 'markdown';
}

const route = useRoute();
const router = useRouter();
const store = useNotepadStore();
const isMobile = detectPlatform() === 'android';

const noteId = route.params.id as string;
const title = ref('');
const content = ref('');
const saving = ref(false);
const editorMode = ref<EditorMode>(loadEditorMode());
const editorContainer = ref<HTMLDivElement | null>(null);
let editorView: EditorView | null = null;

const renderedHtml = computed(() => marked.parse(content.value) as string);

function createEditor() {
    if (!editorContainer.value) return;
    editorView?.destroy();
    editorView = new EditorView({
        doc: content.value,
        extensions: [
            basicSetup,
            // 纯文本模式不加载 Markdown 语法扩展
            ...(editorMode.value === 'markdown' ? [markdown()] : []),
            oneDark,
            EditorView.updateListener.of((update) => {
                if (update.docChanged) {
                    content.value = update.state.doc.toString();
                }
            }),
        ],
        parent: editorContainer.value,
    });
}

onMounted(async () => {
    await store.selectNote(noteId);
    if (!store.currentNote) {
        alert('笔记不存在');
        router.push('/notepad/');
        return;
    }
    title.value = store.currentNote.title;
    content.value = store.currentNote.content;
    createEditor();
});

onUnmounted(() => {
    editorView?.destroy();
    editorView = null;
});

watch(editorMode, (mode) => {
    localStorage.setItem(EDITOR_MODE_STORAGE_KEY, mode);
    logger.info(`[Notepad] 切换编辑模式: ${mode === 'markdown' ? 'Markdown' : '纯文本'}`);
    // 重建编辑器以应用/移除 Markdown 语法支持，内容保存在 content 中不会丢失
    createEditor();
});

// 标题防抖保存
let saveTimer: number | null = null;
watch([title, content], () => {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = window.setTimeout(save, 800);
});

async function save() {
    if (!store.currentNote) return;
    saving.value = true;
    try {
        await store.editNote({
            id: noteId,
            title: title.value,
            content: content.value,
        });
        const now = Date.now();
        if (now - lastSaveLogAt >= SAVE_LOG_INTERVAL_MS) {
            lastSaveLogAt = now;
            logger.info(`[Notepad] 保存笔记: ${title.value || '(无标题)'}`);
        }
    } finally {
        saving.value = false;
    }
}

async function handleBack() {
    await save();
    router.push('/notepad/');
}

async function handleDelete() {
    if (!(await confirmAction('确定删除这条笔记？', '删除笔记'))) return;
    await store.removeNote(noteId);
    router.push('/notepad/');
}
</script>

<template>
    <div class="note-edit-view">
        <FolderTree v-if="!isMobile" navigate-to-list :before-navigate="save" />

        <div class="edit-main">
            <div class="edit-toolbar">
                <button class="btn" @click="handleBack">← 返回</button>
                <input v-model="title" class="title-input" placeholder="无标题" />
                <span class="save-status">{{ saving ? '保存中...' : '已保存' }}</span>
                <button class="btn btn-danger" @click="handleDelete">删除</button>
            </div>

            <TagEditor v-if="store.currentNote" :note="store.currentNote">
                <template #right>
                    <div class="mode-switch" role="group" aria-label="编辑模式">
                        <button class="mode-btn" :class="{ active: editorMode === 'markdown' }"
                            @click="editorMode = 'markdown'">Markdown</button>
                        <button class="mode-btn" :class="{ active: editorMode === 'text' }"
                            @click="editorMode = 'text'">纯文本</button>
                    </div>
                </template>
            </TagEditor>

            <div class="edit-body">
                <div class="editor-pane" :class="{ 'full-width': editorMode === 'text' }">
                    <div ref="editorContainer" class="editor-container"></div>
                </div>
                <div v-if="editorMode === 'markdown'" class="preview-pane">
                    <div class="preview-content" v-html="renderedHtml"></div>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.note-edit-view {
    display: flex;
    height: 100%;
    overflow: hidden;
}

.edit-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.edit-toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    border-bottom: 1px solid #e0e0e0;
    background: #fafafa;
}

.title-input {
    flex: 1;
    font-size: 16px;
    font-weight: 600;
    border: none;
    background: transparent;
    outline: none;
    padding: 4px 8px;
}

.save-status {
    font-size: 12px;
    color: #999;
    min-width: 60px;
    text-align: right;
}

.mode-switch {
    display: inline-flex;
    border: 1px solid #d0d0d0;
    border-radius: 4px;
    overflow: hidden;
}

.mode-btn {
    border: none;
    background: #fff;
    color: #666;
    font-family: inherit;
    font-size: 12px;
    padding: 3px 10px;
    cursor: pointer;
}

.mode-btn + .mode-btn {
    border-left: 1px solid #d0d0d0;
}

.mode-btn:hover {
    background: #f0f2f4;
}

.mode-btn.active {
    background: #2c3e50;
    color: #fff;
}

.edit-body {
    display: flex;
    flex: 1;
    overflow: hidden;
}

.editor-pane,
.preview-pane {
    flex: 1;
    overflow: auto;
}

.editor-pane {
    border-right: 1px solid #e0e0e0;
}

.editor-pane.full-width {
    border-right: none;
}

.editor-container {
    height: 100%;
}

.preview-content {
    padding: 20px;
    line-height: 1.7;
    font-size: 15px;
    color: #333;
}

.preview-content :deep(h1),
.preview-content :deep(h2),
.preview-content :deep(h3) {
    margin: 16px 0 8px;
}

.preview-content :deep(code) {
    background: #f4f4f4;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 13px;
}

.preview-content :deep(pre) {
    background: #2d2d2d;
    color: #f8f8f2;
    padding: 12px;
    border-radius: 4px;
    overflow-x: auto;
}

.preview-content :deep(blockquote) {
    border-left: 3px solid #d0d0d0;
    padding-left: 12px;
    color: #666;
    margin: 8px 0;
}
</style>
