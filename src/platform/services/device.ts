/**
 * 平台设备标识服务。
 *
 * 设备 ID 是同步协议中标识本机的稳定标识，首次调用时生成并缓存到 localStorage。
 */

const DEVICE_ID_KEY = 'mysystem_device_id';

/**
 * 获取设备唯一标识（首次调用时生成 UUID v4 并持久化）
 */
export function getDeviceId(): string {
    let id = localStorage.getItem(DEVICE_ID_KEY);
    if (!id) {
        id = crypto.randomUUID();
        localStorage.setItem(DEVICE_ID_KEY, id);
    }
    return id;
}
