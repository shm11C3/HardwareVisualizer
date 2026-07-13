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

test.describe("Hardware Category Screens", () => {
  test("reaches every category from the collapsed icon rail", async ({
    page,
  }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);

    for (const target of [
      "hardwareCpu",
      "hardwareGpu",
      "hardwareMemory",
      "hardwareStorage",
      "hardwareSystem",
    ] as const) {
      await expect(
        page.getByRole("button", { name: `open ${target}` }),
      ).toBeVisible({ timeout: BOOTSTRAP_TIMEOUT });
    }

    await navigateTo(page, "hardwareCpu");
    await expect(page.getByText(CPU_NAME).first()).toBeVisible();

    await navigateTo(page, "hardwareGpu");
    await expect(page.getByTestId("hardware-category-gpu")).toBeVisible();

    await navigateTo(page, "hardwareMemory");
    await expect(page.getByTestId("hardware-category-memory")).toBeVisible();

    await navigateTo(page, "hardwareStorage");
    await expect(page.getByTestId("hardware-category-storage")).toBeVisible();

    await navigateTo(page, "hardwareSystem");
    await expect(page.getByTestId("hardware-category-system")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Copy hardware report" }),
    ).toBeVisible();

    await saveCapture(page, "hardware-system-desktop");
  });

  test("shows the expanded hardware tree at desktop and compact widths", async ({
    page,
  }) => {
    await gotoApp(page);
    await page.getByRole("button", { name: "Expand sidebar" }).click();
    await expect(
      page.getByRole("button", { name: "Collapse sidebar" }),
    ).toBeVisible();
    const expandedSideMenu = page.getByTestId("expanded-side-menu");
    await expect
      .poll(async () =>
        Math.round((await expandedSideMenu.boundingBox())?.x ?? -1),
      )
      .toBe(0);
    for (const label of ["CPU", "GPU", "Memory", "Storage", "System"]) {
      await expect(page.getByRole("tab", { name: `${label} tab` })).toBeVisible(
        { timeout: BOOTSTRAP_TIMEOUT },
      );
    }

    await saveCapture(page, "hardware-navigation-expanded-desktop");

    await page.setViewportSize({ width: 520, height: 800 });
    await expect(page.getByRole("tab", { name: "System tab" })).toBeVisible();
    await saveCapture(page, "hardware-navigation-expanded-compact-window");
  });

  for (const scenario of [
    {
      name: "GPU",
      path: "/?gpuDevices=0",
      target: "hardwareGpu",
      message: "GPU information is unavailable on this system.",
    },
    {
      name: "Memory",
      path: "/?memoryModules=0",
      target: "hardwareMemory",
      message: "Memory information is unavailable on this system.",
    },
    {
      name: "Storage",
      path: "/?storageDevices=0",
      target: "hardwareStorage",
      message: "Storage information is unavailable on this system.",
    },
    {
      name: "Network",
      path: "/",
      target: "hardwareSystem",
      message: "Network information is unavailable on this system.",
    },
  ] as const) {
    test(`shows an explicit state when ${scenario.name} is unavailable`, async ({
      page,
    }) => {
      await gotoApp(page, { path: scenario.path });
      await navigateTo(page, scenario.target);

      await expect(page.getByText(scenario.message)).toBeVisible();
      await expect(
        page.getByTestId("hardware-category-unavailable"),
      ).toBeVisible();
    });
  }
});
