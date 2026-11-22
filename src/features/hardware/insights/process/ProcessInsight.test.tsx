import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import { ProcessInsight } from "@/features/hardware/insights/process/ProcessInsight";
import type { ProcessGpuStats, ProcessStatRecord } from "@/rspc/bindings";
import { displayTargetAtom } from "@/store/navigation";

const hoisted = vi.hoisted(() => ({
  getProcessStats: vi.fn(),
  getProcessGpuStats: vi.fn(),
  getDatabaseConversionState: vi.fn(),
  period: 60,
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getProcessStats: hoisted.getProcessStats,
    getProcessGpuStats: hoisted.getProcessGpuStats,
    getDatabaseConversionState: hoisted.getDatabaseConversionState,
  },
}));
vi.mock("@/hooks/tauri/useTauriStore", () => ({
  useTauriStore: () => [hoisted.period, vi.fn()],
}));
vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: {
      selectedBackgroundImg: null,
      backgroundImgOpacity: 50,
      language: "en",
    },
  }),
}));
vi.mock("@/hooks/window/useStickyObserver", () => ({
  useStickyObserver: () => ({ sentinelRef: { current: null }, isStuck: false }),
}));
// The bubble chart is out of this slice and still reads CPU only.
vi.mock("@/features/hardware/insights/process/chart/Bubble", () => ({
  ProcessBubbleChart: () => null,
}));
vi.mock("@/features/hardware/insights/components/SelectPeriod", () => ({
  SelectPeriod: () => null,
}));

const processStats: ProcessStatRecord[] = [
  {
    pid: 42,
    process_name: "game",
    avg_cpu_usage: 30,
    avg_memory_usage: 1024,
    total_execution_sec: 600,
    latest_timestamp: "2026-11-20T11:58:00+00:00",
  },
  {
    pid: 42,
    process_name: "updater",
    avg_cpu_usage: 2,
    avg_memory_usage: 512,
    total_execution_sec: 120,
    latest_timestamp: "2026-11-20T11:30:00+00:00",
  },
];
const ok = <T,>(data: T) => ({ status: "ok" as const, data });
const available = (recorded_since: string | null): ProcessGpuStats => ({
  kind: "available",
  records: [
    {
      pid: 42,
      process_name: "game",
      avg_gpu_usage: 87.5,
      covered_seconds: 600,
    },
  ],
  recorded_since,
});

const flush = () =>
  act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });

const renderInsight = async () => {
  const store = createStore();
  render(
    <Provider store={store}>
      <ProcessInsight />
    </Provider>,
  );
  await flush();
  return store;
};

const rowFor = (name: string) => {
  const cell = within(screen.getByRole("table")).getByText(name);
  const row = cell.closest("tr");
  if (row == null) throw new Error(`no row for ${name}`);
  return row;
};
const gpuHeader = () =>
  screen.queryByRole("columnheader", { name: "Avg GPU Usage" });

