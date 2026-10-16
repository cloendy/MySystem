import type {
    ModuleInterface,
    ModuleContext,
    ModuleRegistration,
    Platform,
} from './module-types';
import { createEventBus } from './event-bus';

// 所有已加载的模块
const loadedModules = new Map<string, ModuleInterface>();

// 所有模块注册信息（来自 modules.json）
let registrations: ModuleRegistration[] = [];

/**
 * 检测当前运行平台
 */
export function detectPlatform(): Platform {
    // Tauri 环境下可以通过 @tauri-apps/api/os 判断，但为了不引入额外依赖，
    // 先用 navigator.userAgent 做粗略判断
    const ua = navigator.userAgent.toLowerCase();
    if (ua.includes('android')) return 'android';
    return 'windows';
}

/**
 * 读取 modules.json
 */
export async function loadRegistrations(): Promise<ModuleRegistration[]> {
    if (registrations.length > 0) return registrations;

    const res = await fetch('/modules.json');
    const data = await res.json();
    registrations = data.modules as ModuleRegistration[];
    return registrations;
}

/**
 * 用 Vite 的 glob 动态发现所有模块入口
 */
const moduleEntries = import.meta.glob<{ default: ModuleInterface }>(
    '../../modules/*/index.ts'
);

/**
 * 按平台加载所有启用的模块
 */
export async function loadEnabledModules(
    platform: Platform
): Promise<ModuleInterface[]> {
    const regs = await loadRegistrations();
    const eventBus = createEventBus();
    const context: ModuleContext = { platform, events: eventBus };

    const loaded: ModuleInterface[] = [];

    for (const reg of regs) {
        if (!reg.enabled[platform]) {
            console.log(`[ModuleLoader] 跳过模块 ${reg.id}（平台 ${platform} 未启用）`);
            continue;
        }

        // 从 glob 结果中找到匹配的入口
        const entryKey = Object.keys(moduleEntries).find((k) =>
            k.endsWith(`/${reg.id}/index.ts`)
        );

        if (!entryKey) {
            console.warn(`[ModuleLoader] 模块 ${reg.id} 的入口文件未找到`);
            continue;
        }

        try {
            const mod = await moduleEntries[entryKey]();
            const instance = mod.default;

            if (!instance || typeof instance.onLoad !== 'function') {
                console.warn(`[ModuleLoader] 模块 ${reg.id} 未正确实现 ModuleInterface`);
                continue;
            }

            await instance.onLoad(context);
            loadedModules.set(reg.id, instance);
            loaded.push(instance);
            console.log(`[ModuleLoader] 已加载模块: ${reg.id} v${reg.version}`);
        } catch (err) {
            console.error(`[ModuleLoader] 模块 ${reg.id} 加载失败:`, err);
        }
    }

    return loaded;
}

/**
 * 获取已加载的模块
 */
export function getLoadedModules(): ModuleInterface[] {
    return Array.from(loadedModules.values());
}

/**
 * 按 ID 获取模块
 */
export function getModule(id: string): ModuleInterface | undefined {
    return loadedModules.get(id);
}

/**
 * 卸载所有模块
 */
export async function unloadAllModules(): Promise<void> {
    for (const mod of loadedModules.values()) {
        try {
            await mod.onUnload?.();
        } catch (err) {
            console.error(`[ModuleLoader] 模块 ${mod.id} 卸载失败:`, err);
        }
    }
    loadedModules.clear();
}