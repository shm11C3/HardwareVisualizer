import { expect, test } from "@playwright/test";
import { GPU_FIXTURES } from "../src/e2e/fixtures/hardware";
import { gotoApp, saveCapture, seedHardwareHistory } from "./helpers";

test.describe("dashboard captures", () => {
  test("dashboard renders fixture hardware data", async ({ page }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);

    // GPU selector tablist renders because the fixture exposes two GPUs.
    await expect(
      page.getByRole("tab", { name: GPU_FIXTURES[0].name }),
    ).toBeVisible();

    await saveCapture(page, "dashboard");
  });

  test("gpu selector switches via accessible tab roles", async ({ page }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);

    const secondaryGpuTab = page.getByRole("tab", {
      name: GPU_FIXTURES[1].name,
    });
    await secondaryGpuTab.click();
    await expect(secondaryGpuTab).toHaveAttribute("aria-selected", "true");

    await saveCapture(page, "dashboard-gpu-secondary");
  });
});
