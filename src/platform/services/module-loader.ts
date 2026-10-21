import { logger } from './logger'
import type {
    ModuleInterface,
    ModuleContext,
    Platform,
} from './module-types';
import { createEventBus } from './event-bus';

// 所有已加载的模块
const loadedModules = new Map<string, ModuleInterface>();

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
 * 用 Vite 的 glob 自动发现 src/modules 下的所有模块入口。
 *
 * 约定：`src/modules/<模块目录>/index.ts` 默认导出 ModuleInterface。
 * 模块的 id/name/version/icon/order/enabled 全部由模块实例自己声明，
 * 不再维护额外的注册表文件。
 *
 * 注意：glob 在开发服务器启动时展开，新增/删除模块目录后需要重启 `pnpm dev`。
 */
const moduleEntries = import.meta.glob<{ default: ModuleInterface }>(
    '../../modules/*/index.ts'
);

/**
 * 自动发现并加载所有启用的模块
 */
export async function loadEnabledModules(
    platform: Platform
): Promise<ModuleInterface[]> {
    const eventBus = createEventBus();
    const context: ModuleContext = { platform, events: eventBus };

    const loaded: ModuleInterface[] = [];

    // 先按入口路径排序，保证加载顺序在不同文件系统下稳定
    const entries = Object.entries(moduleEntries).sort(([a], [b]) =>
        a.localeCompare(b)
    );

    for (const [entryPath, loadEntry] of entries) {
        let instance: ModuleInterface;

        try {
            const mod = await loadEntry();
            instance = mod.default;
        } catch (err) {
            logger.error(`[ModuleLoader] 模块入口加载失败 ${entryPath}:`, err);
            continue;
        }

        if (
            !instance ||
            typeof instance.onLoad !== 'function' ||
            typeof instance.getRoutes !== 'function' ||
            !instance.id
        ) {
            logger.warn(`[ModuleLoader] 模块未正确实现 ModuleInterface: ${entryPath}`);
            continue;
        }

        if (loadedModules.has(instance.id)) {
            logger.warn(
                `[ModuleLoader] 模块 id 重复，已跳过: ${instance.id}（${entryPath}）`
            );
            continue;
        }

        // enabled 未声明时默认所有平台启用
        if (instance.enabled?.[platform] === false) {
            logger.info(`[ModuleLoader] 跳过模块 ${instance.id}（平台 ${platform} 未启用）`);
            continue;
        }

        try {
            await instance.onLoad(context);
            loadedModules.set(instance.id, instance);
            loaded.push(instance);
            logger.info(
                `[ModuleLoader] 已加载模块: ${instance.id} v${instance.version} ← ${entryPath}`
            );
        } catch (err) {
            logger.error(`[ModuleLoader] 模块 ${instance.id} 加载失败:`, err);
        }
    }

    // order 数字小的排前面；未声明按 0 处理
    loaded.sort((a, b) => (a.order ?? 0) - (b.order ?? 0));

    return loaded;
}

/**
 * 获取已加载的模块
 */
export function getLoadedModules(): ModuleInterface[] {
    // 与 loadEnabledModules 的返回值保持一致：按 order 升序（未声明按 0）
    return Array.from(loadedModules.values()).sort(
        (a, b) => (a.order ?? 0) - (b.order ?? 0)
    );
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
    if (loadedModules.size === 0) return;
    for (const mod of loadedModules.values()) {
        try {
            await mod.onUnload?.();
        } catch (err) {
            logger.error(`[ModuleLoader] 模块 ${mod.id} 卸载失败:`, err);
        }
    }
    logger.info(`[ModuleLoader] 已卸载模块: ${Array.from(loadedModules.keys()).join(', ')}`);
    loadedModules.clear();
}
