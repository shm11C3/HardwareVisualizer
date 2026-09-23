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

  test("elevation is refused outside Program Files and says why", async ({
    page,
  }) => {
    await gotoApp(page, { path: "/?elevation=unprotected" });

    const notice = page.getByRole("complementary", {
      name: "Run as administrator on startup was not applied",
    });
    await expect(notice).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });

    await navigateTo(page, "settings");
    const toggle = page.getByRole("switch", {
      name: "Run as administrator on startup",
    });
    await toggle.scrollIntoViewIfNeeded();
    await expect(toggle).toBeEnabled();
    await expect(page.getByText(/Saved as on, but not applied/)).toBeVisible();
    await expect(page.getByRole("button", { name: "Install" })).toBeDisabled();
    await page.waitForTimeout(400);

    await saveCapture(page, "settings-elevation-unprotected");

    await notice.getByRole("button", { name: "Dismiss" }).click();
    await expect(notice).toHaveCount(0);
  });

  test("grouped navigation is the default and Classic switches immediately", async ({
    page,
  }) => {
    await gotoApp(page);

    // Grouped navigation lists Performance and System Specifications as their
    // own destinations; the classic-only screens stay hidden.
    await expect(
      page.getByRole("button", { name: "Open Performance" }),
    ).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await expect(
      page.getByRole("button", { name: "Open System Specifications" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Open Dashboard" }),
    ).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Open Usage" })).toHaveCount(
      0,
    );
    await expect(page.getByRole("button", { name: "Open CPU" })).toHaveCount(0);

    await navigateTo(page, "settings");
    const classicNavigation = page.getByRole("switch", {
      name: "Classic navigation",
    });
    await classicNavigation.click();

    await expect(classicNavigation).toBeChecked();
    await expect(
      page.getByRole("button", { name: "Open Dashboard" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Open Usage" }),
    ).toBeVisible();
    await expect(page.getByRole("button", { name: "Open CPU" })).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Open System Specifications" }),
    ).toHaveCount(0);
  });

  test("navigation notice links to Settings and can be dismissed", async ({
    page,
  }) => {
    await gotoApp(page, { path: "/?showNavigationNotice=1" });

    const notice = page.getByRole("complementary", {
      name: "Navigation has been reorganized",
    });
    await expect(notice).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });

    await notice
      .getByRole("button", { name: "Open navigation settings" })
      .click();
    const classicNavigation = page.getByRole("switch", {
      name: "Classic navigation",
    });
    await expect(classicNavigation).toBeVisible();
    await expect(classicNavigation).toBeFocused();

    await notice
      .getByRole("button", { name: "Dismiss navigation notice" })
      .click();
    await expect(notice).toHaveCount(0);
    await expect
      .poll(() =>
        page.evaluate(
          () =>
            window.__E2E__?.getInvokeCount(
              "acknowledge_navigation_restructure_announcement",
            ) ?? 0,
        ),
      )
      .toBe(1);
  });
});
