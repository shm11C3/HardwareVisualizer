import { expect, test } from "@playwright/test";
import { sysInfoFixture } from "../hardviz/frontend/src/e2e/fixtures/hardware";
import {
  BOOTSTRAP_TIMEOUT,
  gotoApp,
  navigateTo,
  saveCapture,
  seedHardwareHistory,
} from "./helpers";

const CPU_NAME = sysInfoFixture.cpu?.name ?? "";

test.describe("cpu detail captures", () => {
  test("cpu detail shows fixture info and per-core charts", async ({
    page,
  }) => {
    await gotoApp(page, { path: "/?navigationLayout=classic" });
    await seedHardwareHistory(page);

    await navigateTo(page, "cpuDetail");

    // Info table renders the fixture CPU name on the detail screen.
    await expect(page.getByText(CPU_NAME).first()).toBeVisible({
      timeout: BOOTSTRAP_TIMEOUT,
    });
    await page.waitForTimeout(600);

    await saveCapture(page, "cpu-detail");
  });

  test("CPU core temperatures preserve unavailable slots on desktop and compact windows", async ({
    page,
  }) => {
    await gotoApp(page, {
      path: "/?navigationLayout=classic&cpuCoreTemperatures=1",
    });
    await seedHardwareHistory(page);
    await navigateTo(page, "cpuDetail");

    const panel = page.locator("section").filter({
      has: page.getByRole("heading", { name: "CPU core temperatures" }),
    });
    await expect(panel.getByText("43 °C")).toBeVisible();
    await expect(panel.getByText("N/A")).toBeVisible();
    await saveCapture(page, "cpu-core-temperatures-desktop");

    await page.setViewportSize({ width: 520, height: 800 });
    await expect(panel.getByText("Core at P2")).toBeVisible();
    await expect(panel.getByText("N/A")).toBeVisible();
    await saveCapture(page, "cpu-core-temperatures-compact");
  });

  test("CPU core temperatures show both processor groups on desktop and compact windows", async ({
    page,
  }) => {
    await gotoApp(page, {
      path: "/?navigationLayout=classic&cpuCoreTemperatures=2",
    });
    await seedHardwareHistory(page);
    await navigateTo(page, "cpuDetail");

    const panel = page.locator("section").filter({
      has: page.getByRole("heading", { name: "CPU core temperatures" }),
    });
    const first = panel.getByText("Core at group 0, P0");
    const second = panel.getByText("Core at group 1, P0");
    await expect(first.locator("..").getByText("37 °C")).toBeVisible();
    await expect(second.locator("..").getByText("N/A")).toBeVisible();
    await saveCapture(page, "cpu-two-group-temperatures-desktop");

    await page.setViewportSize({ width: 520, height: 800 });
    await expect(first).toBeVisible();
    await expect(second).toBeVisible();
    await expect(second.locator("..").getByText("N/A")).toBeVisible();
    await saveCapture(page, "cpu-two-group-temperatures-compact");
  });

  test("CPU detail displays CCD slots independently of core temperatures", async ({
    page,
  }) => {
    await gotoApp(page, {
      path: "/?navigationLayout=classic&cpuCcdTemperatures=1",
    });
    await seedHardwareHistory(page);
    await navigateTo(page, "cpuDetail");

    const panel = page.locator("section").filter({
      has: page.getByRole("heading", { name: "CPU CCD temperatures" }),
    });
    await expect(panel.getByText("N/A")).toBeVisible();
    await expect(panel.getByText("54 °C")).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "CPU core temperatures" }),
    ).toHaveCount(0);
    await saveCapture(page, "cpu-ccd-temperatures-desktop");
    await page.setViewportSize({ width: 520, height: 800 });
    await expect(panel.getByText("CCD 2")).toBeVisible();
    await saveCapture(page, "cpu-ccd-temperatures-compact");
  });
});
