import { expect, test } from "@playwright/test";
import {
  BOOTSTRAP_TIMEOUT,
  capturePath,
  gotoApp,
  navigateTo,
  seedHardwareHistory,
} from "./helpers";

/**
 * Insights derives its archive query ranges from `Date.now()`, so the clock
 * is pinned to a fixed time. The mocked archive commands synthesize records
 * from the requested start/end range, making charts fully deterministic.
 */
const FIXED_TIME = new Date("2026-01-15T12:00:00Z");

test.describe("insights captures", () => {
  test.beforeEach(async ({ page }) => {
    await page.clock.setFixedTime(FIXED_TIME);
  });

  test("insights main chart renders archive fixtures", async ({ page }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);

    await navigateTo(page, "insights");

    await expect(page.getByRole("tab", { name: "CPU / Memory" })).toBeVisible({
      timeout: BOOTSTRAP_TIMEOUT,
    });
    // Wait for the debounced archive query (250ms) + chart render.
    await page.waitForTimeout(1_000);

    await page.screenshot({ path: capturePath("insights-main") });
  });

  test("insights process tab lists fixture process stats", async ({ page }) => {
    await gotoApp(page);
    await navigateTo(page, "insights");

    const processTab = page.getByRole("tab", { name: "Process" });
    await expect(processTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await processTab.click();

    await expect(page.getByText("hv-fixture-app").first()).toBeVisible();
    await page.waitForTimeout(600);

    await page.screenshot({ path: capturePath("insights-process") });
  });
});
