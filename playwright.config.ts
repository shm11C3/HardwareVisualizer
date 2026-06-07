import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright web/mock E2E harness (issue #1609).
 *
 * Runs the Vite/React frontend in a plain browser with Tauri IPC and event
 * mocks (see `src/e2e/mocks/installTauriMocks.ts`, enabled via
 * `VITE_E2E_MOCK=true`). Captures PNG evidence screenshots under
 * `test-results/captures/`.
 *
 * A dedicated port (1521) is used so a regular `npm run dev` server
 * (1520, without mocks) is never reused by accident.
 */
const E2E_PORT = 1521;

export default defineConfig({
  testDir: "./e2e",
  outputDir: "./test-results/output",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI
    ? [
        ["list"],
        ["html", { outputFolder: "test-results/report", open: "never" }],
      ]
    : [["list"]],
  use: {
    baseURL: `http://localhost:${E2E_PORT}`,
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
        // Locked viewport + scale factor for deterministic captures.
        viewport: { width: 1280, height: 800 },
        deviceScaleFactor: 1,
      },
    },
  ],
  webServer: {
    command: `vite dev --port ${E2E_PORT} --strictPort`,
    url: `http://localhost:${E2E_PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    env: { VITE_E2E_MOCK: "true" },
  },
});
