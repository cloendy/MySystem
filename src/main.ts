import { createApp } from "vue";
import { createPinia } from 'pinia';
import App from "./App.vue";
import { initDatabase } from './platform/services/database';
import { createAppRouter } from './platform/router';
import './style.css';
import './styles/common.css';

async function bootstrap() {
    try {
        await initDatabase();
    } catch (err) {
        console.error('[DB] Init failed:', err);
    }
    // createApp(App).mount("#app");
    const app = createApp(App);
    app.use(createPinia());
    app.use(createAppRouter());
    app.mount('#app');
}

bootstrap();