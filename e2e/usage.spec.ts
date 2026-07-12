import { expect, test } from "@playwright/test";
import { settingsFixture } from "../src/e2e/fixtures/settings";
import {
  BOOTSTRAP_TIMEOUT,
  gotoApp,
  navigateTo,
  saveCapture,
  seedHardwareHistory,
} from "./helpers";

test.describe("usage captures", () => {
  test("usage charts render seeded history", async ({ page }) => {
    await gotoApp(page, { path: "/?navigationLayout=classic" });
    await seedHardwareHistory(page);

    await navigateTo(page, "usage");

    // The mixed usage chart (SVG) renders with its CPU/RAM/GPU legend.
    await expect(page.locator("span").filter({ hasText: /^RAM$/ })).toBeVisible(
      { timeout: BOOTSTRAP_TIMEOUT },
    );
    await expect(
      page.locator("span").filter({ hasText: /^GPU$/ }),
    ).toBeVisible();

    // Guard against silently-broken series styling: compare the *computed*
    // stroke of each area curve against the fixture colors. The browser
    // parses the stroke attribute, so a malformed color (which once rendered
    // every series black while all visibility assertions still passed)
    // computes to rgb(0, 0, 0) and fails the comparison.
    const expectedStrokes = (["cpu", "memory", "gpu"] as const).map(
      (target) =>
        `rgb(${settingsFixture.lineGraphColor[target]
          .split(",")
          .map((component) => component.trim())
          .join(", ")})`,
    );
    const renderedStrokes = await page
      .locator("path.recharts-area-curve")
      .evaluateAll((elements) =>
        elements.map((element) => getComputedStyle(element).stroke),
      );
    expect(renderedStrokes.sort()).toEqual([...expectedStrokes].sort());
    await page.waitForTimeout(600);

    await saveCapture(page, "usage");
  });
});
