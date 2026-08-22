import { render, screen } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import type { PropsWithChildren } from "react";
import { expect, it, vi } from "vitest";
import { asLiveGpuId, liveGpuRecord } from "@/features/hardware/gpuIdentity";
import {
  gpuNamesAtom,
  gpuUsageHistoriesAtom,
  selectedGpuIdAtom,
} from "@/features/hardware/store/chart";
import { GPUInfo } from "./DashboardItems";

const mocks = vi.hoisted(() => ({
  hardwareInfo: {
    gpus: [
      {
        id: "inventory-a",
        name: "GPU A",
        vendorName: "Vendor A",
        memorySize: "8 GB",
        memorySizeDedicated: "8 GB",
        coreCount: null,
      },
      {
        id: "inventory-b",
        name: "GPU B",
        vendorName: "Vendor B",
        memorySize: "4 GB",
        memorySizeDedicated: "4 GB",
        coreCount: null,
      },
    ],
  },
}));

vi.mock("@tauri-apps/plugin-os", () => ({
  platform: () => "windows",
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/components/charts/DoughnutChart", () => ({
  DoughnutChart: ({
    chartValue,
  }: {
    chartValue: number | null | undefined;
  }) => <div data-testid="doughnut-chart">{chartValue}</div>,
}));

vi.mock("@/components/shared/InfoTable", () => ({
  InfoTable: () => <div data-testid="info-table" />,
}));

vi.mock("@/components/ui/tooltip", () => ({
  TooltipProvider: ({ children }: PropsWithChildren) => <>{children}</>,
  Tooltip: ({ children }: PropsWithChildren) => <>{children}</>,
  TooltipTrigger: ({ children }: PropsWithChildren) => <>{children}</>,
  TooltipContent: ({ children }: PropsWithChildren) => <>{children}</>,
}));

vi.mock("@/features/hardware/hooks/useHardwareInfoAtom", () => ({
  useHardwareInfoAtom: () => ({
    hardwareInfo: mocks.hardwareInfo,
  }),
}));

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: () => [false],
}));

vi.mock("@/hooks/useWindowSize", () => ({
  useWindowSize: () => ({
    isBreak: () => true,
  }),
}));

it("shows the effective live fallback while preserving a retired selection", () => {
  const store = createStore();
  store.set(selectedGpuIdAtom, asLiveGpuId("nvapi:0"));
  store.set(gpuNamesAtom, liveGpuRecord([[asLiveGpuId("nvapi:1"), "GPU B"]]));
  store.set(
    gpuUsageHistoriesAtom,
    liveGpuRecord([[asLiveGpuId("nvapi:1"), [70]]]),
  );

  render(
    <Provider store={store}>
      <GPUInfo />
    </Provider>,
  );

  expect(screen.getByRole("tab", { name: "GPU A" })).toHaveAttribute(
    "aria-selected",
    "false",
  );
  expect(screen.getByRole("tab", { name: "GPU B" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  expect(store.get(selectedGpuIdAtom)).toBe("nvapi:0");
});
