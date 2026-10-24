import { expect, type Page, test } from "@playwright/test";
import type { ExternalComponentGuidanceCandidate } from "../src/rspc/bindings";
import { capturePath, gotoApp, navigateTo } from "./helpers";

const CPU_PERMISSION: ExternalComponentGuidanceCandidate = {
  key: "pawnio:cpu-package-temperature:v1",
  component: "pawnio",
  usage: "cpuPackageTemperature",
  reasonKind: "permission",
  missingSignals: ["cpu-temperature"],
  affectedDeviceCount: null,
  diagnosticDetail: "Access denied",
};
const MOTHERBOARD_PERMISSION: ExternalComponentGuidanceCandidate = {
  ...CPU_PERMISSION,
  key: "pawnio:motherboard-sensors:v1",
  usage: "motherboardSensors",
  missingSignals: ["motherboard-temperature", "motherboard-fan-speed"],
};
const CTA = "Enable Run as administrator on startup";

test.beforeEach(async ({ page }) => {
  await page.clock.install();
});

const publishCandidates = async (
  page: Page,
  candidates: ExternalComponentGuidanceCandidate[],
) => {
  await page.evaluate(
    (items) => window.__E2E__?.setGuidanceCandidates(items),
    candidates,
  );
  // Exercise the unchanged visible polling cadence without a real minute's wait.
  await page.clock.fastForward(60_000);
};

const hideGuidance = async (page: Page, permanently = false) => {
  await page.getByRole("button", { name: "Hide", exact: true }).click();
  await page
    .getByRole("menuitem", {
      name: permanently ? "Never show again" : "Hide for this session",
      exact: true,
    })
    .click();
};

test("fresh startup waits for a relevant reading and the explicit CTA saves the setting", async ({
  page,
}) => {
  await gotoApp(page, { path: "/?firstClose=1&showNavigationNotice=1" });
  await expect
    .poll(() =>
      page.evaluate(() =>
        window.__E2E__?.getInvokeCount(
          "get_external_component_guidance_candidates",
        ),
      ),
    )
    .toBeGreaterThan(0);
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
  await publishCandidates(page, [CPU_PERMISSION]);
  const dialog = page.getByRole("alertdialog");
  await expect(dialog).toBeVisible();
  await expect(
    dialog.getByText("CPU package temperature", { exact: true }),
  ).toBeVisible();
  await expect(dialog.getByText(/requests Windows approval/)).toBeVisible();
  await page.screenshot({
    path: capturePath("performance-permission-guidance"),
    fullPage: true,
    animations: "disabled",
  });
  await page.setViewportSize({ width: 520, height: 800 });
  await page.screenshot({
    path: capturePath("performance-permission-guidance-narrow"),
    fullPage: true,
    animations: "disabled",
  });
  await dialog.getByRole("button", { name: CTA }).click();
  expect(
    await page.evaluate(() =>
      window.__E2E__?.getInvokeCount("set_elevated_startup_mode"),
    ),
  ).toBe(1);
  await hideGuidance(page);
  await navigateTo(page, "settings");
  await expect(
    page.getByRole("switch", { name: "Run as administrator on startup" }),
  ).toBeChecked();
});

test("Performance ignores unrelated components and non-permission candidates", async ({
  page,
}) => {
  await gotoApp(page);
  await publishCandidates(page, [
    { ...CPU_PERMISSION, reasonKind: "missing" },
    {
      ...CPU_PERMISSION,
      key: "smartctl:storage-health:v1",
      component: "smartctl",
      usage: "storageHealth",
    },
  ]);
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
});

test("motherboard guidance follows the enabled panel and Monitor has no temperature guidance", async ({
  page,
}) => {
  await gotoApp(page, { path: "/?legacyPerformanceLayout=1" });
  await publishCandidates(page, [MOTHERBOARD_PERMISSION]);
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
  await page.getByTestId("performance-edit-toggle").click();
  await page.getByRole("button", { name: "Show Motherboard sensors" }).click();
  await expect(page.getByRole("alertdialog")).toBeVisible();
  await expect(
    page.getByText("Motherboard temperature sensors and fan speeds", {
      exact: true,
    }),
  ).toBeVisible();
  await hideGuidance(page);
  await page.getByRole("tab", { name: "Monitor", exact: true }).click();
  await publishCandidates(page, [CPU_PERMISSION]);
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
  await page.getByRole("tab", { name: "Panels", exact: true }).click();
  await expect(page.getByRole("button", { name: CTA })).toBeVisible();
});

test("expanded Compact waits until its normal view is restored", async ({
  page,
}) => {
  await gotoApp(page);
  await page.getByRole("tab", { name: "Compact", exact: true }).click();
  await page.getByTestId("performance-compact-expand").click();
  await publishCandidates(page, [CPU_PERMISSION]);
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
  await page.getByTestId("performance-compact-collapse").click();
  await expect(page.getByRole("button", { name: CTA })).toBeVisible();
  await page.screenshot({
    path: capturePath("performance-compact-permission-guidance"),
    fullPage: true,
    animations: "disabled",
  });
});

test("elevated processes have no Performance elevation offer", async ({
  page,
}) => {
  await gotoApp(page, { path: "/?processElevated=1" });
  await publishCandidates(page, [CPU_PERMISSION]);
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
});

