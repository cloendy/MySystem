import { nextTick, onBeforeUnmount, onMounted, ref, type Ref } from 'vue';
import type { MenuAnchor } from '../types';

/**
 * 悬浮菜单的公共行为：跟随锚点定位、越界收边、点击外部 / 滚动 / Esc 关闭。
 *
 * 使用方需要在菜单内容变化后自行调用 `updatePosition()`。
 */
export function useFloatingMenu(
    menuEl: Ref<HTMLElement | null>,
    getAnchor: () => MenuAnchor,
    onClose: () => void,
) {
    // ready 为 false 时先隐藏，避免定位前闪现在左上角
    const position = ref({ left: 0, top: 0, ready: false });

    function updatePosition() {
        const node = menuEl.value;
        if (!node) return;
        const anchor = getAnchor();
        const width = node.offsetWidth;
        const height = node.offsetHeight;
        const left = Math.max(8, Math.min(anchor.right - width, window.innerWidth - width - 8));
        let top = anchor.bottom + 4;
        // 下方空间不足时翻转到锚点上方
        if (top + height > window.innerHeight - 8) {
            top = Math.max(8, anchor.top - height - 4);
        }
        position.value = { left, top, ready: true };
    }

    function handleOutside(event: MouseEvent) {
        if (menuEl.value?.contains(event.target as Node)) return;
        onClose();
    }

    function handleKeydown(event: KeyboardEvent) {
        if (event.key === 'Escape') onClose();
    }

    function handleScroll(event: Event) {
        // 菜单自身滚动不关闭
        if (menuEl.value?.contains(event.target as Node)) return;
        onClose();
    }

    let listenerTimer: number | null = null;

    onMounted(async () => {
        await nextTick();
        updatePosition();
        // 延迟挂载，避免打开菜单的这次点击立刻触发外部点击关闭
        listenerTimer = window.setTimeout(() => {
            document.addEventListener('mousedown', handleOutside, true);
            document.addEventListener('keydown', handleKeydown);
            window.addEventListener('scroll', handleScroll, true);
            window.addEventListener('resize', onClose);
        }, 0);
    });

    onBeforeUnmount(() => {
        if (listenerTimer) clearTimeout(listenerTimer);
        document.removeEventListener('mousedown', handleOutside, true);
        document.removeEventListener('keydown', handleKeydown);
        window.removeEventListener('scroll', handleScroll, true);
        window.removeEventListener('resize', onClose);
    });

    return { position, updatePosition };
}
