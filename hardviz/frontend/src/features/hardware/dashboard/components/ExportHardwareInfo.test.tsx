import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import { ExportHardwareInfo } from "./ExportHardwareInfo";

const mocks = vi.hoisted(() => ({
  record: vi.fn(),
  exportToClipboard: vi.fn(),
}));

vi.mock("@/hooks/analytics/useProductEvent", () => ({
  useProductEvent: () => mocks.record,
}));
vi.mock("@/features/hardware/dashboard/hooks/useExportToClipboard", () => ({
  useExportToClipboard: () => ({ exportToClipboard: mocks.exportToClipboard }),
}));
vi.mock("@/components/HardwareCompatibilityReport", () => ({
  HardwareCompatibilityReport: () => null,
}));

describe("ExportHardwareInfo", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("counts the hardware report export when the user starts it, even if the copy then fails", async () => {
    mocks.exportToClipboard.mockRejectedValue(new Error("clipboard denied"));
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    render(<ExportHardwareInfo />);

    await userEvent.click(
      screen.getByRole("button", { name: "Copy hardware report" }),
    );

    expect(mocks.record).toHaveBeenCalledTimes(1);
    expect(mocks.record).toHaveBeenCalledWith({
      capability: "hardwareReportExported",
    });
    consoleError.mockRestore();
  });
});
