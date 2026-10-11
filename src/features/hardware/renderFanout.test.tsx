/**
 * Render fan-out regression test for #1638 (Live Metrics subscriptions).
 *
 * Mounts each real live screen in a fresh Provider, fires ten fixture
 * `HardwareMonitorUpdate` payloads through the real `useHardwareEventListener`,
 * and counts what one 1 Hz sample costs in steady state: root commits, fibers
 * that performed work (through a minimal DevTools hook), and renders of the
 * heavy chart leaves (counting stubs). Everything it asserts is a count, so it
 * is deterministic; durations depend on machine load and are only reported.
 *
 * Vitest does not run the React Compiler that the shipped app is built with,
 * so these counts are an upper bound on what production renders.
 *
 * Set `RENDER_FANOUT_OUT=<file>` to also write the full measurement as JSON to
 * that file and print a Markdown summary, e.g. to compare before and after a
 * change:
 *
 *   RENDER_FANOUT_OUT=test-results/render-fanout/now.json \
 *     npx vitest run src/features/hardware/renderFanout.test.tsx
 */

// Not part of the app's tsconfig types: writing the optional report needs
// Node's fs/path and `process.env`, and this is the only source file that does.
/// <reference types="node" />
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { act, cleanup, render } from "@testing-library/react";
import { type Atom, createStore, Provider } from "jotai";
import {
  type ComponentType,
  Profiler,
  type ProfilerOnRenderCallback,
} from "react";
import { afterAll, describe, expect, it, vi } from "vitest";
import {
  buildHardwareUpdateSeries,
  GPU_FIXTURES,
  sysInfoFixture,
} from "@/e2e/fixtures/hardware";
import { Dashboard } from "@/features/hardware/dashboard/Dashboard";
import { asLiveGpuId } from "@/features/hardware/gpuIdentity";
import { useHardwareEventListener } from "@/features/hardware/hooks/useHardwareEventListener";
import { Performance } from "@/features/hardware/performance/Performance";
import {
  effectiveGpuUsageCurrentAtom,
  effectiveGpuUsageSeriesAtom,
  gpuDedicatedMemoryKbAtom,
  gpuDedicatedMemoryKbMapAtom,
  gpuFanSpeedAtom,
  gpuFanSpeedMapAtom,
  gpuNamesAtom,
  gpuTempAtom,
  gpuTempMapAtom,
  gpuUsageHistoriesAtom,
  gpuUsageSourceAtom,
  gpuUsageSourcesAtom,
  graphicUsageHistoryAtom,
} from "@/features/hardware/store/gpu";
import {
  cpuUsageCurrentAtom,
  cpuUsageSeriesAtom,
  gpuUsageSeriesAtom,
  latestProcessorUsagesAtom,
  memoryUsageCurrentAtom,
  memoryUsageSeriesAtom,
  powerCurrentAtom,
  powerDrawSeriesAtom,
  processorUsageSeriesAtom,
} from "@/features/hardware/store/liveMetrics";
import {
  cpuUsageHistoryAtom,
  memoryUsageHistoryAtom,
  processorsUsageHistoryAtom,
} from "@/features/hardware/store/liveUsage";
import {
  cpuPowerSupportAtom,
  powerDrawAtom,
  powerDrawAvailableAtom,
  powerDrawHistoryAtom,
} from "@/features/hardware/store/power";
import { selectedGpuIdAtom } from "@/features/hardware/store/selection";
import {
  cpuTempAtom,
  motherboardFanSpeedsAtom,
  motherboardFanSupportAtom,
  motherboardTempsAtom,
  sensorTempsAtom,
} from "@/features/hardware/store/sensors";
import { CpuUsages } from "@/features/hardware/usage/cpu/CpuUsage";
import { ChartTemplate } from "@/features/hardware/usage/Usage";
import type { HardwareMonitorUpdate } from "@/rspc/bindings";

