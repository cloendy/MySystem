<script setup lang="ts">
import { onMounted, ref } from 'vue';
import AppShell from './platform/shell/AppShell.vue';
import MobileShell from './platform/shell/MobileShell.vue';
import {
  detectPlatform,
  loadEnabledModules,
} from './platform/services/module-loader';
import { createAppRouter, registerModuleRoutes } from './platform/router';
import type { Platform } from './platform/services/module-types';
// import * as notepadApi from './modules/notepad/api';
// import * as readerApi from './modules/reader/api';

const ready = ref(false);
const platform = ref<Platform>('windows');

onMounted(async () => {
  platform.value = detectPlatform();
  // platform.value = 'android'; // 临时测试用
  console.log('[App] 当前平台:', platform.value);

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

  // ===== 临时测试代码，第 17 步会删除 =====
  // try {
  //   const note = await notepadApi.createNote({
  //     title: '骨架测试',
  //     content: '这是一条测试笔记',
  //   });
  //   console.log('[测试] 创建成功:', note);

  //   const list = await notepadApi.listNotes(null);
  //   console.log('[测试] 笔记列表:', list);

  //   await notepadApi.deleteNote(note.id);
  //   console.log('[测试] 删除成功');
  // } catch (err) {
  //   console.error('[测试] 失败:', err);
  // }
  // ===== 临时测试代码结束 =====

  // // ===== 临时测试代码 29=====

  // try {
  //   const book = await readerApi.createBook({
  //     title: '测试书籍',
  //     author: '测试作者',
  //     filePath: '/tmp/test.epub',
  //     fileFormat: 'epub',
  //   });
  //   console.log('[测试] 创建书籍:', book);

  //   const list = await readerApi.listBooks();
  //   console.log('[测试] 书籍列表:', list);

  //   await readerApi.deleteBook(book.id);
  //   console.log('[测试] 删除书籍成功');
  // } catch (err) {
  //   console.error('[测试] 失败:', err);
  // }
  // // ===== 临时测试代码结束 =====
});

</script>

<template>
  <AppShell v-if="ready && platform === 'windows'" />
  <MobileShell v-else-if="ready && platform === 'android'" />

  <div v-else style="padding: 20px;">
    <p>加载中...</p>
  </div>
</template>