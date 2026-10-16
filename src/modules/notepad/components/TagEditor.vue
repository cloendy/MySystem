<script setup lang="ts">
import { ref } from 'vue';
import { useNotepadStore } from '../stores/notepad';
import type { Note } from '../types';

const props = defineProps<{ note: Note }>();
const store = useNotepadStore();

const input = ref('');
const editing = ref(false);

async function handleAdd() {
    const tag = input.value.trim();
    if (!tag) return;
    if (props.note.tags.includes(tag)) {
        input.value = '';
        return;
    }
    await store.addTagToNote(props.note.id, tag);
    input.value = '';
}

async function handleRemove(tag: string) {
    await store.removeTagFromNote(props.note.id, tag);
}

function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
        e.preventDefault();
        handleAdd();
    } else if (e.key === 'Escape') {
        input.value = '';
        editing.value = false;
    }
}
</script>

<template>
    <div class="tag-editor">
        <span v-for="tag in note.tags" :key="tag" class="tag-chip">
            #{{ tag }}
            <button class="tag-remove" @click="handleRemove(tag)" title="删除标签">×</button>
        </span>

        <input v-if="editing" ref="inputEl" v-model="input" class="text-input tag-input" placeholder="输入标签，回车确认"
            @keydown="handleKeydown" @blur="editing = false" />
        <button v-else class="tag-add" @click="editing = true">+ 添加标签</button>
    </div>
</template>

<style scoped>
.tag-editor {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    padding: 8px 16px;
    border-bottom: 1px solid #e0e0e0;
    background: #fafafa;
}

.tag-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: #5a7fa0;
    background: #eaf1f8;
    padding: 3px 8px;
    border-radius: 3px;
}

.tag-remove {
    background: transparent;
    border: none;
    color: #7a9ab8;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    padding: 0;
}

.tag-remove:hover {
    color: #c0392b;
}

.tag-input {
    font-size: 12px;
    padding: 3px 8px;
    border: 1px solid #d0d0d0;
    border-radius: 3px;
    outline: none;
    width: 140px;
}

.tag-add {
    font-size: 12px;
    color: #666;
    background: transparent;
    border: 1px dashed #c0c0c0;
    padding: 3px 8px;
    border-radius: 3px;
    cursor: pointer;
}

.tag-add:hover {
    color: #2c3e50;
    border-color: #2c3e50;
}
</style>