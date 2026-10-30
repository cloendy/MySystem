import { logger } from '../../platform/services/logger';
import type { ModuleInterface } from '../../platform/services/module-types';

const jsonviewModule: ModuleInterface = {
    id: 'jsonview',
    name: 'JSON 分析',
    version: '1.0.0',
    icon: '🧩',
    order: 30,
    enabled: { windows: true, android: true },

    async onLoad(context) {
        logger.info('[JsonView] 模块已加载，平台:', context.platform);
    },

    getRoutes() {
        return [
            {
                path: '/',
                name: 'json-analyzer',
                component: () => import('./views/JsonAnalyzerView.vue'),
            },
        ];
    },
};

export default jsonviewModule;
