import { confirm } from '@tauri-apps/plugin-dialog';
import { reactive } from 'vue';

/**
 * 统一的二次确认弹窗。
 *
 * 原生 `window.confirm` 在 Tauri 的 WebView 中点击「取消」仍可能返回 true，
 * 因此改用 dialog 插件，保证返回值可靠。
 */
export async function confirmAction(message: string, title = '请确认'): Promise<boolean> {
    return await confirm(message, { title, kind: 'warning' });
}

export type ChoiceResult = 'first' | 'second' | 'cancel';

interface ChoiceState {
    visible: boolean;
    title: string;
    message: string;
    first: string;
    second: string;
    cancelText: string;
}

/** 三选一对话框状态，由全局挂载的 ChoiceDialog 渲染 */
export const choiceState = reactive<ChoiceState>({
    visible: false,
    title: '请选择',
    message: '',
    first: '',
    second: '',
    cancelText: '取消',
});

let choiceResolver: ((result: ChoiceResult) => void) | null = null;

export interface ChooseActionOptions {
    first: string;
    second: string;
    title?: string;
    cancelText?: string;
}

/**
 * 三选一对话框：返回点中的按钮（first / second / cancel）。
 * 原生 dialog 插件无法自定义第三个按钮，因此改用组件实现。
 */
export function chooseAction(
    message: string,
    options: ChooseActionOptions,
): Promise<ChoiceResult> {
    // 若上一个对话框还没关闭，先按取消处理，避免 promise 悬挂
    choiceResolver?.('cancel');

    choiceState.title = options.title ?? '请选择';
    choiceState.message = message;
    choiceState.first = options.first;
    choiceState.second = options.second;
    choiceState.cancelText = options.cancelText ?? '取消';
    choiceState.visible = true;

    return new Promise<ChoiceResult>((resolve) => {
        choiceResolver = resolve;
    });
}

/** 由 ChoiceDialog 调用，关闭对话框并返回结果 */
export function resolveChoice(result: ChoiceResult) {
    choiceState.visible = false;
    const resolver = choiceResolver;
    choiceResolver = null;
    resolver?.(result);
}
