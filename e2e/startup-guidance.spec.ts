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
const CTA = "Open sensor settings";
const setupNotice = (page: Page) =>
  page.getByRole("complementary", {
    name: "Some sensor readings are unavailable",
  });
const publishCandidates = async (
  page: Page,
  candidates: ExternalComponentGuidanceCandidate[],
) => {
  await page.evaluate(
    (items) => window.__E2E__?.setGuidanceCandidates(items),
    candidates,
  );
};
test.beforeEach(async ({ page }) => {
  await page.clock.install();
});

test("failed sensor collection offers settings without a modal or a status query", async ({
  page,
}) => {
  await gotoApp(page, { path: "/?sensorService=missing" });
  await publishCandidates(page, [{ ...CPU_PERMISSION, reasonKind: "missing" }]);
  await expect(setupNotice(page)).toBeVisible();
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
  await expect(
    setupNotice(page).getByText(/Check PawnIO and its signed modules/),
  ).toBeVisible();
  expect(
    await page.evaluate(() =>
      window.__E2E__?.getInvokeCount("get_sensor_service_status"),
    ),
  ).toBe(0);
  expect(
    await page.evaluate(() =>
      window.__E2E__?.getInvokeCount(
        "get_external_component_guidance_candidates",
      ),
    ),
  ).toBe(0);
  await page.screenshot({
    path: capturePath("sensor-service-setup-notice"),
    animations: "disabled",
  });
  await page.setViewportSize({ width: 520, height: 800 });
  await page.screenshot({
    path: capturePath("sensor-service-setup-notice-narrow"),
    animations: "disabled",
  });
  await setupNotice(page).getByRole("button", { name: CTA }).click();
  const section = page.getByRole("region", {
    name: "Windows sensor service",
    exact: true,
  });
  await expect(section).toBeFocused();
  await expect(
    section.getByText(/Set up PawnIO and its modules/),
  ).toBeVisible();
  await expect(
    page.getByRole("switch", { name: "Run as administrator on startup" }),
  ).not.toBeChecked();
});

test("late failures and recovery follow the data without interrupting the current screen", async ({
  page,
}) => {
  await gotoApp(page);
  await page.clock.fastForward(60_000);
  await publishCandidates(page, [CPU_PERMISSION]);
  await expect(setupNotice(page)).toBeVisible();
  await expect(page.getByTestId("performance-screen")).toBeVisible();
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
  await publishCandidates(page, []);
  await expect(setupNotice(page)).toHaveCount(0);
  expect(
    await page.evaluate(() =>
      window.__E2E__?.getInvokeCount("get_sensor_service_status"),
    ),
  ).toBe(0);
});

test("sensor guidance follows visible panels and does not appear in Monitor", async ({
  page,
}) => {
  await gotoApp(page, { path: "/?legacyPerformanceLayout=1" });
  await publishCandidates(page, [MOTHERBOARD_PERMISSION]);
  await expect(setupNotice(page)).toHaveCount(0);
  await page.getByTestId("performance-edit-toggle").click();
  await page.getByRole("button", { name: "Show Motherboard sensors" }).click();
  await expect(setupNotice(page)).toBeVisible();
  await page.getByRole("tab", { name: "Monitor", exact: true }).click();
  await expect(setupNotice(page)).toHaveCount(0);
  await page.getByRole("tab", { name: "Panels", exact: true }).click();
  await expect(setupNotice(page)).toBeVisible();
});

