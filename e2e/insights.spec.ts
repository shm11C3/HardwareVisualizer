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
    await expect(page.getByTestId("cooling-temperature-lane")).toBeVisible();
    await expect(page.getByTestId("cooling-load-lane")).toBeVisible();
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

  test("insights cooling tab merges avg/max/min into one lane at 30 days", async ({
    page,
  }) => {
    await gotoApp(page);
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();

    await page.getByTestId("cooling-period-select").click();
    await page.getByRole("option", { name: "30 Days" }).click();

    const lane = page.getByTestId("cooling-thermal-timeline-lane");
    await expect(lane).toBeVisible();
    // The three separate CPU-temperature cards are gone: one lane now holds
    // the band and both lines.
    await expect(page.getByTestId("cooling-temperature-lane")).toHaveCount(1);
    await expect(page.getByTestId("cooling-load-lane")).toHaveCount(1);
    await expect(lane.getByText("Average")).toBeVisible();
    await expect(lane.getByText("Min-max")).toBeVisible();
    // The power charts stay available below the timeline.
    await expect(page.getByTestId("cooling-legacy-power-charts")).toBeVisible();

    await page.waitForTimeout(1_000);

    await saveCapture(page, "insights-cooling-timeline-30d");
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

  test("insights cooling tab shows a sustained mild rise observation", async ({
    page,
  }) => {
    await gotoApp(page, { path: "/?coolingObservation=sustainedMildRise" });
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();

    const strip = page.getByTestId("cooling-observation-strip");
    await expect(strip.getByText(/above baseline for 3 days/)).toBeVisible();
    await expect(page.getByTestId("cooling-load-band-dumbbell")).toBeVisible();

    // The confirmation checklist starts collapsed and expands on click,
    // revealing the observation-not-diagnosis footnote.
    const checklistTrigger = strip.getByText("Things worth checking");
    await expect(checklistTrigger).toBeVisible();
    await expect(strip.getByText("Case airflow")).not.toBeVisible();
    await checklistTrigger.click();
    await expect(strip.getByText("Case airflow")).toBeVisible();
    await expect(
      strip.getByText(
        "These are observation-based points to check, not a fault diagnosis.",
      ),
    ).toBeVisible();

    await page.waitForTimeout(600);

    await saveCapture(page, "insights-cooling-observation-mild");
  });

  test("insights cooling tab shows a not-comparable observation", async ({
    page,
  }) => {
    await gotoApp(page, { path: "/?coolingObservation=notComparable" });
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();

    const strip = page.getByTestId("cooling-observation-strip");
    await expect(
      strip.getByText("Not comparable — recent idle samples are insufficient."),
    ).toBeVisible();
    await expect(page.getByTestId("cooling-load-band-dumbbell")).toBeVisible();

    await page.waitForTimeout(600);

    await saveCapture(page, "insights-cooling-observation-not-comparable");
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

  test("insights cooling tab renders both daily lanes with the same gaps at 90 days", async ({
    page,
  }) => {
    await gotoApp(page);
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();

    await page.getByTestId("cooling-period-select").click();
    await page.getByRole("option", { name: "90 Days" }).click();

    const lane = page.getByTestId("cooling-thermal-timeline-lane");
    await expect(page.getByTestId("cooling-temperature-lane")).toBeVisible();
    await expect(page.getByTestId("cooling-load-lane")).toBeVisible();
    // The daily lane adds the idle series and the load-band composition the
    // archive buckets cannot produce.
    await expect(lane.getByText("Idle").first()).toBeVisible();
    await expect(lane.getByText("High")).toBeVisible();
    // The fixture skips every 13th day. Without `connectNulls` the average
    // line breaks at each skipped day, so its path has one move-to command
    // per recorded run instead of a single stroke across the gaps.
    const averageLine = lane.locator(".recharts-line-curve").first();
    const path = await averageLine.getAttribute("d");
    expect((path?.match(/M/g) ?? []).length).toBeGreaterThan(1);
    // No archive-backed power charts exist for the daily routes.
    await expect(page.getByTestId("cooling-legacy-power-charts")).toHaveCount(
      0,
    );

    await page.waitForTimeout(600);

    await saveCapture(page, "insights-cooling-timeline-90d");
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
