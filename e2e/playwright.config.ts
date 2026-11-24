import path from "node:path";
import { defineConfig, devices } from "@playwright/test";

const repoRoot = path.resolve(import.meta.dirname, "..");

/**
 * Playwright web/mock E2E harness (issue #1609).
 *
 * Runs the Vite/React frontend in a plain browser with Tauri IPC and event
 * mocks (see `hardviz/frontend/src/main.e2e.tsx`, enabled via `vite --mode e2e`). Captures PNG
 * evidence screenshots under `test-results/captures/`.
 *
 * A dedicated port (1521) is used so a regular `npm run dev` server
 * (1520, without mocks) is never reused by accident.
 */
const E2E_PORT = 1521;

export default defineConfig({
  testDir: ".",
  testIgnore: ["**/perf/**"],
  outputDir: "../test-results/output",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI
    ? [
        ["list"],
        ["html", { outputFolder: "../test-results/report", open: "never" }],
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
    // Vite resolves its config and the web server runs from the repository root.
    cwd: repoRoot,
    command: `vite dev --config hardviz/frontend/vite.config.ts --mode e2e --port ${E2E_PORT} --strictPort`,
    url: `http://localhost:${E2E_PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    env: { VITE_E2E_MOCK: "true" },
  },
});
