<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { EditorView, basicSetup } from 'codemirror';
import { markdown } from '@codemirror/lang-markdown';
import { oneDark } from '@codemirror/theme-one-dark';
import { marked } from 'marked';
import { useNotepadStore } from '../stores/notepad';
import TagEditor from '../components/TagEditor.vue';

const route = useRoute();
const router = useRouter();
const store = useNotepadStore();

const noteId = route.params.id as string;
const title = ref('');
const content = ref('');
const saving = ref(false);
const editorContainer = ref<HTMLDivElement | null>(null);
let editorView: EditorView | null = null;

const renderedHtml = computed(() => marked.parse(content.value) as string);

onMounted(async () => {
    await store.selectNote(noteId);
    if (!store.currentNote) {
        alert('笔记不存在');
        router.push('/notepad/');
        return;
    }
    title.value = store.currentNote.title;
    content.value = store.currentNote.content;

    if (editorContainer.value) {
        editorView = new EditorView({
            doc: content.value,
            extensions: [
                basicSetup,
                markdown(),
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
    } finally {
        saving.value = false;
    }
}

async function handleBack() {
    await save();
    router.push('/notepad/');
}

function handleDelete() {
    if (!confirm('确定删除这条笔记？')) return;
    store.removeNote(noteId).then(() => {
        router.push('/notepad/');
    });
}
</script>

<template>
    <div class="note-edit-view">
        <div class="edit-toolbar">
            <button class="btn" @click="handleBack">← 返回</button>
            <input v-model="title" class="title-input" placeholder="无标题" />
            <span class="save-status">{{ saving ? '保存中...' : '已保存' }}</span>
            <button class="btn btn-danger" @click="handleDelete">删除</button>
        </div>

        <TagEditor v-if="store.currentNote" :note="store.currentNote" />

        <div class="edit-body">
            <div class="editor-pane">
                <div ref="editorContainer" class="editor-container"></div>
            </div>
            <div class="preview-pane">
                <div class="preview-content" v-html="renderedHtml"></div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.note-edit-view {
    display: flex;
    flex-direction: column;
    height: 100%;
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