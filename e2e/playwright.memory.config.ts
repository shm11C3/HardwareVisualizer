import path from "node:path";
import { defineConfig, devices } from "@playwright/test";

const repoRoot = path.resolve(import.meta.dirname, "..");
const RENDER_MEMORY_PORT = 1523;

export default defineConfig({
  testDir: "./perf",
  testMatch: ["**/render-memory.spec.ts"],
  outputDir: "../test-results/render-memory/output",
  timeout: 150_000,
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI
    ? [
        ["list"],
        [
          "html",
          {
            outputFolder: "../test-results/render-memory/report",
            open: "never",
          },
        ],
      ]
    : [["list"]],
  use: {
    baseURL: `http://localhost:${RENDER_MEMORY_PORT}`,
    colorScheme: "dark",
    locale: "en-US",
    timezoneId: "UTC",
    contextOptions: { reducedMotion: "reduce" },
    screenshot: "only-on-failure",
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1280, height: 800 },
        deviceScaleFactor: 1,
      },
    },
  ],
  webServer: {
    // Vite resolves its config and the web server runs from the repository root.
    cwd: repoRoot,
    command: `vite dev --config hardviz/frontend/vite.config.ts --mode e2e --port ${RENDER_MEMORY_PORT} --strictPort`,
    url: `http://localhost:${RENDER_MEMORY_PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    env: { VITE_E2E_MOCK: "true" },
  },
});
