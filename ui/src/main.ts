import { createApp } from "vue";
import { createPinia } from "pinia";
import router from "./router";
import App from "./App.vue";
// Fonts bundled with the app (the kiosk is offline). Latin subsets
// only: Chinese comes from Noto Sans CJK installed in the image.
import "@fontsource/noto-sans/latin-400.css";
import "@fontsource/noto-sans/latin-ext-400.css";
import "@fontsource/noto-sans/latin-500.css";
import "@fontsource/noto-sans/latin-ext-500.css";
import "@fontsource/noto-sans/latin-700.css";
import "@fontsource/noto-sans/latin-ext-700.css";
import "@fontsource/noto-sans/latin-800.css";
import "@fontsource/noto-sans/latin-ext-800.css";
import "@fontsource/jetbrains-mono/latin-400.css";
import "@fontsource/jetbrains-mono/latin-500.css";
import "@fontsource/jetbrains-mono/latin-700.css";
import "@fontsource/archivo/latin-900.css";

const app = createApp(App);
app.use(createPinia());
app.use(router);
app.mount("#app");
