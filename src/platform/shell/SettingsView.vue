<script setup lang="ts">
import { onMounted, ref } from 'vue';
import {
    getSyncReceiveEnabled,
    setSyncReceiveEnabled,
    getDeviceName,
    setDeviceName,
} from '../services/config';
import { getDeviceId } from '../services/storage';
import {
    startDiscoveryListener,
    stopDiscoveryListener,
    scanDevices,
    startSyncServer,
    stopSyncServer,
    checkRemoteDevice,
    type DeviceInfo,
    type RemoteDeviceInfo,
} from '../services/config';
import { detectPlatform } from '../services/module-loader';
import {
    getSyncPayload,
    exchangeSyncData,
    applyLocalPayload,
} from '../services/config';

const syncEnabled = ref(false);
const deviceName = ref('');
const deviceId = ref('');
const saving = ref(false);
const scanning = ref(false);
const scannedDevices = ref<DeviceInfo[]>([]);
const scanPerformed = ref(false);

onMounted(async () => {
    syncEnabled.value = await getSyncReceiveEnabled();
    deviceName.value = await getDeviceName();
    deviceId.value = getDeviceId();

    // 如果开关已开，启动监听
    if (syncEnabled.value) {
        await startDiscoveryListener(deviceId.value, deviceName.value, detectPlatform());
        await startSyncServer(deviceId.value, deviceName.value, detectPlatform());
        console.log('[Settings] 监听与 HTTP 服务已自动启动');
    }
});

async function toggleSync() {
    saving.value = true;
    try {
        syncEnabled.value = !syncEnabled.value;
        await setSyncReceiveEnabled(syncEnabled.value);

        if (syncEnabled.value) {
            await startDiscoveryListener(deviceId.value, deviceName.value, detectPlatform());
            await startSyncServer(deviceId.value, deviceName.value, detectPlatform());
            console.log('[Settings] 监听与 HTTP 服务已启动');
        } else {
            await stopDiscoveryListener();
            await stopSyncServer();
            console.log('[Settings] 监听与 HTTP 服务已停止');
        }
    } finally {
        saving.value = false;
    }
}

async function handleScan() {
    scanning.value = true;
    scannedDevices.value = [];
    scanPerformed.value = false;
    deviceStates.value = new Map();
    syncStates.value = new Map();
    try {
        // 正式代码
        // scannedDevices.value = await scanDevices(deviceId.value, 2000);
        // console.log('[Settings] 扫描结果:', scannedDevices.value);

        // // ===== 临时测试：把自己加进去 =====
        const found = await scanDevices(deviceId.value, 2000);
        const testSelf = {
            deviceId: deviceId.value,
            deviceName: deviceName.value + '(自测)',
            platform: detectPlatform(),
            ipAddress: '127.0.0.1',
        };
        scannedDevices.value = [...found, testSelf];
        // // ===== 临时测试结束 =====
    } catch (err) {
        console.error('[Settings] 扫描失败:', err);
    } finally {
        scanPerformed.value = true;
        scanning.value = false;
    }
}

async function handleRenameDevice() {
    const name = prompt('设备名称：', deviceName.value);
    if (!name?.trim() || name === deviceName.value) return;
    deviceName.value = name.trim();
    await setDeviceName(deviceName.value);
}

interface DeviceConnectionState {
    status: 'idle' | 'connecting' | 'connected' | 'failed';
    info?: RemoteDeviceInfo;
    error?: string;
}

const deviceStates = ref<Map<string, DeviceConnectionState>>(new Map());

function getDeviceState(deviceId: string): DeviceConnectionState {
    return deviceStates.value.get(deviceId) ?? { status: 'idle' };
}

async function handleConnect(device: DeviceInfo) {
    const newMap = new Map(deviceStates.value);
    newMap.set(device.deviceId, { status: 'connecting' });
    deviceStates.value = newMap;
    addLog('info', `正在连接 ${device.deviceName} (${device.ipAddress})...`);

    try {
        const info = await checkRemoteDevice(device.ipAddress);
        const doneMap = new Map(deviceStates.value);
        doneMap.set(device.deviceId, { status: 'connected', info });
        deviceStates.value = doneMap;
        addLog('success', `已连接 ${info.deviceName}`);
    } catch (err) {
        const failMap = new Map(deviceStates.value);
        const msg = err instanceof Error ? err.message : String(err);
        failMap.set(device.deviceId, { status: 'failed', error: msg });
        deviceStates.value = failMap;
        addLog('error', `连接失败：${msg}`);
    }
}

interface SyncState {
    status: 'idle' | 'syncing' | 'success' | 'failed';
    message?: string;
}
const syncStates = ref<Map<string, SyncState>>(new Map());

