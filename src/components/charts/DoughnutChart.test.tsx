import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DoughnutChart, gaugeAnimationDurationMs } from "./DoughnutChart";

/** Hardware metrics are pushed once per second. */
const HARDWARE_UPDATE_INTERVAL_MS = 1_000;

const mocks = vi.hoisted(() => ({
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

const gaugeRing = (container: HTMLElement) =>
  container.querySelector("circle[stroke-dasharray]");

const sweptFraction = (container: HTMLElement) => {
  const ring = gaugeRing(container);
  const dashArray = Number(ring?.getAttribute("stroke-dasharray"));
  const dashOffset = Number(ring?.getAttribute("stroke-dashoffset"));

  return 1 - dashOffset / dashArray;
};

afterEach(cleanup);

describe("DoughnutChart", () => {
  it("finishes its tween within one hardware update interval", () => {
    const { container } = render(
      <DoughnutChart chartValue={42} dataType="usage" />,
    );

    // A tween longer than the tick restarts before it settles, so the gauge
    // never shows the current value and the compositor never idles — that is
    // what spun up the fan on macOS.
    expect(gaugeAnimationDurationMs).toBeLessThan(HARDWARE_UPDATE_INTERVAL_MS);
    expect(gaugeRing(container)).toHaveStyle({
      transitionDuration: `${gaugeAnimationDurationMs}ms`,
    });
  });

  it("moves only the dash offset when the value changes", () => {
    const { container, rerender } = render(
      <DoughnutChart chartValue={42} dataType="usage" />,
    );
    const before = gaugeRing(container)?.getAttribute("stroke-dasharray");

    rerender(<DoughnutChart chartValue={57} dataType="usage" />);

    // The ring geometry is stable across ticks; only the offset moves, which
    // is what lets the transition run without a React frame per step.
    expect(gaugeRing(container)?.getAttribute("stroke-dasharray")).toBe(before);
    expect(sweptFraction(container)).toBeCloseTo(0.57, 5);
  });

  it("sweeps the ring in proportion to the value", () => {
    const { container } = render(
      <DoughnutChart chartValue={25} dataType="usage" />,
    );

    expect(sweptFraction(container)).toBeCloseTo(0.25, 5);
  });

  it("renders a placeholder instead of the gauge when no value is available", () => {
    const { container } = render(
      <DoughnutChart chartValue={null} dataType="usage" />,
    );

    expect(gaugeRing(container)).toBeNull();
  });

  it("shows the reading and its unit", () => {
    const { getByText } = render(
      <DoughnutChart chartValue={42} dataType="usage" />,
    );

    expect(getByText("42%")).toBeInTheDocument();
  });
});
