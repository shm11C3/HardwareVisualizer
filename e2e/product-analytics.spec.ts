import { expect, type Page, test } from "@playwright/test";
import { capturePath, gotoApp, navigateTo } from "./helpers";

const UNDECIDED = "/?productAnalytics=undecided";
const dialog = (page: Page) =>
  page.getByRole("alertdialog", { name: "Share anonymous usage statistics?" });
const recordCount = (page: Page) =>
  page.evaluate(() => window.__E2E__?.getInvokeCount("record_product_event"));
const recordedEvents = (page: Page) =>
  page.evaluate(() => window.__E2E__?.getProductEvents());

test("the undecided dialog records nothing before a choice and declining keeps the app silent", async ({
  page,
}) => {
  await gotoApp(page, { path: UNDECIDED });
  await expect(dialog(page)).toBeVisible();
  // Settings are loaded and a screen is shown, yet no event call was made.
  expect(await recordCount(page)).toBe(0);

  await page.screenshot({
    path: capturePath("product-analytics-consent-desktop"),
    animations: "disabled",
  });
  await page.setViewportSize({ width: 800, height: 600 });
  await page.screenshot({
    path: capturePath("product-analytics-consent-compact"),
    animations: "disabled",
  });
  // The actions stay reachable when the window is compact.
  await expect(
    dialog(page).getByRole("button", { name: "Don't share" }),
  ).toBeInViewport();
  await expect(
    dialog(page).getByRole("button", { name: "Share", exact: true }),
  ).toBeInViewport();

  await dialog(page).getByRole("button", { name: "Don't share" }).click();
  await expect(dialog(page)).toHaveCount(0);

  await navigateTo(page, "systemSpecifications");
  await navigateTo(page, "insights");
  await navigateTo(page, "settings");
  expect(await recordCount(page)).toBe(0);
  await expect(
    page.getByRole("switch", { name: "Product analytics" }),
  ).not.toBeChecked();
  await expect(page.getByText("Not sharing")).toBeVisible();
});

test("sharing starts counting from the next navigation and Settings shows the state", async ({
  page,
}) => {
  await gotoApp(page, { path: UNDECIDED });
  await dialog(page)
    .getByRole("button", { name: "Share", exact: true })
    .click();
  await expect(dialog(page)).toHaveCount(0);
  // Granting consent does not record the screen that was already open.
  expect(await recordedEvents(page)).toEqual([]);

  await navigateTo(page, "systemSpecifications");
  await expect
    .poll(() => recordedEvents(page))
    .toContainEqual({ screenView: "systemSpecifications" });

  await navigateTo(page, "settings");
  await expect(
    page.getByRole("switch", { name: "Product analytics" }),
  ).toBeChecked();
  await expect(
    page.getByText("Sharing aggregate usage under contract v1"),
  ).toBeVisible();
});

test("a declined installation is never asked and can opt in from Settings", async ({
  page,
}) => {
  await gotoApp(page);
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
  expect(await recordCount(page)).toBe(0);

  await navigateTo(page, "settings");
  const toggle = page.getByRole("switch", { name: "Product analytics" });
  await expect(toggle).not.toBeChecked();
  await toggle.click();
  await expect(toggle).toBeChecked();

  await navigateTo(page, "systemSpecifications");
  await expect
    .poll(() => recordedEvents(page))
    .toContainEqual({ screenView: "systemSpecifications" });
});
