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

/**
 * Settings-section-focused tests below only care about
 * `DatabaseConversionSettings`, not the app-root prompt dialog (#2136,
 * mounted separately, see the second `describe` block below). Every path
 * here seeds the dialog's own "dismissed" flag so it never auto-opens over
 * the same `?databaseConversion=sqliteAuthoritative` scenario and produces
 * a second "Convert Now" button/heading on screen.
 */
const gotoSettings = async (
  page: import("@playwright/test").Page,
  path: string,
) => {
  const separator = path.includes("?") ? "&" : "?";
  await gotoApp(page, {
    path: `${path}${separator}databaseConversionPromptDismissed=1`,
  });
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
    await gotoSettings(page, "/?databaseConversion=converting");

    await expect(page.getByText(/Converting…/)).toBeVisible({
      timeout: BOOTSTRAP_TIMEOUT,
    });

    // The hook's own converting -> nativeAuthoritative transition (not a
    // fixed initial state) is what triggers the notice - driven explicitly
    // here rather than by a timer, see `completeDatabaseConversion`'s own
    // documentation for why.
    await page.evaluate(() => window.__E2E__?.completeDatabaseConversion());
    await expect(
      page.getByRole("heading", { name: "Conversion complete" }),
    ).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    // The offer copy and the plain "converted" line are gone: the notice
    // is a follow-up to the action, not a description of the setting.
    await expect(page.getByText(/^Switch the local database/)).toHaveCount(0);
    await expect(
      page.getByText("The database has been converted to the native format."),
    ).toHaveCount(0);

    const noticeTitle = page.getByText(
      "More history now fits in the same space",
    );
    await expect(noticeTitle).toBeVisible();
    await expect(page.getByText(/This notice appears only once/)).toBeVisible();
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
    // Nothing is left to configure, so the section leaves with the notice.
    await expect(
      page.getByRole("heading", { name: "Native Database" }),
    ).toHaveCount(0);
  });
});

test.describe("database conversion app-root prompt dialog", () => {
  test("appears on the default landing screen (no Insights visit needed), converts in place, and completes", async ({
    page,
  }) => {
    // Default landing screen for grouped navigation is Performance, not
    // Settings or Insights - the dialog must appear there too, since users
    // who already have Insights recording on must see it right after
    // updating without opening the Insights screen.
    await gotoApp(page, {
      path: "/?databaseConversion=sqliteAuthoritative",
    });

    const dialog = page.getByRole("alertdialog");
    await expect(dialog).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await expect(
      dialog.getByText("A more efficient database is available"),
    ).toBeVisible();
    const convertButton = dialog.getByRole("button", { name: "Convert Now" });
    await expect(convertButton).toBeVisible();
    await expect(dialog.getByRole("button", { name: "Later" })).toBeVisible();
    await saveCapture(page, "database-conversion-prompt-dialog");

    await page.setViewportSize(NARROW_VIEWPORT);
    await expect(convertButton).toBeVisible();
    const overflow = await dialog.evaluate((el) => ({
      scrollWidth: el.scrollWidth,
      clientWidth: document.documentElement.clientWidth,
    }));
    expect(overflow.scrollWidth).toBeLessThanOrEqual(overflow.clientWidth);
    await saveCapture(page, "database-conversion-prompt-dialog-narrow");

    await page.setViewportSize({ width: 1280, height: 800 });
    await convertButton.click();

    await expect(page.getByText(/Converting…/)).toBeVisible({
      timeout: BOOTSTRAP_TIMEOUT,
    });
    await expect(dialog.getByRole("button", { name: "Later" })).toHaveCount(0);
    await saveCapture(page, "database-conversion-prompt-dialog-converting");

    // Driven explicitly rather than by a timer - see
    // `completeDatabaseConversion`'s own documentation for why.
    await page.evaluate(() => window.__E2E__?.completeDatabaseConversion());
    await expect(
      dialog.getByRole("heading", { name: "Conversion complete" }),
    ).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    const noticeTitle = page.getByText(
      "More history now fits in the same space",
    );
    await expect(noticeTitle).toBeVisible();
    await saveCapture(page, "database-conversion-prompt-dialog-complete");

    await dialog.getByRole("button", { name: "Keep Current Setting" }).click();
    await expect(page.getByRole("alertdialog")).toHaveCount(0);
  });

  test("Later dismisses the prompt for good", async ({ page }) => {
    await gotoApp(page, {
      path: "/?databaseConversion=sqliteAuthoritative",
    });

    const dialog = page.getByRole("alertdialog");
    await expect(dialog).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });

    await dialog.getByRole("button", { name: "Later" }).click();
    await expect(page.getByRole("alertdialog")).toHaveCount(0);
  });

  test("never appears while Insights recording is disabled", async ({
    page,
  }) => {
    await gotoApp(page, {
      path: "/?databaseConversion=sqliteAuthoritative&insightsRecording=disabled",
    });

    // The app's own bootstrap content proves the page loaded and settled;
    // the prompt must never appear once Insights recording is off, even
    // though SQLite is authoritative.
    await expect(page.getByTestId("performance-screen")).toBeVisible({
      timeout: BOOTSTRAP_TIMEOUT,
    });
    await expect(page.getByRole("alertdialog")).toHaveCount(0);
  });

  test("never appears while converting, nor once native is already authoritative", async ({
    page,
  }) => {
    await gotoApp(page, { path: "/?databaseConversion=converting" });

    await expect(page.getByTestId("performance-screen")).toBeVisible({
      timeout: BOOTSTRAP_TIMEOUT,
    });
    await expect(page.getByRole("alertdialog")).toHaveCount(0);

    await page.evaluate(() => window.__E2E__?.completeDatabaseConversion());
    // A negative assertion: give the hook's active poll (armed while
    // converting) a full cycle to pick up the explicit transition before
    // confirming the dialog still never appeared.
    await page.waitForTimeout(1_000);
    await expect(page.getByRole("alertdialog")).toHaveCount(0);
  });

  test("the Settings entry point still offers the conversion independently", async ({
    page,
  }) => {
    await gotoSettings(page, "/?databaseConversion=sqliteAuthoritative");

    // `gotoSettings` seeds the dialog as already dismissed, so only the
    // Settings section's own "Convert Now" is present.
    await expect(page.getByRole("alertdialog")).toHaveCount(0);
    await expect(
      page.getByRole("heading", { name: "Native Database" }),
    ).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await expect(
      page.getByRole("button", { name: "Convert Now" }),
    ).toBeVisible();
  });
});
