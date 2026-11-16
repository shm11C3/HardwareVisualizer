import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SystemSpecificationsActions } from "./SystemSpecificationsActions";

vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({
  writeText: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/features/hardware/hooks/useHardwareInfoAtom", () => ({
  useHardwareInfoAtom: () => ({
    hardwareInfo: {
      cpu: null,
      memory: null,
      gpus: null,
      storage: [],
      motherboard: null,
    },
    networkInfo: [],
  }),
}));

vi.mock("@/features/hardware/hooks/useProcessInfo", () => ({
  useProcessInfo: () => ({ processes: [] }),
}));

vi.mock("@/rspc/bindings", () => ({ commands: {} }));

const copyKey = "pages.dashboard.systemSpecifications.copySpecs";

describe("SystemSpecificationsActions copy feedback", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.mocked(writeText).mockReset();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("confirms a successful copy for a moment, then returns to the copy label", async () => {
    vi.mocked(writeText).mockResolvedValue(undefined);
    render(<SystemSpecificationsActions />);

    fireEvent.click(screen.getByRole("button", { name: copyKey }));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });

    expect(screen.getByText("pages.dashboard.copied")).toBeVisible();
    expect(screen.queryByTestId("action-failure")).toBeNull();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(2000);
    });
    expect(screen.getByText(copyKey)).toBeVisible();
  });

  it("shows a failure line under the button when the clipboard write is rejected", async () => {
    vi.mocked(writeText).mockRejectedValue(new Error("denied"));
    render(<SystemSpecificationsActions />);

    fireEvent.click(screen.getByRole("button", { name: copyKey }));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });

    expect(screen.getByTestId("action-failure")).toHaveTextContent(
      "pages.dashboard.copyFailed",
    );
    expect(screen.queryByText("pages.dashboard.copied")).toBeNull();
  });
});
