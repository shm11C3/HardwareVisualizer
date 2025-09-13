import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    setupFiles: ["src/test/setup.ts"],
    include: ["src/test/unit/**/*.test.ts", "src/test/unit/**/*.test.tsx"],
    coverage: {
      include: ["src/**/*.ts"],
      exclude: [
        "src/rspc/**",
        "src/test/**",
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