// ── Bounds ──
//
// Per-screen upper bounds on what one steady-state update (updates 2-10 of the
// series) may cost. They are the values measured after slice 3 of #1638
// (the screens reading through the live hooks), with no margin because the counts are
// deterministic. They may only go down: when a change lowers a count, lower
// the number here in the same change; if a change raises one, the screen has
// started re-rendering something it did not before.
//
//   components        components that re-rendered (function ran, no bailout)
//   leaves            chart-leaf renders (counting stubs for the chart/sparkline/
//                     table components)
//   commits           root commits per steady update
//   firstUpdateCommits  commits for the first update, which also lets lazy
//                       panels and GPU auto-selection settle
const BOUNDS = {
  dashboard: { components: 14, leaves: 6, commits: 1, firstUpdateCommits: 1 },
  "performance-panels": {
    components: 19,
    leaves: 11,
    commits: 1,
    firstUpdateCommits: 2,
  },
  "performance-panels-all": {
    components: 21,
    leaves: 11,
    commits: 1,
    firstUpdateCommits: 2,
  },
  "performance-compact": {
    components: 7,
    leaves: 3,
    commits: 1,
    firstUpdateCommits: 1,
  },
  "performance-monitor": {
    components: 7,
    leaves: 3,
    commits: 1,
    firstUpdateCommits: 2,
  },
  "performance-monitor-graph": {
    components: 6,
    leaves: 3,
    commits: 1,
    firstUpdateCommits: 2,
  },
  usage: { components: 6, leaves: 3, commits: 1, firstUpdateCommits: 1 },
  "usage-mixed": {
    components: 2,
    leaves: 1,
    commits: 1,
    firstUpdateCommits: 1,
  },
  "cpu-detail": {
    components: 18,
    leaves: 9,
    commits: 1,
    firstUpdateCommits: 1,
  },
} as const;

type ScreenId = keyof typeof BOUNDS;

// ── Hoisted state shared with the mock factories ──

type FiberLike = {
  tag: number;
  flags: number;
  type: unknown;
  elementType: unknown;
  child: FiberLike | null;
  sibling: FiberLike | null;
  alternate: FiberLike | null;
};

