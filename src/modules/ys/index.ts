import { logger } from '../../platform/services/logger';
import type { ModuleInterface } from '../../platform/services/module-types';

const ysModule: ModuleInterface = {
    id: 'ys',
    name: '角色图鉴',
    version: '1.0.0',
    icon: '🎴',
    order: 40,
    enabled: { windows: true, android: true },

    async onLoad(context) {
        logger.info('[YS] 模块已加载，平台:', context.platform);
    },

    getRoutes() {
        return [
            {
                path: '/',
                name: 'ys-list',
                component: () => import('./views/YsListView.vue'),
            },
        ];
    },
};

export default ysModule;
