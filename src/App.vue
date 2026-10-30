<script setup lang="ts">

import { logger } from './platform/services/logger'
import { onMounted, ref } from 'vue';
import AppShell from './platform/shell/AppShell.vue';
import MobileShell from './platform/shell/MobileShell.vue';
import ChoiceDialog from './platform/components/ChoiceDialog.vue';
import {
  detectPlatform,
  loadEnabledModules,
} from './platform/services/module-loader';
import { createAppRouter, registerModuleRoutes } from './platform/router';
import type { Platform } from './platform/services/module-types';

const ready = ref(false);
const platform = ref<Platform>('windows');

onMounted(async () => {
  platform.value = detectPlatform();
  logger.info('[App] 当前平台:', platform.value);

  createAppRouter();

  const modules = await loadEnabledModules(platform.value);
  registerModuleRoutes(modules);

  // 启动后默认跳到第一个模块
  if (modules.length > 0) {
    const first = modules[0];
    const router = (await import('./platform/router')).getRouter();
    router.push(`/${first.id}/`);
  }

  ready.value = true;
});

</script>

<template>
  <AppShell v-if="ready && platform === 'windows'" />
  <MobileShell v-else-if="ready && platform === 'android'" />

  <div v-else style="padding: 20px;">
    <p>加载中...</p>
  </div>

  <!-- 全局三选一对话框 -->
  <ChoiceDialog />
</template>