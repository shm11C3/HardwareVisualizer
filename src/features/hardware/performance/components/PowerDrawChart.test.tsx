import { render, screen } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { powerDrawHistoryAtom } from "@/features/hardware/store/chart";
import { PowerDrawChart } from "./PowerDrawChart";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: { seconds?: number }) =>
      params?.seconds == null ? key : `${key} ${params.seconds}`,
  }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
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
  it("renders selected series while retaining null gaps in the chart data", () => {
    const store = createStore();
    store.set(powerDrawHistoryAtom, {
      cpuWatts: [null, 10.1, null],
      gpuWatts: [null, 2.2, 2.4],
      aneWatts: [null, null, null],
      packageWatts: [null, 12.3, null],
    });

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
});
