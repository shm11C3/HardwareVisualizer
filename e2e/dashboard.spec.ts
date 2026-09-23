import { expect, type Page, test } from "@playwright/test";
import { GPU_FIXTURES } from "../src/e2e/fixtures/hardware";
import {
  BOOTSTRAP_TIMEOUT,
  gotoApp,
  saveCapture,
  seedHardwareHistory,
} from "./helpers";

test.describe("dashboard captures", () => {
  test("dashboard renders fixture hardware data", async ({ page }) => {
    await gotoApp(page, { path: "/?navigationLayout=classic" });
    await seedHardwareHistory(page);

    // GPU selector tablist renders because the fixture exposes two GPUs.
    await expect(
      page.getByRole("tab", { name: GPU_FIXTURES[0].name }),
    ).toBeVisible();

    await saveCapture(page, "dashboard");
  });

  test("gpu selector switches via accessible tab roles", async ({ page }) => {
    await gotoApp(page, { path: "/?navigationLayout=classic" });
    await seedHardwareHistory(page);

    const gpuCard = page.getByTestId("dashboard-gpu-readings");
    const usageValue = gpuCard.locator("svg").first().locator("text").first();
    await expect(usageValue).toBeVisible();
    const usageBefore = await usageValue.textContent();

    const secondaryGpuTab = page.getByRole("tab", {
      name: GPU_FIXTURES[1].name,
    });
    await secondaryGpuTab.click();
    await expect(secondaryGpuTab).toHaveAttribute("aria-selected", "true");

    // The pressed state is not the point: the usage gauge has to render the
    // other adapter's value. The fixtures keep inventory and live ids in
    // different namespaces, so an inventory id written to the shared
    // selection would leave the live usage map on GPU #1.
    await expect
      .poll(async () => usageValue.textContent())
      .not.toBe(usageBefore);

    await saveCapture(page, "dashboard-gpu-secondary");
  });

  test("storage selector scrolls horizontally with many stubbed devices", async ({
    page,
  }) => {
    await gotoApp(page, {
      path: "/?navigationLayout=classic&storageDevices=12",
    });
    await seedHardwareHistory(page);

    const storageSelector = page.getByRole("tablist", {
      name: "Select storage device",
    });
    await expect(storageSelector).toBeVisible();

    const scrollMetrics = await storageSelector.evaluate((element) => ({
      clientWidth: element.clientWidth,
      overflowX: window.getComputedStyle(element).overflowX,
      scrollWidth: element.scrollWidth,
    }));

    expect(scrollMetrics.scrollWidth).toBeGreaterThan(
      scrollMetrics.clientWidth,
    );
    expect(scrollMetrics.overflowX).toBe("auto");

    const lastDevice = page.getByRole("tab", { name: /FIXTURE-SSD-12/ });
    await lastDevice.scrollIntoViewIfNeeded();
    await expect(lastDevice).toContainText("NVMe");
    await lastDevice.click();
    await expect(lastDevice).toHaveAttribute("aria-selected", "true");
    const selectedDeviceLabel = await storageSelector.evaluate((element) =>
      element.nextElementSibling?.textContent?.replace(/\s+/g, " ").trim(),
    );
    expect(selectedDeviceLabel).toContain("FIXTURE-SSD-12");
    expect(selectedDeviceLabel).toContain("NVMe");

    await saveCapture(page, "dashboard-storage-many");
  });

  test("NSIS build shows the MSI migration notice", async ({
    context,
    page: firstPage,
  }) => {
    // Boots the app three times, which can exceed the default timeout when
    // the other captures run in parallel.
    test.slow();
    const path = "/?navigationLayout=classic&nsisMigrationNotice=1";
    const noticeOn = (page: Page) =>
      page.getByRole("alertdialog", { name: "Switch to the MSI installer" });
    // A fresh page in the same context stands in for the next app launch:
    // in-memory state is gone, the mocked settings file is not.
    const relaunch = async (previous: Page) => {
      await previous.close();
      const next = await context.newPage();
      await gotoApp(next, { path });
      return next;
    };

    let page = firstPage;
    await gotoApp(page, { path });

    const notice = noticeOn(page);
    await expect(notice).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    await saveCapture(page, "nsis-migration-notice");

    // In a compact window the dialog scrolls, so its actions stay reachable.
    await page.setViewportSize({ width: 520, height: 420 });
    const hide = notice.getByRole("button", { name: "Hide" });
    await hide.scrollIntoViewIfNeeded();
    await expect(hide).toBeInViewport();
    const box = await notice.boundingBox();
    expect(box?.height ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(420);

    // "Remind me next time" hides it for this session only.
    await hide.click();
    await page.getByRole("menuitem", { name: "Remind me next time" }).click();
    await expect(notice).toHaveCount(0);
    page = await relaunch(page);
    const remindedNotice = noticeOn(page);
    await expect(remindedNotice).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });

    // "Never show again" is saved, so the next launch does not show it.
    await remindedNotice.getByRole("button", { name: "Hide" }).click();
    await page.getByRole("menuitem", { name: "Never show again" }).click();
    await expect(remindedNotice).toHaveCount(0);
    page = await relaunch(page);
    await expect
      .poll(() =>
        page.evaluate(() => window.__E2E__?.getInvokeCount("get_settings")),
      )
      .toBeGreaterThan(0);
    await expect(noticeOn(page)).toHaveCount(0);
    expect(
      await page.evaluate(() =>
        window.__E2E__?.getInvokeCount("plugin:app|bundle_type"),
      ),
    ).toBe(0);
  });
});
