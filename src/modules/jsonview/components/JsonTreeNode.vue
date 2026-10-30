<script setup lang="ts">
import type { JsonNode } from '../types';
import { jsonTypeLabel } from '../utils/analyzer';

defineProps<{
    node: JsonNode;
    defaultOpen?: boolean;
}>();
</script>

<template>
    <div class="json-node">
        <details v-if="node.children.length" :open="defaultOpen ?? false" class="node-details">
            <summary class="node-row" :title="node.path">
                <span class="node-key">{{ node.key }}</span>
                <span class="type-badge" :class="`type-${node.type}`">{{ jsonTypeLabel(node.type) }}</span>
                <span class="node-count">{{ node.value }}</span>
                <span class="node-path">{{ node.path }}</span>
                <span v-if="node.truncated" class="node-note">已截断</span>
            </summary>

            <div class="node-children">
                <JsonTreeNode v-for="child in node.children" :key="child.path" :node="child" />
                <div v-if="node.truncated" class="node-more">
                    仅显示前 {{ node.children.length }} 项，共 {{ node.childCount }} 项
                </div>
            </div>
        </details>

        <div v-else class="node-row leaf" :title="node.path">
            <span class="node-key">{{ node.key }}</span>
            <span class="type-badge" :class="`type-${node.type}`">{{ jsonTypeLabel(node.type) }}</span>
            <span class="node-value">{{ node.value }}</span>
            <span class="node-path">{{ node.path }}</span>
            <span v-if="node.truncated" class="node-note">已截断</span>
        </div>
    </div>
</template>

<style scoped>
.json-node {
    font-family: 'Cascadia Code', Consolas, 'Courier New', monospace;
    font-size: 13px;
}

.node-details > summary {
    list-style: none;
    cursor: pointer;
    user-select: none;
}

.node-details > summary::-webkit-details-marker {
    display: none;
}

.node-details > summary::before {
    content: '▸';
    flex-shrink: 0;
    width: 14px;
    color: #888;
    transition: transform 0.15s;
}

.node-details[open] > summary::before {
    transform: rotate(90deg);
}

.node-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 26px;
    padding: 2px 8px;
    border-radius: 3px;
}

.node-row:hover {
    background: #f3f6f9;
}

.node-key {
    color: #2c3e50;
    font-weight: 600;
    flex-shrink: 0;
}

.node-value {
    color: #6b7c8c;
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.node-count {
    color: #95a5a6;
    flex-shrink: 0;
}

.node-path {
    color: #b2bec3;
    font-size: 12px;
    margin-left: auto;
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.node-note {
    flex-shrink: 0;
    font-size: 12px;
    color: #c0392b;
}

.node-children {
    margin-left: 14px;
    padding-left: 10px;
    border-left: 1px dashed #dde3e8;
}

.node-more {
    padding: 4px 8px 4px 22px;
    color: #95a5a6;
    font-size: 12px;
}

.type-badge {
    flex-shrink: 0;
    padding: 0 6px;
    border-radius: 3px;
    font-size: 11px;
    line-height: 18px;
    font-family: inherit;
}

.type-object {
    background: #e8f1fb;
    color: #2471a3;
}

.type-array {
    background: #eaf7ef;
    color: #1e8449;
}

.type-string {
    background: #fdf3e3;
    color: #b9770e;
}

.type-number {
    background: #f4ecf7;
    color: #7d3c98;
}

.type-boolean {
    background: #fdecea;
    color: #c0392b;
}

.type-null {
    background: #f0f0f0;
    color: #7f8c8d;
}
</style>
