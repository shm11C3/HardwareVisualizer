import { expect, test } from "@playwright/test";
import { gotoApp, navigateTo, seedHardwareHistory } from "../helpers";
import {
  collectRenderPerfResult,
  installRenderPerfObserver,
  type RenderPerfResult,
  resetRenderPerf,
  writeRenderPerfReport,
} from "./renderPerf";

const results: RenderPerfResult[] = [];
// The perf suite runs extra browser instrumentation, so cold-start bootstrap
// gets more room than BOOTSTRAP_TIMEOUT in the regular capture suite.
const PERF_BOOTSTRAP_TIMEOUT = 60_000;
const THRESHOLDS = {
  dashboardUpdates: {
    interactionMs: 2_000,
    domElementCount: 1_500,
    longTaskCount: 10,
    maxLongTaskMs: 500,
    totalLongTaskMs: 1_500,
  },
  usageChart: {
    interactionMs: 3_000,
    domElementCount: 1_000,
    longTaskCount: 10,
    maxLongTaskMs: 500,
    totalLongTaskMs: 1_000,
  },
  performanceDetailed: {
    interactionMs: 3_500,
    domElementCount: 2_000,
    longTaskCount: 12,
    maxLongTaskMs: 500,
    totalLongTaskMs: 1_750,
  },
};

const measureMs = async (action: () => Promise<void>) => {
  const start = Date.now();
  await action();
  return Date.now() - start;
};

test.describe.configure({ mode: "serial" });

test.describe("frontend render performance", () => {
  test.beforeEach(async ({ page }) => {
    await installRenderPerfObserver(page);
  });

  test.afterAll(async () => {
    await writeRenderPerfReport(results);
  });

  test("dashboard repeated metric updates stay bounded", async ({ page }) => {
    const readyMs = await measureMs(async () => {
      await gotoApp(page, {
        path: "/?navigationLayout=classic",
        timeout: PERF_BOOTSTRAP_TIMEOUT,
      });
    });

    await resetRenderPerf(page);
    const interactionMs = await measureMs(async () => {
      await page.evaluate(async () => {
        if (!window.__E2E__) {
          throw new Error(
            "window.__E2E__ is not installed (VITE_E2E_MOCK unset?)",
          );
        }
        await window.__E2E__.emitHardwareUpdateSeries(60);
      });
      await page.waitForTimeout(600);
    });

    const result = await collectRenderPerfResult(page, {
      screen: "dashboard-updates",
      readyMs,
      interactionMs,
    });
    results.push(result);

    expect.soft(result.readyMs).toBeLessThanOrEqual(PERF_BOOTSTRAP_TIMEOUT);
    expect
      .soft(result.interactionMs)
      .toBeLessThanOrEqual(THRESHOLDS.dashboardUpdates.interactionMs);
    expect
      .soft(result.domElementCount)
      .toBeLessThanOrEqual(THRESHOLDS.dashboardUpdates.domElementCount);
    expect
      .soft(result.longTaskCount)
      .toBeLessThanOrEqual(THRESHOLDS.dashboardUpdates.longTaskCount);
    expect
      .soft(result.maxLongTaskMs)
      .toBeLessThanOrEqual(THRESHOLDS.dashboardUpdates.maxLongTaskMs);
    expect
      .soft(result.totalLongTaskMs)
      .toBeLessThanOrEqual(THRESHOLDS.dashboardUpdates.totalLongTaskMs);
  });

  test("usage chart repeated metric updates stay bounded", async ({ page }) => {
    await gotoApp(page, {
      path: "/?navigationLayout=classic",
      timeout: PERF_BOOTSTRAP_TIMEOUT,
    });
    await seedHardwareHistory(page);
    await navigateTo(page, "usage");
    await expect(page.locator("span").filter({ hasText: /^RAM$/ })).toBeVisible(
      { timeout: PERF_BOOTSTRAP_TIMEOUT },
    );
    await expect(
      page.locator("span").filter({ hasText: /^GPU$/ }),
    ).toBeVisible();
    await resetRenderPerf(page);

    const interactionMs = await measureMs(async () => {
      await page.evaluate(async () => {
        if (!window.__E2E__) {
          throw new Error(
            "window.__E2E__ is not installed (VITE_E2E_MOCK unset?)",
          );
        }
        await window.__E2E__.emitHardwareUpdateSeries(60);
      });
      await page.waitForTimeout(600);
    });

    const result = await collectRenderPerfResult(page, {
      screen: "usage-chart",
      interactionMs,
    });
    results.push(result);

    expect
      .soft(result.interactionMs)
      .toBeLessThanOrEqual(THRESHOLDS.usageChart.interactionMs);
    expect
      .soft(result.domElementCount)
      .toBeLessThanOrEqual(THRESHOLDS.usageChart.domElementCount);
    expect
      .soft(result.longTaskCount)
      .toBeLessThanOrEqual(THRESHOLDS.usageChart.longTaskCount);
    expect
      .soft(result.maxLongTaskMs)
      .toBeLessThanOrEqual(THRESHOLDS.usageChart.maxLongTaskMs);
    expect
      .soft(result.totalLongTaskMs)
      .toBeLessThanOrEqual(THRESHOLDS.usageChart.totalLongTaskMs);
  });

  test("Detailed Performance updates stay bounded against the classic baselines", async ({
    page,
  }) => {
    const readyMs = await measureMs(async () => {
      await gotoApp(page, { timeout: PERF_BOOTSTRAP_TIMEOUT });
    });
    await seedHardwareHistory(page);
    await expect(page.getByTestId("performance-current-values")).toBeVisible();
    await expect(page.getByTestId("performance-usage-graphs")).toBeVisible();
    await resetRenderPerf(page);

    const interactionMs = await measureMs(async () => {
      await page.evaluate(async () => {
        if (!window.__E2E__) {
          throw new Error(
            "window.__E2E__ is not installed (VITE_E2E_MOCK unset?)",
          );
        }
        await window.__E2E__.emitHardwareUpdateSeries(60);
      });
      await page.waitForTimeout(600);
    });

    const result = await collectRenderPerfResult(page, {
      screen: "performance-detailed-updates",
      readyMs,
      interactionMs,
    });
    results.push(result);

    expect.soft(result.readyMs).toBeLessThanOrEqual(PERF_BOOTSTRAP_TIMEOUT);
    expect
      .soft(result.interactionMs)
      .toBeLessThanOrEqual(THRESHOLDS.performanceDetailed.interactionMs);
    expect
      .soft(result.domElementCount)
      .toBeLessThanOrEqual(THRESHOLDS.performanceDetailed.domElementCount);
    expect
      .soft(result.longTaskCount)
      .toBeLessThanOrEqual(THRESHOLDS.performanceDetailed.longTaskCount);
    expect
      .soft(result.maxLongTaskMs)
      .toBeLessThanOrEqual(THRESHOLDS.performanceDetailed.maxLongTaskMs);
    expect
      .soft(result.totalLongTaskMs)
      .toBeLessThanOrEqual(THRESHOLDS.performanceDetailed.totalLongTaskMs);
  });
});
