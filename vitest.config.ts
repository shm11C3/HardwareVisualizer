import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    setupFiles: ["src/test/setup.ts"],
    include: ["src/test/unit/**/*.test.ts", "src/test/unit/**/*.test.tsx"],
    coverage: {
      include: ["src/**/*.ts"],
      exclude: ["src/rspc/**", "src/test/**", "src/**/*.d.ts"],
      reporter: ["text", "html", "json-summary"],
      thresholds: {
        statements: 50,
        branches: 50,
        functions: 50,
        lines: 50,
      },
    },
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "src"),
    },
  },
});
