import path from "node:path";
import { expect, type Page } from "@playwright/test";
import { sysInfoFixture } from "../src/e2e/fixtures/hardware";
// Type-only import: pulls in the `window.__E2E__` global declaration without
// bundling app code into the test runner.
import type {} from "../src/e2e/mocks/installTauriMocks";

/**
 * Evidence captures are written to a predictable directory so CI can upload
 * them as artifacts (`test-results/captures/<name>.png`).
 */
export const capturePath = (name: string) =>
  path.join("test-results", "captures", `${name}.png`);

/**
 * Save a full-page evidence capture (the whole scrollable page, not just the
 * viewport) under `test-results/captures/<name>.png`.
 */
export const saveCapture = (page: Page, name: string) =>
  page.screenshot({ path: capturePath(name), fullPage: true });

/**
 * The first page load pays Vite's cold module-transform cost
 * (babel + react-compiler), which can take >10s — hence the long timeout.
 */
export const BOOTSTRAP_TIMEOUT = 30_000;

const CPU_NAME = sysInfoFixture.cpu?.name ?? "";

/**
 * Load the app and wait until the dashboard rendered fixture data,
 * i.e. settings/mock bootstrap completed.
 */
export const gotoApp = async (page: Page) => {
  await page.goto("/");
  await expect(page.getByText(CPU_NAME).first()).toBeVisible({
    timeout: BOOTSTRAP_TIMEOUT,
  });
};

/**
 * Push a deterministic series of hardware-monitor-update events so charts
 * render a stable history, then wait for the UI to settle.
 */
export const seedHardwareHistory = async (page: Page) => {
  await page.evaluate(async () => {
    await window.__E2E__?.emitHardwareUpdateSeries(30);
  });
  // Allow chart re-render/animation to settle before capturing.
  await page.waitForTimeout(600);
};

/**
 * Navigate via the closed side menu's accessible buttons
 * (`aria-label="open <type>"`, see SideMenu.tsx).
 */
export const navigateTo = async (
  page: Page,
  type: "dashboard" | "usage" | "cpuDetail" | "insights" | "settings",
) => {
  await page.getByRole("button", { name: `open ${type}` }).click();
};
