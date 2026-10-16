<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { getLoadedModules } from '../services/module-loader';
import type { ModuleInterface } from '../services/module-types';

const modules = ref<ModuleInterface[]>([]);

onMounted(() => {
    modules.value = getLoadedModules();
});
</script>

<template>
    <div style="padding: 20px;">
        <h1>MySystem</h1>
        <p>已加载 {{ modules.length }} 个模块：</p>
        <ul>
            <li v-for="m in modules" :key="m.id">
                {{ m.name }} v{{ m.version }}
            </li>
        </ul>
    </div>
</template>