import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    include: ["src/test/unit/**/*.test.ts", "src/test/unit/**/*.test.tsx"],
    coverage: {
      include: ["src/**/*.ts"], // [TODO] "src/**/*.tsx"
      exclude: ["src/rspc/**"],
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
