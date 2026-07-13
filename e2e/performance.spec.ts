import { expect, test } from "@playwright/test";
import {
  gotoApp,
  navigateTo,
  saveCapture,
  seedHardwareHistory,
} from "./helpers";

test.describe("Performance Screen", () => {
  test("switches presets and unmounts panels outside the active preset", async ({
    page,
  }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);

    await expect(page.getByRole("tab", { name: "Detailed" })).toHaveAttribute(
      "data-state",
      "active",
    );
    await expect(page.getByTestId("performance-current-values")).toBeVisible();
    await expect(page.getByTestId("performance-usage-graphs")).toBeVisible();
    await expect(page.getByTestId("live-process-table")).toBeVisible();

    await page.getByRole("tab", { name: "Compact" }).click();
    await expect(page.getByTestId("performance-current-values")).toBeVisible();
    await expect(page.getByTestId("performance-usage-graphs")).toHaveCount(0);
    await expect(page.getByTestId("live-process-table")).toHaveCount(0);

    await navigateTo(page, "settings");
    await navigateTo(page, "performance");
    await expect(page.getByRole("tab", { name: "Compact" })).toHaveAttribute(
      "data-state",
      "active",
    );

    await page.getByRole("tab", { name: "Monitor" }).click();
    await expect(page.getByTestId("performance-usage-graphs")).toBeVisible();
    await expect(page.getByTestId("performance-current-values")).toHaveCount(0);
    await expect(page.getByTestId("live-process-table")).toHaveCount(0);
    await saveCapture(page, "performance-monitor-desktop");
  });

  test("persists Custom visibility while switching presets", async ({
    page,
  }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);

    await page.getByRole("tab", { name: "Custom" }).click();
    const usageGraphs = page.getByRole("checkbox", { name: "Usage graphs" });
    await expect(usageGraphs).toBeChecked();
    await usageGraphs.click();
    await expect(page.getByTestId("performance-usage-graphs")).toHaveCount(0);

    await page.getByRole("tab", { name: "Detailed" }).click();
    await expect(page.getByTestId("performance-usage-graphs")).toBeVisible();
    await page.getByRole("tab", { name: "Custom" }).click();
    await expect(usageGraphs).not.toBeChecked();
    await expect(page.getByTestId("performance-usage-graphs")).toHaveCount(0);

    await navigateTo(page, "settings");
    const classicNavigation = page.getByRole("switch", {
      name: "Classic navigation",
    });
    await classicNavigation.click();
    await expect(classicNavigation).toBeChecked();
    await classicNavigation.click();
    await expect(classicNavigation).not.toBeChecked();
    await navigateTo(page, "performance");
    await expect(page.getByRole("tab", { name: "Custom" })).toHaveAttribute(
      "data-state",
      "active",
    );
    await expect(usageGraphs).not.toBeChecked();
    await expect(page.getByTestId("performance-usage-graphs")).toHaveCount(0);

    await saveCapture(page, "performance-custom-desktop");

    await page.setViewportSize({ width: 520, height: 800 });
    await expect(page.getByTestId("performance-current-values")).toBeVisible();
    await saveCapture(page, "performance-custom-compact-window");
  });

  test("renders at desktop and compact viewports", async ({ page }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);
    await saveCapture(page, "performance-detailed-desktop");

    await page.setViewportSize({ width: 520, height: 800 });
    await page.getByRole("tab", { name: "Compact" }).click();
    await expect(page.getByTestId("performance-current-values")).toBeVisible();
    await saveCapture(page, "performance-compact-window");
  });
});
