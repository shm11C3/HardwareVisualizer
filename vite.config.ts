import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
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
    react(),
    reactCompilerPreset(),
    tailwindcss(),
    reactDevTools(),
  ],

  build: {
    target: target,
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            { name: "vendor-react", test: /[\\/]node_modules[\\/](react|react-dom)[\\/]/ },
            { name: "vendor-i18n", test: /[\\/]node_modules[\\/](i18next|react-i18next)[\\/]/ },
            { name: "vendor-state", test: /[\\/]node_modules[\\/]jotai[\\/]/ },
            { name: "vendor-dnd", test: /[\\/]node_modules[\\/]@dnd-kit[\\/]/ },
            { name: "vendor-radix", test: /[\\/]node_modules[\\/]@radix-ui[\\/]/ },
            { name: "vendor-tauri", test: /[\\/]node_modules[\\/]@tauri-apps[\\/]/ },
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
