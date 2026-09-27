import { defineConfig, loadEnv, type Plugin } from "vite";
import vue from "@vitejs/plugin-vue";

// With VITE_FEATURE_HUBRISE=false (certification build) the webshop
// view is dead code, but Vite still emits the assets it imports (logos,
// display font) as soon as the module is transformed. Resolving the
// view to an empty stub keeps those files out of the bundle.
function dropWebshopView(): Plugin {
  const stubId = "\0pili-webshop-view-stub";
  return {
    name: "pili:drop-webshop-view",
    enforce: "pre",
    resolveId(source) {
      if (source.endsWith("/views/WebshopOrdersView.vue")) return stubId;
      return null;
    },
    load(id) {
      if (id === stubId) return "export default {};";
      return null;
    },
  };
}

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "VITE_");
  const hubriseEnabled = env.VITE_FEATURE_HUBRISE !== "false";

  return {
    plugins: [vue(), ...(hubriseEnabled ? [] : [dropWebshopView()])],
    clearScreen: false,
    server: {
      port: 1420,
      strictPort: true,
    },
    envPrefix: ["VITE_", "TAURI_"],
    build: {
      target: "esnext",
      minify: !process.env.TAURI_DEBUG ? "oxc" : false,
      sourcemap: !!process.env.TAURI_DEBUG,
    },
  };
});
