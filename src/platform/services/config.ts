import { invoke } from '@tauri-apps/api/core';

export async function getConfig(key: string): Promise<string | null> {
    return await invoke<string | null>('get_config', { key });
}

export async function setConfig(key: string, value: string): Promise<void> {
    await invoke('set_config', { key, value });
}

export async function deleteConfig(key: string): Promise<void> {
    await invoke('delete_config', { key });
}

// ============ 同步相关配置键 ============
export const CONFIG_KEYS = {
    SYNC_RECEIVE_ENABLED: 'sync.receive.enabled',
    SYNC_PORT: 'sync.port',
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

// export async function applySyncResponse(response: SyncResponse): Promise<void> {
//     // 用返回的 payload 再次合并到本地
//     // 复用 exchange_sync_data 逻辑的合并部分：通过再次调用 merge 命令
//     // 简化方案：把 serverPayload 当作从对方收到的数据，直接走一次本地合并
//     // 这里直接调用 Rust 命令：把 serverPayload 推给"自己"是不行的
//     // 更好的方式：新增一个 merge_local_payload 命令，见第 11 小步
//     throw new Error('Not implemented, see step 11');
// }

export async function applyLocalPayload(
    payload: SyncPayload,
): Promise<[number, number, number]> {
    return await invoke<[number, number, number]>('apply_local_payload', { payload });
}