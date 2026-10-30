<script setup lang="ts">
/** 选中文字后弹出的标注颜色气泡，三种阅读器共用 */
defineProps<{
    x: number;
    y: number;
}>();

const emit = defineEmits<{
    (e: 'select', color: string): void;
}>();

const COLORS = ['#FFEB3B', '#A5D6A7', '#90CAF9', '#F48FB1'];
</script>

<template>
    <div class="annotation-bubble" :style="{ left: `${x}px`, top: `${y}px` }" @click.stop @mousedown.stop>
        <span class="bubble-label">添加标注</span>
        <div class="color-row">
            <button v-for="c in COLORS" :key="c" class="color-btn" :style="{ background: c }" :title="`使用 ${c}`"
                @click="emit('select', c)"></button>
        </div>
    </div>
</template>

<style scoped>
.annotation-bubble {
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
