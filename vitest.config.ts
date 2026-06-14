import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    setupFiles: ["src/setupVitest.ts"],
    include: [
      "src/**/*.{test,spec}.ts",
      "src/**/*.{test,spec}.tsx",
      "e2e-native/**/*.{test,spec}.ts",
    ],
    coverage: {
      include: ["src/**/*.ts"],
      exclude: [
        "src/rspc/**",
        "src/e2e/**",
        "src/**/*.test.*",
        "src/**/*.spec.*",
        "src/**/*.d.ts",
        "src/**/types/**",
      ],
      reporter: ["text", "html", "json-summary"],
      thresholds: {
        statements: 60,
        branches: 60,
        functions: 60,
        lines: 60,
      },
    },
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "src"),
    },
  },
});
