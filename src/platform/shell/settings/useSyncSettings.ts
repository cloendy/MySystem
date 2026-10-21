import { ref } from 'vue';
import {
    applyLocalPayload,
    checkRemoteDevice,
    exchangeSyncData,
    getDeviceName,
    getSyncPayload,
    getSyncReceiveEnabled,
    scanDevices,
    setDeviceName,
    setSyncReceiveEnabled,
    startDiscoveryListener,
    startSyncServer,
    stopDiscoveryListener,
    stopSyncServer,
    type DeviceInfo,
    type RemoteDeviceInfo,
} from '../../services/config';
import { getDeviceId } from '../../services/device';
import { logger } from '../../services/logger';
import { detectPlatform } from '../../services/module-loader';

export interface DeviceConnectionState {
    status: 'idle' | 'connecting' | 'connected' | 'failed';
    info?: RemoteDeviceInfo;
    error?: string;
}

export interface SyncState {
    status: 'idle' | 'syncing' | 'success' | 'failed';
    message?: string;
}

export interface SyncLogEntry {
    time: number;
    level: 'info' | 'success' | 'error';
    message: string;
}

// 模块级单例：设置页内的多个子组件共享同一份同步状态
const syncEnabled = ref(false);
const deviceName = ref('');
const deviceId = ref('');
const saving = ref(false);
const scanning = ref(false);
const scannedDevices = ref<DeviceInfo[]>([]);
const scanPerformed = ref(false);
const deviceStates = ref<Map<string, DeviceConnectionState>>(new Map());
const syncStates = ref<Map<string, SyncState>>(new Map());
const syncLogs = ref<SyncLogEntry[]>([]);

let initialized = false;

function addLog(level: SyncLogEntry['level'], message: string) {
    syncLogs.value.unshift({ time: Date.now(), level, message });
    if (syncLogs.value.length > 50) {
        syncLogs.value = syncLogs.value.slice(0, 50);
    }
    // 同步日志同时落到系统日志文件，便于事后排查
    if (level === 'error') {
        logger.error(`[Sync] ${message}`);
    } else {
        logger.info(`[Sync] ${message}`);
    }
}

function clearLogs() {
    syncLogs.value = [];
}

function formatLogTime(ms: number): string {
    const d = new Date(ms);
    return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}:${String(d.getSeconds()).padStart(2, '0')}`;
}

function getDeviceState(id: string): DeviceConnectionState {
    return deviceStates.value.get(id) ?? { status: 'idle' };
}

function getSyncState(id: string): SyncState {
    return syncStates.value.get(id) ?? { status: 'idle' };
}

async function startServices() {
    const platform = detectPlatform();
    await startDiscoveryListener(deviceId.value, deviceName.value, platform);
    await startSyncServer(deviceId.value, deviceName.value, platform);
}

async function stopServices() {
    await stopDiscoveryListener();
    await stopSyncServer();
}

/**
 * 首次进入设置页时读取配置并恢复同步服务
 */
async function initSyncSettings() {
    if (initialized) return;

    try {
        syncEnabled.value = await getSyncReceiveEnabled();
        deviceName.value = await getDeviceName();
        deviceId.value = getDeviceId();

        if (syncEnabled.value) {
            await startServices();
            logger.info('[Settings] 监听与 HTTP 服务已自动启动');
        }
        // 初始化成功后才标记，失败时下次进入设置页可重试
        initialized = true;
    } catch (err) {
        logger.error('[Settings] 同步设置初始化失败:', err);
    }
}

async function toggleSync() {
    saving.value = true;
    try {
        syncEnabled.value = !syncEnabled.value;
        await setSyncReceiveEnabled(syncEnabled.value);

        if (syncEnabled.value) {
            await startServices();
            logger.info('[Settings] 监听与 HTTP 服务已启动');
        } else {
            await stopServices();
            logger.info('[Settings] 监听与 HTTP 服务已停止');
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
        scannedDevices.value = await scanDevices(deviceId.value, 2000);
    } catch (err) {
        logger.error('[Settings] 扫描失败:', err);
    } finally {
        scanPerformed.value = true;
        scanning.value = false;
    }
}

async function handleRenameDevice() {
    const name = prompt('设备名称：', deviceName.value);
    if (!name?.trim() || name === deviceName.value) return;

    const oldName = deviceName.value;
    deviceName.value = name.trim();
    await setDeviceName(deviceName.value);
    logger.info(`[Settings] 重命名设备: ${oldName} -> ${deviceName.value}`);
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

export function useSyncSettings() {
    return {
        syncEnabled,
        deviceName,
        deviceId,
        saving,
        scanning,
        scannedDevices,
        scanPerformed,
        deviceStates,
        syncStates,
        syncLogs,
        initSyncSettings,
        toggleSync,
        handleScan,
        handleRenameDevice,
        handleConnect,
        handleSync,
        getDeviceState,
        getSyncState,
        clearLogs,
        formatLogTime,
    };
}
