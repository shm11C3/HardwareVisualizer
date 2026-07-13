import { cleanup, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ChartTemplate } from "./Usage";

const mockSettings = vi.hoisted(() => ({
  displayTargets: ["cpu", "memory", "gpu"],
  graphFitToWindow: false,
  graphMarginPx: 32,
  graphSize: "xl",
  lineGraphMix: false,
  burnInShift: false,
  burnInShiftMode: "jump",
  burnInShiftOptions: null,
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({ settings: mockSettings }),
}));

vi.mock("@/features/hardware/store/chart", () => ({
  cpuUsageHistoryAtom: {},
  graphicUsageHistoryAtom: {},
  memoryUsageHistoryAtom: {},
}));

vi.mock("jotai", () => ({
  useAtom: () => [[]],
}));

vi.mock("@/hooks/useBurnInShift", () => ({
  useBurnInShift: vi.fn(),
}));

vi.mock("@/components/charts/LineChart", () => ({
  LineChartComponent: (
    props: ComponentProps<"div"> & {
      fitToContainer?: boolean;
      lineGraphMix: boolean;
    },
  ) => (
    <div
      data-testid="usage-chart"
      data-fit-to-container={String(props.fitToContainer)}
      data-mixed={String(props.lineGraphMix)}
    />
  ),
}));

describe("ChartTemplate", () => {
  afterEach(cleanup);

  beforeEach(() => {
    mockSettings.displayTargets = ["cpu", "memory", "gpu"];
    mockSettings.graphFitToWindow = false;
    mockSettings.graphMarginPx = 32;
    mockSettings.lineGraphMix = false;
  });

  it("keeps fixed-size charts when fit-to-window is disabled", () => {
    render(<ChartTemplate />);

    const layout = screen.getByTestId("usage-chart-layout");
    expect(layout).toHaveClass("p-8");
    expect(layout).not.toHaveClass("space-y-4");
    expect(layout).not.toHaveStyle({ padding: "32px" });
    expect(screen.getAllByTestId("usage-chart")).toHaveLength(3);
    for (const chart of screen.getAllByTestId("usage-chart")) {
      expect(chart).toHaveAttribute("data-fit-to-container", "false");
    }
  });

  it("splits visible charts across the available height with the configured margin", () => {
    mockSettings.displayTargets = ["cpu", "gpu"];
    mockSettings.graphFitToWindow = true;
    mockSettings.graphMarginPx = 24;

    render(<ChartTemplate />);

    const layout = screen.getByTestId("usage-chart-layout");
    expect(layout).toHaveClass("flex", "overflow-y-auto");
    expect(layout).toHaveStyle({
      height: "calc(100dvh - var(--burnin-padding) - var(--burnin-padding))",
      padding: "24px",
    });
    expect(screen.getAllByTestId("usage-chart")).toHaveLength(2);
    for (const chart of screen.getAllByTestId("usage-chart")) {
      expect(chart).toHaveAttribute("data-fit-to-container", "true");
    }
  });

  it("removes the outer burn-in padding when the configured margin is zero", () => {
    mockSettings.graphFitToWindow = true;
    mockSettings.graphMarginPx = 0;

    const { container } = render(<ChartTemplate />);

    expect(container.querySelector(".burnin-root")).toHaveStyle(
      "--burnin-padding: 0px",
    );
    expect(screen.getByTestId("usage-chart-layout")).toHaveStyle({
      padding: "0px",
    });
  });

  it("does not reserve sidebar space in fullscreen", () => {
    mockSettings.graphFitToWindow = true;

    render(<ChartTemplate isFullScreen />);

    expect(screen.getByTestId("usage-chart-layout")).not.toHaveClass("ml-16");
  });

  it("fits the single mixed chart to the available height", () => {
    mockSettings.graphFitToWindow = true;
    mockSettings.lineGraphMix = true;

    render(<ChartTemplate />);

    const charts = screen.getAllByTestId("usage-chart");
    expect(charts).toHaveLength(1);
    expect(charts[0]).toHaveAttribute("data-fit-to-container", "true");
    expect(charts[0]).toHaveAttribute("data-mixed", "true");
  });
});
