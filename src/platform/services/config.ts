import { invoke } from '@tauri-apps/api/core';

export async function getConfig(key: string): Promise<string | null> {
    return await invoke<string | null>('get_config', { key });
}

export async function setConfig(key: string, value: string): Promise<void> {
    await invoke('set_config', { key, value });
}

// ============ 同步相关配置键 ============
export const CONFIG_KEYS = {
    SYNC_RECEIVE_ENABLED: 'sync.receive.enabled',
    DEVICE_NAME: 'device.name',
} as const;

export async function getSyncReceiveEnabled(): Promise<boolean> {
    const val = await getConfig(CONFIG_KEYS.SYNC_RECEIVE_ENABLED);
    return val === 'true';
}

export async function setSyncReceiveEnabled(enabled: boolean): Promise<void> {
    await setConfig(CONFIG_KEYS.SYNC_RECEIVE_ENABLED, enabled ? 'true' : 'false');
}

export async function getDeviceName(): Promise<string> {
    const val = await getConfig(CONFIG_KEYS.DEVICE_NAME);
    return val ?? '未命名设备';
}

export async function setDeviceName(name: string): Promise<void> {
    await setConfig(CONFIG_KEYS.DEVICE_NAME, name);
}

export interface DeviceInfo {
    deviceId: string;
    deviceName: string;
    platform: string;
    ipAddress: string;
}

/**
 * 启动设备发现监听（同步开关打开时调用）
 */
export async function startDiscoveryListener(
    deviceId: string,
    deviceName: string,
    platform: string,
): Promise<void> {
    await invoke('start_discovery_listener', { deviceId, deviceName, platform });
}

/**
 * 停止设备发现监听（同步开关关闭时调用）
 */
export async function stopDiscoveryListener(): Promise<void> {
    await invoke('stop_discovery_listener');
}

/**
 * 扫描局域网内已打开同步开关的设备
 */
export async function scanDevices(
    selfDeviceId: string,
    timeoutMs = 2000,
): Promise<DeviceInfo[]> {
    return await invoke<DeviceInfo[]>('scan_devices', {
        timeoutMs,
        selfDeviceId,
    });
}

export async function startSyncServer(
    deviceId: string,
    deviceName: string,
    platform: string,
): Promise<void> {
    await invoke('start_sync_server', { deviceId, deviceName, platform });
}

export async function stopSyncServer(): Promise<void> {
    await invoke('stop_sync_server');
}

export interface RemoteDeviceInfo {
    deviceId: string;
    deviceName: string;
    platform: string;
    protocolVersion: number;
}

export async function checkRemoteDevice(ipAddress: string): Promise<RemoteDeviceInfo> {
    return await invoke<RemoteDeviceInfo>('check_remote_device', { ipAddress });
}

export interface SyncPayload {
    documents: Array<{
        id: string;
        type: string;
        title: string;
        createdAt: number;
        updatedAt: number;
        deletedAt: number | null;
        version: number;
        deviceId: string;
    }>;
    /** 笔记已改用独立的 Note_Content 表，暂不参与同步；该字段仅为兼容保留，始终为空 */
    notes: Array<{
        documentId: string;
        content: string;
        tags: string;
        folderId: string | null;
        isPinned: number;
    }>;
    senderDeviceId: string;
    senderDeviceName: string;
}

export interface SyncResponse {
    accepted: number;
    rejected: number;
    merged: number;
    serverPayload: SyncPayload;
}

export async function getSyncPayload(
    deviceId: string,
    deviceName: string,
): Promise<SyncPayload> {
    return await invoke<SyncPayload>('get_sync_payload', { deviceId, deviceName });
}

export async function exchangeSyncData(
    ipAddress: string,
    payload: SyncPayload,
): Promise<SyncResponse> {
    return await invoke<SyncResponse>('exchange_sync_data', { ipAddress, payload });
}

export async function applyLocalPayload(
    payload: SyncPayload,
): Promise<[number, number, number]> {
    return await invoke<[number, number, number]>('apply_local_payload', { payload });
}