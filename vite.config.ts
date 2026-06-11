import path from "node:path";
import babel from "@rolldown/plugin-babel";
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

const e2eEntry = (): PluginOption => {
  return {
    name: "hardware-visualizer-e2e-entry",
    apply: "serve",
    transformIndexHtml(html) {
      return html.replace("/src/main.tsx", "/src/main.e2e.tsx");
    },
  };
};

// https://vitejs.dev/config/
export default defineConfig(async ({ mode }) => ({
  plugins: [
    react(),
    babel({ presets: [reactCompilerPreset()] }),
    tailwindcss(),
    mode === "react-devtools" && reactDevTools(),
    mode === "e2e" && e2eEntry(),
  ],

  build: {
    target: target,
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
              name: "vendor-chart",
              test: /[\\/]node_modules[\\/](recharts|d3-[^\\/]+|victory-vendor|es-toolkit)[\\/]/,
            },
            {
              name: "vendor-react",
              test: /[\\/]node_modules[\\/](react|react-dom)[\\/]/,
            },
            {
              name: "vendor-i18n",
              test: /[\\/]node_modules[\\/](i18next|react-i18next)[\\/]/,
            },
            { name: "vendor-state", test: /[\\/]node_modules[\\/]jotai[\\/]/ },
            { name: "vendor-dnd", test: /[\\/]node_modules[\\/]@dnd-kit[\\/]/ },
            {
              name: "vendor-radix",
              test: /[\\/]node_modules[\\/]@radix-ui[\\/]/,
            },
            {
              name: "vendor-tauri",
              test: /[\\/]node_modules[\\/]@tauri-apps[\\/]/,
            },
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
  optimizeDeps: {
    // Pre-bundle the Tauri mock module only in E2E mode so the regular app
    // entry path stays independent from the web/mock harness.
    include: mode === "e2e" ? ["@tauri-apps/api/mocks"] : [],
  },
}));
