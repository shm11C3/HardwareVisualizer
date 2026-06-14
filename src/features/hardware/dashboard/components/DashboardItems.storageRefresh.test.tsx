import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { StorageHealthRecord } from "@/rspc/bindings";
import { commands } from "@/rspc/bindings";
import { StorageDataInfo } from "./DashboardItems";

const mocks = vi.hoisted(() => {
  const translations = {
    "pages.dashboard.storageHealth.errors.fetchLatest":
      "Failed to fetch storage health data.",
    "pages.dashboard.storageHealth.errors.refresh":
      "Failed to re-detect storage devices.",
    "pages.dashboard.storageHealth.lastRecorded": "Last recorded 2026-05-10",
    "pages.dashboard.storageHealth.metrics.powerOn": "Power-on",
    "pages.dashboard.storageHealth.metrics.temperature": "Temp",
    "pages.dashboard.storageHealth.refresh": "Re-detect storage devices",
    "pages.dashboard.storageHealth.title": "Storage Health",
    "pages.dashboard.storageHealth.temperatureSources.record":
      "Record: 2026-05-10 09:12",
    "shared.driveFileSystem": "File system",
    "shared.driveType": "Type",
  } as Record<string, string>;

  return {
    commands: {
      getLiveStorageHealth: vi.fn(),
      getStorageHealthLatestRecords: vi.fn(),
      refreshStorageDevices: vi.fn(),
    },
    dialogError: vi.fn(),
    settings: {
      storageHealth: {
        enabled: true,
      },
    },
    storage: [
      {
        name: "Disk A",
        size: 100,
        sizeUnit: "GB",
        free: 60,
        freeUnit: "GB",
        storageType: "ssd",
        fileSystem: "APFS",
      },
    ],
    t: (key: string) => translations[key] ?? key,
    translations,
  };
});

vi.mock("@tauri-apps/plugin-os", () => ({
  platform: () => "macos",
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: mocks.t,
  }),
}));

vi.mock("@/components/charts/Bar", () => ({
  StorageBarChart: () => <div data-testid="storage-bar-chart" />,
}));

vi.mock("@/features/hardware/hooks/useHardwareInfoAtom", () => ({
  useHardwareInfoAtom: () => ({
    hardwareInfo: {
      storage: mocks.storage,
    },
  }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: mocks.settings,
  }),
}));

vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({
    error: mocks.dialogError,
  }),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: mocks.commands,
}));

const todayDateKey = () => new Date().toISOString().slice(0, 10);

const record = (
  overrides: Partial<StorageHealthRecord> = {},
): StorageHealthRecord => ({
  deviceId: "disk-a",
  displayName: "Disk A",
  model: "Example SSD",
  protocol: "NVMe",
  capacityBytes: 1_000_000,
  date: todayDateKey(),
  healthStatus: "good",
  warningLevel: "none",
  temperatureCelsius: 42,
  powerOnHours: 100,
  percentageUsed: null,
  availableSparePercent: null,
  reallocatedSectorCount: null,
  currentPendingSectorCount: null,
  offlineUncorrectableCount: null,
  mediaErrors: null,
  errorLogEntries: null,
  unsafeShutdownCount: null,
  warningReasons: [],
  collectedAt: `${todayDateKey()}T09:12:00Z`,
  ...overrides,
});

describe("StorageDataInfo storage device re-detection", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "ResizeObserver",
      class ResizeObserver {
        observe() {}
        unobserve() {}
        disconnect() {}
      },
    );
    vi.clearAllMocks();
    mocks.settings.storageHealth.enabled = true;
    mocks.commands.getLiveStorageHealth.mockResolvedValue({
      status: "ok",
      data: [],
    });
    mocks.commands.getStorageHealthLatestRecords.mockResolvedValue({
      status: "ok",
      data: [record()],
    });
    mocks.commands.refreshStorageDevices.mockResolvedValue({
      status: "ok",
      data: [record({ temperatureCelsius: 45 })],
    });
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("re-detects storage devices from the refresh button", async () => {
    const user = userEvent.setup();
    render(<StorageDataInfo />);

    await screen.findByRole("button", { name: "Re-detect storage devices" });
    await user.click(
      screen.getByRole("button", { name: "Re-detect storage devices" }),
    );

    expect(commands.refreshStorageDevices).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(screen.getByText("45°C")).toBeInTheDocument());
  });

  it("shows an inline error while keeping the existing summary", async () => {
    const user = userEvent.setup();
    mocks.commands.refreshStorageDevices.mockResolvedValueOnce({
      status: "error",
      error: "refresh failed",
    });
    render(<StorageDataInfo />);

    await screen.findByText("42°C");
    await user.click(
      screen.getByRole("button", { name: "Re-detect storage devices" }),
    );

    expect(
      await screen.findByText(/Failed to re-detect storage devices/),
    ).toBeInTheDocument();
    expect(screen.getByText("42°C")).toBeInTheDocument();
  });

  it("does not offer refresh when Storage Health is disabled", () => {
    mocks.settings.storageHealth.enabled = false;

    render(<StorageDataInfo />);

    expect(
      screen.queryByRole("button", { name: "Re-detect storage devices" }),
    ).not.toBeInTheDocument();
    expect(commands.getStorageHealthLatestRecords).not.toHaveBeenCalled();
    expect(commands.getLiveStorageHealth).not.toHaveBeenCalled();
  });
});
