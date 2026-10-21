import { confirm } from '@tauri-apps/plugin-dialog';

/**
 * 统一的二次确认弹窗。
 *
 * 原生 `window.confirm` 在 Tauri 的 WebView 中点击「取消」仍可能返回 true，
 * 因此改用 dialog 插件，保证返回值可靠。
 */
export async function confirmAction(message: string, title = '请确认'): Promise<boolean> {
    return await confirm(message, { title, kind: 'warning' });
}