const harness = vi.hoisted(() => {
  /** Stub render counters, keyed by stub name. */
  const stubRenders = new Map<string, number>();
  /** Component renders counted from committed fibers, keyed by component name. */
  const fiberRenders = new Map<string, number>();
  const fiberStats = { commits: 0, components: 0 };
  const store = new Map<string, unknown>();
  const listener: {
    current: null | ((event: { payload: HardwareMonitorUpdate }) => void);
  } = { current: null };

  // A function, not a property read at the call site: `measureScreen` resets
  // the property, and TypeScript would otherwise narrow it to `null` there.
  const currentListener = () => listener.current;

  const countStub = (name: string) => {
    stubRenders.set(name, (stubRenders.get(name) ?? 0) + 1);
  };

  // Install a minimal DevTools hook before react-dom loads so each commit can
  // report which components actually ran. React sets PerformedWork on a fiber
  // when its function ran and did not bail out. A subtree whose first child is
  // the same object as its alternate's child had no work in it, and its
  // (possibly stale) flags must not be read.
  const FUNCTION_COMPONENT = 0;
  const CLASS_COMPONENT = 1;
  const FORWARD_REF = 11;
  const SIMPLE_MEMO_COMPONENT = 15;
  const PERFORMED_WORK = 1;
  const nameOf = (fiber: FiberLike): string => {
    for (const candidate of [fiber.type, fiber.elementType]) {
      if (typeof candidate === "function" && candidate.name !== "") {
        return candidate.name;
      }
      if (typeof candidate === "object" && candidate !== null) {
        const named = candidate as {
          displayName?: string;
          render?: { name?: string };
        };
        const label = named.displayName ?? named.render?.name;
        if (label != null && label !== "") {
          return label;
        }
      }
    }
    return "(anonymous)";
  };
  const walk = (fiber: FiberLike, isNew: boolean) => {
    const isComponent =
      fiber.tag === FUNCTION_COMPONENT ||
      fiber.tag === CLASS_COMPONENT ||
      fiber.tag === FORWARD_REF ||
      fiber.tag === SIMPLE_MEMO_COMPONENT;
    if (isComponent && (isNew || (fiber.flags & PERFORMED_WORK) !== 0)) {
      fiberStats.components += 1;
      const name = nameOf(fiber);
      fiberRenders.set(name, (fiberRenders.get(name) ?? 0) + 1);
    }
    const alternate = fiber.alternate;
    if (isNew || alternate == null || fiber.child !== alternate.child) {
      for (let child = fiber.child; child != null; child = child.sibling) {
        walk(child, isNew || child.alternate == null);
      }
    }
  };

  Object.defineProperty(globalThis, "__REACT_DEVTOOLS_GLOBAL_HOOK__", {
    configurable: true,
    value: {
      supportsFiber: true,
      renderers: new Map(),
      inject: () => 1,
      checkDCE: () => undefined,
      onScheduleFiberRoot: () => undefined,
      onCommitFiberUnmount: () => undefined,
      onPostCommitFiberRoot: () => undefined,
      onCommitFiberRoot: (_id: number, root: { current: FiberLike }) => {
        fiberStats.commits += 1;
        walk(root.current, false);
      },
    },
  });

  return {
    stubRenders,
    fiberRenders,
    fiberStats,
    store,
    listener,
    currentListener,
    countStub,
    settings: {
      graphFitToWindow: false,
      graphMarginPx: 16,
      graphSize: "xl",
      lineGraphMix: false,
      lineGraphType: "default",
      lineGraphFill: true,
      lineGraphBorder: false,
      lineGraphShowScale: true,
      lineGraphShowTooltip: true,
      lineGraphShowLegend: true,
      temperatureUnit: "C",
      displayTargets: ["cpu", "memory", "gpu"],
      powerDisplayTargets: ["cpu", "gpu", "package"],
      lineGraphColor: {
        cpu: "75, 192, 192",
        memory: "255, 99, 132",
        gpu: "255, 206, 86",
      },
      selectedBackgroundImg: null,
      backgroundImgOpacity: 50,
      burnInShift: false,
      burnInShiftMode: "jump",
      burnInShiftOptions: null,
      storageHealth: { enabled: false },
    },
  };
});

// ── Mock surface ──
// Every screen needs the same environment seams: backend bindings, Tauri
// store, settings, i18n, OS platform, and the heavy visual leaves.

vi.mock("@/rspc/bindings", () => ({
  events: {
    hardwareMonitorUpdate: {
      listen: (
        callback: (event: { payload: HardwareMonitorUpdate }) => void,
      ) => {
        harness.listener.current = callback;
        return Promise.resolve(() => undefined);
      },
    },
  },
  commands: {
    getHardwareInfo: () =>
      Promise.resolve({ status: "ok", data: sysInfoFixture }),
  },
}));

// In-memory Tauri Store: the real `useTauriStore` and `usePerformanceLayout`
// run against it, so layout selection follows the production path.
vi.mock("@/lib/tauriStore", () => ({
  getStoreInstance: () =>
    Promise.resolve({
      has: (key: string) => Promise.resolve(harness.store.has(key)),
      get: (key: string) => Promise.resolve(harness.store.get(key)),
      set: (key: string, value: unknown) => {
        harness.store.set(key, value);
        return Promise.resolve();
      },
      save: () => Promise.resolve(),
    }),
}));

// Stable `settings` identity, as in production where it only changes on edit.
vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({ settings: harness.settings }),
}));

// Stable `t`, as react-i18next returns until the language changes.
vi.mock("react-i18next", () => {
  const t = (key: string, params?: Record<string, unknown>) =>
    params == null ? key : `${key} ${Object.values(params).join(" ")}`;
  const translation = { t };
  return { useTranslation: () => translation };
});

