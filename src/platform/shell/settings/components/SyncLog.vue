<script setup lang="ts">
import { useSyncSettings } from '../useSyncSettings';

const { syncLogs, clearLogs, formatLogTime } = useSyncSettings();
</script>

<template>
    <section class="settings-section">
        <h2 class="settings-section-title">同步日志</h2>

        <div v-if="syncLogs.length === 0" class="log-empty">暂无日志</div>
        <ul v-else class="log-list">
            <li v-for="(log, idx) in syncLogs" :key="idx" class="log-item" :class="log.level">
                <span class="log-time">{{ formatLogTime(log.time) }}</span>
                <span class="log-msg">{{ log.message }}</span>
            </li>
        </ul>

        <button v-if="syncLogs.length > 0" class="btn-clear-log" @click="clearLogs">
            清空日志
        </button>
    </section>
</template>

<style scoped>
.log-empty {
    font-size: 13px;
    color: #999;
    padding: 8px 0;
}

.log-list {
    list-style: none;
    padding: 0;
    margin: 0;
    max-height: 240px;
    overflow-y: auto;
    border: 1px solid #e0e0e0;
    border-radius: 4px;
    background: #fafafa;
}

.log-item {
    display: flex;
    gap: 12px;
    padding: 6px 10px;
    font-size: 12px;
    border-bottom: 1px solid #ececec;
}

.log-item:last-child {
    border-bottom: none;
}

.log-item.info {
    color: #555;
}

.log-item.success {
    color: #27ae60;
}

.log-item.error {
    color: #c0392b;
}

.log-time {
    font-family: 'Consolas', monospace;
    color: #999;
    flex-shrink: 0;
}

.log-msg {
    word-break: break-all;
}

.btn-clear-log {
    margin-top: 8px;
    padding: 4px 10px;
    background: transparent;
    border: 1px solid #d0d0d0;
    border-radius: 3px;
    cursor: pointer;
    font-size: 12px;
    color: #666;
}

.btn-clear-log:hover {
    background: #f0f0f0;
}
</style>