test("unprotected installs explain the refusal and keep a details action", async ({
  page,
}) => {
  await gotoApp(page, { path: "/?elevation=unprotected" });
  await publishCandidates(page, [CPU_PERMISSION]);
  const dialog = page.getByRole("alertdialog");
  await expect(
    dialog.getByText(/not installed under Program Files/),
  ).toBeVisible();
  await expect(
    dialog.getByRole("button", { name: "Open details" }),
  ).toBeVisible();
  await expect(dialog.getByRole("button", { name: CTA })).toHaveCount(0);
});

test("deferral lasts for the session and acknowledgement survives reload", async ({
  page,
}) => {
  await gotoApp(page);
  await publishCandidates(page, [CPU_PERMISSION]);
  await expect(page.getByRole("button", { name: CTA })).toBeVisible();
  await hideGuidance(page);
  await publishCandidates(page, [CPU_PERMISSION]);
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
  await page.reload();
  await expect(page.getByTestId("performance-screen")).toBeVisible();
  await publishCandidates(page, [CPU_PERMISSION]);
  await expect(page.getByRole("button", { name: CTA })).toBeVisible();
  await hideGuidance(page, true);
  await page.reload();
  await expect(page.getByTestId("performance-screen")).toBeVisible();
  await publishCandidates(page, [CPU_PERMISSION]);
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
});

for (const choice of ["Continue in background", "Quit app"]) {
  test(`first close still offers and saves ${choice}`, async ({ page }) => {
    await gotoApp(page, { path: "/?firstClose=1" });
    await expect
      .poll(() =>
        page.evaluate(() =>
          window.__E2E__?.getInvokeCount("mark_close_to_tray_listener_ready"),
        ),
      )
      .toBe(1);
    await expect(page.getByRole("alertdialog")).toHaveCount(0);
    if (choice === "Continue in background") {
      await page.getByRole("tab", { name: "Compact", exact: true }).click();
      await page.getByTestId("performance-compact-expand").click();
    }
    await page.evaluate(() => window.__E2E__?.requestFirstClose());
    await expect(page.getByRole("alertdialog")).toBeVisible();
    await page
      .getByRole("button", { name: choice, exact: true })
      .last()
      .click();
    await expect(page.getByRole("alertdialog")).toHaveCount(0);
    expect(
      await page.evaluate(() =>
        window.__E2E__?.getInvokeCount("set_close_to_tray_preference"),
      ),
    ).toBe(1);
    const command =
      choice === "Continue in background"
        ? "hide_main_window_to_tray"
        : "quit_app";
    expect(
      await page.evaluate(
        (cmd) => window.__E2E__?.getInvokeCount(cmd),
        command,
      ),
    ).toBe(1);
  });
}

test("late guidance and a close request wait behind an active conversion", async ({
  page,
}) => {
  await gotoApp(page, {
    path: "/?databaseConversion=sqliteAuthoritative&nsisMigrationNotice=1&firstClose=1",
  });
  const conversion = page.getByRole("alertdialog", {
    name: "A more efficient database is available",
  });
  await expect(conversion).toBeVisible();
  await publishCandidates(page, [CPU_PERMISSION]);
  await page.evaluate(() => window.__E2E__?.requestFirstClose());
  await expect(page.getByRole("alertdialog")).toHaveCount(1);
  await expect(conversion).toBeVisible();
  await conversion.getByRole("button", { name: "Later", exact: true }).click();
  await expect(
    page.getByRole("alertdialog", { name: "Keep HardwareVisualizer running?" }),
  ).toBeVisible();
  await expect(page.getByRole("alertdialog")).toHaveCount(1);
  await page
    .getByRole("button", { name: "Continue in background", exact: true })
    .click();
  await expect(page.getByRole("button", { name: CTA })).toBeVisible();
  await expect(page.getByRole("alertdialog")).toHaveCount(1);
  await hideGuidance(page);
  await expect(page.getByRole("alertdialog")).toHaveCount(1);
  await expect(
    page
      .getByRole("alertdialog")
      .getByText(/MSI installer/)
      .first(),
  ).toBeVisible();
});

test("a failed settings load still permits the first-close Quit action", async ({
  page,
}) => {
  await page.goto("/?settingsReadFailure=1&firstClose=1");
  await expect
    .poll(() =>
      page.evaluate(() =>
        window.__E2E__?.getInvokeCount("plugin:dialog|message"),
      ),
    )
    .toBe(1);
  await expect
    .poll(() =>
      page.evaluate(() =>
        window.__E2E__?.getInvokeCount("mark_close_to_tray_listener_ready"),
      ),
    )
    .toBe(1);
  await page.evaluate(() => window.__E2E__?.requestFirstClose());
  const dialog = page.getByRole("alertdialog", {
    name: "Keep HardwareVisualizer running?",
  });
  await expect(dialog).toBeVisible();
  await dialog
    .getByRole("button", { name: "Quit app", exact: true })
    .last()
    .click();
  await expect
    .poll(() => page.evaluate(() => window.__E2E__?.getInvokeCount("quit_app")))
    .toBe(1);
  expect(
    await page.evaluate(() =>
      window.__E2E__?.getInvokeCount("set_close_to_tray_preference"),
    ),
  ).toBe(1);
});