describe("ProcessInsight GPU usage", () => {
  beforeEach(() => {
    hoisted.period = 60;
    hoisted.getProcessStats.mockResolvedValue(ok(processStats));
    hoisted.getDatabaseConversionState.mockResolvedValue({
      kind: "sqliteAuthoritative",
    });
  });
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("reads GPU usage for the same window and joins it by process identity", async () => {
    hoisted.getProcessGpuStats.mockResolvedValue(
      ok(available("2026-11-01T00:00:00+00:00")),
    );

    await renderInsight();

    expect(hoisted.getProcessGpuStats).toHaveBeenCalledWith(
      ...hoisted.getProcessStats.mock.calls[0],
    );
    expect(gpuHeader()).toBeInTheDocument();
    expect(within(rowFor("game")).getByText("87.5%")).toBeInTheDocument();
    expect(within(rowFor("updater")).getByText("—")).toBeInTheDocument();
    expect(
      screen.queryByTestId("process-gpu-coverage-note"),
    ).not.toBeInTheDocument();
  });

  it("joins GPU usage only once it answers the same window as the shown stats", async () => {
    hoisted.getProcessGpuStats.mockResolvedValue(
      ok(available("2026-11-01T00:00:00+00:00")),
    );
    const store = createStore();
    const tree = () => (
      <Provider store={store}>
        <ProcessInsight />
      </Provider>
    );
    const { rerender } = render(tree());
    await flush();
    expect(within(rowFor("game")).getByText("87.5%")).toBeInTheDocument();

    // Switch period: the new stats arrive first, while GPU still holds the
    // previous period's averages.
    let resolveGpu: (value: unknown) => void = () => {};
    hoisted.getProcessGpuStats.mockReturnValue(
      new Promise((resolve) => {
        resolveGpu = resolve;
      }),
    );
    hoisted.getProcessStats.mockResolvedValue(
      ok([{ ...processStats[0], avg_cpu_usage: 55 }, processStats[1]]),
    );
    hoisted.period = 180;
    rerender(tree());
    await flush();

    expect(within(rowFor("game")).getByText("55%")).toBeInTheDocument();
    expect(gpuHeader()).not.toBeInTheDocument();
    expect(screen.queryByText("87.5%")).not.toBeInTheDocument();
    expect(screen.queryByText("—")).not.toBeInTheDocument();
    expect(screen.queryByTestId("load-failure")).not.toBeInTheDocument();

    // The GPU read for the new window arrives and joins.
    await act(async () => {
      resolveGpu(
        ok({
          kind: "available",
          records: [
            {
              pid: 42,
              process_name: "game",
              avg_gpu_usage: 12,
              covered_seconds: 1800,
            },
          ],
          recorded_since: "2026-11-01T00:00:00+00:00",
        }),
      );
    });
    await flush();

    expect(gpuHeader()).toBeInTheDocument();
    expect(within(rowFor("game")).getByText("12.0%")).toBeInTheDocument();
    expect(within(rowFor("updater")).getByText("—")).toBeInTheDocument();
  });

  it("hides the column and points to the Settings conversion on the SQLite archive", async () => {
    hoisted.getProcessGpuStats.mockResolvedValue(
      ok({ kind: "needsNativeDatabase" }),
    );

    const store = await renderInsight();

    expect(gpuHeader()).not.toBeInTheDocument();
    expect(screen.queryByText("—")).not.toBeInTheDocument();
    expect(rowFor("game")).toBeInTheDocument();
    const notice = screen.getByTestId("process-gpu-needs-native-database");
    expect(notice).toHaveTextContent(
      "GPU usage history is recorded only in the native database.",
    );

    fireEvent.click(
      within(notice).getByRole("button", { name: "Convert in Settings" }),
    );
    expect(store.get(displayTargetAtom)).toBe("settings");
  });

  it("explains without a conversion link in a build that cannot convert", async () => {
    hoisted.getProcessGpuStats.mockResolvedValue(
      ok({ kind: "needsNativeDatabase" }),
    );
    hoisted.getDatabaseConversionState.mockResolvedValue({
      kind: "notSupported",
    });

    await renderInsight();

    const notice = screen.getByTestId("process-gpu-needs-native-database");
    expect(notice).toHaveTextContent(
      "GPU usage history is recorded only in the native database.",
    );
    expect(within(notice).queryByRole("button")).not.toBeInTheDocument();
  });

  it("offers no conversion link until conversion support is known", async () => {
    hoisted.getProcessGpuStats.mockResolvedValue(
      ok({ kind: "needsNativeDatabase" }),
    );
    hoisted.getDatabaseConversionState.mockReturnValue(new Promise(() => {}));

    await renderInsight();

    const notice = screen.getByTestId("process-gpu-needs-native-database");
    expect(within(notice).queryByRole("button")).not.toBeInTheDocument();
  });

  it("keeps the conversion link when support cannot be read", async () => {
    hoisted.getProcessGpuStats.mockResolvedValue(
      ok({ kind: "needsNativeDatabase" }),
    );
    hoisted.getDatabaseConversionState.mockRejectedValue(new Error("boom"));
    vi.spyOn(console, "error").mockImplementation(() => {});

    await renderInsight();

    expect(
      screen.getByRole("button", { name: "Convert in Settings" }),
    ).toBeInTheDocument();
  });

  it("does not read conversion support when GPU history is available", async () => {
    hoisted.getProcessGpuStats.mockResolvedValue(
      ok(available("2026-11-01T00:00:00+00:00")),
    );

    await renderInsight();

    expect(hoisted.getDatabaseConversionState).not.toHaveBeenCalled();
  });

  it("keeps the CPU and memory table when the GPU read fails, and retries GPU alone", async () => {
    hoisted.getProcessGpuStats.mockRejectedValueOnce(new Error("boom"));
    vi.spyOn(console, "error").mockImplementation(() => {});

    await renderInsight();

    expect(rowFor("game")).toBeInTheDocument();
    expect(gpuHeader()).not.toBeInTheDocument();
    const failure = screen.getByTestId("load-failure");
    expect(failure).toHaveTextContent("Could not load GPU usage history.");

    hoisted.getProcessGpuStats.mockResolvedValue(
      ok(available("2026-11-01T00:00:00+00:00")),
    );
    fireEvent.click(within(failure).getByRole("button", { name: "Try again" }));
    await flush();

    expect(hoisted.getProcessStats).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId("load-failure")).not.toBeInTheDocument();
    expect(gpuHeader()).toBeInTheDocument();
  });

  it("notes when the period starts before GPU usage was first recorded", async () => {
    // Ten minutes ago: inside the default 60-minute period.
    const recordedSince = new Date(Date.now() - 10 * 60_000).toISOString();
    hoisted.getProcessGpuStats.mockResolvedValue(ok(available(recordedSince)));

    await renderInsight();

    expect(screen.getByTestId("process-gpu-coverage-note")).toHaveTextContent(
      `GPU usage has been recorded since ${new Date(recordedSince).toLocaleString()}`,
    );
    expect(gpuHeader()).toBeInTheDocument();
  });

  it("notes when no GPU usage has been recorded yet", async () => {
    hoisted.getProcessGpuStats.mockResolvedValue(
      ok({ kind: "available", records: [], recorded_since: null }),
    );

    await renderInsight();

    expect(screen.getByTestId("process-gpu-coverage-note")).toHaveTextContent(
      "No GPU usage has been recorded yet.",
    );
  });
});
