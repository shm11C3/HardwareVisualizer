import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import { Insights } from "./Insights";

const mocks = vi.hoisted(() => ({
  record: vi.fn(),
  gpuNames: [] as string[],
  gpuNamesSettled: true,
  target: "main",
  pending: false,
  setTarget: vi.fn(),
}));

vi.mock("@/hooks/analytics/useProductEvent", () => ({
  useProductEvent: () => mocks.record,
}));
vi.mock("@/hooks/tauri/useTauriStore", () => ({
  useTauriStore: (key: string, fallback: unknown) =>
    key === "insightDisplayTarget"
      ? [mocks.target, mocks.setTarget, mocks.pending]
      : [fallback, vi.fn(), false],
}));
vi.mock("@/features/hardware/hooks/useGpuNames", () => ({
  useGpuNames: () => ({
    gpuNames: mocks.gpuNames,
    hasError: false,
    settled: mocks.gpuNamesSettled,
    retry: vi.fn(),
  }),
}));
vi.mock("@/features/hardware/hooks/useHardwareInfoAtom", () => ({
  useHardwareInfoAtom: () => ({ init: vi.fn() }),
}));
vi.mock("@/features/hardware/insights/hooks/useArchivePeriodOptions", () => ({
  useArchivePeriodOptions: () => [],
}));
vi.mock("@/features/hardware/insights/components/InsightChart", () => ({
  InsightChart: () => null,
  GpuInsightChart: () => null,
}));
vi.mock("./components/RetentionNote", () => ({ RetentionNote: () => null }));
vi.mock("./components/SelectPeriod", () => ({ SelectPeriod: () => null }));
vi.mock("./cooling/CoolingInsightView", () => ({
  CoolingInsightView: () => null,
}));
vi.mock("./process/ProcessInsight", () => ({ ProcessInsight: () => null }));
vi.mock("./snapshot/Snapshot", () => ({ Snapshot: () => null }));

describe("Insights product events", () => {
  beforeEach(() => {
    mocks.gpuNames = [];
    mocks.gpuNamesSettled = true;
    mocks.target = "main";
    mocks.pending = false;
    mocks.setTarget.mockReset();
  });
  afterEach(() => {
    cleanup();
    mocks.record.mockClear();
  });

  it("records the first resolved tab once, mapping main to hardware", () => {
    const { rerender } = render(<Insights />);

    expect(mocks.record).toHaveBeenCalledTimes(1);
    expect(mocks.record).toHaveBeenCalledWith({
      capability: { insightOpened: "hardware" },
    });

    // A rerender of the same resolved tab is not a new view.
    rerender(<Insights />);
    expect(mocks.record).toHaveBeenCalledTimes(1);
  });

  it("records nothing while the stored tab is still loading", () => {
    mocks.pending = true;
    const { rerender } = render(<Insights />);
    expect(mocks.record).not.toHaveBeenCalled();

    mocks.pending = false;
    mocks.target = "cooling";
    rerender(<Insights />);
    expect(mocks.record).toHaveBeenCalledTimes(1);
    expect(mocks.record).toHaveBeenCalledWith({
      capability: { insightOpened: "cooling" },
    });
  });

  it("records each tab change, with every GPU tab counted as gpu", () => {
    mocks.gpuNames = ["GPU A", "GPU B"];
    mocks.target = "GPU A";
    const { rerender } = render(<Insights />);
    mocks.target = "GPU B";
    rerender(<Insights />);
    mocks.target = "process";
    rerender(<Insights />);

    expect(mocks.record.mock.calls.map(([event]) => event)).toEqual([
      { capability: { insightOpened: "gpu" } },
      { capability: { insightOpened: "gpu" } },
      { capability: { insightOpened: "process" } },
    ]);
  });

  it("records the tab a stored target resolves to, not the stored name", () => {
    // A stored GPU tab that no longer exists falls back to the main tab.
    mocks.target = "Removed GPU";
    render(<Insights />);

    expect(mocks.record).toHaveBeenCalledTimes(1);
    expect(mocks.record).toHaveBeenCalledWith({
      capability: { insightOpened: "hardware" },
    });
  });

  it("waits for the GPU names so a stored GPU tab is not counted as hardware", () => {
    // Until the names load, a stored GPU tab is shown as the main tab.
    mocks.gpuNamesSettled = false;
    mocks.target = "GPU A";
    const { rerender } = render(<Insights />);
    expect(mocks.record).not.toHaveBeenCalled();

    mocks.gpuNames = ["GPU A"];
    mocks.gpuNamesSettled = true;
    rerender(<Insights />);

    expect(mocks.record.mock.calls.map(([event]) => event)).toEqual([
      { capability: { insightOpened: "gpu" } },
    ]);
  });
});
