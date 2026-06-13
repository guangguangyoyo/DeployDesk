import { createApp } from "vue";
import { createPinia } from 'pinia'
import router from './router'
import "./assets/styles.css";
import App from "./App.vue";

const app = createApp(App);

app.use(createPinia());
app.use(router);

app.mount("#app");

async function applyTauriWindowIcon() {
  if (!("__TAURI_INTERNALS__" in window)) {
    return;
  }

  try {
    const [{ defaultWindowIcon }, { getCurrentWindow }] = await Promise.all([
      import("@tauri-apps/api/app"),
      import("@tauri-apps/api/window"),
    ]);
    const icon = await defaultWindowIcon();

    if (icon) {
      await getCurrentWindow().setIcon(icon);
    }
  } catch (error) {
    console.warn("Failed to apply Tauri window icon.", error);
  }
}

void applyTauriWindowIcon();