vi.mock("@/hooks/appearance/useBurnInShift", () => ({
  useBurnInShift: () => undefined,
}));

vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => "windows" }));

vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({
  writeText: () => Promise.resolve(),
}));

// Process polling is a 3 s concern of its own, not part of the 1 Hz fan-out.
vi.mock("@/features/hardware/hooks/useProcessInfo", () => {
  const result = { processes: [], hasError: false };
  return { useProcessInfo: () => result };
});

// Counting stubs. Each is a real component that records its render and keeps
// the production props contract; none adds memoization. `MiniLineChart` is the
// exception: production wraps it in `memo`, so the stub does too, or its
// prop-identity bailout would be measured differently.

vi.mock("@/components/charts/LineChart", () => ({
  LineChartComponent: (props: {
    dataType?: string;
    lineGraphMix: boolean;
    chartData?: unknown;
  }) => {
    harness.countStub(
      props.lineGraphMix ? "LineChart:mix" : `LineChart:${props.dataType}`,
    );
    return <div data-stub="LineChart" />;
  },
}));

vi.mock("@/components/charts/DoughnutChart", () => ({
  DoughnutChart: (props: { dataType: string; chartValue: number | null }) => {
    harness.countStub(`DoughnutChart:${props.dataType}`);
    return <div data-stub="DoughnutChart" />;
  },
}));

vi.mock("@/components/charts/Sparkline", () => ({
  Sparkline: (props: { values: (number | null)[] }) => {
    harness.countStub("Sparkline(charts)");
    return <div data-stub="Sparkline" data-length={props.values.length} />;
  },
}));

vi.mock("@/features/hardware/performance/components/Sparkline", () => ({
  Sparkline: (props: { values: (number | null)[] }) => {
    harness.countStub("Sparkline(performance)");
    return <div data-stub="Sparkline" data-length={props.values.length} />;
  },
}));

vi.mock("@/features/hardware/dashboard/components/MiniLineChart", async () => {
  const { memo } = await import("react");
  return {
    MiniLineChart: memo(
      (props: { hardwareType: string; usage: (number | null)[] }) => {
        harness.countStub(`MiniLineChart:${props.hardwareType}`);
        return <div data-stub="MiniLineChart" />;
      },
    ),
  };
});

vi.mock("@/features/hardware/performance/components/PowerDrawChart", () => ({
  PowerDrawChart: () => {
    harness.countStub("PowerDrawChart");
    return <div data-stub="PowerDrawChart" />;
  },
}));

vi.mock("@/features/hardware/dashboard/components/ProcessTable", () => ({
  ProcessesTable: () => {
    harness.countStub("ProcessesTable");
    return <div data-stub="ProcessesTable" />;
  },
}));

// ── Measurement ──

const UPDATE_COUNT = 10;
const ALL_PANELS_LAYOUT = {
  order: [
    "usageGraphs",
    "processTable",
    "perCore",
    "motherboardSensors",
    "power",
  ],
  visible: [
    "usageGraphs",
    "processTable",
    "perCore",
    "motherboardSensors",
    "power",
  ],
};

type ScreenSpec = {
  id: ScreenId;
  label: string;
  Screen: ComponentType;
  /** CSS selector that proves the screen has settled into its live layout. */
  ready: string;
  /** Tauri Store keys that select the screen's view. */
  store?: Record<string, unknown>;
  /** Settings overrides applied for this screen only. */
  settings?: Partial<typeof harness.settings>;
};

