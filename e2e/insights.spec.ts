import { expect, test } from "@playwright/test";
import {
  BOOTSTRAP_TIMEOUT,
  gotoApp,
  navigateTo,
  saveCapture,
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

    await saveCapture(page, "insights-main");
  });

  test("insights cooling tab renders the new layout with default period", async ({
    page,
  }) => {
    await gotoApp(page);
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();

    await expect(
      page.getByTestId("cooling-thermal-timeline-lane"),
    ).toBeVisible();
    await expect(
      page.getByTestId("cooling-unsupported-sensor-note"),
    ).toBeVisible();
    await expect(page.getByTestId("cooling-load-band-panel")).toBeVisible();
    // Default period (24h) routes to the archive query, so no coverage strip.
    await expect(page.getByTestId("cooling-coverage-strip")).toHaveCount(0);

    // Wait for the debounced archive query (250ms) + chart render.
    await page.waitForTimeout(1_000);

    await saveCapture(page, "insights-cooling");
  });

  test("insights cooling tab shows the establishing baseline empty state", async ({
    page,
  }) => {
    await gotoApp(page, { path: "/?coolingBaseline=establishing" });
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();

    await expect(
      page.getByTestId("cooling-observation-strip").getByText("4 / 7"),
    ).toBeVisible();

    await page.waitForTimeout(600);

    await saveCapture(page, "insights-cooling-establishing");
  });

  test("insights cooling tab renders the coverage strip at 90 days", async ({
    page,
  }) => {
    await gotoApp(page);
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();

    await page.getByTestId("cooling-period-select").click();
    await page.getByRole("option", { name: "90 Days" }).click();

    await expect(page.getByTestId("cooling-coverage-strip")).toBeVisible();
    await page.waitForTimeout(600);

    await saveCapture(page, "insights-cooling-coverage-90d");
  });

  test("insights process tab lists fixture process stats", async ({ page }) => {
    await gotoApp(page);
    await navigateTo(page, "insights");

    const processTab = page.getByRole("tab", { name: "Process" });
    await expect(processTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await processTab.click();

    await expect(page.getByText("hv-fixture-app").first()).toBeVisible();
    await page.waitForTimeout(600);

    await saveCapture(page, "insights-process");
  });
});
