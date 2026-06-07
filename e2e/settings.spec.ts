import { expect, test } from "@playwright/test";
import { BOOTSTRAP_TIMEOUT, gotoApp, navigateTo, saveCapture } from "./helpers";

test.describe("settings captures", () => {
  test("settings sections render", async ({ page }) => {
    await gotoApp(page);

    await navigateTo(page, "settings");

    // Section headings come from i18n (fixture language is "en").
    await expect(page.getByRole("heading", { name: "General" })).toBeVisible({
      timeout: BOOTSTRAP_TIMEOUT,
    });
    await expect(page.getByRole("heading", { name: "About" })).toBeVisible();
    await page.waitForTimeout(600);

    await saveCapture(page, "settings");
  });
});