const specs: ScreenSpec[] = [
  {
    id: "dashboard",
    label: "Dashboard (classic: CPU, GPU, Memory, Process, Motherboard)",
    Screen: Dashboard,
    ready: '[data-testid="dashboard-gpu-readings"]',
    // Storage and Network cards read no live atom and need their own backend
    // commands, so they are hidden rather than stubbed.
    store: {
      dashboardVisibleItems: [
        "cpu",
        "gpu",
        "memory",
        "process",
        "motherboard",
        "title",
      ],
      dashboardVisibleItemsVersion: 1,
    },
  },
  {
    id: "performance-panels",
    label: "Performance / Panels (default layout)",
    Screen: Performance,
    ready: '[data-testid="performance-current-values"]',
    store: { performanceLayoutPreset: "panels" },
  },
  {
    id: "performance-panels-all",
    label: "Performance / Panels (all five panels visible)",
    Screen: Performance,
    ready: '[data-testid="performance-current-values"]',
    store: {
      performanceLayoutPreset: "panels",
      performanceCustomLayout: ALL_PANELS_LAYOUT,
    },
  },
  {
    id: "performance-compact",
    label: "Performance / Compact",
    Screen: Performance,
    ready: '[data-testid="performance-compact-strip"]',
    store: { performanceLayoutPreset: "compact" },
  },
  {
    id: "performance-monitor",
    label: "Performance / Monitor (power: current rail)",
    Screen: Performance,
    ready: '[data-testid="performance-usage-graphs"]',
    store: { performanceLayoutPreset: "monitor" },
  },
  {
    id: "performance-monitor-graph",
    label: "Performance / Monitor (power: graph)",
    Screen: Performance,
    ready: '[data-testid="performance-usage-graphs"]',
    store: {
      performanceLayoutPreset: "monitor",
      performanceMonitorPowerMode: "graph",
    },
  },
  {
    id: "usage",
    label: "Usage (three charts)",
    Screen: ChartTemplate,
    ready: '[data-testid="usage-chart-layout"]',
  },
  {
    id: "usage-mixed",
    label: "Usage (mixed chart)",
    Screen: ChartTemplate,
    ready: '[data-testid="usage-chart-layout"]',
    settings: { lineGraphMix: true },
  },
  {
    id: "cpu-detail",
    label: "CPU detail (CpuUsages)",
    Screen: CpuUsages,
    ready: '[data-stub="LineChart"]',
  },
];

/** Atoms the event listener rewrites each tick, by the name used in reports. */
const liveAtoms: ReadonlyArray<readonly [string, Atom<unknown>]> = [
  ["cpuUsageHistory", cpuUsageHistoryAtom],
  ["memoryUsageHistory", memoryUsageHistoryAtom],
  ["processorsUsageHistory", processorsUsageHistoryAtom],
  ["gpuUsageHistories", gpuUsageHistoriesAtom],
  ["gpuNames", gpuNamesAtom],
  ["gpuUsageSources", gpuUsageSourcesAtom],
  ["gpuDedicatedMemoryKbMap", gpuDedicatedMemoryKbMapAtom],
  ["gpuTempMap", gpuTempMapAtom],
  ["gpuFanSpeedMap", gpuFanSpeedMapAtom],
  ["gpuTemp (derived)", gpuTempAtom],
  ["gpuFanSpeed (derived)", gpuFanSpeedAtom],
  ["graphicUsageHistory (derived)", graphicUsageHistoryAtom],
  ["gpuUsageSource (derived)", gpuUsageSourceAtom],
  ["gpuDedicatedMemoryKb (derived)", gpuDedicatedMemoryKbAtom],
  ["selectedGpuId", selectedGpuIdAtom],
  ["cpuTemp", cpuTempAtom],
  ["sensorTemps", sensorTempsAtom],
  ["motherboardTemps", motherboardTempsAtom],
  ["motherboardFanSpeeds", motherboardFanSpeedsAtom],
  ["motherboardFanSupport", motherboardFanSupportAtom],
  ["cpuPowerSupport", cpuPowerSupportAtom],
  ["powerDraw", powerDrawAtom],
  ["powerDrawHistory", powerDrawHistoryAtom],
  ["powerDrawAvailable", powerDrawAvailableAtom],
  // The live channels the screens read through the hooks (#1638 slice 3).
  ["cpuUsageSeries", cpuUsageSeriesAtom],
  ["memoryUsageSeries", memoryUsageSeriesAtom],
  ["cpuUsageCurrent", cpuUsageCurrentAtom],
  ["memoryUsageCurrent", memoryUsageCurrentAtom],
  ["effectiveGpuUsageSeries", effectiveGpuUsageSeriesAtom],
  ["effectiveGpuUsageCurrent", effectiveGpuUsageCurrentAtom],
  ["latestProcessorUsages", latestProcessorUsagesAtom],
  ...Array.from(
    { length: 64 },
    (_, index) =>
      ["processorUsageSeries", processorUsageSeriesAtom(index)] as const,
  ),
  ...GPU_FIXTURES.map(
    ({ liveId }) =>
      ["gpuUsageSeries", gpuUsageSeriesAtom(asLiveGpuId(liveId))] as const,
  ),
  ...(["cpuWatts", "gpuWatts", "aneWatts", "packageWatts"] as const).flatMap(
    (key) =>
      [
        [`powerDrawSeries (${key})`, powerDrawSeriesAtom(key)],
        [`powerCurrent (${key})`, powerCurrentAtom(key)],
      ] as const,
  ),
];

