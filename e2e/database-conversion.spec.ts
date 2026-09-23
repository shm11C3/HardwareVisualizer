import { expect, test } from "@playwright/test";
import { BOOTSTRAP_TIMEOUT, gotoApp, navigateTo, saveCapture } from "./helpers";

/**
 * Renders the #2136 native database conversion section
 * (`DatabaseConversionSettings`, mounted in the Settings screen's Insights
 * block) at every lifecycle state the mock's `?databaseConversion=`
 * scenario selects - see `installTauriMocks.ts` for what each scenario
 * does. Captured at the suite's default desktop viewport (1280x800) and
 * the narrowest viewport already used elsewhere in this harness
 * (520x800, matching `performance.spec.ts`'s compact capture).
 */
const NARROW_VIEWPORT = { width: 520, height: 800 };

const gotoSettings = async (
  page: import("@playwright/test").Page,
  path: string,
) => {
  await gotoApp(page, { path });
  await navigateTo(page, "settings");
};

test.describe("database conversion captures", () => {
  test("hidden when this build does not support conversion", async ({
    page,
  }) => {
    await gotoSettings(page, "/");

    await expect(
      page.getByRole("heading", { name: "Native Database" }),
    ).toHaveCount(0);

    await saveCapture(page, "database-conversion-not-supported");
  });

  test("offers Convert Now while SQLite is authoritative", async ({ page }) => {
    await gotoSettings(page, "/?databaseConversion=sqliteAuthoritative");

    const heading = page.getByRole("heading", { name: "Native Database" });
    await expect(heading).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    const convertButton = page.getByRole("button", { name: "Convert Now" });
    await expect(convertButton).toBeVisible();
    await saveCapture(page, "database-conversion-sqlite-authoritative");

    await page.setViewportSize(NARROW_VIEWPORT);
    await expect(convertButton).toBeVisible();
    const overflow = await heading.evaluate((el) => {
      const section = el.closest("div");
      return {
        scrollWidth: section?.scrollWidth ?? 0,
        clientWidth: document.documentElement.clientWidth,
      };
    });
    expect(overflow.scrollWidth).toBeLessThanOrEqual(overflow.clientWidth);
    await saveCapture(page, "database-conversion-sqlite-authoritative-narrow");
  });

  test("starting the conversion shows Converting progress", async ({
    page,
  }) => {
    await gotoSettings(page, "/?databaseConversion=sqliteAuthoritative");

    await page.getByRole("button", { name: "Convert Now" }).click();

    await expect(page.getByText(/Converting…/)).toBeVisible({
      timeout: BOOTSTRAP_TIMEOUT,
    });
    await expect(page.getByRole("button", { name: "Cancel" })).toBeVisible();
    await expect
      .poll(() =>
        page.evaluate(
          () =>
            window.__E2E__?.getInvokeCount("start_database_conversion") ?? 0,
        ),
      )
      .toBe(1);
  });

  test("shows step progress and Cancel while a fixed conversion is running", async ({
    page,
  }) => {
    await gotoSettings(page, "/?databaseConversion=converting");

    await expect(page.getByText(/Converting…/)).toBeVisible({
      timeout: BOOTSTRAP_TIMEOUT,
    });
    const cancelButton = page.getByRole("button", { name: "Cancel" });
    await expect(cancelButton).toBeVisible();
    await saveCapture(page, "database-conversion-converting");

    await page.setViewportSize(NARROW_VIEWPORT);
    await expect(cancelButton).toBeVisible();
    await saveCapture(page, "database-conversion-converting-narrow");

    await cancelButton.click();
    await expect
      .poll(() =>
        page.evaluate(
          () =>
            window.__E2E__?.getInvokeCount("cancel_database_conversion") ?? 0,
        ),
      )
      .toBe(1);
  });

  test("shows an actionable message and technical details on failure", async ({
    page,
  }) => {
    await gotoSettings(page, "/?databaseConversion=actionRequired");

    await expect(
      page.getByText(
        "The conversion failed. Your existing data is unaffected and you can try again.",
      ),
    ).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });

    const details = page.getByText("Technical details");
    await expect(details).toBeVisible();
    // Collapsed by default - the diagnostic is reachable, not shown by
    // default in the normal flow.
    await expect(
      page.getByText("ConversionFailed { step: Reconciling", { exact: false }),
    ).not.toBeVisible();
    await saveCapture(page, "database-conversion-action-required-collapsed");

    await details.click();
    await expect(
      page.getByText("ConversionFailed { step: Reconciling", { exact: false }),
    ).toBeVisible();
    await saveCapture(page, "database-conversion-action-required-expanded");

    await page.setViewportSize(NARROW_VIEWPORT);
    await expect(details).toBeVisible();
    await saveCapture(page, "database-conversion-action-required-narrow");
  });

  test("shows the one-time completion notice after a successful conversion", async ({
    page,
  }) => {
    await gotoSettings(page, "/?databaseConversion=justCompleted");

    // The hook's own converting -> nativeAuthoritative transition (not a
    // fixed initial state) is what triggers the notice - see the mock's
    // `justCompleted` scenario.
    await expect(
      page.getByText("The database has been converted to the native format."),
    ).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });

    const noticeTitle = page.getByText(
      "More history now fits in the same space",
    );
    await expect(noticeTitle).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Set to 1 Year" }),
    ).toBeVisible();
    const keepButton = page.getByRole("button", {
      name: "Keep Current Setting",
    });
    await expect(keepButton).toBeVisible();
    await saveCapture(page, "database-conversion-complete-notice");

    await page.setViewportSize(NARROW_VIEWPORT);
    await expect(noticeTitle).toBeVisible();
    await saveCapture(page, "database-conversion-complete-notice-narrow");

    await keepButton.click();
    await expect(noticeTitle).toHaveCount(0);
  });
});

