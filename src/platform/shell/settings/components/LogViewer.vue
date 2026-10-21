<script setup lang="ts">
import { computed, ref } from 'vue';
import {
    listLogFiles,
    readLogFile,
    getLogDir,
    type LogFileInfo,
} from '../../../services/logger';

type LogLevelFilter = 'all' | 'ERROR' | 'WARN' | 'INFO' | 'DEBUG';

const logPanelOpen = ref(false);
const logFiles = ref<LogFileInfo[]>([]);
const logDir = ref('');
const selectedLogFile = ref('');
const logContent = ref('');
const logLoading = ref(false);
const logError = ref('');
const logLevelFilter = ref<LogLevelFilter>('all');

/** 按日志级别筛选后的行 */
const logLines = computed(() => {
    const lines = logContent.value.split('\n').filter((line) => line.length > 0);
    if (logLevelFilter.value === 'all') return lines;
    return lines.filter((line) => line.includes(`[${logLevelFilter.value}]`));
});

function formatFileSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

function formatModifiedAt(ms: number): string {
    if (!ms) return '';
    const d = new Date(ms);
    const pad = (n: number) => String(n).padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

async function openLogPanel() {
    logPanelOpen.value = true;
    logError.value = '';
    try {
        logDir.value = await getLogDir();
    } catch {
        logDir.value = '';
    }
    await refreshLogFiles();
}

function closeLogPanel() {
    logPanelOpen.value = false;
}

async function refreshLogFiles() {
    logLoading.value = true;
    logError.value = '';
    try {
        logFiles.value = await listLogFiles();
        if (logFiles.value.length === 0) {
            selectedLogFile.value = '';
            logContent.value = '';
            return;
        }
        if (!logFiles.value.some((file) => file.name === selectedLogFile.value)) {
            selectedLogFile.value = logFiles.value[0].name;
        }
        logContent.value = await readLogFile(selectedLogFile.value);
    } catch (err) {
        logError.value = err instanceof Error ? err.message : String(err);
    } finally {
        logLoading.value = false;
    }
}

async function handleSelectLogFile() {
    logError.value = '';
    if (!selectedLogFile.value) {
        logContent.value = '';
        return;
    }
    logLoading.value = true;
    try {
        logContent.value = await readLogFile(selectedLogFile.value);
    } catch (err) {
        logError.value = err instanceof Error ? err.message : String(err);
    } finally {
        logLoading.value = false;
    }
}
</script>

<template>
    <section class="settings-section">
        <h2 class="settings-section-title">系统日志</h2>
        <div class="setting-row">
            <div class="setting-info">
                <div class="setting-label">运行日志</div>
                <div class="setting-desc">
                    应用与各模块的运行日志按天保存在 data/log 目录
                </div>
            </div>
            <button class="btn-secondary" @click="openLogPanel">查看日志</button>
        </div>
    </section>

    <!-- 日志分析面板（覆盖在设置页上方，关闭后消失） -->
    <div v-if="logPanelOpen" class="log-overlay" @click.self="closeLogPanel">
        <div class="log-panel">
            <header class="log-panel-header">
                <div class="log-panel-title">
                    <h2>日志分析</h2>
                    <div class="log-dir">{{ logDir || '正在获取日志目录...' }}</div>
                </div>
                <button class="btn-close" title="关闭" @click="closeLogPanel">×</button>
            </header>

            <div class="log-toolbar">
                <select v-model="selectedLogFile" class="log-select" @change="handleSelectLogFile">
                    <option v-if="logFiles.length === 0" value="">暂无日志文件</option>
                    <option v-for="file in logFiles" :key="file.name" :value="file.name">
                        {{ file.name }}（{{ formatFileSize(file.size) }}）
                    </option>
                </select>

                <select v-model="logLevelFilter" class="log-select log-level-select">
                    <option value="all">全部级别</option>
                    <option value="ERROR">仅 ERROR</option>
                    <option value="WARN">仅 WARN</option>
                    <option value="INFO">仅 INFO</option>
                    <option value="DEBUG">仅 DEBUG</option>
                </select>

                <button class="btn-secondary" :disabled="logLoading" @click="refreshLogFiles">
                    {{ logLoading ? '加载中...' : '刷新' }}
                </button>
            </div>

            <div class="log-stats">
                <span v-if="selectedLogFile">
                    {{ selectedLogFile }}
                    <template v-if="logFiles.length > 0">
                        · {{ formatModifiedAt(logFiles.find((f) => f.name === selectedLogFile)?.modifiedAt ?? 0) }}
                    </template>
                    · 当前显示 {{ logLines.length }} 行
                </span>
                <span v-else>暂无日志</span>
            </div>

            <div v-if="logError" class="log-panel-error">{{ logError }}</div>
            <pre v-else class="log-content">{{ logLines.join('\n') || '（暂无日志内容）' }}</pre>
        </div>
    </div>
</template>

<style scoped>
/* ===== 日志分析面板 ===== */

.log-overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.35);
}

.log-panel {
    display: flex;
    flex-direction: column;
    width: min(960px, 100%);
    height: min(720px, 90vh);
    background: white;
    border-radius: 8px;
    overflow: hidden;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.25);
}

.log-panel-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 20px 12px;
    border-bottom: 1px solid #e0e0e0;
}

.log-panel-title h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: #2c3e50;
}

.log-dir {
    margin-top: 4px;
    font-size: 12px;
    color: #888;
    word-break: break-all;
}

.btn-close {
    flex-shrink: 0;
    width: 30px;
    height: 30px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: #666;
    font-size: 22px;
    line-height: 1;
    cursor: pointer;
}

.btn-close:hover {
    background: #f0f0f0;
    color: #333;
}

.log-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 20px;
    border-bottom: 1px solid #ececec;
    background: #fafafa;
}

.log-select {
    min-width: 0;
    max-width: 360px;
    padding: 6px 8px;
    border: 1px solid #d0d0d0;
    border-radius: 4px;
    background: white;
    font-size: 13px;
    color: #333;
}

.log-level-select {
    max-width: 120px;
}

.log-stats {
    padding: 8px 20px;
    font-size: 12px;
    color: #777;
    border-bottom: 1px solid #ececec;
}

.log-panel-error {
    margin: 16px 20px;
    padding: 10px 12px;
    border-radius: 4px;
    background: #fdeaea;
    color: #c0392b;
    font-size: 13px;
    word-break: break-all;
}

.log-content {
    flex: 1;
    margin: 0;
    padding: 12px 16px;
    overflow: auto;
    background: #1e1e1e;
    color: #d4d4d4;
    font-family: 'Consolas', 'Monaco', monospace;
    font-size: 12px;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-all;
}
</style>
