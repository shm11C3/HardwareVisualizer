import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import type { ExternalComponentGuidanceCandidate } from "@/rspc/bindings";
import { ExternalComponentGuidanceDialog } from "./ExternalComponentGuidanceDialog";

const mocks = vi.hoisted(() => ({
  acknowledgeExternalComponentGuidanceKey: vi.fn(),
  deferExternalComponentGuidanceForSession: vi.fn(),
  error: vi.fn(),
  getExternalComponentGuidanceCandidates: vi.fn(),
  openURL: vi.fn(),
  platform: vi.fn(() => "windows"),
  setElevatedStartupMode: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-os", () => ({
  platform: mocks.platform,
}));

vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({
    error: mocks.error,
  }),
}));

vi.mock("@/lib/openUrl", () => ({
  openURL: mocks.openURL,
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    acknowledgeExternalComponentGuidanceKey:
      mocks.acknowledgeExternalComponentGuidanceKey,
    deferExternalComponentGuidanceForSession:
      mocks.deferExternalComponentGuidanceForSession,
    getExternalComponentGuidanceCandidates:
      mocks.getExternalComponentGuidanceCandidates,
    setElevatedStartupMode: mocks.setElevatedStartupMode,
  },
}));

const candidate = (
  overrides: Partial<ExternalComponentGuidanceCandidate> = {},
): ExternalComponentGuidanceCandidate => ({
  key: "pawnio:cpu-package-temperature:v1",
  component: "pawnio",
  usage: "cpuPackageTemperature",
  reasonKind: "permission",
  missingSignals: ["cpu-temperature"],
  affectedDeviceCount: null,
  diagnosticDetail: "pawnio_open failed: 0x80070005",
  ...overrides,
});

describe("ExternalComponentGuidanceDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.platform.mockReturnValue("windows");
    mocks.getExternalComponentGuidanceCandidates.mockResolvedValue({
      status: "ok",
      data: [candidate()],
    });
    mocks.setElevatedStartupMode.mockResolvedValue({
      status: "ok",
      data: null,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("shows the elevated startup action instead of details for Windows permission guidance", async () => {
    const user = userEvent.setup();

    render(<ExternalComponentGuidanceDialog displayTarget="dashboard" />);

    await user.click(
      await screen.findByRole("button", {
        name: "Restart as administrator",
      }),
    );

    expect(screen.queryByRole("button", { name: "Open details" })).toBeNull();
    expect(mocks.setElevatedStartupMode).toHaveBeenCalledWith(true);
    expect(mocks.openURL).not.toHaveBeenCalled();
  });

  it("keeps the details action for permission guidance outside Windows", async () => {
    const user = userEvent.setup();
    mocks.platform.mockReturnValue("linux");

    render(<ExternalComponentGuidanceDialog displayTarget="dashboard" />);

    await user.click(
      await screen.findByRole("button", {
        name: "Open details",
      }),
    );

    expect(
      screen.queryByRole("button", { name: "Restart as administrator" }),
    ).toBeNull();
    expect(mocks.openURL).toHaveBeenCalledTimes(1);
    expect(mocks.setElevatedStartupMode).not.toHaveBeenCalled();
  });
});