/** A store that counts active component subscriptions to the live atoms. */
const createCountingStore = () => {
  const store = createStore();
  const names = new Map<Atom<unknown>, string>(
    liveAtoms.map(([name, atom]) => [atom, name]),
  );
  const subscribers = new Map<string, number>();
  const sub = store.sub;
  store.sub = function countedSub(atom: Atom<unknown>, listener: () => void) {
    const name = names.get(atom);
    if (name != null) {
      subscribers.set(name, (subscribers.get(name) ?? 0) + 1);
    }
    const unsubscribe = sub(atom, listener);
    return () => {
      if (name != null) {
        subscribers.set(name, (subscribers.get(name) ?? 1) - 1);
      }
      unsubscribe();
    };
  };
  return { store, subscribers };
};

const HookHost = () => {
  useHardwareEventListener();
  return null;
};

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const settle = async (rounds = 4) => {
  for (let round = 0; round < rounds; round += 1) {
    await act(async () => {
      await sleep(15);
    });
  }
};

const waitForSelector = async (selector: string) => {
  for (let attempt = 0; attempt < 200; attempt += 1) {
    if (document.querySelector(selector) != null) {
      return;
    }
    await act(async () => {
      await sleep(15);
    });
  }
  throw new Error(`Screen never reached its live layout: ${selector}`);
};

const snapshot = (counters: Map<string, number>) =>
  Object.fromEntries([...counters].filter(([, count]) => count > 0));

const sum = (counters: Record<string, number>) =>
  Object.values(counters).reduce((total, count) => total + count, 0);

type UpdateRecord = {
  /** Commits seen by the root Profiler around the screen. */
  commits: number;
  /** Commits React reported for the whole root (cross-check). */
  rootCommits: number;
  /** Sum of Profiler `actualDuration`, ms. Indicative under load. */
  actualDurationMs: number;
  /** Components whose function ran and did not bail out. */
  componentsRendered: number;
  stubRenders: Record<string, number>;
  componentRenders: Record<string, number>;
};

const round = (value: number) => Math.round(value * 100) / 100;

/** The most any single update in `records` cost, per measure and per name. */
const worstOf = (records: UpdateRecord[]): UpdateRecord => {
  const worst = (pick: (record: UpdateRecord) => number) =>
    round(Math.max(...records.map(pick)));
  const worstMaps = (
    pick: (record: UpdateRecord) => Record<string, number>,
  ) => {
    const keys = new Set(
      records.flatMap((record) => Object.keys(pick(record))),
    );
    return Object.fromEntries(
      [...keys]
        .sort()
        .map((key) => [key, worst((record) => pick(record)[key] ?? 0)]),
    );
  };
  return {
    commits: worst((r) => r.commits),
    rootCommits: worst((r) => r.rootCommits),
    actualDurationMs: worst((r) => r.actualDurationMs),
    componentsRendered: worst((r) => r.componentsRendered),
    stubRenders: worstMaps((r) => r.stubRenders),
    componentRenders: worstMaps((r) => r.componentRenders),
  };
};

