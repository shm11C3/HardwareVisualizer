import { expect, test } from "@playwright/test";
import { GPU_FIXTURES } from "../src/e2e/fixtures/hardware";
import { gotoApp, saveCapture, seedHardwareHistory } from "./helpers";

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

    const secondaryGpuTab = page.getByRole("tab", {
      name: GPU_FIXTURES[1].name,
    });
    await secondaryGpuTab.click();
    await expect(secondaryGpuTab).toHaveAttribute("aria-selected", "true");

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
});
