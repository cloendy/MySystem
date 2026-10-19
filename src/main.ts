import { createApp } from "vue";
import { createPinia } from 'pinia';
import App from "./App.vue";
import { createAppRouter } from './platform/router';
import { installUiLogging } from './platform/services/ui-logging';
import './style.css';
import './styles/common.css';

function bootstrap() {
    const app = createApp(App);
    app.use(createPinia());
    app.use(createAppRouter());
    installUiLogging();
    app.mount('#app');
}

bootstrap();