<script setup lang="ts">
import { useSyncSettings } from '../useSyncSettings';

const {
    scanning,
    scannedDevices,
    scanPerformed,
    handleScan,
    getDeviceState,
    getSyncState,
    handleConnect,
    handleSync,
} = useSyncSettings();
</script>

<template>
    <div class="scan-section">
        <button class="btn btn-primary scan-btn" :disabled="scanning" @click="handleScan">
            {{ scanning ? '扫描中...' : '扫描局域网设备' }}
        </button>

        <ul v-if="scannedDevices.length > 0" class="device-list">
            <li v-for="dev in scannedDevices" :key="dev.deviceId" class="device-item">
                <div class="device-row">
                    <div class="device-info-block">
                        <div class="device-name">{{ dev.deviceName }}</div>
                        <div class="device-meta">{{ dev.platform }} · {{ dev.ipAddress }}</div>
                    </div>
                    <div class="device-actions">
                        <button class="btn-connect" :disabled="getDeviceState(dev.deviceId).status === 'connecting'"
                            @click="handleConnect(dev)">
                            {{ getDeviceState(dev.deviceId).status === 'connecting' ? '连接中...' : '连接' }}
                        </button>
                        <button class="btn-sync" :disabled="getDeviceState(dev.deviceId).status !== 'connected' ||
                            getSyncState(dev.deviceId).status === 'syncing'
                            " @click="handleSync(dev)">
                            {{ getSyncState(dev.deviceId).status === 'syncing' ? '同步中...' : '同步' }}
                        </button>
                    </div>
                </div>

                <div v-if="getDeviceState(dev.deviceId).status === 'connected'" class="device-status connected">
                    ✅ 已就绪 · 协议版本 {{ getDeviceState(dev.deviceId).info?.protocolVersion }}
                </div>
                <div v-else-if="getDeviceState(dev.deviceId).status === 'failed'" class="device-status failed">
                    ❌ {{ getDeviceState(dev.deviceId).error }}
                </div>

                <div v-if="getSyncState(dev.deviceId).status === 'success'" class="device-status connected">
                    ✅ {{ getSyncState(dev.deviceId).message }}
                </div>
                <div v-else-if="getSyncState(dev.deviceId).status === 'failed'" class="device-status failed">
                    ❌ {{ getSyncState(dev.deviceId).message }}
                </div>
            </li>
        </ul>

        <p v-else-if="!scanning && scannedDevices.length === 0 && scanPerformed" class="no-device">
            未发现已打开同步开关的设备
        </p>
    </div>
</template>

<style scoped>
.scan-section {
    margin-top: 16px;
}

.scan-btn {
    padding: 8px 16px;
    margin-top: 12px;
}

.device-list {
    list-style: none;
    margin-top: 12px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.device-item {
    padding: 10px 12px;
    border: 1px solid #e0e0e0;
    border-radius: 4px;
    background: #fafafa;
}

.device-name {
    font-size: 14px;
    font-weight: 500;
    color: #333;
}

.device-meta {
    font-size: 12px;
    color: #888;
    margin-top: 2px;
}

.no-device {
    margin-top: 12px;
    font-size: 13px;
    color: #999;
}

.device-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
}

.device-info-block {
    flex: 1;
    min-width: 0;
}

.btn-connect {
    padding: 6px 14px;
    background: white;
    border: 1px solid #2c3e50;
    color: #2c3e50;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
    flex-shrink: 0;
}

.btn-connect:hover:not(:disabled) {
    background: #2c3e50;
    color: white;
}

.btn-connect:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.device-status {
    margin-top: 8px;
    padding: 6px 10px;
    border-radius: 4px;
    font-size: 12px;
}

.device-status.connected {
    background: #e8f5e9;
    color: #27ae60;
}

.device-status.failed {
    background: #fdeaea;
    color: #c0392b;
    word-break: break-all;
}

.device-actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
}

.btn-sync {
    padding: 6px 14px;
    background: #2c3e50;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
}

.btn-sync:hover:not(:disabled) {
    background: #34495e;
}

.btn-sync:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}
</style>
