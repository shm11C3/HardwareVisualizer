import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // npm scripts run from the repository root. Vite's absolute `/src/...`
    // globs (import.meta.glob) resolve against this root.
    root: import.meta.dirname,
    environment: "jsdom",
    setupFiles: ["src/setupVitest.ts"],
    include: [
      "src/**/*.{test,spec}.ts",
      "src/**/*.{test,spec}.tsx",
      // The e2e-native support tests live outside the Frontend root.
      "../../e2e-native/**/*.{test,spec}.ts",
    ],
    coverage: {
      // Components are measured so untested screens show up in the report.
      // Rendered lines count as covered without any assertion, so read the
      // .tsx numbers as "was this executed at all", not as "is this verified".
      include: ["src/**/*.{ts,tsx}"],
      exclude: [
        "src/rspc/**",
        "src/e2e/**",
        // App entry points only mount the root; they are exercised by E2E.
        "src/main.tsx",
        "src/main.e2e.tsx",
        // shadcn/ui primitives copied from the registry (see components.json).
        "src/components/ui/**",
        "src/**/*.test.*",
        "src/**/*.spec.*",
        "src/**/*.d.ts",
        "src/**/types/**",
      ],
      reportsDirectory: "coverage",
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
      "@": path.resolve(import.meta.dirname, "src"),
    },
  },
});
