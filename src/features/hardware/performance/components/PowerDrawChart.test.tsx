import { cleanup, render, screen } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { liveSample } from "@/features/hardware/live/liveSamples.testHelpers";
import { publishLiveSampleAtom } from "@/features/hardware/store/liveMetrics";
import { PowerDrawChart } from "./PowerDrawChart";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: { seconds?: number }) =>
      params?.seconds == null ? key : `${key} ${params.seconds}`,
  }),
}));

vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: {
      powerDisplayTargets: ["cpu", "package"],
      lineGraphColor: {
        cpu: "75, 192, 192",
        gpu: "255, 206, 86",
      },
      lineGraphShowScale: false,
      lineGraphShowTooltip: false,
      lineGraphType: "default",
      lineGraphFill: true,
    },
  }),
}));

vi.mock("@/components/ui/chart", () => ({
  ChartContainer: ({ children }: { children: ReactNode }) => <>{children}</>,
  ChartTooltip: () => null,
  ChartTooltipContent: () => null,
}));

vi.mock("recharts", () => ({
  AreaChart: ({
    children,
    data,
  }: {
    children: ReactNode;
    data: Record<string, unknown>[];
  }) => (
    <div data-testid="power-area-chart" data-series={JSON.stringify(data)}>
      {children}
    </div>
  ),
  Area: ({ dataKey }: { dataKey: string }) => (
    <span data-testid={`power-area-${dataKey}`} />
  ),
  CartesianGrid: () => null,
  XAxis: () => null,
  YAxis: () => null,
}));

describe("PowerDrawChart", () => {
  afterEach(cleanup);

  it("renders selected series while retaining null gaps in the chart data", () => {
    const store = createStore();
    for (const [cpu, gpu, package_] of [
      [null, null, null],
      [10.1, 2.2, 12.3],
      [null, 2.4, null],
    ]) {
      store.set(
        publishLiveSampleAtom,
        liveSample({
          cpuPowerWatts: cpu,
          gpuPowerWatts: gpu,
          packagePowerWatts: package_,
        }),
        0,
      );
    }

    render(
      <Provider store={store}>
        <PowerDrawChart />
      </Provider>,
    );

    expect(screen.getByTestId("power-area-cpu")).toBeVisible();
    expect(screen.getByTestId("power-area-package")).toBeVisible();
    expect(screen.queryByTestId("power-area-gpu")).toBeNull();
    expect(screen.queryByTestId("power-area-ane")).toBeNull();

    const data = JSON.parse(
      screen.getByTestId("power-area-chart").getAttribute("data-series") ??
        "[]",
    );
    expect(data.at(-1)).toMatchObject({ cpu: null, package: null });
    expect(data.at(-2)).toMatchObject({ cpu: 10.1, package: 12.3 });
  });

  it("uses a full-width layout when embedded in the Power panel", () => {
    const store = createStore();
    store.set(
      publishLiveSampleAtom,
      liveSample({ cpuPowerWatts: 10, packagePowerWatts: 12 }),
      0,
    );

    render(
      <Provider store={store}>
        <PowerDrawChart showHeading={false} variant="panel" />
      </Provider>,
    );

    expect(screen.getByTestId("performance-power-graph")).toHaveClass(
      "w-full",
      "h-56",
    );
    expect(screen.getByTestId("performance-power-graph")).not.toHaveClass(
      "flex-[2]",
    );
    expect(screen.getByText("pages.performance.power.cpu")).toBeVisible();
    expect(screen.getByText("10.0 W")).toBeVisible();
  });
});
