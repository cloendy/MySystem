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
    <div class="app-shell">
        <!-- 顶部工具栏 -->
        <header class="topbar">
            <div class="topbar-left">
                <span class="logo">MySystem</span>
            </div>
            <div class="topbar-center">
                <input class="search-input" type="text" placeholder="搜索..." disabled />
            </div>
            <div class="topbar-right">
                <span class="sync-status">同步: 未连接</span>
            </div>
        </header>

        <div class="main-area">
            <!-- 左侧导航 -->
            <aside class="sidebar">
                <nav class="nav-list">
                    <button v-for="m in modules" :key="m.id" class="nav-item" :class="{ active: isActive(m.id) }"
                        @click="goToModule(m.id)">
                        {{ m.name }}
                    </button>
                </nav>

                <div class="sidebar-footer">
                    <button class="nav-item settings-btn" :class="{ active: isSettingsActive() }" @click="goToSettings">
                        ⚙️ 设置
                    </button>
                </div>
            </aside>

            <!-- 主内容区 -->
            <main class="content">
                <RouterView />
            </main>
        </div>

        <!-- 底部状态栏 -->
        <footer class="statusbar">
            <span>就绪</span>
        </footer>
    </div>
</template>

<style scoped>
.app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

.topbar {
    display: flex;
    align-items: center;
    height: 48px;
    background: #2c3e50;
    color: white;
    padding: 0 16px;
    flex-shrink: 0;
}

.topbar-left {
    flex: 0 0 200px;
}

.logo {
    font-weight: 600;
    font-size: 16px;
}

.topbar-center {
    flex: 1;
    display: flex;
    justify-content: center;
}

.search-input {
    width: 400px;
    padding: 6px 12px;
    border-radius: 4px;
    border: none;
    background: rgba(255, 255, 255, 0.15);
    color: white;
    outline: none;
}

.search-input::placeholder {
    color: rgba(255, 255, 255, 0.5);
}

.topbar-right {
    flex: 0 0 200px;
    text-align: right;
    font-size: 13px;
}

.sync-status {
    opacity: 0.8;
}

.main-area {
    display: flex;
    flex: 1;
    overflow: hidden;
}

.sidebar {
    width: 200px;
    background: #f5f5f5;
    border-right: 1px solid #e0e0e0;
    padding: 12px 0;
    flex-shrink: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
}

.nav-list {
    flex: 1;
}

.sidebar-footer {
    border-top: 1px solid #e0e0e0;
    padding-top: 8px;
    margin-top: 8px;
}

.settings-btn {
    color: #666;
}

.nav-item {
    display: block;
    width: 100%;
    padding: 10px 20px;
    text-align: left;
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 14px;
    color: #333;
    transition: background 0.15s;
}

.nav-item:hover {
    background: #e8e8e8;
}

.nav-item.active {
    background: #e0e8f0;
    color: #2c3e50;
    font-weight: 600;
    border-left: 3px solid #2c3e50;
    padding-left: 17px;
}

.content {
    flex: 1;
    overflow-y: auto;
    background: #fff;
}

.statusbar {
    height: 24px;
    background: #f0f0f0;
    border-top: 1px solid #e0e0e0;
    display: flex;
    align-items: center;
    padding: 0 16px;
    font-size: 12px;
    color: #666;
    flex-shrink: 0;
}
</style>