function getSyncState(deviceId: string): SyncState {
    return syncStates.value.get(deviceId) ?? { status: 'idle' };
}

async function handleSync(device: DeviceInfo) {
    const newMap = new Map(syncStates.value);
    newMap.set(device.deviceId, { status: 'syncing', message: '正在同步...' });
    syncStates.value = newMap;
    addLog('info', `开始与 ${device.deviceName} 同步...`);

    try {
        const localPayload = await getSyncPayload(deviceId.value, deviceName.value);
        const response = await exchangeSyncData(device.ipAddress, localPayload);
        await applyLocalPayload(response.serverPayload);

        const doneMap = new Map(syncStates.value);
        const summary = `新增 ${response.accepted}，合并 ${response.merged}，拒绝 ${response.rejected}`;
        doneMap.set(device.deviceId, { status: 'success', message: summary });
        syncStates.value = doneMap;
        addLog('success', `同步完成：${summary}`);

        window.dispatchEvent(new CustomEvent('mysystem:sync-completed'));
    } catch (err) {
        const failMap = new Map(syncStates.value);
        const msg = err instanceof Error ? err.message : String(err);
        failMap.set(device.deviceId, { status: 'failed', message: msg });
        syncStates.value = failMap;
        addLog('error', `同步失败：${msg}`);
    }
}

interface SyncLogEntry {
    time: number;
    level: 'info' | 'success' | 'error';
    message: string;
}
const syncLogs = ref<SyncLogEntry[]>([]);

function addLog(level: SyncLogEntry['level'], message: string) {
    syncLogs.value.unshift({ time: Date.now(), level, message });
    if (syncLogs.value.length > 50) {
        syncLogs.value = syncLogs.value.slice(0, 50);
    }
}

function formatLogTime(ms: number): string {
    const d = new Date(ms);
    return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}:${String(d.getSeconds()).padStart(2, '0')}`;
}
</script>

<template>
    <div class="settings-view">
        <h1 class="page-title">设置</h1>

        <section class="section">
            <h2 class="section-title">同步</h2>

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

            <div class="scan-section">
                <button class="btn-primary" :disabled="scanning" @click="handleScan">
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
                                <button class="btn-connect"
                                    :disabled="getDeviceState(dev.deviceId).status === 'connecting'"
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
        </section>

        <section class="section">
            <h2 class="section-title">同步日志</h2>
            <div v-if="syncLogs.length === 0" class="log-empty">暂无日志</div>
            <ul v-else class="log-list">
                <li v-for="(log, idx) in syncLogs" :key="idx" class="log-item" :class="log.level">
                    <span class="log-time">{{ formatLogTime(log.time) }}</span>
                    <span class="log-msg">{{ log.message }}</span>
                </li>
            </ul>
            <button v-if="syncLogs.length > 0" class="btn-clear-log" @click="syncLogs = []">
                清空日志
            </button>
        </section>

        <section class="section">
            <h2 class="section-title">关于</h2>
            <p class="about-line">MySystem v0.1.0</p>
            <p class="about-line">个人跨平台系统</p>
        </section>
    </div>
</template>

<style scoped>
.settings-view {
    padding: 24px;
    max-width: 640px;
    height: 100%;
    overflow-y: auto;
}

.page-title {
    font-size: 24px;
    font-weight: 600;
    margin-bottom: 24px;
}

.section {
    margin-bottom: 28px;
}

.section-title {
    font-size: 14px;
    font-weight: 600;
    color: #666;
    text-transform: uppercase;
    margin-bottom: 12px;
    padding-bottom: 6px;
    border-bottom: 1px solid #e0e0e0;
}

.setting-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 0;
    gap: 16px;
}

.setting-info {
    flex: 1;
}

.setting-label {
    font-size: 15px;
    color: #333;
    font-weight: 500;
}

.setting-desc {
    font-size: 13px;
    color: #888;
    margin-top: 2px;
    word-break: break-all;
}

.setting-desc.mono {
    font-family: 'Consolas', 'Monaco', monospace;
    font-size: 12px;
}

.btn-secondary {
    padding: 6px 14px;
    background: white;
    border: 1px solid #d0d0d0;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
}

.btn-secondary:hover {
    background: #f5f5f5;
}

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

.about-line {
    font-size: 14px;
    color: #333;
    line-height: 1.6;
}

.btn-primary {
    padding: 8px 16px;
    background: #2c3e50;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    margin-top: 12px;
}

.btn-primary:hover:not(:disabled) {
    background: #34495e;
}

.btn-primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
}

.scan-section {
    margin-top: 16px;
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