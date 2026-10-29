<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { useRouter, useRoute, RouterView } from 'vue-router';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { logger } from '../services/logger';
import { getLoadedModules } from '../services/module-loader';
import type { ModuleInterface } from '../services/module-types';

const SIDEBAR_COLLAPSED_KEY = 'mysystem_sidebar_collapsed';

const router = useRouter();
const route = useRoute();
const modules = ref<ModuleInterface[]>([]);
const sidebarCollapsed = ref(readSidebarCollapsed());

// 自定义标题栏：窗口控制按钮
const appWindow = getCurrentWindow();
const isMaximized = ref(false);
let unlistenResized: (() => void) | null = null;

function readSidebarCollapsed(): boolean {
    try {
        return localStorage.getItem(SIDEBAR_COLLAPSED_KEY) === '1';
    } catch {
        return false;
    }
}

onMounted(() => {
    modules.value = getLoadedModules();

    // 监听窗口尺寸变化，同步最大化状态（浏览器开发环境无此 API，忽略错误）
    appWindow.isMaximized().then((v) => { isMaximized.value = v; }).catch(() => { });
    appWindow.onResized(async () => {
        try {
            isMaximized.value = await appWindow.isMaximized();
        } catch {
            // 浏览器环境忽略
        }
    }).then((unlisten) => { unlistenResized = unlisten; }).catch(() => { });
});

onUnmounted(() => {
    unlistenResized?.();
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

function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value;
    try {
        localStorage.setItem(SIDEBAR_COLLAPSED_KEY, sidebarCollapsed.value ? '1' : '0');
    } catch {
        // localStorage 不可用时仅本次会话生效
    }
    logger.info(`[Shell] 侧边栏已${sidebarCollapsed.value ? '收起' : '展开'}`);
}

function minimizeWindow() {
    appWindow.minimize();
}

function toggleMaximizeWindow() {
    appWindow.toggleMaximize();
}

function closeWindow() {
    appWindow.close();
}
</script>

<template>
    <div class="app-shell">
        <!-- 窗口标题栏（自定义，可拖拽移动窗口） -->
        <header class="topbar" data-tauri-drag-region>
            <span class="logo" data-tauri-drag-region>MySystem</span>
            <div class="topbar-spacer" data-tauri-drag-region></div>
            <div class="window-controls">
                <button class="win-btn" title="最小化" @click="minimizeWindow">─</button>
                <button class="win-btn" :title="isMaximized ? '还原' : '最大化'"
                    @click="toggleMaximizeWindow">{{ isMaximized ? '❐' : '□' }}</button>
                <button class="win-btn close" title="关闭" @click="closeWindow">✕</button>
            </div>
        </header>

        <div class="main-area">
            <!-- 左侧导航（可收起为窄栏） -->
            <aside class="sidebar" :class="{ collapsed: sidebarCollapsed }">
                <div class="sidebar-header">
                    <span v-if="!sidebarCollapsed" class="sidebar-title">模块</span>
                    <button class="sidebar-toggle" :title="sidebarCollapsed ? '展开侧边栏' : '收起侧边栏'"
                        @click="toggleSidebar">
                        {{ sidebarCollapsed ? '»' : '«' }}
                    </button>
                </div>

                <nav class="nav-list">
                    <button v-for="m in modules" :key="m.id" class="nav-item" :class="{ active: isActive(m.id) }"
                        :title="m.name" @click="goToModule(m.id)">
                        <span class="nav-icon">{{ m.icon ?? m.name.slice(0, 1) }}</span>
                        <span v-if="!sidebarCollapsed" class="nav-label">{{ m.name }}</span>
                    </button>
                </nav>

                <div class="sidebar-footer">
                    <button class="nav-item settings-btn" :class="{ active: isSettingsActive() }" title="设置"
                        @click="goToSettings">
                        <span class="nav-icon">⚙️</span>
                        <span v-if="!sidebarCollapsed" class="nav-label">设置</span>
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
    height: 40px;
    background: #2c3e50;
    color: white;
    padding: 0 0 0 16px;
    flex-shrink: 0;
    user-select: none;
}

.logo {
    font-weight: 600;
    font-size: 16px;
}

.topbar-spacer {
    flex: 1;
    height: 100%;
}

.window-controls {
    display: flex;
    align-items: stretch;
    height: 100%;
}

.win-btn {
    width: 46px;
    border: none;
    background: transparent;
    color: white;
    font-size: 12px;
    cursor: pointer;
    padding: 0;
}

.win-btn:hover {
    background: rgba(255, 255, 255, 0.15);
}

.win-btn.close:hover {
    background: #e81123;
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
    padding: 8px 0 12px;
    flex-shrink: 0;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
    transition: width 0.2s ease;
}

.sidebar.collapsed {
    width: 56px;
}

.sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px 6px;
    min-height: 32px;
    flex-shrink: 0;
}

.sidebar.collapsed .sidebar-header {
    justify-content: center;
    padding: 0 0 6px;
}

.sidebar-title {
    font-size: 12px;
    font-weight: 600;
    color: #888;
    letter-spacing: 0.5px;
}

.sidebar-toggle {
    border: none;
    background: transparent;
    color: #666;
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 4px;
    font-size: 14px;
    line-height: 1;
}

.sidebar-toggle:hover {
    background: #e8e8e8;
    color: #2c3e50;
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
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 10px 20px;
    text-align: left;
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 14px;
    color: #333;
    transition: background 0.15s;
    white-space: nowrap;
}

.nav-icon {
    width: 20px;
    flex-shrink: 0;
    text-align: center;
    font-size: 16px;
    line-height: 1;
}

.nav-label {
    overflow: hidden;
    text-overflow: ellipsis;
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

.sidebar.collapsed .nav-item {
    justify-content: center;
    gap: 0;
    padding: 10px 0;
}

.sidebar.collapsed .nav-item.active {
    border-left: none;
    padding-left: 0;
    box-shadow: inset 3px 0 0 #2c3e50;
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