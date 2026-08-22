import { act, cleanup, render, screen } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { asLiveGpuId, type LiveGpuId } from "@/features/hardware/gpuIdentity";
import {
  cpuUsageHistoryAtom,
  gpuDedicatedMemoryKbMapAtom,
  gpuNamesAtom,
  gpuTempMapAtom,
  gpuUsageHistoriesAtom,
  memoryUsageHistoryAtom,
  selectedGpuIdAtom,
} from "@/features/hardware/store/chart";
import { Performance } from "./Performance";

/** Seeds mint live ids the way the event listener does at the boundary. */
// biome-ignore format: keep the generic arrow readable
const liveMap = <T,>(map: Record<string, T>) =>
  map as unknown as Record<LiveGpuId, T>;

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
  gpus: null as
    | null
    | {
        id: string;
        name: string;
        vendorName: string;
        clock: number;
        memorySize: string;
        memorySizeDedicated: string;
        coreCount: string | null;
      }[],
}));

const gpuFixture = (id: string, name: string) => ({
  id,
  name,
  vendorName: "Vendor",
  clock: 2100,
  memorySize: "8 GB",
  memorySizeDedicated: "8 GB",
  coreCount: null,
});

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
    // Interpolated values are appended so tests can assert on what the string
    // actually carries, not just which key was reached.
    t: (key: string, params?: Record<string, unknown>) =>
      params == null ? key : `${key} ${Object.values(params).join(" ")}`,
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
      gpus: state.gpus,
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
    state.gpus = null;
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

  it("keeps a GPU tick out of the panels that do not show GPU data", () => {
    // The GPU atoms are rewritten on every sample. A subscription in the
    // Performance parent would rerender the whole screen once a second.
    const store = createStore();
    store.set(gpuUsageHistoriesAtom, liveMap({ "gpu-1": [25] }));

    render(
      <Provider store={store}>
        <Performance />
      </Provider>,
    );

    const before = { ...state.chartRenders, process: state.processRenders };

    act(() => store.set(gpuUsageHistoriesAtom, liveMap({ "gpu-1": [25, 40] })));

    expect(state.processRenders).toBe(before.process);
    expect(state.chartRenders.cpu).toBe(before.cpu);
    expect(state.chartRenders.memory).toBe(before.memory);
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
    store.set(selectedGpuIdAtom, asLiveGpuId("gpu-2"));
    store.set(
      gpuUsageHistoriesAtom,
      liveMap({
        "gpu-1": [25],
        "gpu-2": [50],
      }),
    );
    store.set(
      gpuTempMapAtom,
      liveMap({
        "gpu-1": { name: "GPU 1", value: 45 },
        "gpu-2": { name: "GPU 2", value: 67 },
      }),
    );

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
    // A selection left over from an adapter that is no longer detected: it
    // appears in no live map and in no detected list, so it cannot be honored.
    store.set(selectedGpuIdAtom, asLiveGpuId("removed-gpu"));
    store.set(
      gpuUsageHistoriesAtom,
      liveMap({
        "gpu-1": [25],
      }),
    );
    store.set(
      gpuTempMapAtom,
      liveMap({
        "gpu-1": { name: "GPU 1", value: 45 },
        "gpu-2": { name: "GPU 2", value: 67 },
      }),
    );

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

  it("names the adapter behind the GPU readings without offering a choice there is none of", () => {
    const store = createStore();
    state.gpus = [gpuFixture("gpu-1", "NVIDIA GeForce RTX 4080")];
    store.set(gpuNamesAtom, liveMap({ "gpu-1": "NVIDIA GeForce RTX 4080" }));
    store.set(gpuUsageHistoriesAtom, liveMap({ "gpu-1": [42] }));

    render(
      <Provider store={store}>
        <Performance />
      </Provider>,
    );

    expect(screen.getByTestId("performance-gpu-adapter")).toHaveTextContent(
      "GeForce RTX 4080",
    );
    expect(screen.queryByTestId("performance-gpu-selector")).toBeNull();
  });

  it("drops the VRAM total when the inventory name cannot pick out one card", () => {
    const store = createStore();
    // Two identical cards in the inventory, only one of them reporting. The
    // live side looks unambiguous, but the name still cannot say which
    // capacity belongs to the reading.
    state.gpus = [
      gpuFixture("inventory-a", "NVIDIA GeForce RTX 4090"),
      gpuFixture("inventory-b", "NVIDIA GeForce RTX 4090"),
    ];
    store.set(gpuNamesAtom, liveMap({ "nvapi:1": "NVIDIA GeForce RTX 4090" }));
    store.set(gpuUsageHistoriesAtom, liveMap({ "nvapi:1": [40] }));
    store.set(
      gpuDedicatedMemoryKbMapAtom,
      liveMap({ "nvapi:1": 4 * 1024 * 1024 }),
    );

    render(
      <Provider store={store}>
        <Performance />
      </Provider>,
    );

    const gpuMetric = screen.getByTestId("performance-metric-gpu");
    expect(gpuMetric).toHaveTextContent("VRAM 4.0 GB");
    expect(gpuMetric).not.toHaveTextContent("VRAM 4.0/8 GB");
  });

  it("keeps the VRAM total when exactly one inventory entry matches", () => {
    const store = createStore();
    state.gpus = [gpuFixture("inventory-a", "NVIDIA GeForce RTX 4090")];
    store.set(gpuNamesAtom, liveMap({ "nvapi:1": "NVIDIA GeForce RTX 4090" }));
    store.set(gpuUsageHistoriesAtom, liveMap({ "nvapi:1": [40] }));
    store.set(
      gpuDedicatedMemoryKbMapAtom,
      liveMap({ "nvapi:1": 4 * 1024 * 1024 }),
    );

    render(
      <Provider store={store}>
        <Performance />
      </Provider>,
    );

    expect(screen.getByTestId("performance-metric-gpu")).toHaveTextContent(
      "VRAM 4.0/8 GB",
    );
  });

  it("moves every GPU reading to the adapter the user picks", () => {
    const store = createStore();
    state.gpus = [
      gpuFixture("gpu-1", "NVIDIA GeForce RTX 4080"),
      gpuFixture("gpu-2", "Intel UHD Graphics 770"),
    ];
    store.set(
      gpuNamesAtom,
      liveMap({
        "gpu-1": "NVIDIA GeForce RTX 4080",
        "gpu-2": "Intel UHD Graphics 770",
      }),
    );
    store.set(gpuUsageHistoriesAtom, liveMap({ "gpu-1": [25], "gpu-2": [50] }));
    store.set(
      gpuTempMapAtom,
      liveMap({
        "gpu-1": { name: "GPU 1", value: 45 },
        "gpu-2": { name: "GPU 2", value: 67 },
      }),
    );

    render(
      <Provider store={store}>
        <Performance />
      </Provider>,
    );

    const gpuMetric = screen.getByTestId("performance-metric-gpu");
    expect(gpuMetric).toHaveTextContent("25%");
    expect(gpuMetric).toHaveTextContent("45°C");

    act(() => {
      screen.getByRole("button", { name: "Intel UHD Graphics 770" }).click();
    });

    expect(store.get(selectedGpuIdAtom)).toBe("gpu-2");
    expect(gpuMetric).toHaveTextContent("50%");
    expect(gpuMetric).toHaveTextContent("67°C");
    expect(gpuMetric).not.toHaveTextContent("45°C");
    expect(
      screen.getByRole("button", { name: "Intel UHD Graphics 770" }),
    ).toHaveAttribute("aria-pressed", "true");
  });

  it("keeps a silent adapter selected and says so instead of borrowing readings", () => {
    const store = createStore();
    state.gpus = [
      gpuFixture("gpu-1", "NVIDIA GeForce RTX 4080"),
      gpuFixture("gpu-2", "Intel UHD Graphics 770"),
    ];
    store.set(selectedGpuIdAtom, asLiveGpuId("gpu-2"));
    // gpu-2 named itself in the stream but reported no values at all.
    store.set(
      gpuNamesAtom,
      liveMap({
        "gpu-1": "NVIDIA GeForce RTX 4080",
        "gpu-2": "Intel UHD Graphics 770",
      }),
    );
    store.set(gpuUsageHistoriesAtom, liveMap({ "gpu-1": [25] }));
    store.set(
      gpuTempMapAtom,
      liveMap({ "gpu-1": { name: "GPU 1", value: 45 } }),
    );

    render(
      <Provider store={store}>
        <Performance />
      </Provider>,
    );

    const gpuMetric = screen.getByTestId("performance-metric-gpu");
    expect(gpuMetric).toHaveTextContent("pages.performance.gpuNoLiveReadings");
    expect(gpuMetric).not.toHaveTextContent("25%");
    expect(gpuMetric).not.toHaveTextContent("45°C");
    // The rest of the adapter's information stays reachable.
    expect(
      screen.getByRole("button", { name: "Intel UHD Graphics 770" }),
    ).toHaveAttribute("aria-pressed", "true");
  });

  it("does not call an adapter unavailable before the first sample arrives", () => {
    const store = createStore();
    state.gpus = [gpuFixture("gpu-1", "NVIDIA GeForce RTX 4080")];
    store.set(gpuNamesAtom, liveMap({ "gpu-1": "NVIDIA GeForce RTX 4080" }));

    render(
      <Provider store={store}>
        <Performance />
      </Provider>,
    );

    expect(screen.getByTestId("performance-metric-gpu")).not.toHaveTextContent(
      "pages.performance.gpuNoLiveReadings",
    );
  });

  it("says why a Compact row is dashed when its adapter is silent", () => {
    const store = createStore();
    state.view = "compact";
    store.set(selectedGpuIdAtom, asLiveGpuId("gpu-2"));
    store.set(
      gpuNamesAtom,
      liveMap({
        "gpu-1": "NVIDIA GeForce RTX 4080",
        "gpu-2": "Intel UHD Graphics 770",
      }),
    );
    store.set(gpuUsageHistoriesAtom, liveMap({ "gpu-1": [25] }));

    render(
      <Provider store={store}>
        <Performance />
      </Provider>,
    );

    const strip = screen.getByTestId("performance-compact-strip");
    expect(strip).toHaveTextContent("pages.performance.gpuNoLiveReadings");
    // And it is still the selected adapter's row, not the reporting one's.
    expect(strip).not.toHaveTextContent("25%");
  });

  it("carries the selected adapter into the Compact strip", () => {
    const store = createStore();
    state.view = "compact";
    state.gpus = [
      gpuFixture("gpu-1", "NVIDIA GeForce RTX 4080"),
      gpuFixture("gpu-2", "Intel UHD Graphics 770"),
    ];
    store.set(selectedGpuIdAtom, asLiveGpuId("gpu-2"));
    store.set(
      gpuNamesAtom,
      liveMap({
        "gpu-1": "NVIDIA GeForce RTX 4080",
        "gpu-2": "Intel UHD Graphics 770",
      }),
    );
    store.set(gpuUsageHistoriesAtom, liveMap({ "gpu-1": [25], "gpu-2": [50] }));

    render(
      <Provider store={store}>
        <Performance />
      </Provider>,
    );

    const strip = screen.getByTestId("performance-compact-strip");
    expect(strip).toHaveTextContent(
      "pages.performance.compactGpuAdapter UHD Graphics 770",
    );
    expect(screen.getByTestId("performance-compact-row-gpu")).toHaveTextContent(
      "50%",
    );
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
        name: /^pages\.performance\.showPanel/,
      }),
    ).toHaveLength(2);
  });
});
