import { expect, test } from "@playwright/test";
import {
  gotoApp,
  navigateTo,
  saveCapture,
  seedHardwareHistory,
} from "./helpers";

test.describe("Grouped Dashboard", () => {
  test("switches between Performance and System Specifications without keeping inactive content mounted", async ({
    page,
  }) => {
    await gotoApp(page);

    const performanceTab = page.getByRole("tab", { name: "Performance" });
    const specificationsTab = page.getByRole("tab", {
      name: "System Specifications",
    });

    await expect(performanceTab).toHaveAttribute("data-state", "active");
    await expect(page.getByTestId("performance-screen")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Copy hardware report" }),
    ).toHaveCount(0);

    await specificationsTab.click();
    await expect(specificationsTab).toHaveAttribute("data-state", "active");
    await expect(page.getByTestId("performance-screen")).toHaveCount(0);
    await expect(page.getByTestId("system-specifications-sheet")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Copy hardware report" }),
    ).toBeVisible();
    await expect(page.getByTestId("live-process-table")).toHaveCount(0);
    await expect(page.getByText("Thread Count")).toHaveCount(0);
    await expect(page.getByText("Operating system")).toBeVisible();
    await expect(page.getByText("x86_64")).toBeVisible();
    await expect(
      page.getByText("Network information is unavailable on this system."),
    ).toBeVisible();

    await navigateTo(page, "settings");
    await navigateTo(page, "dashboard");
    await expect(specificationsTab).toHaveAttribute("data-state", "active");
    await saveCapture(page, "dashboard-system-specifications-desktop");

    await page.setViewportSize({ width: 520, height: 800 });
    await saveCapture(page, "dashboard-system-specifications-compact-window");

    await performanceTab.click();
    await expect(page.getByTestId("performance-screen")).toBeVisible();
  });

  test("switches views and unmounts content outside the active view", async ({
    page,
  }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);

    await expect(page.getByRole("tab", { name: "Panels" })).toHaveAttribute(
      "data-state",
      "active",
    );
    await expect(page.getByTestId("performance-current-values")).toBeVisible();
    await expect(page.getByTestId("performance-usage-graphs")).toBeVisible();
    await expect(page.getByTestId("live-process-table")).toBeVisible();

    await page.getByRole("tab", { name: "Compact" }).click();
    await expect(page.getByTestId("performance-compact-strip")).toBeVisible();
    await expect(page.getByTestId("performance-current-values")).toHaveCount(0);
    await expect(page.getByTestId("performance-usage-graphs")).toHaveCount(0);
    await expect(page.getByTestId("live-process-table")).toHaveCount(0);

    await navigateTo(page, "settings");
    await navigateTo(page, "dashboard");
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

  test("persists panel visibility edited in the panels view", async ({
    page,
  }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);

    await page.getByTestId("performance-edit-toggle").click();
    await expect(page.getByTestId("performance-hidden-panels")).toBeVisible();

    await page.getByRole("button", { name: "Hide Usage graphs" }).click();
    await expect(page.getByTestId("performance-usage-graphs")).toHaveCount(0);

    await saveCapture(page, "performance-panels-editing-desktop");

    await page.getByTestId("performance-edit-toggle").click();
    await expect(page.getByTestId("performance-hidden-panels")).toHaveCount(0);

    await navigateTo(page, "settings");
    await navigateTo(page, "dashboard");
    await expect(page.getByTestId("performance-usage-graphs")).toHaveCount(0);
    await expect(page.getByTestId("live-process-table")).toBeVisible();

    await page.getByTestId("performance-edit-toggle").click();
    await page.getByRole("button", { name: "Show Usage graphs" }).click();
    await expect(page.getByTestId("performance-usage-graphs")).toBeVisible();
  });

  test("adds a default-hidden panel from the edit strip", async ({ page }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);

    await expect(
      page.getByTestId("performance-panel-motherboardSensors"),
    ).toHaveCount(0);

    await page.getByTestId("performance-edit-toggle").click();
    await page
      .getByRole("button", { name: "Show Motherboard sensors" })
      .click();

    const sensorsPanel = page.getByTestId(
      "performance-panel-motherboardSensors",
    );
    await expect(sensorsPanel).toBeVisible();
    await expect(sensorsPanel).toContainText("SYSTIN");
  });

  test("offers two columns only while the window can hold them", async ({
    page,
  }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);

    const grid = page.locator("[data-panel-columns]");
    await expect(grid).toHaveAttribute("data-panel-columns", "1");

    await page.setViewportSize({ width: 1600, height: 900 });
    await page.getByRole("tab", { name: "Two columns" }).click();
    await expect(grid).toHaveAttribute("data-panel-columns", "2");

    const wideColumns = await grid.evaluate(
      (element) => window.getComputedStyle(element).gridTemplateColumns,
    );
    expect(wideColumns.split(" ")).toHaveLength(2);

    await saveCapture(page, "performance-panels-two-column-desktop");

    // The request is an upper bound: a narrow window still renders one column.
    await page.setViewportSize({ width: 900, height: 900 });
    const narrowColumns = await grid.evaluate(
      (element) => window.getComputedStyle(element).gridTemplateColumns,
    );
    expect(narrowColumns.split(" ")).toHaveLength(1);
    await expect(grid).toHaveAttribute("data-panel-columns", "2");
  });

  test("expands Compact to a chrome-free mini monitor", async ({ page }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);

    await page.getByRole("tab", { name: "Compact" }).click();
    await page.getByTestId("performance-compact-expand").click();

    const fullScreen = page.getByTestId("performance-compact-fullscreen");
    await expect(fullScreen).toBeVisible();
    await expect(page.getByTestId("performance-compact-strip")).toBeVisible();
    // Nothing else is reachable: the rest of the app is inert behind the
    // layer, so no tabs, headings, or side-menu buttons remain exposed.
    await expect(page.getByRole("tab")).toHaveCount(0);
    await expect(page.getByRole("heading", { name: "Dashboard" })).toHaveCount(
      0,
    );
    await expect(
      page.getByRole("button", { name: "open settings" }),
    ).toHaveCount(0);
    await expect(
      page.getByTestId("performance-compact-collapse"),
    ).toContainText("Exit full screen");

    // The strip actually fills the viewport rather than sitting in a card.
    const viewport = page.viewportSize();
    const box = await fullScreen.boundingBox();
    expect(box?.width).toBe(viewport?.width);
    expect(box?.height).toBe(viewport?.height);

    await saveCapture(page, "performance-compact-fullscreen");

    await page.keyboard.press("Escape");
    await expect(fullScreen).toHaveCount(0);
    await expect(page.getByRole("tab", { name: "Compact" })).toBeVisible();
    await expect(
      page.getByRole("button", { name: "open settings" }),
    ).toBeVisible();

    // The labelled control exits as well as Escape.
    await page.getByTestId("performance-compact-expand").click();
    await expect(fullScreen).toBeVisible();
    await page.getByTestId("performance-compact-collapse").click();
    await expect(fullScreen).toHaveCount(0);
  });

  test("keeps the expanded strip inside a small corner window", async ({
    page,
  }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);

    // The mini monitor's own use case: a small window kept in a screen corner.
    await page.setViewportSize({ width: 520, height: 420 });
    await page.getByRole("tab", { name: "Compact" }).click();
    await page.getByTestId("performance-compact-expand").click();
    await expect(
      page.getByTestId("performance-compact-fullscreen"),
    ).toBeVisible();

    // burnin-root hides overflow, so a row wider than the window would clip
    // the sparkline instead of shrinking.
    const overflow = await page
      .getByTestId("performance-compact-row-cpu")
      .evaluate((row) => ({
        scrollWidth: row.scrollWidth,
        clientWidth: row.clientWidth,
      }));
    expect(overflow.scrollWidth).toBeLessThanOrEqual(overflow.clientWidth);

    await saveCapture(page, "performance-compact-fullscreen-small-window");
  });

  test("renders at desktop and compact viewports", async ({ page }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);
    await saveCapture(page, "performance-panels-desktop");

    await page.setViewportSize({ width: 520, height: 800 });
    await page.getByRole("tab", { name: "Compact" }).click();
    await expect(page.getByTestId("performance-compact-strip")).toBeVisible();
    await saveCapture(page, "performance-compact-window");
  });
});
