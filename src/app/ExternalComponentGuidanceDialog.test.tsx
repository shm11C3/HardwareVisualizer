import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
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
  updateSettingAtom: vi.fn(),
  useElevationAvailability: vi.fn((): string | null => "available"),
  useProcessElevated: vi.fn((): boolean | null => false),
}));

vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: { elevatedStartupMode: false },
    updateSettingAtom: mocks.updateSettingAtom,
  }),
}));

vi.mock(
  "@/hooks/settings/useElevationAvailability",
  async (importOriginal) => ({
    ...(await importOriginal<
      typeof import("@/hooks/settings/useElevationAvailability")
    >()),
    useElevationAvailability: mocks.useElevationAvailability,
    useProcessElevated: mocks.useProcessElevated,
  }),
);

vi.mock("@tauri-apps/plugin-os", () => ({
  platform: mocks.platform,
}));

vi.mock("@/hooks/tauri/useTauriDialog", () => ({
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
    mocks.useElevationAvailability.mockReturnValue("available");
    mocks.useProcessElevated.mockReturnValue(false);
    mocks.getExternalComponentGuidanceCandidates.mockResolvedValue({
      status: "ok",
      data: [candidate()],
    });
    mocks.setElevatedStartupMode.mockResolvedValue({
      status: "ok",
      data: null,
    });
    mocks.updateSettingAtom.mockResolvedValue(true);
  });

  afterEach(() => {
    cleanup();
  });

  it("shows the elevated startup action instead of details for Windows permission guidance", async () => {
    const user = userEvent.setup();

    render(<ExternalComponentGuidanceDialog displayTarget="dashboard" />);

    await user.click(
      await screen.findByRole("button", {
        name: "Enable Run as administrator on startup",
      }),
    );

    expect(screen.queryByRole("button", { name: "Open details" })).toBeNull();
    // Through the settings atom so the toggle reflects the saved value even
    // when the backend persists without relaunching (already elevated).
    expect(mocks.updateSettingAtom).toHaveBeenCalledWith(
      "elevatedStartupMode",
      true,
    );
    expect(mocks.setElevatedStartupMode).not.toHaveBeenCalled();
    expect(mocks.openURL).not.toHaveBeenCalled();
  });

  it("shows the localized error dialog when the settings updater rethrows", async () => {
    const user = userEvent.setup();
    // The atom restores its own value and rethrows a rejected command; the
    // dialog owns the user-facing message.
    mocks.updateSettingAtom.mockRejectedValue(new Error("ipc failed"));
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    render(<ExternalComponentGuidanceDialog displayTarget="dashboard" />);

    await user.click(
      await screen.findByRole("button", {
        name: "Enable Run as administrator on startup",
      }),
    );

    await waitFor(() => expect(mocks.error).toHaveBeenCalledTimes(1));
    expect(mocks.error).toHaveBeenCalledWith(
      expect.stringMatching(/administrator/i),
    );
    expect(consoleError).toHaveBeenCalled();
    expect(
      screen.getByRole("button", {
        name: "Enable Run as administrator on startup",
      }),
    ).toBeEnabled();
    consoleError.mockRestore();
  });

  it("offers details instead of a restart when elevation is refused here", async () => {
    mocks.useElevationAvailability.mockReturnValue("unprotectedLocation");

    render(<ExternalComponentGuidanceDialog displayTarget="dashboard" />);

    expect(
      await screen.findByRole("button", { name: "Open details" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", {
        name: "Enable Run as administrator on startup",
      }),
    ).toBeNull();
    expect(
      screen.getByText(/not installed under Program Files/),
    ).toBeInTheDocument();
    expect(screen.queryByText(/enable Run as administrator/)).toBeNull();
  });

  it("does not blame the install folder when availability is unknown", async () => {
    mocks.useElevationAvailability.mockReturnValue("unknown");

    render(<ExternalComponentGuidanceDialog displayTarget="dashboard" />);

    expect(
      await screen.findByText(/could not verify whether it can safely run/),
    ).toBeInTheDocument();
    expect(screen.queryByText(/not installed under Program Files/)).toBeNull();
  });

  it("reports whether it is open so other startup dialogs can wait", async () => {
    const onOpenChange = vi.fn();

    render(
      <ExternalComponentGuidanceDialog
        displayTarget="dashboard"
        onOpenChange={onOpenChange}
      />,
    );

    expect(onOpenChange).toHaveBeenLastCalledWith(false);
    await screen.findByRole("button", {
      name: "Enable Run as administrator on startup",
    });
    expect(onOpenChange).toHaveBeenLastCalledWith(true);
  });

  it("waits while another startup dialog is open and shows after it closes", async () => {
    const onOpenChange = vi.fn();
    const onPendingChange = vi.fn();
    let resolveCandidates: (value: unknown) => void = () => {};
    mocks.getExternalComponentGuidanceCandidates.mockReturnValue(
      new Promise((resolve) => {
        resolveCandidates = resolve;
      }),
    );
    const { rerender } = render(
      <ExternalComponentGuidanceDialog
        displayTarget="dashboard"
        deferred
        onOpenChange={onOpenChange}
        onPendingChange={onPendingChange}
      />,
    );
    expect(onPendingChange).toHaveBeenLastCalledWith(true);

    // The candidate arrives while still deferred: not shown, and still
    // reported pending, since it opens as soon as the blocker closes.
    await act(async () => {
      resolveCandidates({ status: "ok", data: [candidate()] });
    });
    expect(onPendingChange).toHaveBeenLastCalledWith(true);
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(onOpenChange).toHaveBeenLastCalledWith(false);

    rerender(
      <ExternalComponentGuidanceDialog
        displayTarget="dashboard"
        deferred={false}
        onOpenChange={onOpenChange}
        onPendingChange={onPendingChange}
      />,
    );
    expect(
      await screen.findByRole("button", {
        name: "Enable Run as administrator on startup",
      }),
    ).toBeInTheDocument();
    expect(onOpenChange).toHaveBeenLastCalledWith(true);
    expect(onPendingChange).toHaveBeenLastCalledWith(false);
  });

  it("is pending again in the render that switches to a screen with guidance", async () => {
    const onPendingChange = vi.fn();
    const { rerender } = render(
      <ExternalComponentGuidanceDialog
        displayTarget="usage"
        onPendingChange={onPendingChange}
      />,
    );
    expect(onPendingChange).toHaveBeenLastCalledWith(false);

    let resolveCandidates: (value: unknown) => void = () => {};
    mocks.getExternalComponentGuidanceCandidates.mockReturnValue(
      new Promise((resolve) => {
        resolveCandidates = resolve;
      }),
    );
    rerender(
      <ExternalComponentGuidanceDialog
        displayTarget="dashboard"
        onPendingChange={onPendingChange}
      />,
    );
    // Reported before the lookup even starts, not after an effect.
    expect(onPendingChange).toHaveBeenLastCalledWith(true);

    await act(async () => {
      resolveCandidates({ status: "ok", data: [] });
    });
    expect(onPendingChange).toHaveBeenLastCalledWith(false);
  });

  it("is not pending on a screen without guidance", () => {
    const onPendingChange = vi.fn();
    render(
      <ExternalComponentGuidanceDialog
        displayTarget="usage"
        onPendingChange={onPendingChange}
      />,
    );
    expect(onPendingChange).toHaveBeenLastCalledWith(false);
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
      screen.queryByRole("button", {
        name: "Enable Run as administrator on startup",
      }),
    ).toBeNull();
    expect(mocks.openURL).toHaveBeenCalledTimes(1);
    expect(mocks.updateSettingAtom).not.toHaveBeenCalled();
    expect(mocks.setElevatedStartupMode).not.toHaveBeenCalled();
  });
  it.each([true, null])(
    "does not offer elevation when process status is %s",
    async (elevated) => {
      mocks.useProcessElevated.mockReturnValue(elevated);
      render(<ExternalComponentGuidanceDialog displayTarget="dashboard" />);
      await screen.findByRole("alertdialog");
      expect(
        screen.queryByRole("button", {
          name: "Enable Run as administrator on startup",
        }),
      ).toBeNull();
    },
  );

  it("filters Performance candidates before selecting the first relevant reading", async () => {
    mocks.getExternalComponentGuidanceCandidates.mockResolvedValue({
      status: "ok",
      data: [
        candidate({
          key: "smartctl:storage-health:v1",
          component: "smartctl",
          usage: "storageHealth",
        }),
        candidate({
          key: "pawnio:motherboard-sensors:v1",
          usage: "motherboardSensors",
        }),
        candidate(),
      ],
    });
    const context = {
      settled: true,
      cpuTemperatureVisible: true,
      motherboardSensorsVisible: false,
      expanded: false,
    };
    const { rerender } = render(
      <ExternalComponentGuidanceDialog
        displayTarget="performance"
        performanceContext={context}
      />,
    );
    await screen.findByText(/CPU package temperature/);
    expect(mocks.getExternalComponentGuidanceCandidates).toHaveBeenCalledWith(
      "performance",
    );
    rerender(
      <ExternalComponentGuidanceDialog
        displayTarget="performance"
        performanceContext={{ ...context, expanded: true }}
      />,
    );
    expect(screen.queryByRole("alertdialog")).toBeNull();
    rerender(
      <ExternalComponentGuidanceDialog
        displayTarget="performance"
        performanceContext={{
          ...context,
          cpuTemperatureVisible: false,
          motherboardSensorsVisible: true,
        }}
      />,
    );
    await screen.findByText(/Motherboard temperature sensors and fan speeds/);
    rerender(
      <ExternalComponentGuidanceDialog
        displayTarget="performance"
        performanceContext={{ ...context, cpuTemperatureVisible: false }}
      />,
    );
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it.each([true, null])(
    "keeps Performance guidance hidden with elevated status %s",
    async (elevated) => {
      mocks.useProcessElevated.mockReturnValue(elevated);
      const onPendingChange = vi.fn();
      render(
        <ExternalComponentGuidanceDialog
          displayTarget="performance"
          performanceContext={{
            settled: true,
            cpuTemperatureVisible: true,
            motherboardSensorsVisible: true,
            expanded: false,
          }}
          onPendingChange={onPendingChange}
        />,
      );
      await waitFor(() =>
        expect(onPendingChange).toHaveBeenLastCalledWith(false),
      );
      expect(screen.queryByRole("alertdialog")).toBeNull();
    },
  );

  it("does not offer the CTA while elevation availability is loading", async () => {
    mocks.useElevationAvailability.mockReturnValue(null);
    const { rerender } = render(
      <ExternalComponentGuidanceDialog displayTarget="dashboard" />,
    );
    await waitFor(() =>
      expect(mocks.getExternalComponentGuidanceCandidates).toHaveBeenCalled(),
    );
    expect(screen.queryByRole("alertdialog")).toBeNull();
    mocks.useElevationAvailability.mockReturnValue("available");
    rerender(<ExternalComponentGuidanceDialog displayTarget="dashboard" />);
    await screen.findByRole("button", {
      name: "Enable Run as administrator on startup",
    });
    expect(screen.getByText(/requests Windows approval/)).toBeInTheDocument();
  });

  it("keeps the candidate and reports failure when enabling the setting is cancelled", async () => {
    mocks.updateSettingAtom.mockResolvedValue(false);
    const user = userEvent.setup();
    render(<ExternalComponentGuidanceDialog displayTarget="dashboard" />);
    await user.click(
      await screen.findByRole("button", {
        name: "Enable Run as administrator on startup",
      }),
    );
    expect(mocks.updateSettingAtom).toHaveBeenCalledWith(
      "elevatedStartupMode",
      true,
    );
    expect(mocks.error).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
    expect(
      mocks.acknowledgeExternalComponentGuidanceKey,
    ).not.toHaveBeenCalled();
  });
});
