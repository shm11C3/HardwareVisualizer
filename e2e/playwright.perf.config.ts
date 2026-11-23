import path from "node:path";
import { defineConfig, devices } from "@playwright/test";

const repoRoot = path.resolve(import.meta.dirname, "..");

/**
 * Lightweight frontend render performance checks.
 *
 * This reuses the web/mock E2E mode, but keeps results separate from the
 * evidence capture suite so it can start as a non-blocking PR signal.
 */
const RENDER_PERF_PORT = 1522;

export default defineConfig({
  testDir: "./perf",
  testMatch: ["**/render-perf.spec.ts"],
  outputDir: "../test-results/render-perf/output",
  timeout: 90_000,
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI
    ? [
        ["list"],
        [
          "html",
          { outputFolder: "../test-results/render-perf/report", open: "never" },
        ],
      ]
    : [["list"]],
  use: {
    baseURL: `http://localhost:${RENDER_PERF_PORT}`,
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
    command: `vite dev --config hardviz/frontend/vite.config.ts --mode e2e --port ${RENDER_PERF_PORT} --strictPort`,
    url: `http://localhost:${RENDER_PERF_PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    env: { VITE_E2E_MOCK: "true" },
  },
});
