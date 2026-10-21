<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import * as api from '../api';
import type { Annotation } from '../types';
import { confirmAction } from '../../../platform/services/dialog';

const props = defineProps<{ bookId: string }>();
const emit = defineEmits<{
    (e: 'jump', cfi: string): void;
    (e: 'remove', cfi: string): void;
}>();

const annotations = ref<Annotation[]>([]);
const loading = ref(true);

async function loadAnnotations() {
    loading.value = true;
    try {
        annotations.value = await api.listAnnotations(props.bookId);
    } finally {
        loading.value = false;
    }
}

async function handleDelete(ann: Annotation) {
    if (!(await confirmAction('确定删除这条标注？', '删除标注'))) return;
    await api.deleteAnnotation(ann.id);
    // 通知父组件移除高亮
    try {
        const pos = JSON.parse(ann.position || '{}');
        if (pos.cfi) emit('remove', pos.cfi);
    } catch { }
    annotations.value = annotations.value.filter((a) => a.id !== ann.id);
}

function handleJump(ann: Annotation) {
    try {
        const pos = JSON.parse(ann.position || '{}');
        if (pos.cfi) emit('jump', pos.cfi);
    } catch { }
}

function handleAnnotationAdded() {
    loadAnnotations();
}

onMounted(() => {
    loadAnnotations();
    window.addEventListener('mysystem:annotation-added', handleAnnotationAdded);
});

onUnmounted(() => {
    window.removeEventListener('mysystem:annotation-added', handleAnnotationAdded);
});
</script>

<template>
    <div class="side-panel right">
        <div class="side-panel-header">
            <span>标注 ({{ annotations.length }})</span>
            <button class="btn-refresh" @click="loadAnnotations" title="刷新">⟳</button>
        </div>

        <div v-if="loading" class="hint">加载中...</div>
        <div v-else-if="annotations.length === 0" class="hint">
            还没有标注<br />
            <span class="small">选中文字后点击颜色即可添加</span>
        </div>

        <ul v-else class="annotation-list">
            <li v-for="ann in annotations" :key="ann.id" class="annotation-item">
                <div class="ann-color" :style="{ background: ann.color }"></div>
                <div class="ann-content" @click="handleJump(ann)">
                    <div class="ann-text">{{ ann.text }}</div>
                    <div v-if="ann.note" class="ann-note">{{ ann.note }}</div>
                </div>
                <button class="btn btn-danger btn-icon ann-delete" @click="handleDelete(ann)" title="删除">×</button>
            </li>
        </ul>
    </div>
</template>

<style scoped>
.btn-refresh {
    background: transparent;
    border: none;
    cursor: pointer;
    color: #999;
    font-size: 14px;
    padding: 0 4px;
}

.btn-refresh:hover {
    color: #2c3e50;
}

.hint {
    padding: 20px 16px;
    font-size: 13px;
    color: #999;
    text-align: center;
    line-height: 1.6;
}

.hint .small {
    font-size: 11px;
    color: #bbb;
}

.annotation-list {
    list-style: none;
    padding: 0;
    margin: 0;
}

.annotation-item {
    display: flex;
    gap: 8px;
    padding: 10px 12px;
    border-bottom: 1px solid #ececec;
    align-items: flex-start;
}

.ann-color {
    width: 4px;
    align-self: stretch;
    border-radius: 2px;
    flex-shrink: 0;
}

.ann-content {
    flex: 1;
    cursor: pointer;
    min-width: 0;
}

.ann-text {
    font-size: 13px;
    color: #333;
    line-height: 1.5;
    word-break: break-word;
}

.ann-note {
    font-size: 12px;
    color: #888;
    margin-top: 4px;
    font-style: italic;
}
</style>