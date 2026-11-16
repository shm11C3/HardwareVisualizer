import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProcessInfo } from "@/rspc/bindings";

const mocks = vi.hoisted(() => ({ processes: [] as ProcessInfo[] }));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/features/hardware/hooks/useProcessInfo", () => ({
  useProcessInfo: () => ({ processes: mocks.processes, hasError: false }),
}));

vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: { selectedBackgroundImg: null, backgroundImgOpacity: 100 },
  }),
}));

import { ProcessesTable } from "./ProcessTable";

describe("ProcessesTable GPU column", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders a measured value with % and an unavailable reading as a dash, never 0%", () => {
    mocks.processes = [
      { pid: 1, name: "a", cpuUsage: "1", memoryUsage: "2", gpuUsage: "130.5" },
      { pid: 2, name: "b", cpuUsage: "1", memoryUsage: "2", gpuUsage: null },
    ];

    render(<ProcessesTable />);

    const rows = screen.getAllByRole("row");
    expect(within(rows[1] as HTMLElement).getByText("130.5%")).toBeVisible();
    const unavailable = within(rows[2] as HTMLElement).getByText("—");
    expect(unavailable).toHaveAttribute(
      "title",
      "shared.processGpuUsageUnavailable",
    );
    expect(within(rows[2] as HTMLElement).queryByText("0%")).toBeNull();
  });

  it("sorts GPU usage with Enter and Space and exposes the direction", async () => {
    const user = userEvent.setup();
    mocks.processes = [
      { pid: 1, name: "high", cpuUsage: "1", memoryUsage: "2", gpuUsage: "80" },
      { pid: 2, name: "low", cpuUsage: "1", memoryUsage: "2", gpuUsage: "10" },
    ];
    render(<ProcessesTable />);

    const button = screen.getByRole("button", { name: "shared.gpuUsage" });
    const header = screen.getByRole("columnheader", {
      name: "shared.gpuUsage",
    });
    expect(header).not.toHaveAttribute("aria-sort");
    // The expand control precedes the GPU sort button in the tab order.
    await user.tab();
    await user.tab();
    expect(button).toHaveFocus();

    await user.keyboard("{Enter}");
    expect(header).toHaveAttribute("aria-sort", "ascending");
    expect(
      within(screen.getAllByRole("row")[1] as HTMLElement).getByText("low"),
    ).toBeVisible();

    await user.keyboard(" ");
    expect(header).toHaveAttribute("aria-sort", "descending");
    expect(
      within(screen.getAllByRole("row")[1] as HTMLElement).getByText("high"),
    ).toBeVisible();
  });
});
