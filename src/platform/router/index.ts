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
                path: '/',
                name: 'home',
                component: () => import('../shell/HomeView.vue'),
            },
            {
                path: '/settings',
                name: 'settings',
                component: () => import('../shell/SettingsView.vue'),
            },
        ],
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
            console.log(`[Router] 注册路由: ${fullPath} (${mod.id})`);
        }
    }
}