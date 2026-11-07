import { expect, test } from "@playwright/test";
import { capturePath, gotoApp, navigateTo } from "./helpers";

test("reviews the report before saving or opening the public form", async ({
  page,
}) => {
  await gotoApp(page, { path: "/?navigationLayout=classic" });
  const count = (command: string) =>
    page.evaluate((name) => window.__E2E__?.getInvokeCount(name), command);
  const reportButton = page.getByRole("button", {
    name: "Hardware Compatibility Report",
  });
  await reportButton.click();
  const dialog = page.getByRole("dialog", {
    name: "Hardware Compatibility Report",
  });
  const preview = dialog.getByLabel("Full Markdown report");
  await expect(preview).toHaveValue(/CPU model: Example CPU/);
  expect(await count("save_hardware_compatibility_report")).toBe(0);
  expect(await count("plugin:clipboard-manager|write_text")).toBe(0);
  expect(await count("plugin:shell|open")).toBe(0);
  expect(await count("refresh_storage_devices")).toBe(0);
  expect(await count("get_super_io_chip_id_diagnostics")).toBe(0);
  await page.screenshot({
    path: capturePath("hardware-compatibility"),
    animations: "disabled",
  });
  await page.setViewportSize({ width: 520, height: 720 });
  await expect(preview).toBeVisible();
  await page.screenshot({
    path: capturePath("hardware-compatibility-narrow"),
    animations: "disabled",
  });
  await dialog.getByRole("button", { name: "Save Markdown" }).click();
  await expect.poll(() => count("save_hardware_compatibility_report")).toBe(1);
  await dialog
    .getByRole("button", { name: "Copy and open GitHub form" })
    .click();
  await expect.poll(() => count("plugin:clipboard-manager|write_text")).toBe(1);
  await expect.poll(() => count("plugin:shell|open")).toBe(1);
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(reportButton).toBeFocused();
  await navigateTo(page, "settings");
  await expect(reportButton).toBeVisible();
  const inventoryReads = await count("get_hardware_info");
  const storageReads = await count("get_storage_health_latest_records");
  await reportButton.click();
  await expect(preview).toHaveValue(/CPU model: Example CPU/);
  expect(await count("get_hardware_info")).toBe(inventoryReads);
  expect(await count("get_storage_health_latest_records")).toBe(storageReads);
});
