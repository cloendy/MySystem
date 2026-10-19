// 平台级 UI 自动日志。
//
// 在 `main.ts` 启动时调用 `installUiLogging()`，自动记录按钮点击。
// 页面切换由 `platform/router/index.ts` 的 `router.afterEach` 记录。
// 业务结果日志由各 store / 视图按需补充。

import { logger } from './logger'
import { getRouter } from '../router'

/** 同一按钮在该时间内重复点击只记录一次（避免双击/连点刷屏） */
const CLICK_DEDUP_MS = 300

let lastClickKey = ''
let lastClickAt = 0

function currentRoutePath(): string {
    try {
        return getRouter().currentRoute.value.fullPath
    } catch {
        return '-'
    }
}

/** 从元素上提取一个可读名称 */
function describeElement(el: Element): string {
    const aria = el.getAttribute('aria-label')?.trim()
    const title = el.getAttribute('title')?.trim()
    const text = (el.textContent ?? '').replace(/\s+/g, ' ').trim()
    const value = el instanceof HTMLInputElement ? el.value.trim() : ''
    const fallback = el.id || el.getAttribute('class') || el.tagName.toLowerCase()
    const name = aria || title || text || value || fallback
    return name.length > 40 ? `${name.slice(0, 40)}…` : name
}

function handleClick(event: MouseEvent): void {
    const target = event.target
    if (!(target instanceof Element)) return

    const el = target.closest(
        'button, [role="button"], input[type="button"], input[type="submit"], a[href]',
    )
    if (!el || el.hasAttribute('data-log-ignore')) return
    if (el.hasAttribute('disabled')) return

    const label = describeElement(el)
    if (!label) return

    const route = currentRoutePath()
    const key = `${route}|${label}`
    const now = Date.now()
    if (key === lastClickKey && now - lastClickAt < CLICK_DEDUP_MS) return
    lastClickKey = key
    lastClickAt = now

    logger.info(`[UI] 点击「${label}」 @ ${route}`)
}

/** 安装全局 UI 日志（点击事件委托） */
export function installUiLogging(): void {
    document.addEventListener('click', handleClick, true)
}
