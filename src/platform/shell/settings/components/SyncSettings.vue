<script setup lang="ts">
import { onMounted } from 'vue';
import { useSyncSettings } from '../useSyncSettings';
import DeviceScanner from './DeviceScanner.vue';

const {
    syncEnabled,
    deviceName,
    deviceId,
    saving,
    initSyncSettings,
    toggleSync,
    handleRenameDevice,
} = useSyncSettings();

onMounted(initSyncSettings);
</script>

<template>
    <section class="settings-section">
        <h2 class="settings-section-title">同步</h2>

        <div class="setting-row">
            <div class="setting-info">
                <div class="setting-label">接收同步请求</div>
                <div class="setting-desc">
                    开启后，本设备可被局域网内其他设备发现并接受同步请求
                </div>
            </div>
            <button class="switch" :class="{ on: syncEnabled }" :disabled="saving" @click="toggleSync">
                <span class="switch-knob"></span>
            </button>
        </div>

        <div class="setting-row">
            <div class="setting-info">
                <div class="setting-label">设备名称</div>
                <div class="setting-desc">{{ deviceName }}</div>
            </div>
            <button class="btn-secondary" @click="handleRenameDevice">修改</button>
        </div>

        <div class="setting-row">
            <div class="setting-info">
                <div class="setting-label">设备 ID</div>
                <div class="setting-desc mono">{{ deviceId }}</div>
            </div>
        </div>

        <div class="status-indicator">
            <span class="dot" :class="{ on: syncEnabled }"></span>
            {{ syncEnabled ? '可被其他设备发现' : '当前不可被发现' }}
        </div>

        <DeviceScanner />
    </section>
</template>

<style scoped>
.status-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    padding: 8px 12px;
    background: #f5f5f5;
    border-radius: 4px;
    font-size: 13px;
    color: #666;
}

.dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #ccc;
}

.dot.on {
    background: #27ae60;
    box-shadow: 0 0 6px rgba(39, 174, 96, 0.5);
}
</style>
