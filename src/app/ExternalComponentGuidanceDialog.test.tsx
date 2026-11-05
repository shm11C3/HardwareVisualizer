import { cleanup, render, screen, waitFor } from "@testing-library/react";
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

const storageCandidate = () =>
  candidate({
    key: "smartctl:storage-health:v1",
    component: "smartctl",
    usage: "storageHealth",
    missingSignals: ["smart-overall-health"],
  });

describe("ExternalComponentGuidanceDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.platform.mockReturnValue("windows");
    mocks.useProcessElevated.mockReturnValue(false);
    mocks.useElevationAvailability.mockReturnValue("available");
    mocks.getExternalComponentGuidanceCandidates.mockResolvedValue({
      status: "ok",
      data: [storageCandidate()],
    });
    mocks.acknowledgeExternalComponentGuidanceKey.mockResolvedValue({
      status: "ok",
      data: null,
    });
    mocks.deferExternalComponentGuidanceForSession.mockResolvedValue({
      status: "ok",
      data: null,
    });
    mocks.updateSettingAtom.mockResolvedValue(true);
  });
  afterEach(cleanup);

  it("retains explicit elevated startup for storage permission guidance", async () => {
    render(<ExternalComponentGuidanceDialog displayTarget="dashboard" />);
    await userEvent.click(
      await screen.findByRole("button", {
        name: "Enable Run as administrator on startup",
      }),
    );
    expect(mocks.updateSettingAtom).toHaveBeenCalledWith(
      "elevatedStartupMode",
      true,
    );
  });

  it("leaves sensor guidance to the monitor stream rather than a modal", async () => {
    mocks.getExternalComponentGuidanceCandidates.mockResolvedValue({
      status: "ok",
      data: [candidate()],
    });
    const onPendingChange = vi.fn();
    render(
      <ExternalComponentGuidanceDialog
        displayTarget="dashboard"
        onPendingChange={onPendingChange}
      />,
    );
    await waitFor(() =>
      expect(onPendingChange).toHaveBeenLastCalledWith(false),
    );
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it.each(["performance", "cpuDetail", "settings"] as const)(
    "does not query component guidance on %s",
    (displayTarget) => {
      render(<ExternalComponentGuidanceDialog displayTarget={displayTarget} />);
      expect(
        mocks.getExternalComponentGuidanceCandidates,
      ).not.toHaveBeenCalled();
    },
  );

  it.each([true, null])(
    "does not offer app elevation with process status %s",
    async (value) => {
      mocks.useProcessElevated.mockReturnValue(value);
      render(<ExternalComponentGuidanceDialog displayTarget="dashboard" />);
      expect(
        await screen.findByRole("button", { name: "Open details" }),
      ).toBeInTheDocument();
    },
  );

  it("does not offer elevation from an unprotected installation", async () => {
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
  });

  it.each([false, true])(
    "preserves storage guidance dismissal choices (permanent=%s)",
    async (permanent) => {
      render(<ExternalComponentGuidanceDialog displayTarget="dashboard" />);
      await userEvent.click(
        await screen.findByRole("button", { name: "Hide" }),
      );
      await userEvent.click(
        screen.getByRole("menuitem", {
          name: permanent ? "Never show again" : "Hide for this session",
        }),
      );
      expect(
        permanent
          ? mocks.acknowledgeExternalComponentGuidanceKey
          : mocks.deferExternalComponentGuidanceForSession,
      ).toHaveBeenCalledWith("smartctl:storage-health:v1");
      expect(screen.queryByRole("alertdialog")).toBeNull();
    },
  );

  it("defers storage guidance behind an active startup flow", async () => {
    const { rerender } = render(
      <ExternalComponentGuidanceDialog displayTarget="dashboard" deferred />,
    );
    await waitFor(() =>
      expect(
        mocks.getExternalComponentGuidanceCandidates,
      ).toHaveBeenCalledOnce(),
    );
    expect(screen.queryByRole("alertdialog")).toBeNull();
    rerender(
      <ExternalComponentGuidanceDialog
        displayTarget="dashboard"
        deferred={false}
      />,
    );
    expect(await screen.findByRole("alertdialog")).toBeInTheDocument();
  });
});
