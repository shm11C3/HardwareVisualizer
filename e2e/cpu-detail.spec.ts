import { expect, test } from "@playwright/test";
import { sysInfoFixture } from "../src/e2e/fixtures/hardware";
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
    await gotoApp(page);
    await seedHardwareHistory(page);

    await navigateTo(page, "cpuDetail");

    // Info table renders the fixture CPU name on the detail screen.
    await expect(page.getByText(CPU_NAME).first()).toBeVisible({
      timeout: BOOTSTRAP_TIMEOUT,
    });
    await page.waitForTimeout(600);

    await saveCapture(page, "cpu-detail");
  });
});
