import type { ModuleInterface } from '../../platform/services/module-types';

const readerModule: ModuleInterface = {
    id: 'reader',
    name: '阅读器',
    version: '1.0.0',

    async onLoad(context) {
        console.log('[Reader] 模块已加载，平台:', context.platform);
    },

    getRoutes() {
        return [
            {
                path: '/',
                name: 'bookshelf',
                component: () => import('./views/BookshelfView.vue'),
            },
            {
                path: '/book/:id',
                name: 'book-reader',
                component: () => import('./views/BookReaderView.vue'),
            },
        ];
    },
};

export default readerModule;