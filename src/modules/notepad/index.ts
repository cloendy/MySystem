import type { ModuleInterface } from '../../platform/services/module-types';

const notepadModule: ModuleInterface = {
    id: 'notepad',
    name: '记事本',
    version: '1.0.0',

    async onLoad(context) {
        console.log('[Notepad] 模块已加载，平台:', context.platform);
        // 后续可以在这里预加载数据
    },

    getRoutes() {
        return [
            {
                path: '/',
                name: 'note-list',
                component: () => import('./views/NoteListView.vue'),
            },
            {
                path: '/edit/:id',
                name: 'note-edit',
                component: () => import('./views/NoteEditView.vue'),
            },
        ];
    },
};

export default notepadModule;