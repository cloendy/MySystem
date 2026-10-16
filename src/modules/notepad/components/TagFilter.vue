<script setup lang="ts">
import { useNotepadStore } from '../stores/notepad';

const store = useNotepadStore();
</script>

<template>
    <div v-if="store.allTags.length > 0 || store.selectedTag" class="tag-bar">
        <button class="tag-pill" :class="{ active: store.selectedTag === null }" @click="store.selectTag(null)">
            全部
        </button>

        <button v-for="tag in store.allTags" :key="tag.name" class="tag-pill"
            :class="{ active: store.selectedTag === tag.name }" @click="store.selectTag(tag.name)">
            #{{ tag.name }}
            <span class="count">{{ tag.count }}</span>
        </button>
    </div>
</template>

<style scoped>
.tag-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 4px 0 12px;
    border-bottom: 1px solid #f0f0f0;
    margin-bottom: 12px;
}

.tag-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    border-radius: 12px;
    background: #f0f0f0;
    font-size: 12px;
    color: #666;
    cursor: pointer;
    border: none;
    font-family: inherit;
    line-height: 1.4;
    transition: background 0.15s, color 0.15s;
}

.tag-pill:hover {
    background: #e5e5e5;
}

.tag-pill.active {
    background: #2c3e50;
    color: white;
}

.tag-pill.active:hover {
    background: #34495e;
}

.count {
    font-size: 11px;
    opacity: 0.65;
    margin-left: 2px;
}

.tag-pill.active .count {
    opacity: 0.85;
}
</style>