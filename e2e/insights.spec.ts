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
    await expect(page.getByTestId("cooling-power-lane")).toBeVisible();
    await expect(page.getByTestId("cooling-fan-lane")).toBeVisible();
    // Both lanes render now, so there is no pending sensor left to name and
    // the note disappears entirely rather than claiming one that arrived.
    await expect(page.getByTestId("cooling-sensor-status-note")).toHaveCount(0);
    await expect(page.getByTestId("cooling-load-band-panel")).toBeVisible();
    // Default period (24h) routes to the archive query, so no coverage strip.
    await expect(page.getByTestId("cooling-coverage-strip")).toHaveCount(0);

    // Wait for the debounced archive query (250ms) + chart render.
    await page.waitForTimeout(1_000);

    // No environmental sensor: the co-variate panel (#2068) has no Thermal
    // Delta to read the factors against and stays out of the layout.
    await expect(page.getByTestId("cooling-covariate-panel")).toHaveCount(0);

    await saveCapture(page, "insights-cooling");
  });

  test("insights cooling tab hides the power lane without a power source", async ({
    page,
  }) => {
    await gotoApp(page, { path: "/?coolingPower=none" });
    await seedHardwareHistory(page);
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();

    // The temperature and load lanes are unaffected: power is a separate
    // capability, so its absence must not degrade what does work.
    await expect(page.getByTestId("cooling-temperature-lane")).toBeVisible();
    await expect(page.getByTestId("cooling-load-lane")).toBeVisible();
    // No lane at all rather than one pinned at 0 W.
    await expect(page.getByTestId("cooling-power-lane")).toHaveCount(0);
    // The fan lane is unaffected, so the note names power alone.
    await expect(page.getByTestId("cooling-fan-lane")).toBeVisible();
    await expect(page.getByTestId("cooling-sensor-status-note")).toHaveText(
      /current hardware does not support power collection\./,
    );

    await page.waitForTimeout(1_000);

    await saveCapture(page, "insights-cooling-no-power");
  });

  test("insights cooling tab distinguishes supported power that is not collected", async ({
    page,
  }) => {
    await gotoApp(page, { path: "/?coolingPower=uncollected" });
    await seedHardwareHistory(page);
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();

    await expect(page.getByTestId("cooling-power-lane")).toHaveCount(0);
    await expect(page.getByTestId("cooling-sensor-status-note")).toHaveText(
      /Power data has not been collected for this period yet\./,
    );
  });

  test("insights cooling tab hides the fan lane without a readable fan", async ({
    page,
  }) => {
    await gotoApp(page, { path: "/?coolingFan=none" });
    await seedHardwareHistory(page);
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();

    // Fans are a separate capability: the lanes above keep working.
    await expect(page.getByTestId("cooling-temperature-lane")).toBeVisible();
    await expect(page.getByTestId("cooling-load-lane")).toBeVisible();
    await expect(page.getByTestId("cooling-power-lane")).toBeVisible();
    // No lane at all rather than one pinned at a fabricated 0 rpm, which
    // is a real Inactive Fan Reading.
    await expect(page.getByTestId("cooling-fan-lane")).toHaveCount(0);
    await expect(page.getByTestId("cooling-sensor-status-note")).toHaveText(
      /current hardware does not support fan speed collection\./,
    );

    await page.waitForTimeout(1_000);

    await saveCapture(page, "insights-cooling-no-fan");
  });

  test("insights cooling tab stays unchanged without an environmental sensor", async ({
    page,
  }) => {
    // The pixel-identical requirement of #2046, asserted structurally: the
    // default fixture machine has no ambient sensor, so nothing ambient may
    // mount anywhere in the view. The capture that proves the pixels is
    // "insights-cooling" above, which this guards against silent drift.
    await gotoApp(page);
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();

    await expect(page.getByTestId("cooling-temperature-lane")).toBeVisible();
    await expect(page.getByTestId("cooling-fan-lane")).toBeVisible();

    await expect(page.getByTestId("cooling-ambient-lane")).toHaveCount(0);
    await expect(
      page.getByTestId("cooling-ambient-adjusted-observation"),
    ).toHaveCount(0);
    await expect(
      page.getByTestId("cooling-load-band-dumbbell-ambient"),
    ).toHaveCount(0);
    await expect(
      page.getByTestId("cooling-data-state-ambient-source"),
    ).toHaveCount(0);
    await expect(
      page.getByTestId("cooling-data-state-ambient-coverage"),
    ).toHaveCount(0);
    // The single unlabeled comparison keeps its original shape too: the
    // window line only appears once a second, differently-windowed chart
    // sits beside it.
    await expect(page.getByTestId("cooling-load-band-dumbbell")).toBeVisible();
    await expect(
      page.getByTestId("cooling-load-band-panel").getByText(/vs\. recent/),
    ).toHaveCount(0);
  });

  test("insights cooling tab renders the ambient lane and adjusted comparison", async ({
    page,
  }) => {
    await gotoApp(page, { path: "/?coolingAmbient=present" });
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();

    // The fifth lane, below the fan lane, on the same shared axis.
    await expect(page.getByTestId("cooling-ambient-lane")).toBeVisible();
    await expect(page.getByTestId("cooling-power-lane")).toBeVisible();
    await expect(page.getByTestId("cooling-fan-lane")).toBeVisible();

    // The absolute observation still reports its +6.2 degC rise; the
    // ambient-adjusted line beside it reports the +0.3 degC that survives
    // normalization, which is the whole point of the analysis.
    const strip = page.getByTestId("cooling-observation-strip");
    await expect(strip.getByText(/\+6\.2°C/)).toBeVisible();
    const adjusted = page.getByTestId("cooling-ambient-adjusted-observation");
    await expect(adjusted.getByText(/\+0\.3°C/)).toBeVisible();
    // Its own window, not the absolute baseline's 2025-11-01–2025-11-14.
    await expect(adjusted.getByText(/2025-12-01/)).toBeVisible();

    // Both comparisons render, each labeled with the window it used.
    const panel = page.getByTestId("cooling-load-band-panel");
    await expect(page.getByTestId("cooling-load-band-dumbbell")).toBeVisible();
    await expect(
      page.getByTestId("cooling-load-band-dumbbell-ambient"),
    ).toBeVisible();
    await expect(panel.getByText(/2025-11-01.+2025-11-14/)).toBeVisible();
    await expect(panel.getByText(/2025-12-01.+2025-12-14/)).toBeVisible();
    // The mid band has ambient data but too thin a window, and the high
    // band never paired at all: both stay honestly not comparable.
    await expect(
      page
        .getByTestId("cooling-load-band-dumbbell-ambient")
        .getByText("Not enough samples to compare"),
    ).toHaveCount(2);

    await expect(
      page.getByTestId("cooling-data-state-ambient-source"),
    ).toContainText("Desk sensor");
    await expect(
      page.getByTestId("cooling-data-state-ambient-coverage"),
    ).toBeVisible();

    // The co-variate panel (#2068) below: the lead reads the ΔT change at
    // the baseline's median power, the fan that moved is tagged as such,
    // the fan neither window archived reads as a dash rather than 0 rpm,
    // and the fitted lines carry their slopes.
    const covariate = page.getByTestId("cooling-covariate-panel");
    await expect(covariate.getByTestId("cooling-covariate-lead")).toContainText(
      "+0.8°C",
    );
    await expect(
      covariate.getByTestId("cooling-covariate-row-fan").first(),
    ).toContainText("moved");
    await expect(
      covariate.getByTestId("cooling-covariate-row-fan").nth(1),
    ).toContainText("not archived");
    await expect(
      covariate.getByTestId("cooling-covariate-row-fan").nth(1),
    ).not.toContainText("0 rpm");
    await expect(
      covariate.getByTestId("cooling-covariate-chart"),
    ).toContainText("1.52 K/W");

    // Wait for the debounced archive query (250ms) + chart render.
    await page.waitForTimeout(1_000);

    await saveCapture(page, "insights-cooling-ambient");
  });

  test("insights cooling tab keeps an ambient-only window out of the empty state", async ({
    page,
  }) => {
    // The ambient archive is written independently of the hardware one, so
    // a window can hold room readings for minutes the machine recorded
    // nothing in. That is partial data, not an empty period (DP-02).
    //
    // Paired with an establishing baseline on purpose: an established one
    // supplies the temperature lane's reference band, and the domain that
    // band produces would keep the lane mounted whatever the archive said.
    // Without it the temperature domain is genuinely null, which is the
    // state that used to reach the empty message.
    await gotoApp(page, {
      path: "/?coolingAmbient=only&coolingBaseline=establishing",
    });
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();

    const lane = page.getByTestId("cooling-thermal-timeline-lane");
    await expect(
      lane.getByText("No data found for the selected period"),
    ).toHaveCount(0);
    // The temperature lane degrades to its notice, as it already does for a
    // window with CPU load and no temperature.
    await expect(
      page.getByTestId("cooling-temperature-lane-unavailable"),
    ).toBeVisible();
    await expect(page.getByTestId("cooling-temperature-lane")).toHaveCount(0);
    await expect(page.getByTestId("cooling-power-lane")).toHaveCount(0);
    await expect(page.getByTestId("cooling-fan-lane")).toHaveCount(0);
    // The reading that did arrive still renders.
    await expect(page.getByTestId("cooling-ambient-lane")).toBeVisible();
    // The co-variate panel has a sensor with a qualified baseline but not
    // one minute that paired a Thermal Delta with package power: it says
    // so, and claims no fit and no lead.
    const covariate = page.getByTestId("cooling-covariate-panel");
    await expect(
      covariate.getByTestId("cooling-covariate-not-comparable"),
    ).toBeVisible();
    await expect(covariate.getByTestId("cooling-covariate-lead")).toHaveCount(
      0,
    );
    await expect(covariate.getByTestId("cooling-covariate-chart")).toHaveCount(
      0,
    );

    await page.waitForTimeout(1_000);

    await saveCapture(page, "insights-cooling-ambient-only");
  });

  test("insights cooling tab drops the ambient lane on the long-range routes", async ({
    page,
  }) => {
    // The daily rollup carries the per-band thermal delta but no ambient
    // temperature series, so 90d has nothing to draw - and must not
    // inherit the short window's lane.
    await gotoApp(page, { path: "/?coolingAmbient=present" });
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();
    await expect(page.getByTestId("cooling-ambient-lane")).toBeVisible();

    await page.getByTestId("cooling-period-select").click();
    await page.getByRole("option", { name: "90 Days" }).click();

    await expect(page.getByTestId("cooling-coverage-strip")).toBeVisible();
    await expect(page.getByTestId("cooling-ambient-lane")).toHaveCount(0);
    // The comparison is not routed, so its ambient variant stays.
    await expect(
      page.getByTestId("cooling-load-band-dumbbell-ambient"),
    ).toBeVisible();
    // But no source may be named for a window that carries none.
    await expect(
      page.getByTestId("cooling-data-state-ambient-source"),
    ).toHaveCount(0);
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
    await expect(lane.getByText("Average").first()).toBeVisible();
    await expect(lane.getByText("Min-max").first()).toBeVisible();
    // The separate power charts are gone: package power is now a lane on
    // the same synchronized axis (#2021).
    await expect(page.getByTestId("cooling-power-lane")).toHaveCount(1);
    await expect(lane.getByText("CPU package power (W)")).toBeVisible();
    // And the fan lane joins the same axis below it (#2022).
    await expect(page.getByTestId("cooling-fan-lane")).toHaveCount(1);
    await expect(lane.getByText("Fan speed (rpm)")).toBeVisible();

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
    // The daily rollup now carries power too, so the lane is present on
    // the long-range routes as well as the archive-backed ones.
    await expect(page.getByTestId("cooling-power-lane")).toBeVisible();

    await page.waitForTimeout(600);

    await saveCapture(page, "insights-cooling-timeline-90d");
  });

  test("insights cooling tab keeps the load-temperature explorer collapsed until opened", async ({
    page,
  }) => {
    await gotoApp(page);
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();

    const panel = page.getByTestId("cooling-explorer-panel");
    await expect(panel).toBeVisible();
    // Collapsed by default: the scatter is not rendered, and - the point
    // of collapsing a secondary analysis - no query has been issued.
    await expect(page.getByTestId("cooling-explorer-scatter")).toHaveCount(0);
    expect(
      await page.evaluate(() =>
        window.__E2E__?.getInvokeCount("get_cooling_load_temperature_explorer"),
      ),
    ).toBe(0);

    await page.getByTestId("cooling-explorer-trigger").click();

    await expect(page.getByTestId("cooling-explorer-scatter")).toBeVisible();
    await expect(page.getByTestId("cooling-explorer-minimap")).toBeVisible();
    // Both windows scatter, plus one median trend line each.
    await expect(panel.locator(".recharts-scatter").first()).toBeVisible();
    // Core reported the high band as not comparable; the row says so
    // instead of showing a delta.
    const deltas = page.getByTestId("cooling-explorer-deltas");
    await expect(deltas.getByText(/Not comparable/)).toBeVisible();
    expect(
      await page.evaluate(() =>
        window.__E2E__?.getInvokeCount("get_cooling_load_temperature_explorer"),
      ),
    ).toBe(1);

    await page.waitForTimeout(600);

    await saveCapture(page, "insights-cooling-explorer");
  });

  test("insights cooling tab refetches the explorer when the recent window changes", async ({
    page,
  }) => {
    await gotoApp(page);
    await navigateTo(page, "insights");

    const coolingTab = page.getByRole("tab", { name: "Cooling" });
    await expect(coolingTab).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await coolingTab.click();

    // Switch to a daily period first, matching the 90d capture above; the
    // Explorer's window selector then sits well inside the viewport.
    await page.getByTestId("cooling-period-select").click();
    await page.getByRole("option", { name: "90 Days" }).click();
    await expect(page.getByTestId("cooling-coverage-strip")).toBeVisible();

    await page.getByTestId("cooling-explorer-trigger").click();
    await expect(page.getByTestId("cooling-explorer-scatter")).toBeVisible();

    const windowSelect = page.getByTestId("cooling-explorer-window-select");
    await expect(windowSelect).toHaveText("Last 28 days");
    await windowSelect.scrollIntoViewIfNeeded();
    await windowSelect.click();
    await page.getByRole("option", { name: "Last 90 days" }).click();

    await expect(windowSelect).toHaveText("Last 90 days");
    await expect(page.getByTestId("cooling-explorer-scatter")).toBeVisible();
    expect(
      await page.evaluate(() =>
        window.__E2E__?.getInvokeCount("get_cooling_load_temperature_explorer"),
      ),
    ).toBe(2);
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