const measureScreen = async (
  spec: ScreenSpec,
  series: HardwareMonitorUpdate[],
) => {
  harness.stubRenders.clear();
  harness.fiberRenders.clear();
  harness.listener.current = null;
  harness.store.clear();
  for (const [key, value] of Object.entries(spec.store ?? {})) {
    harness.store.set(key, value);
  }
  Object.assign(harness.settings, { lineGraphMix: false }, spec.settings);

  const profile = { commits: 0, durationMs: 0 };
  const onRender: ProfilerOnRenderCallback = (_id, _phase, actualDuration) => {
    profile.commits += 1;
    profile.durationMs += actualDuration;
  };
  const { store, subscribers } = createCountingStore();
  const { Screen } = spec;

  render(
    <Provider store={store}>
      <HookHost />
      <Profiler id={spec.id} onRender={onRender}>
        <Screen />
      </Profiler>
    </Provider>,
  );

  await waitForSelector(spec.ready);
  await settle();

  const emit = harness.currentListener();
  if (emit == null) {
    throw new Error(`${spec.id}: the listener was never registered`);
  }

  const updates: UpdateRecord[] = [];
  for (const payload of series) {
    harness.stubRenders.clear();
    harness.fiberRenders.clear();
    harness.fiberStats.commits = 0;
    harness.fiberStats.components = 0;
    profile.commits = 0;
    profile.durationMs = 0;

    await act(async () => {
      emit({ payload });
    });
    // Effects that react to the new data (lazy power panels, GPU
    // auto-selection) run in follow-up commits; let them land in this update.
    await settle(2);

    updates.push({
      commits: profile.commits,
      rootCommits: harness.fiberStats.commits,
      actualDurationMs: round(profile.durationMs),
      componentsRendered: harness.fiberStats.components,
      stubRenders: snapshot(harness.stubRenders),
      componentRenders: snapshot(harness.fiberRenders),
    });
  }

  const liveSubscribers = Object.fromEntries(
    [...subscribers].filter(([, count]) => count > 0).sort(),
  );
  cleanup();

  return {
    id: spec.id,
    label: spec.label,
    update1: updates[0],
    steady: worstOf(updates.slice(1)),
    updates,
    liveAtomSubscribers: liveSubscribers,
    liveAtomSubscriberCount: sum(liveSubscribers),
  };
};

type ScreenResult = Awaited<ReturnType<typeof measureScreen>>;

const fixed = (value: number, digits = 1) => value.toFixed(digits);

const renderMarkdown = (results: ScreenResult[]) => {
  const lines: string[] = [];
  lines.push(
    "",
    `### Render fan-out per HardwareMonitorUpdate (${UPDATE_COUNT} updates; steady = worst of updates 2-${UPDATE_COUNT})`,
    "",
    "| screen | commits u1 | commits steady | components u1 | components steady | leaves u1 | leaves steady | duration ms u1 | duration ms steady | live-atom subs |",
    "|---|---|---|---|---|---|---|---|---|---|",
  );
  for (const result of results) {
    lines.push(
      `| ${result.id} | ${result.update1.commits} | ${fixed(result.steady.commits, 0)} | ${result.update1.componentsRendered} | ${fixed(result.steady.componentsRendered, 0)} | ${sum(result.update1.stubRenders)} | ${fixed(sum(result.steady.stubRenders), 0)} | ${fixed(result.update1.actualDurationMs)} | ${fixed(result.steady.actualDurationMs)} | ${result.liveAtomSubscriberCount} |`,
    );
  }
  lines.push("", "### Leaf renders per update (u1 / steady)", "");
  for (const result of results) {
    lines.push(`- **${result.id}**: ${describeLeaves(result)}`);
  }
  lines.push("", "### Components rendered per steady update", "");
  for (const result of results) {
    lines.push(`- **${result.id}**: ${describeComponents(result)}`);
  }
  lines.push("", "### Live-atom subscribers (direct store subscriptions)", "");
  for (const result of results) {
    const cells = Object.entries(result.liveAtomSubscribers).map(
      ([name, count]) => `${name} x${count}`,
    );
    lines.push(`- **${result.id}**: ${cells.join(", ")}`);
  }
  lines.push(
    "",
    "Durations are indicative only: they depend on machine load. Commits and render counts are deterministic.",
    "",
  );
  return lines.join("\n");
};

