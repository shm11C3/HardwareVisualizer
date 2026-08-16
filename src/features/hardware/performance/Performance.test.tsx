import { act, cleanup, render, screen } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cpuUsageHistoryAtom,
  gpuTempMapAtom,
  gpuUsageHistoriesAtom,
  memoryUsageHistoryAtom,
  selectedGpuIdAtom,
} from "@/features/hardware/store/chart";
import { Performance } from "./Performance";
import type {
  PerformanceCustomLayout,
  PerformancePanelColumns,
  PerformanceView,
} from "./types/performanceLayout";

const state = vi.hoisted(() => ({
  view: "panels" as PerformanceView,
  columns: 1 as PerformancePanelColumns,
  compactExpanded: false,
  customLayout: {
    order: ["usageGraphs", "processTable", "perCore", "motherboardSensors"],
    visible: ["usageGraphs", "processTable"],
  } as PerformanceCustomLayout,
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

vi.mock("@/features/hardware/hooks/useHardwareInfoAtom", () => ({
  useHardwareInfoAtom: () => ({
    hardwareInfo: {
      cpu: null,
      memory: null,
      gpus: null,
      storage: [],
      motherboard: null,
    },
    init: vi.fn().mockResolvedValue(undefined),
  }),
}));

vi.mock("@/components/charts/DoughnutChart", () => ({
  DoughnutChart: ({
    chartValue,
    dataType,
    unit,
  }: {
    chartValue: number | null;
    dataType: "usage" | "temp" | "memoryUsageValue";
    unit?: string;
  }) => (
    <div data-testid={`doughnut-${dataType}`}>
      {chartValue == null
        ? "—"
        : `${chartValue}${
            dataType === "temp"
              ? "°C"
              : dataType === "memoryUsageValue"
                ? unit
                : "%"
          }`}
    </div>
  ),
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
    view: state.view,
    setView: vi.fn(),
    columns: state.columns,
    setColumns: vi.fn(),
    compactExpanded: state.compactExpanded,
    setCompactExpanded: vi.fn(),
    customLayout: state.customLayout,
    togglePanel: vi.fn(),
    handlePanelDragEnd: vi.fn(),
    isPending: false,
  }),
}));

describe("Performance", () => {
  beforeEach(() => {
    state.view = "panels";
    state.columns = 1;
    state.compactExpanded = false;
    state.customLayout = {
      order: ["usageGraphs", "processTable", "perCore", "motherboardSensors"],
      visible: ["usageGraphs", "processTable"],
    };
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

  it("mounts only the dense strip in Compact", () => {
    state.view = "compact";

    render(<Performance />);

    expect(screen.getByTestId("performance-compact-strip")).toBeVisible();
    expect(screen.queryByTestId("performance-current-values")).toBeNull();
    expect(screen.queryByTestId("performance-usage-graphs")).toBeNull();
    expect(screen.queryByTestId("live-process-table")).toBeNull();
    expect(state.chartRenders).toEqual({ cpu: 0, memory: 0, gpu: 0 });
  });

  it("drops every other surface in the expanded Compact view", () => {
    state.view = "compact";
    state.compactExpanded = true;

    render(<Performance />);

    expect(
      screen.getByTestId("performance-compact-fullscreen"),
    ).toBeInTheDocument();
    expect(screen.getByTestId("performance-compact-strip")).toBeVisible();
    expect(screen.getByTestId("performance-compact-collapse")).toBeVisible();
    // No screen chrome survives: no view switcher, no title, no panels.
    expect(screen.queryByTestId("performance-screen")).toBeNull();
    expect(screen.queryByRole("tab")).toBeNull();
    expect(screen.queryByTestId("performance-usage-graphs")).toBeNull();
  });

  it("mounts only the graph in Monitor", () => {
    state.view = "monitor";

    render(<Performance />);

    expect(screen.getByTestId("performance-usage-graphs")).toBeVisible();
    expect(screen.queryByTestId("performance-current-values")).toBeNull();
    expect(screen.queryByTestId("live-process-table")).toBeNull();
  });

  it("keeps hidden panels unmounted in the panels view", () => {
    state.customLayout = {
      order: ["usageGraphs", "processTable", "perCore", "motherboardSensors"],
      visible: ["usageGraphs"],
    };

    render(<Performance />);

    expect(screen.getByTestId("performance-current-values")).toBeVisible();
    expect(screen.getByTestId("performance-usage-graphs")).toBeVisible();
    expect(screen.queryByTestId("live-process-table")).toBeNull();
    expect(screen.queryByTestId("performance-panel-perCore")).toBeNull();
  });

  it("shows the temperature for the GPU selected by the usage history", () => {
    const store = createStore();
    store.set(selectedGpuIdAtom, "gpu-2");
    store.set(gpuUsageHistoriesAtom, {
      "gpu-1": [25],
      "gpu-2": [50],
    });
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

  it("leaves visible gaps between unavailable sparkline samples", () => {
    const store = createStore();
    store.set(cpuUsageHistoryAtom, [10, 20, null, 30, 40]);

    render(
      <Provider store={store}>
        <Performance />
      </Provider>,
    );

    expect(
      screen.getByTestId("performance-metric-cpu").querySelectorAll("polyline"),
    ).toHaveLength(2);
  });

  it("uses one effective GPU for history and temperature fallbacks", () => {
    const store = createStore();
    store.set(selectedGpuIdAtom, "stale-gpu");
    store.set(gpuUsageHistoriesAtom, {
      "gpu-1": [25],
    });
    store.set(gpuTempMapAtom, {
      "gpu-1": { name: "GPU 1", value: 45 },
      "stale-gpu": { name: "Stale GPU", value: 67 },
    });

    render(
      <Provider store={store}>
        <Performance />
      </Provider>,
    );

    const gpuMetric = screen.getByTestId("performance-metric-gpu");
    expect(gpuMetric).toHaveTextContent("25%");
    expect(gpuMetric).toHaveTextContent("45°C");
    expect(gpuMetric).not.toHaveTextContent("67°C");
  });

  it("keeps the two-column request collapsible to one column", () => {
    state.columns = 2;

    render(<Performance />);

    const grid = screen
      .getByTestId("performance-panel-usageGraphs")
      .closest("[data-panel-columns]");
    expect(grid).toHaveAttribute("data-panel-columns", "2");
    // Two columns are an upper bound: narrow windows still render one.
    expect(grid).toHaveClass("grid-cols-1", "xl:grid-cols-2");
  });

  it("shows panel controls and the hidden-panel strip only while editing", () => {
    render(<Performance />);

    expect(screen.queryByTestId("performance-hidden-panels")).toBeNull();

    act(() => {
      screen.getByTestId("performance-edit-toggle").click();
    });

    expect(screen.getByTestId("performance-hidden-panels")).toBeVisible();
    expect(
      screen.getAllByRole("button", {
        name: "pages.performance.showPanel",
      }),
    ).toHaveLength(2);
  });
});
