import { act, cleanup, render, screen } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cpuUsageHistoryAtom,
  gpuTempMapAtom,
  memoryUsageHistoryAtom,
  selectedGpuIdAtom,
} from "@/features/hardware/store/chart";
import { Performance } from "./Performance";
import type { PerformanceLayoutPreset } from "./types/performanceLayout";

const state = vi.hoisted(() => ({
  preset: "detailed" as PerformanceLayoutPreset,
  chartRenders: { cpu: 0, memory: 0, gpu: 0 },
  processRenders: 0,
}));

const settings = vi.hoisted(() => ({
  graphFitToWindow: false,
  graphMarginPx: 16,
  graphSize: "xl",
  lineGraphMix: false,
  temperatureUnit: "C" as const,
  displayTargets: ["cpu", "memory", "gpu"],
  lineGraphColor: {
    cpu: "75, 192, 192",
    memory: "255, 99, 132",
    gpu: "255, 206, 86",
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({ settings }),
}));

vi.mock("@/hooks/useBurnInShift", () => ({
  useBurnInShift: () => ({
    rootStyle: {},
    shiftStyle: {},
    rootClassName: "",
    shiftClassName: "",
  }),
}));

vi.mock("@/components/charts/LineChart", () => ({
  LineChartComponent: (props: {
    dataType?: "cpu" | "memory" | "gpu";
    lineGraphMix: boolean;
  }) => {
    const dataType = props.dataType ?? "cpu";
    state.chartRenders[dataType] += 1;
    return <div data-testid={`chart-${dataType}`} />;
  },
}));

vi.mock("@/features/hardware/dashboard/components/ProcessTable", () => ({
  ProcessesTable: () => {
    state.processRenders += 1;
    return <div data-testid="live-process-table" />;
  },
}));

vi.mock("./hooks/usePerformanceLayout", () => ({
  usePerformanceLayout: () => ({
    preset: state.preset,
    setPreset: vi.fn(),
    customLayout: {
      order: ["currentValues", "usageGraphs", "processTable"],
      visible: ["currentValues"],
    },
    togglePanel: vi.fn(),
    handlePanelDragEnd: vi.fn(),
    isPending: false,
  }),
}));

describe("Performance", () => {
  beforeEach(() => {
    state.preset = "detailed";
    state.chartRenders = { cpu: 0, memory: 0, gpu: 0 };
    state.processRenders = 0;
  });

  afterEach(cleanup);

  it("limits a CPU tick to the mounted CPU chart instead of fanning out across charts", () => {
    const store = createStore();
    store.set(cpuUsageHistoryAtom, [20]);
    store.set(memoryUsageHistoryAtom, [40]);

    render(
      <Provider store={store}>
        <Performance />
      </Provider>,
    );

    expect(state.chartRenders).toEqual({ cpu: 1, memory: 1, gpu: 1 });

    act(() => store.set(cpuUsageHistoryAtom, [20, 30]));

    expect(state.chartRenders).toEqual({ cpu: 2, memory: 1, gpu: 1 });
    expect(state.processRenders).toBe(1);
  });

  it("unmounts graph and process panels in Compact", () => {
    state.preset = "compact";

    render(<Performance />);

    expect(screen.getByTestId("performance-current-values")).toBeVisible();
    expect(screen.queryByTestId("performance-usage-graphs")).toBeNull();
    expect(screen.queryByTestId("live-process-table")).toBeNull();
    expect(state.chartRenders).toEqual({ cpu: 0, memory: 0, gpu: 0 });
  });

  it("mounts only visible Custom panels", () => {
    state.preset = "custom";

    render(<Performance />);

    expect(screen.getByTestId("performance-current-values")).toBeVisible();
    expect(screen.queryByTestId("performance-usage-graphs")).toBeNull();
    expect(screen.queryByTestId("live-process-table")).toBeNull();
  });

  it("shows the temperature for the GPU selected by the usage history", () => {
    state.preset = "compact";
    const store = createStore();
    store.set(selectedGpuIdAtom, "gpu-2");
    store.set(gpuTempMapAtom, {
      "gpu-1": { name: "GPU 1", value: 45 },
      "gpu-2": { name: "GPU 2", value: 67 },
    });

    render(
      <Provider store={store}>
        <Performance />
      </Provider>,
    );

    expect(screen.getByTestId("performance-metric-gpu")).toHaveTextContent(
      "67°C",
    );
  });
});
