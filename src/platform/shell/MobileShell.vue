<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useRouter, useRoute, RouterView } from 'vue-router';
import { getLoadedModules } from '../services/module-loader';
import type { ModuleInterface } from '../services/module-types';

const router = useRouter();
const route = useRoute();
const modules = ref<ModuleInterface[]>([]);

onMounted(() => {
    modules.value = getLoadedModules();
});

function goToModule(moduleId: string) {
    router.push(`/${moduleId}/`);
}

function isActive(moduleId: string): boolean {
    return route.path.startsWith(`/${moduleId}`);
}

function goToSettings() {
    router.push('/settings');
}

function isSettingsActive(): boolean {
    return route.path === '/settings';
}
</script>

<template>
    <div class="mobile-shell">
        <!-- 顶部工具栏 -->
        <header class="topbar">
            <span class="title">
                {{ route.path === '/settings' ? '设置' : (route.meta.moduleName || 'MySystem') }}
            </span>
            <button class="icon-btn" disabled>🔍</button>
        </header>

        <!-- 主内容区 -->
        <main class="content">
            <RouterView />
        </main>

        <!-- 底部标签栏 -->
        <nav class="tabbar">
            <button v-for="m in modules" :key="m.id" class="tab-item" :class="{ active: isActive(m.id) }"
                @click="goToModule(m.id)">
                <span class="tab-icon">{{ m.id === 'notepad' ? '📝' : m.id === 'reader' ? '📖' : '📦' }}</span>
                <span class="tab-label">{{ m.name }}</span>
            </button>

            <button class="tab-item" :class="{ active: isSettingsActive() }" @click="goToSettings">
                <span class="tab-icon">⚙️</span>
                <span class="tab-label">设置</span>
            </button>
        </nav>
    </div>
</template>

<style scoped>
.mobile-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    background: #fff;
}

.topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 56px;
    background: #2c3e50;
    color: white;
    padding: 0 16px;
    flex-shrink: 0;
    padding-top: env(safe-area-inset-top);
}

.title {
    font-weight: 600;
    font-size: 18px;
}

.icon-btn {
    background: transparent;
    border: none;
    color: white;
    font-size: 18px;
    cursor: pointer;
    padding: 8px;
}

.content {
    flex: 1;
    overflow-y: auto;
    background: #fff;
    -webkit-overflow-scrolling: touch;
}

.tabbar {
    display: flex;
    height: 56px;
    background: #fafafa;
    border-top: 1px solid #e0e0e0;
    flex-shrink: 0;
    padding-bottom: env(safe-area-inset-bottom);
}

.tab-item {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    cursor: pointer;
    color: #888;
    transition: color 0.15s;
    gap: 2px;
}

.tab-item.active {
    color: #2c3e50;
}

.tab-icon {
    font-size: 20px;
    line-height: 1;
}

.tab-label {
    font-size: 11px;
    line-height: 1;
}
</style>