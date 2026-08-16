import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SysInfo } from "@/rspc/bindings";
import { SystemSpecifications } from "./SystemSpecifications";

const state = vi.hoisted(() => ({
  hardwareInfo: {
    cpu: null,
    memory: null,
    gpus: null,
    storage: [],
    motherboard: null,
  } as SysInfo,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@tauri-apps/plugin-os", () => ({
  platform: () => "windows",
  version: () => "10.0.26100",
  arch: () => "x86_64",
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: {
      lineGraphColor: {
        cpu: "75, 192, 192",
        memory: "255, 99, 132",
        gpu: "255, 206, 86",
      },
    },
  }),
}));

vi.mock("@/features/hardware/hooks/useHardwareInfoAtom", () => ({
  useHardwareInfoAtom: () => ({
    hardwareInfo: state.hardwareInfo,
    init: vi.fn().mockResolvedValue(undefined),
  }),
}));

vi.mock("@/features/hardware/dashboard/components/DashboardItems", () => ({
  FetchDetailButton: () => <button type="button">load</button>,
  NetworkInfo: () => <div data-testid="network-info" />,
  StorageDataInfo: () => <div data-testid="storage-data-info" />,
}));

vi.mock("@/features/hardware/dashboard/components/ExportHardwareInfo", () => ({
  ExportHardwareInfo: () => <div data-testid="export-hardware-info" />,
}));

describe("SystemSpecifications", () => {
  beforeEach(() => {
    state.hardwareInfo = {
      cpu: null,
      memory: null,
      gpus: null,
      storage: [],
      motherboard: null,
    };
  });

  afterEach(cleanup);

  it("renders platform facts and reuses storage/network blocks while data loads", () => {
    render(<SystemSpecifications />);

    expect(screen.getByTestId("system-specifications-sheet")).toBeVisible();
    expect(screen.getByText("Windows")).toBeVisible();
    expect(screen.getByText("10.0.26100")).toBeVisible();
    expect(screen.getByText("x86_64")).toBeVisible();
    expect(screen.getByTestId("storage-data-info")).toBeVisible();
    expect(screen.getByTestId("network-info")).toBeVisible();
    expect(screen.getByTestId("export-hardware-info")).toBeVisible();
    // Motherboard facts are unavailable, so the section is omitted entirely.
    expect(screen.queryByText("shared.motherboard")).toBeNull();
  });

  it("lists CPU and motherboard facts as label-value rows", () => {
    state.hardwareInfo = {
      ...state.hardwareInfo,
      cpu: {
        name: "HV Fixture CPU 8-Core",
        vendor: "FixtureVendor",
        coreCount: 8,
        clock: 3600,
        clockUnit: "MHz",
        cpuName: "HV Fixture CPU 8-Core",
      } as unknown as SysInfo["cpu"],
      motherboard: {
        manufacturer: "FixtureVendor",
        product: "HV Fixture Board",
        version: "1.0",
        serialNumber: "E2E-0000",
        biosVendor: "FixtureVendor",
        biosVersion: "1.0.0",
        biosReleaseDate: "2024-01-01",
      } as unknown as SysInfo["motherboard"],
    };

    render(<SystemSpecifications />);

    expect(screen.getByText("HV Fixture CPU 8-Core")).toBeVisible();
    expect(screen.getByText("3600 MHz")).toBeVisible();
    expect(screen.getByText("shared.motherboard")).toBeVisible();
    expect(screen.getByText("HV Fixture Board")).toBeVisible();
    expect(screen.getByText("E2E-0000")).toBeVisible();
  });
});
