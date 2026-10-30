<script setup lang="ts">
import { computed, ref } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import { logger } from '../../../platform/services/logger';
import * as api from '../api';
import type { JsonAnalysis } from '../types';
import { analyzeJsonText, jsonTypeLabel } from '../utils/analyzer';
import JsonTreeNode from '../components/JsonTreeNode.vue';

const source = ref('');
const result = ref<JsonAnalysis | null>(null);
const analyzedText = ref('');
const error = ref('');
const fileName = ref('');
const fileSize = ref(0);
const treeEl = ref<HTMLElement | null>(null);
const picking = ref(false);

const charCount = computed(() => source.value.length);
const dirty = computed(() => result.value !== null && source.value !== analyzedText.value);

function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}

async function handlePick() {
    if (picking.value) return;
    picking.value = true;
    try {
        const picked = await open({
            multiple: false,
            directory: false,
            filters: [{ name: 'JSON 文件', extensions: ['json', 'txt'] }],
        });
        if (!picked || Array.isArray(picked)) return;

        const file = await api.readJsonFile(picked);
        source.value = file.content;
        fileName.value = file.path.replace(/^.*[\\/]/, '');
        fileSize.value = file.size;
        result.value = null;
        analyzedText.value = '';
        error.value = '';
        logger.info(`[JsonView] 读取文件: ${file.path} (${file.size} 字节)`);
    } catch (e) {
        const message = e instanceof Error ? e.message : String(e);
        error.value = `读取文件失败：${message}`;
        logger.warn(`[JsonView] 读取文件失败: ${message}`);
    } finally {
        picking.value = false;
    }
}

function runAnalyze() {
    const text = source.value.trim();
    if (!text) {
        error.value = '请输入或导入 JSON 内容后再分析';
        result.value = null;
        return;
    }

    try {
        const analysis = analyzeJsonText(text);
        result.value = analysis;
        analyzedText.value = source.value;
        error.value = '';
        logger.info(
            `[JsonView] 分析完成: 根类型=${analysis.stats.rootType}, 节点=${analysis.stats.totalNodes}, 最大层级=${analysis.stats.maxDepth}`,
        );
    } catch (e) {
        const message = e instanceof Error ? e.message : String(e);
        result.value = null;
        error.value = message;
        logger.warn(`[JsonView] 分析失败: ${message}`);
    }
}

function clearAll() {
    source.value = '';
    result.value = null;
    analyzedText.value = '';
    error.value = '';
    fileName.value = '';
    fileSize.value = 0;
}

function setAllExpanded(openState: boolean) {
    const details = treeEl.value?.querySelectorAll('details');
    if (!details) return;
    details.forEach((item) => {
        item.open = openState;
    });
}
</script>

