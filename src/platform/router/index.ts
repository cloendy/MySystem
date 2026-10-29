import { logger } from '../services/logger'
import { createRouter, createWebHashHistory, type Router } from 'vue-router';
import type { ModuleInterface } from '../services/module-types';

let router: Router | null = null;

/**
 * 创建路由实例（全局单例）
 */
export function createAppRouter(): Router {
    if (router) return router;

    router = createRouter({
        history: createWebHashHistory(),
        routes: [
            {
                path: '/settings',
                name: 'settings',
                component: () => import('../shell/SettingsView.vue'),
            },
        ],
    });

    // 记录页面/模块切换，便于还原用户操作路径
    router.afterEach((to, from) => {
        if (to.fullPath === from.fullPath) return;
        const fromLabel = from.matched.length === 0 ? '(启动)' : from.fullPath;
        logger.info(`[Router] 页面切换: ${fromLabel} -> ${to.fullPath}`);
    });

    return router;
}

/**
 * 获取路由实例
 */
export function getRouter(): Router {
    if (!router) {
        throw new Error('Router 尚未初始化，请先调用 createAppRouter()');
    }
    return router;
}

/**
 * 把模块的路由注册到路由器
 */
export function registerModuleRoutes(modules: ModuleInterface[]): void {
    const r = getRouter();

    for (const mod of modules) {
        const routes = mod.getRoutes();
        for (const route of routes) {
            // 模块路由统一加上模块ID前缀，避免冲突
            const fullPath = `/${mod.id}${route.path.startsWith('/') ? route.path : '/' + route.path}`;
            r.addRoute({
                path: fullPath,
                name: `${mod.id}:${route.name}`,
                component: route.component,
                meta: { ...route.meta, moduleId: mod.id, moduleName: mod.name },
            });
            logger.info(`[Router] 注册路由: ${fullPath} (${mod.id})`);
        }
    }

    // 不再提供独立首页，根路径直接跳转到第一个可用模块
    if (modules.length > 0) {
        const firstModulePath = `/${modules[0].id}/`;
        r.addRoute({
            path: '/',
            redirect: firstModulePath,
        });
        logger.info(`[Router] 根路径重定向: / -> ${firstModulePath}`);
    }
}