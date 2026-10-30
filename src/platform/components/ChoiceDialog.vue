<script setup lang="ts">
import { choiceState, resolveChoice } from '../services/dialog';
</script>

<template>
    <div v-if="choiceState.visible" class="choice-overlay" @click.self="resolveChoice('cancel')">
        <div class="choice-card">
            <div class="choice-title">{{ choiceState.title }}</div>
            <div class="choice-message">{{ choiceState.message }}</div>
            <div class="choice-actions">
                <button class="btn btn-primary" @click="resolveChoice('first')">
                    {{ choiceState.first }}
                </button>
                <button class="btn" @click="resolveChoice('second')">
                    {{ choiceState.second }}
                </button>
                <button class="btn btn-ghost" @click="resolveChoice('cancel')">
                    {{ choiceState.cancelText }}
                </button>
            </div>
        </div>
    </div>
</template>

<style scoped>
.choice-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
}

.choice-card {
    min-width: 320px;
    max-width: 460px;
    background: #fff;
    border-radius: 8px;
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.2);
    padding: 20px;
}

.choice-title {
    font-size: 15px;
    font-weight: 600;
    color: #2c3e50;
    margin-bottom: 10px;
}

.choice-message {
    font-size: 13px;
    color: #555;
    line-height: 1.7;
    word-break: break-word;
    margin-bottom: 18px;
}

.choice-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
}
</style>
