<script setup lang="ts">
import SyncSettings from './settings/components/SyncSettings.vue';
import SyncLog from './settings/components/SyncLog.vue';
import LogViewer from './settings/components/LogViewer.vue';
</script>

<template>
    <div class="settings-view">
        <h1 class="page-title">设置</h1>

        <SyncSettings />
        <SyncLog />
        <LogViewer />

        <section class="settings-section">
            <h2 class="settings-section-title">关于</h2>
            <p class="about-line">MySystem v0.1.0</p>
            <p class="about-line">个人跨平台系统</p>
        </section>
    </div>
</template>

<style scoped>
.settings-view {
    padding: 24px;
    max-width: 640px;
    height: 100%;
    overflow-y: auto;
}

.page-title {
    font-size: 24px;
    font-weight: 600;
    margin-bottom: 24px;
}

.about-line {
    font-size: 14px;
    color: #333;
    line-height: 1.6;
}
</style>
