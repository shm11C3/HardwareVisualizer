import { act, render } from "@testing-library/react";
import { Provider } from "jotai";
import type React from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useHardwareEventListener } from "@/features/hardware/hooks/useHardwareEventListener";
import type {
  GpuMonitorData,
  HardwareMonitorUpdate,
  SysInfo,
} from "@/rspc/bindings";
import { CPUInfo } from "./DashboardItems";

type HardwareMonitorUpdateWithOptionalThermals = HardwareMonitorUpdate & {
  cpuTemperature?: number | null;
  sensorTemperatures?: { name: string; value: number }[];
};

const counters = vi.hoisted(() => ({
  capturedHardwareUpdateListener: null as
    | ((event: { payload: HardwareMonitorUpdate }) => void)
    | null,
  infoTableRenders: 0,
  thermalSensorTableRenders: 0,
  thermalSensorRowRenderWork: 0,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/rspc/bindings", () => ({
  events: {
    hardwareMonitorUpdate: {
      listen: vi.fn(
        (listener: (event: { payload: HardwareMonitorUpdate }) => void) => {
          counters.capturedHardwareUpdateListener = listener;
          return Promise.resolve(vi.fn());
        },
      ),
    },
  },
}));

vi.mock("@/components/shared/InfoTable", () => ({
  InfoTable: ({
    className,
    data,
  }: {
    className?: string;
    data: Record<string, React.ReactNode>;
  }) => {
    const keys = Object.keys(data);
    counters.infoTableRenders += 1;
    if (keys.some((key) => key.startsWith("Thermal Zone"))) {
      counters.thermalSensorTableRenders += 1;
      counters.thermalSensorRowRenderWork += keys.length;
    }

    return (
      <div className={className} data-testid="info-table">
        {keys.join(",")}
      </div>
    );
  },
}));

vi.mock("@/components/charts/DoughnutChart", () => ({
  DoughnutChart: () => <div data-testid="doughnut-chart" />,
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: {
      temperatureUnit: "C",
    },
  }),
}));

vi.mock("./MiniLineChart", () => ({
  MiniLineChart: () => <div data-testid="mini-line-chart" />,
}));

const hardwareInfo = {
  cpu: {
    name: "Test CPU",
    vendor: "Test Vendor",
    coreCount: 8,
    clock: 3200,
    clockUnit: "MHz",
    cpuName: "Test CPU",
  },
  memory: null,
  gpus: null,
  storage: [],
  motherboard: null,
} satisfies SysInfo;

vi.mock("@/features/hardware/hooks/useHardwareInfoAtom", () => ({
  useHardwareInfoAtom: () => ({
    hardwareInfo,
  }),
}));

vi.mock("@/features/hardware/hooks/useProcessInfo", () => ({
  useProcessInfo: () => [],
}));

const makeGpu = (overrides: Partial<GpuMonitorData> = {}): GpuMonitorData => ({
  gpuId: "gpu:0",
  gpuName: "Test GPU",
  gpuUsage: 70,
  gpuTemperature: 65,
  gpuSource: "Test",
  gpuDedicatedMemoryUsageKb: null,
  gpuCoolerLevel: null,
  ...overrides,
});

const makePayload = (
  overrides: Partial<
    Omit<HardwareMonitorUpdateWithOptionalThermals, "gpus">
  > & {
    gpus?: GpuMonitorData[];
  } = {},
): HardwareMonitorUpdate => {
  const { gpus, ...rest } = overrides;

  return {
    cpuUsage: 50,
    memoryUsage: 60,
    gpus: gpus ?? [makeGpu()],
    processorsUsage: [40, 50],
    ...rest,
  } as HardwareMonitorUpdate;
};

const emitHardwareUpdate = (payload: HardwareMonitorUpdate) => {
  if (!counters.capturedHardwareUpdateListener) {
    throw new Error("hardware update listener was not registered");
  }

  counters.capturedHardwareUpdateListener({ payload });
};

const setDocumentHidden = (hidden: boolean) => {
  Object.defineProperty(document, "hidden", {
    configurable: true,
    value: hidden,
  });
};

const CpuDashboardProbe = () => {
  useHardwareEventListener();

  return <CPUInfo />;
};

const unchangedSensorTemperatures = Array.from({ length: 24 }, (_, index) => ({
  name: `Thermal Zone ${index}`,
  value: 50 + (index % 3),
}));

describe("Dashboard live metrics render fanout", () => {
  beforeEach(() => {
    counters.capturedHardwareUpdateListener = null;
    counters.infoTableRenders = 0;
    counters.thermalSensorTableRenders = 0;
    counters.thermalSensorRowRenderWork = 0;
    setDocumentHidden(false);
  });

  it("does not amplify unchanged CPU thermal sensor payloads into per-tick row render work", () => {
    render(
      <Provider>
        <CpuDashboardProbe />
      </Provider>,
    );

    act(() => {
      emitHardwareUpdate(
        makePayload({
          cpuUsage: 10,
          cpuTemperature: 50,
          sensorTemperatures: unchangedSensorTemperatures,
        }),
      );
    });

    counters.thermalSensorTableRenders = 0;
    counters.thermalSensorRowRenderWork = 0;

    for (let i = 0; i < 5; i += 1) {
      act(() => {
        emitHardwareUpdate(
          makePayload({
            cpuUsage: 20 + i,
            cpuTemperature: 50,
            sensorTemperatures: unchangedSensorTemperatures,
          }),
        );
      });
    }

    expect(counters.thermalSensorRowRenderWork).toBe(0);
  });
});
