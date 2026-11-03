import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import type { PropsWithChildren } from "react";
import { afterEach, expect, it, vi } from "vitest";
import {
  asLiveGpuId,
  liveGpuRecord,
  liveGpuSelection,
} from "@/features/hardware/gpuIdentity";
import {
  gpuNamesAtom,
  gpuUsageHistoriesAtom,
} from "@/features/hardware/store/gpu";
import { selectedGpuIdAtom } from "@/features/hardware/store/selection";
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

vi.mock("@/components/InfoTable", () => ({
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

vi.mock("@/hooks/tauri/useTauriStore", () => ({
  useTauriStore: () => [false],
}));

vi.mock("@/hooks/window/useWindowSize", () => ({
  useWindowSize: () => ({
    isBreak: () => true,
  }),
}));

afterEach(cleanup);

it("shows the effective live fallback while preserving a retired selection", () => {
  const store = createStore();
  store.set(selectedGpuIdAtom, liveGpuSelection(asLiveGpuId("nvapi:0")));
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
  expect(store.get(selectedGpuIdAtom)?.id).toBe("nvapi:0");
});

it("keeps a click before the first sample as unresolved intent", () => {
  // No live adapter is named yet, so the inventory id addresses no readings.
  // It must not enter the selection as a live id.
  const store = createStore();

  render(
    <Provider store={store}>
      <GPUInfo />
    </Provider>,
  );
  fireEvent.click(screen.getByRole("tab", { name: "GPU B" }));

  expect(store.get(selectedGpuIdAtom)).toEqual({
    kind: "unresolved",
    id: "inventory-b",
  });
  expect(screen.getByRole("tab", { name: "GPU B" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
});

it("writes a live selection when the name picks out one live adapter", () => {
  const store = createStore();
  store.set(
    gpuNamesAtom,
    liveGpuRecord([
      [asLiveGpuId("nvapi:0"), "GPU A"],
      [asLiveGpuId("nvapi:1"), "GPU B"],
    ]),
  );

  render(
    <Provider store={store}>
      <GPUInfo />
    </Provider>,
  );
  fireEvent.click(screen.getByRole("tab", { name: "GPU B" }));

  expect(store.get(selectedGpuIdAtom)).toEqual(
    liveGpuSelection(asLiveGpuId("nvapi:1")),
  );
});