test.describe("database conversion discovery card (Insights page)", () => {
  test("offers discovery and navigates to Settings when SQLite is authoritative", async ({
    page,
  }) => {
    await gotoApp(page, {
      path: "/?databaseConversion=sqliteAuthoritative",
    });
    await navigateTo(page, "insights");

    const card = page.getByTestId("database-conversion-discovery-card");
    await expect(card).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await expect(
      card.getByText("A more efficient database is available"),
    ).toBeVisible();
    await saveCapture(page, "database-conversion-discovery-card-insights");

    await page.setViewportSize(NARROW_VIEWPORT);
    await expect(card).toBeVisible();
    const overflow = await card.evaluate((el) => ({
      scrollWidth: el.scrollWidth,
      clientWidth: document.documentElement.clientWidth,
    }));
    expect(overflow.scrollWidth).toBeLessThanOrEqual(overflow.clientWidth);
    await saveCapture(
      page,
      "database-conversion-discovery-card-insights-narrow",
    );

    await page.setViewportSize({ width: 1280, height: 800 });
    await card
      .getByRole("button", { name: "Open Native Database Settings" })
      .click();

    // Following the discovery card's action lands on the Settings screen's
    // Native Database block, the same entry point `database-conversion.spec.ts`
    // above exercises directly.
    await expect(
      page.getByRole("heading", { name: "Native Database" }),
    ).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
  });

  test("dismissing the card hides it for the rest of the session", async ({
    page,
  }) => {
    await gotoApp(page, {
      path: "/?databaseConversion=sqliteAuthoritative",
    });
    await navigateTo(page, "insights");

    const card = page.getByTestId("database-conversion-discovery-card");
    await expect(card).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });

    await card.getByRole("button", { name: "Later" }).click();
    await expect(card).toHaveCount(0);
  });

  test("stays hidden once native is authoritative", async ({ page }) => {
    await gotoApp(page, { path: "/?databaseConversion=justCompleted" });
    await navigateTo(page, "insights");

    // The Insights page's own bootstrap content proves the page loaded;
    // the discovery card must never appear once native is authoritative.
    await expect(page.getByRole("tab", { name: "CPU / Memory" })).toBeVisible({
      timeout: BOOTSTRAP_TIMEOUT,
    });
    await expect(
      page.getByTestId("database-conversion-discovery-card"),
    ).toHaveCount(0);
  });
});
