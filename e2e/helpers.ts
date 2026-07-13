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
type GotoAppOptions = {
  path?: string;
  timeout?: number;
};

/**
 * Load the app and wait until the selected default screen rendered,
 * i.e. settings/mock bootstrap completed.
 */
export const gotoApp = async (page: Page, options: GotoAppOptions = {}) => {
  const path = options.path ?? "/";
  await page.goto(path);
  const isClassic =
    new URL(path, "http://localhost").searchParams.get("navigationLayout") ===
    "classic";

  if (isClassic) {
    await expect(page.getByText(CPU_NAME).first()).toBeVisible({
      timeout: options.timeout ?? BOOTSTRAP_TIMEOUT,
    });
    return;
  }

  await expect(page.getByTestId("performance-screen")).toBeVisible({
    timeout: options.timeout ?? BOOTSTRAP_TIMEOUT,
  });
};

/**
 * Push a deterministic series of hardware-monitor-update events so charts
 * render a stable history, then wait for the UI to settle.
 */
export const seedHardwareHistory = async (page: Page) => {
  // Fail loudly if the E2E mock bridge is missing rather than silently
  // skipping the seed (which would surface later as an empty-chart capture).
  await page.evaluate(async () => {
    if (!window.__E2E__) {
      throw new Error("window.__E2E__ is not installed (VITE_E2E_MOCK unset?)");
    }
    await window.__E2E__.emitHardwareUpdateSeries(30);
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
  type:
    | "dashboard"
    | "performance"
    | "hardwareCpu"
    | "hardwareGpu"
    | "hardwareMemory"
    | "hardwareStorage"
    | "hardwareSystem"
    | "usage"
    | "cpuDetail"
    | "insights"
    | "settings",
) => {
  const target = page.getByRole("button", { name: `open ${type}` });
  await expect(page.locator('[data-settings-loaded="true"]')).toBeAttached({
    timeout: BOOTSTRAP_TIMEOUT,
  });
  await expect(page.getByRole("button", { name: "open settings" })).toBeVisible(
    { timeout: BOOTSTRAP_TIMEOUT },
  );

  if (type === "usage" || type === "cpuDetail" || type === "dashboard") {
    if ((await target.count()) === 0) {
      await page.getByRole("button", { name: "open settings" }).click();
      const classicNavigation = page.getByRole("switch", {
        name: "Classic navigation",
      });
      await expect(classicNavigation).toBeVisible({
        timeout: BOOTSTRAP_TIMEOUT,
      });
      if (!(await classicNavigation.isChecked())) {
        await classicNavigation.click();
      }
    }
  } else if (type === "performance" && (await target.count()) === 0) {
    await page.getByRole("button", { name: "open settings" }).click();
    const classicNavigation = page.getByRole("switch", {
      name: "Classic navigation",
    });
    await expect(classicNavigation).toBeVisible({
      timeout: BOOTSTRAP_TIMEOUT,
    });
    if (await classicNavigation.isChecked()) {
      await classicNavigation.click();
    }
  }

  await expect(target).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
  await target.click();
};
