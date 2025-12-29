import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { visualizer } from "rollup-plugin-visualizer";
import { defineConfig, type PluginOption } from "vite";

const platform = process.env.TAURI_ENV_PLATFORM;

console.debug("platform : ", platform);

const target = (() => {
  if (platform === "windows") return "chrome139";
  if (platform === "darwin") return "safari16";
  return "es2020";
})();

const reactDevTools = (): PluginOption => {
  return {
    name: "react-devtools",
    apply: "serve",
    transformIndexHtml(html) {
      return {
        html,
        tags: [
          {
            tag: "script",
            attrs: {
              src: "http://localhost:8097",
            },
            injectTo: "head",
          },
        ],
      };
    },
  };
};

// https://vitejs.dev/config/
export default defineConfig(async ({ mode }) => ({
  plugins: [
    react({
      babel: {
        plugins: [["babel-plugin-react-compiler"]],
      },
    }),
    tailwindcss(),
    reactDevTools(),
  ],

  build: {
    target: target,
    rollupOptions: {
      output: {
        manualChunks: {
          "vendor-react": ["react", "react-dom"],
          "vendor-i18n": ["i18next", "react-i18next"],
          "vendor-state": ["jotai"],
          "vendor-dnd": ["@dnd-kit/core", "@dnd-kit/sortable"],
          "vendor-radix": [
            "@radix-ui/react-accordion",
            "@radix-ui/react-alert-dialog",
            "@radix-ui/react-checkbox",
            "@radix-ui/react-dialog",
            "@radix-ui/react-label",
            "@radix-ui/react-radio-group",
            "@radix-ui/react-scroll-area",
            "@radix-ui/react-select",
            "@radix-ui/react-slider",
            "@radix-ui/react-slot",
            "@radix-ui/react-switch",
            "@radix-ui/react-tabs",
            "@radix-ui/react-tooltip",
            "@radix-ui/react-visually-hidden",
          ],
          "vendor-icons": ["@phosphor-icons/react", "lucide-react"],
          "vendor-charts": ["recharts"],
          "vendor-tauri": [
            "@tauri-apps/api",
            "@tauri-apps/plugin-autostart",
            "@tauri-apps/plugin-clipboard-manager",
            "@tauri-apps/plugin-dialog",
            "@tauri-apps/plugin-os",
            "@tauri-apps/plugin-shell",
            "@tauri-apps/plugin-sql",
            "@tauri-apps/plugin-store",
            "@tauri-apps/plugin-updater",
          ],
        },
      },
      plugins: [
        // Enable `npx vite build --mode analyze`
        mode === "analyze" &&
          visualizer({
            open: true,
            gzipSize: true,
            brotliSize: true,
          }),
      ],
    },
  },

  optimizeDeps: {
    esbuildOptions: {
      target: target,
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1520,
    strictPort: true,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "src"),
    },
  },
}));