test("dismissal stays quiet for the launch and permanent suppression survives reload", async ({
  page,
}) => {
  await gotoApp(page);
  await publishCandidates(page, [CPU_PERMISSION]);
  await setupNotice(page)
    .getByRole("button", { name: "Hide sensor guidance for this session" })
    .click();
  await publishCandidates(page, [MOTHERBOARD_PERMISSION, CPU_PERMISSION]);
  await page.clock.fastForward(60_000);
  await expect(setupNotice(page)).toHaveCount(0);
  await page.reload();
  await expect(page.getByTestId("performance-screen")).toBeVisible();
  await publishCandidates(page, [CPU_PERMISSION]);
  await setupNotice(page)
    .getByRole("button", { name: "Never show again" })
    .click();
  await page.reload();
  await expect(page.getByTestId("performance-screen")).toBeVisible();
  await publishCandidates(page, [CPU_PERMISSION]);
  await expect(setupNotice(page)).toHaveCount(0);
});

test("successful elevated upgrades use contextual settings guidance without a migration CTA", async ({
  page,
}) => {
  await page.addInitScript(() =>
    localStorage.setItem("e2e:elevatedStartupMode", "true"),
  );
  await gotoApp(page, { path: "/?processElevated=1" });
  await expect(page.getByRole("complementary")).toHaveCount(0);
  await page.clock.fastForward(60_000);
  await expect(page.getByRole("complementary")).toHaveCount(0);
  expect(
    await page.evaluate(() =>
      window.__E2E__?.getInvokeCount("get_sensor_service_status"),
    ),
  ).toBe(0);
  await navigateTo(page, "settings");
  await expect(
    page.getByText("Service connected", { exact: true }),
  ).toBeVisible();
  const reads = await page.evaluate(() =>
    window.__E2E__?.getInvokeCount("get_sensor_service_status"),
  );
  await page.clock.fastForward(60_000);
  expect(
    await page.evaluate(() =>
      window.__E2E__?.getInvokeCount("get_sensor_service_status"),
    ),
  ).toBe(reads);
  await expect(
    page.getByRole("switch", { name: "Run as administrator on startup" }),
  ).toBeChecked();
  await expect(
    page.getByText(/CPU temperature, power and motherboard readings/),
  ).toBeVisible();
  await expect(
    page.getByText(/MSI installs and updates include the sensor service/),
  ).toBeVisible();
  await page
    .getByRole("switch", { name: "Run as administrator on startup" })
    .scrollIntoViewIfNeeded();
  await page.screenshot({
    path: capturePath("sensor-permissions-settings"),
    animations: "disabled",
  });
  await page.setViewportSize({ width: 520, height: 800 });
  await page
    .getByRole("switch", { name: "Run as administrator on startup" })
    .scrollIntoViewIfNeeded();
  await page.screenshot({
    path: capturePath("sensor-permissions-settings-narrow"),
    animations: "disabled",
  });
  await page
    .getByRole("switch", { name: "Run as administrator on startup" })
    .click();
  await expect(
    page.getByRole("switch", { name: "Run as administrator on startup" }),
  ).not.toBeChecked();
  await expect(page.getByText(/This process is still elevated/)).toBeVisible();
});

test("unprotected installations keep MSI guidance and never offer silent elevation", async ({
  page,
}) => {
  await gotoApp(page, { path: "/?elevation=unprotected" });
  await publishCandidates(page, [CPU_PERMISSION]);
  await expect(setupNotice(page)).toBeVisible();
  await page.screenshot({
    path: capturePath("sensor-service-unprotected-notices"),
    animations: "disabled",
  });
  await setupNotice(page).getByRole("button", { name: CTA }).click();
  const repairGuide = page.getByRole("button", {
    name: "Open installation and repair guide",
  });
  await expect(repairGuide).toBeVisible();
  await repairGuide.click();
  expect(
    await page.evaluate(() =>
      window.__E2E__?.getInvokeCount("plugin:shell|open"),
    ),
  ).toBe(1);
});

for (const choice of ["Continue in background", "Quit app"]) {
  test(`first close still offers and saves ${choice}`, async ({ page }) => {
    await gotoApp(page, { path: "/?firstClose=1" });
    await publishCandidates(page, [CPU_PERMISSION]);
    await expect(setupNotice(page)).toBeVisible();
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
