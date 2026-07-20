import { render, screen } from "@testing-library/react";
import { Provider } from "jotai";
import { useHydrateAtoms } from "jotai/utils";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  motherboardFanSpeedsAtom,
  motherboardTempsAtom,
} from "@/features/hardware/store/chart";
import type {
  MotherboardFanSpeedValues,
  MotherboardTemperatureValues,
} from "@/features/hardware/types/hardwareDataType";
import { MotherboardDataInfo } from "./DashboardItems";

const mocks = vi.hoisted(() => ({
  hardwareInfo: {
    motherboard: null,
  },
  settings: {
    temperatureUnit: "C",
  },
  t: (key: string) =>
    (
      ({
        "pages.dashboard.motherboardSensors.title": "Sensors",
        "pages.dashboard.motherboardSensors.temperatures": "Temperatures",
        "pages.dashboard.motherboardSensors.fanSpeeds": "Fan speeds",
        "pages.dashboard.motherboardSensors.status.active": "Active",
        "pages.dashboard.motherboardSensors.status.inactive": "Inactive",
        "pages.dashboard.motherboardSensors.status.invalid": "Invalid",
        "shared.notAvailable": "N/A",
      }) as Record<string, string>
    )[key] ?? key,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: mocks.t,
  }),
}));

vi.mock("@/features/hardware/hooks/useHardwareInfoAtom", () => ({
  useHardwareInfoAtom: () => ({
    hardwareInfo: mocks.hardwareInfo,
  }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: mocks.settings,
  }),
}));

vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({
    error: vi.fn(),
  }),
}));

const renderMotherboard = () => {
  const HydrateAtoms = ({ children }: { children: ReactNode }) => {
    const temperatures: MotherboardTemperatureValues = [
      { name: "SYSTIN", value: 41, source: "NCT6799D / Super I/O" },
    ];
    const fanSpeeds: MotherboardFanSpeedValues = [
      {
        name: "SYSFANIN",
        rpm: 0,
        status: "inactive",
        source: "NCT6799D / Super I/O",
      },
    ];

    useHydrateAtoms([[motherboardTempsAtom, temperatures]]);
    useHydrateAtoms([[motherboardFanSpeedsAtom, fanSpeeds]]);
    return <>{children}</>;
  };

  return render(
    <Provider>
      <HydrateAtoms>
        <MotherboardDataInfo />
      </HydrateAtoms>
    </Provider>,
  );
};

describe("MotherboardDataInfo sensor display", () => {
  beforeEach(() => {
    mocks.hardwareInfo.motherboard = null;
    mocks.settings.temperatureUnit = "C";
  });

  it("renders live Super I/O sensors even when static motherboard info is unavailable", () => {
    renderMotherboard();

    expect(screen.getByText("Sensors")).toBeInTheDocument();
    expect(screen.getByText("NCT6799D / Super I/O")).toBeInTheDocument();
    expect(screen.getByText("SYSTIN")).toBeInTheDocument();
    expect(screen.getByText("41 °C")).toBeInTheDocument();
    expect(screen.getByText("SYSFANIN")).toBeInTheDocument();
    expect(screen.getByText("0 RPM (Inactive)")).toBeInTheDocument();
  });
});
