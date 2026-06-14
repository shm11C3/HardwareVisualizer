import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Provider } from "jotai";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { StorageHealthRecord } from "@/rspc/bindings";
import { StorageDataInfo } from "./DashboardItems";

const mocks = vi.hoisted(() => {
  const translations = {
    "pages.dashboard.storageHealth.deviceSelector": "Select storage device",
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

const TEST_DATE = "2026-05-10";

const record = (
  overrides: Partial<StorageHealthRecord> = {},
): StorageHealthRecord => ({
  deviceId: "disk-a",
  displayName: "Disk A",
  model: "Disk A SSD",
  protocol: "NVMe",
  capacityBytes: 1_000_000,
  date: TEST_DATE,
  healthStatus: "good",
  warningLevel: "none",
  temperatureCelsius: 38,
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
  collectedAt: `${TEST_DATE}T09:12:00Z`,
  ...overrides,
});

const renderStorage = () =>
  render(
    <Provider>
      <StorageDataInfo />
    </Provider>,
  );

describe("StorageDataInfo storage device selection", () => {
  beforeEach(() => {
    vi.setSystemTime(new Date(`${TEST_DATE}T12:00:00`));
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
      data: [
        record({
          deviceId: "disk-a",
          displayName: "Disk A",
          model: "Disk A SSD",
          healthStatus: "good",
          warningLevel: "none",
          temperatureCelsius: 38,
        }),
        record({
          deviceId: "disk-b",
          displayName: "Disk B",
          model: "Disk B SSD",
          healthStatus: "warning",
          warningLevel: "warning",
          temperatureCelsius: 45,
          warningReasons: ["Temperature is elevated (45°C)"],
        }),
      ],
    });
    mocks.commands.refreshStorageDevices.mockResolvedValue({
      status: "ok",
      data: [],
    });
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("switches the displayed device metrics when another device is selected", async () => {
    const user = userEvent.setup();
    renderStorage();

    // Default focus device is the worst-health drive (Disk B, warning) → 45°C
    await screen.findByText("45°C");

    // Explicitly select the healthy drive
    await user.click(screen.getByRole("tab", { name: /Disk A SSD/ }));

    // The overview now follows the selected device
    await waitFor(() => expect(screen.getByText("38°C")).toBeInTheDocument());
    expect(
      screen.getByRole("tab", { name: /Disk A SSD/, selected: true }),
    ).toBeInTheDocument();
  });

  it("does not render a device selector when only one device is present", async () => {
    mocks.commands.getStorageHealthLatestRecords.mockResolvedValue({
      status: "ok",
      data: [
        record({
          deviceId: "disk-a",
          model: "Disk A SSD",
          temperatureCelsius: 38,
        }),
      ],
    });
    renderStorage();

    await screen.findByText("38°C");
    expect(screen.queryByRole("tab")).not.toBeInTheDocument();
  });
});
