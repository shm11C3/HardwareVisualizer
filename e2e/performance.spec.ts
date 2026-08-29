import { expect, test } from "@playwright/test";
import {
  gotoApp,
  navigateTo,
  saveCapture,
  seedHardwareHistory,
} from "./helpers";

test.describe("Grouped navigation destinations", () => {
  test("moves between the Performance and System Specifications destinations without keeping the other mounted", async ({
    page,
  }) => {
    await gotoApp(page);

    // Both screens are side-menu destinations now, so selection is reported as
    // the current page on the menu entries rather than as a selected tab.
    const performanceEntry = page.getByRole("button", {
      name: "Open Performance",
    });
    const specificationsEntry = page.getByRole("button", {
      name: "Open System Specifications",
    });
    await expect(performanceEntry).toHaveAttribute("aria-current", "page");
    await expect(specificationsEntry).not.toHaveAttribute(
      "aria-current",
      "page",
    );
    await expect(page.getByTestId("performance-screen")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Copy hardware report" }),
    ).toHaveCount(0);

    await navigateTo(page, "systemSpecifications");
    await expect(specificationsEntry).toHaveAttribute("aria-current", "page");
    await expect(performanceEntry).not.toHaveAttribute("aria-current", "page");
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
    await navigateTo(page, "systemSpecifications");
    await expect(page.getByTestId("system-specifications-sheet")).toBeVisible();
    await saveCapture(page, "system-specifications-desktop");

    await page.setViewportSize({ width: 520, height: 800 });
    await saveCapture(page, "system-specifications-compact-window");

    await navigateTo(page, "performance");
    await expect(page.getByTestId("performance-screen")).toBeVisible();
  });

  test("lands upgrading users on a destination that still exists", async ({
    page,
  }) => {
    // A stored selection from the retired Grouped Dashboard destination.
    await gotoApp(page, { path: "/?storedDisplay=groupedDashboard" });
    await expect(page.getByTestId("performance-screen")).toBeVisible();

    // Classic-only screens are not destinations in grouped navigation either.
    await gotoApp(page, { path: "/?storedDisplay=usage" });
    await expect(page.getByTestId("performance-screen")).toBeVisible();

    // A destination both layouts share survives untouched.
    await page.goto("/?storedDisplay=insights");
    await expect(
      page.getByRole("button", { name: "Open Insights" }),
    ).toHaveAttribute("aria-current", "page");
    await expect(page.getByTestId("performance-screen")).toHaveCount(0);
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

  test("switches Monitor Power Draw between current and graph modes", async ({
    page,
  }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);
    await page.getByRole("tab", { name: "Monitor" }).click();

    await expect(
      page.getByTestId("performance-monitor-power-mode-switcher"),
    ).toBeVisible();
    await expect(page.getByRole("tab", { name: "Current" })).toHaveAttribute(
      "data-state",
      "active",
    );
    await expect(
      page.getByTestId("performance-monitor-power-rail"),
    ).toContainText("Package");
    await saveCapture(page, "performance-monitor-power-current");

    await page.getByRole("tab", { name: "Graph" }).click();
    await expect(
      page.getByTestId("performance-monitor-power-graph"),
    ).toBeVisible();
    await expect(
      page.getByTestId("performance-monitor-power-rail"),
    ).toHaveCount(0);
    await saveCapture(page, "performance-monitor-power-graph");

    await page.setViewportSize({ width: 520, height: 800 });
    await expect(
      page.getByTestId("performance-monitor-power-mode-switcher"),
    ).toBeVisible();
    await expect(
      page.getByTestId("performance-monitor-power-graph"),
    ).toBeVisible();
    const assertNoMonitorOverflow = async () => {
      const overflow = await page.evaluate(() => {
        const screen = document.querySelector<HTMLElement>(
          '[data-testid="performance-screen"]',
        );
        return {
          documentScrollWidth: document.documentElement.scrollWidth,
          documentClientWidth: document.documentElement.clientWidth,
          documentScrollHeight: document.documentElement.scrollHeight,
          documentClientHeight: document.documentElement.clientHeight,
          screenScrollWidth: screen?.scrollWidth ?? 0,
          screenClientWidth: screen?.clientWidth ?? 0,
          screenScrollHeight: screen?.scrollHeight ?? 0,
          screenClientHeight: screen?.clientHeight ?? 0,
        };
      });
      expect(overflow.documentScrollWidth).toBeLessThanOrEqual(
        overflow.documentClientWidth,
      );
      expect(overflow.documentScrollHeight).toBeLessThanOrEqual(
        overflow.documentClientHeight,
      );
      expect(overflow.screenScrollWidth).toBeLessThanOrEqual(
        overflow.screenClientWidth,
      );
      expect(overflow.screenScrollHeight).toBeLessThanOrEqual(
        overflow.screenClientHeight,
      );
    };

    await assertNoMonitorOverflow();
    await saveCapture(page, "performance-monitor-power-graph-compact-window");

    await page.getByRole("tab", { name: "Current" }).click();
    await expect(
      page.getByTestId("performance-monitor-power-rail"),
    ).toBeVisible();
    await expect(
      page.getByTestId("performance-monitor-power-graph"),
    ).toHaveCount(0);
    await assertNoMonitorOverflow();
    await saveCapture(page, "performance-monitor-power-current-compact-window");

    await page.getByRole("tab", { name: "Graph" }).click();

    await navigateTo(page, "settings");
    await navigateTo(page, "performance");
    await expect(page.getByRole("tab", { name: "Monitor" })).toHaveAttribute(
      "data-state",
      "active",
    );
    await expect(page.getByRole("tab", { name: "Graph" })).toHaveAttribute(
      "data-state",
      "active",
    );
    await expect(
      page.getByTestId("performance-monitor-power-graph"),
    ).toBeVisible();
  });

  test("attributes the GPU readings to a named adapter the user can change", async ({
    page,
  }) => {
    await gotoApp(page);
    await seedHardwareHistory(page);

    const gpuInstrument = page.getByTestId("performance-metric-gpu");
    const primary = page.getByRole("button", { name: "HV Fixture GPU 8GB" });
    const secondary = page.getByRole("button", { name: "HV Fixture iGPU" });

    // Exactly one control per physical adapter. The fixture's inventory ids
    // and live ids differ on purpose, so a join across the two namespaces
    // would show four controls here, two of them permanently silent.
    await expect(
      page.getByTestId("performance-gpu-selector").getByRole("button"),
    ).toHaveCount(2);

    // Both adapters are reachable and the readings say which one they are.
    await expect(primary).toHaveAttribute("aria-pressed", "true");
    await expect(secondary).toHaveAttribute("aria-pressed", "false");
    await expect(gpuInstrument).toContainText("VRAM 4.0/8 GB");

    await secondary.click();
    await expect(secondary).toHaveAttribute("aria-pressed", "true");
    await expect(gpuInstrument).toContainText("VRAM 1.0/2 GB");
    await expect(gpuInstrument).not.toContainText("VRAM 4.0/8 GB");

    await saveCapture(page, "performance-gpu-selector-desktop");

    // The choice is explicit user intent, so it outlives leaving the screen.
    await navigateTo(page, "systemSpecifications");
    await navigateTo(page, "performance");
    await expect(
      page.getByRole("button", { name: "HV Fixture iGPU" }),
    ).toHaveAttribute("aria-pressed", "true");

    // Monitor mounts only the graph, so the selector is its sole attribution.
    await page.getByRole("tab", { name: "Monitor" }).click();
    await expect(page.getByTestId("performance-usage-graphs")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "HV Fixture iGPU" }),
    ).toHaveAttribute("aria-pressed", "true");
    await saveCapture(page, "performance-gpu-selector-monitor");

    await page.getByRole("tab", { name: "Panels" }).click();

    // ...and the Compact strip reports the same adapter rather than its own.
    await page.getByRole("tab", { name: "Compact" }).click();
    // The label drops the words both fixture adapters share, so a narrow strip
    // keeps the part that tells them apart.
    const strip = page.getByTestId("performance-compact-strip");
    await expect(strip).toContainText("GPU: iGPU");
    await expect(strip).not.toContainText("GPU 8GB");
    await saveCapture(page, "performance-gpu-selector-compact");
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
    await navigateTo(page, "performance");
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
    await expect(
      page.getByRole("heading", { name: "Performance" }),
    ).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Open Settings" }),
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
      page.getByRole("button", { name: "Open Settings" }),
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