const describeLeaves = (result: ScreenResult) => {
  const names = new Set([
    ...Object.keys(result.update1.stubRenders),
    ...Object.keys(result.steady.stubRenders),
  ]);
  return [...names]
    .sort()
    .map(
      (name) =>
        `${name} ${result.update1.stubRenders[name] ?? 0}/${fixed(result.steady.stubRenders[name] ?? 0, 0)}`,
    )
    .join(", ");
};

const describeComponents = (result: ScreenResult) =>
  Object.entries(result.steady.componentRenders)
    .sort(([, a], [, b]) => b - a)
    .map(([name, count]) => `${name} ${fixed(count, 0)}`)
    .join(", ");

/** Every bound a screen exceeded, or none. */
const exceededBounds = (result: ScreenResult) => {
  const bound = BOUNDS[result.id];
  const exceeded: string[] = [];
  const check = (name: string, measured: number, limit: number) => {
    if (measured > limit) {
      exceeded.push(`${name} ${measured} > ${limit}`);
    }
  };
  check("components", result.steady.componentsRendered, bound.components);
  check("leaves", sum(result.steady.stubRenders), bound.leaves);
  check("commits", result.steady.commits, bound.commits);
  check("firstUpdateCommits", result.update1.commits, bound.firstUpdateCommits);
  return exceeded;
};

const results: ScreenResult[] = [];

describe("render fan-out", () => {
  Object.defineProperty(document, "hidden", {
    configurable: true,
    value: false,
  });
  const series = buildHardwareUpdateSeries(UPDATE_COUNT);

  it.each(specs)(
    "$label stays within its re-render bounds",
    {
      timeout: 60_000,
    },
    async (spec) => {
      const result = await measureScreen(spec, series);
      results.push(result);

      // The updates have to have reached the screen, or zero re-renders would
      // pass for the wrong reason.
      expect(result.updates).toHaveLength(UPDATE_COUNT);
      for (const update of result.updates) {
        expect(update.commits).toBeGreaterThan(0);
      }

      // A failure says which leaves and components grew, not just that a total
      // did, so the offending subscription can be found from the message.
      expect(
        exceededBounds(result),
        [
          `${result.id} re-rendered more than its bound on a steady 1 Hz update.`,
          `leaves (first update / worst steady): ${describeLeaves(result)}`,
          `components (worst steady): ${describeComponents(result)}`,
          `live-atom subscribers: ${Object.entries(result.liveAtomSubscribers)
            .map(([name, count]) => `${name} x${count}`)
            .join(", ")}`,
        ].join("\n"),
      ).toEqual([]);
    },
  );

  afterAll(() => {
    const out = process.env["RENDER_FANOUT_OUT"];
    if (out == null || out === "" || results.length === 0) {
      return;
    }
    const target = path.resolve(process.cwd(), out);
    mkdirSync(path.dirname(target), { recursive: true });
    writeFileSync(
      target,
      `${JSON.stringify(
        {
          meta: {
            updatesPerScreen: UPDATE_COUNT,
            steadyState: `worst of updates 2-${UPDATE_COUNT}`,
            note: "actualDurationMs is indicative only; commits and render counts are deterministic.",
          },
          screens: results,
        },
        null,
        2,
      )}\n`,
    );
    // Not `console.*`: Vitest hides the output of passing tests by default.
    process.stdout.write(`${renderMarkdown(results)}\n`);
  });
});