<template>
    <div class="json-view">
        <div class="toolbar">
            <button class="btn btn-primary" :disabled="picking" @click="handlePick">
                {{ picking ? '读取中…' : '导入 JSON 文件' }}
            </button>
            <button class="btn" @click="runAnalyze">分析</button>
            <button class="btn btn-ghost" @click="clearAll">清空</button>

            <span v-if="fileName" class="file-info">
                {{ fileName }}
                <span class="file-size">{{ formatBytes(fileSize) }}</span>
            </span>
            <span class="char-info">{{ charCount }} 字符</span>
        </div>

        <div class="json-body">
            <section class="pane editor-pane">
                <div class="pane-header">
                    <span>JSON 内容</span>
                    <span class="pane-hint">Ctrl + Enter 分析</span>
                </div>
                <textarea
                    v-model="source"
                    class="json-input"
                    spellcheck="false"
                    placeholder="在此粘贴 JSON 文本，或通过上方按钮导入 .json 文件…"
                    @keydown.ctrl.enter.prevent="runAnalyze"
                ></textarea>
            </section>

            <section class="pane result-pane">
                <div class="pane-header">
                    <span>结构</span>
                    <div v-if="result" class="tree-actions">
                        <button class="btn btn-ghost" @click="setAllExpanded(true)">展开全部</button>
                        <button class="btn btn-ghost" @click="setAllExpanded(false)">折叠全部</button>
                    </div>
                </div>

                <div v-if="error" class="error-box">{{ error }}</div>

                <template v-if="result">
                    <div class="summary">
                        <div class="summary-item">
                            <span class="summary-label">根类型</span>
                            <span class="summary-value">{{ jsonTypeLabel(result.stats.rootType) }}</span>
                        </div>
                        <div class="summary-item">
                            <span class="summary-label">节点总数</span>
                            <span class="summary-value">{{ result.stats.totalNodes }}</span>
                        </div>
                        <div class="summary-item">
                            <span class="summary-label">最大层级</span>
                            <span class="summary-value">{{ result.stats.maxDepth }}</span>
                        </div>
                        <div class="summary-item">
                            <span class="summary-label">对象</span>
                            <span class="summary-value">{{ result.stats.objects }}</span>
                        </div>
                        <div class="summary-item">
                            <span class="summary-label">数组</span>
                            <span class="summary-value">{{ result.stats.arrays }}</span>
                        </div>
                        <div class="summary-item">
                            <span class="summary-label">基本值</span>
                            <span class="summary-value">{{ result.stats.primitives }}</span>
                        </div>
                    </div>

                    <div v-if="result.stats.truncated || result.root.truncated" class="truncate-note">
                        数据量较大，部分分支已按上限截断；完整内容请使用其他工具查看。
                    </div>
                    <div v-if="dirty" class="dirty-note">内容已修改，点击「分析」重新生成结构。</div>

                    <div ref="treeEl" class="json-tree">
                        <JsonTreeNode :node="result.root" :default-open="true" />
                    </div>
                </template>

                <div v-else-if="!error" class="empty-state">
                    导入 JSON 文件或粘贴文本后，点击「分析」查看结构。
                </div>
            </section>
        </div>
    </div>
</template>

<style scoped>
.json-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
}

.file-info {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: #2c3e50;
    font-size: 13px;
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.file-size {
    color: #95a5a6;
    font-size: 12px;
}

.char-info {
    margin-left: auto;
    color: #95a5a6;
    font-size: 12px;
}

.json-body {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 12px;
    padding: 0 16px 16px;
}

.pane {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: #fff;
    border: 1px solid #e3e8ee;
    border-radius: 4px;
    overflow: hidden;
}

.pane-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid #edf0f3;
    background: #fafbfc;
    color: #2c3e50;
    font-size: 13px;
    font-weight: 600;
}

.pane-hint {
    color: #95a5a6;
    font-size: 12px;
    font-weight: 400;
}

.tree-actions {
    display: flex;
    gap: 4px;
}

.json-input {
    flex: 1;
    width: 100%;
    min-height: 0;
    padding: 12px;
    border: none;
    outline: none;
    resize: none;
    font-family: 'Cascadia Code', Consolas, 'Courier New', monospace;
    font-size: 13px;
    line-height: 1.6;
    color: #2c3e50;
    background: #fff;
    tab-size: 4;
}

.result-pane {
    overflow: hidden;
}

.summary {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
    padding: 10px 12px;
    border-bottom: 1px solid #edf0f3;
}

.summary-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.summary-label {
    color: #95a5a6;
    font-size: 12px;
}

.summary-value {
    color: #2c3e50;
    font-size: 14px;
    font-weight: 600;
}

.truncate-note,
.dirty-note {
    margin: 8px 12px 0;
    padding: 6px 10px;
    border-radius: 3px;
    font-size: 12px;
}

.truncate-note {
    background: #fdf3e3;
    color: #b9770e;
}

.dirty-note {
    background: #eef4fb;
    color: #2471a3;
}

.json-tree {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 8px 12px 12px;
}

.error-box {
    margin: 12px;
    padding: 10px 12px;
    border-radius: 4px;
    background: #fdecea;
    color: #c0392b;
    font-size: 13px;
    line-height: 1.6;
    word-break: break-word;
}

.empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    color: #95a5a6;
    font-size: 13px;
    text-align: center;
}

@media (max-width: 900px) {
    .json-body {
        flex-direction: column;
    }

    .pane {
        min-height: 240px;
    }
}
</style>
