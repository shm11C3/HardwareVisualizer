import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DoughnutChart, gaugeAnimationDurationMs } from "./DoughnutChart";

/** Hardware metrics are pushed once per second. */
const HARDWARE_UPDATE_INTERVAL_MS = 1_000;

const mocks = vi.hoisted(() => ({
  radialBarProps: [] as Record<string, unknown>[],
  settings: {
    temperatureUnit: "C",
    selectedBackgroundImg: null,
    backgroundImgOpacity: 50,
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({ settings: mocks.settings }),
}));

vi.mock("@/hooks/useWindowSize", () => ({
  useWindowSize: () => ({ isBreak: () => true }),
}));

vi.mock("@/components/ui/chart", () => ({
  ChartContainer: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="chart-container">{children}</div>
  ),
}));

vi.mock("recharts", () => ({
  RadialBarChart: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="radial-bar-chart">{children}</div>
  ),
  RadialBar: (props: Record<string, unknown>) => {
    mocks.radialBarProps.push(props);
    return <div data-testid="radial-bar" />;
  },
  PolarGrid: () => <div />,
  PolarRadiusAxis: () => <div />,
  Label: () => <div />,
}));

afterEach(() => {
  mocks.radialBarProps.length = 0;
  cleanup();
});

describe("DoughnutChart", () => {
  it("finishes its tween within one hardware update interval", () => {
    render(<DoughnutChart chartValue={42} dataType="usage" />);

    expect(mocks.radialBarProps).toHaveLength(1);
    // Recharts' 1500ms default outlives the 1Hz tick, so every update restarts
    // a tween that never settles. The gauge then never shows the current value
    // and the WebKit compositor never idles, which is what spins up the fan on
    // macOS (measured: WebContent 22.1% at 1500ms vs 10.3% at 300ms).
    expect(mocks.radialBarProps[0]?.["animationDuration"]).toBeLessThan(
      HARDWARE_UPDATE_INTERVAL_MS,
    );
  });

  it("keeps the tween bounded across metric updates", () => {
    const { rerender } = render(
      <DoughnutChart chartValue={42} dataType="usage" />,
    );
    rerender(<DoughnutChart chartValue={57} dataType="usage" />);

    expect(mocks.radialBarProps.length).toBeGreaterThan(1);
    for (const props of mocks.radialBarProps) {
      expect(props["animationDuration"]).toBe(gaugeAnimationDurationMs);
    }
  });

  it("renders a placeholder instead of the gauge when no value is available", () => {
    const { queryByTestId } = render(
      <DoughnutChart chartValue={null} dataType="usage" />,
    );

    expect(queryByTestId("radial-bar")).toBeNull();
    expect(mocks.radialBarProps).toHaveLength(0);
  });
});
