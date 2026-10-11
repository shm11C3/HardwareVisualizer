import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CPUInfo, GPUInfo, MemoryInfo } from "./DashboardItems";

const mocks = vi.hoisted(() => ({
  init: vi.fn(),
  inventoryLoadFailed: false,
}));

vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => "windows" }));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/components/charts/DoughnutChart", () => ({
  DoughnutChart: () => <div />,
}));

vi.mock("./MiniLineChart", () => ({
  MiniLineChart: () => <div />,
}));

vi.mock("@/components/ui/tooltip", () => ({
  TooltipProvider: ({ children }: { children: unknown }) => <>{children}</>,
  Tooltip: ({ children }: { children: unknown }) => <>{children}</>,
  TooltipTrigger: ({ children }: { children: unknown }) => <>{children}</>,
  TooltipContent: ({ children }: { children: unknown }) => <>{children}</>,
}));

vi.mock("@/features/hardware/hooks/useProcessInfo", () => ({
  useProcessInfo: () => ({ processes: [], hasError: false }),
}));

// The inventory never arrived: every field is still null.
vi.mock("@/features/hardware/hooks/useHardwareInfoAtom", () => ({
  useHardwareInfoAtom: () => ({
    hardwareInfo: {
      cpu: null,
      memory: null,
      gpus: null,
      storage: [],
      motherboard: null,
    },
    inventoryLoadFailed: mocks.inventoryLoadFailed,
    init: mocks.init,
  }),
}));

vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: {
      selectedBackgroundImg: null,
      backgroundImgOpacity: 100,
      lineGraphColor: { cpu: "0,0,0", gpu: "0,0,0", memory: "0,0,0" },
      temperatureUnit: "C",
    },
  }),
}));

vi.mock("@/hooks/tauri/useTauriStore", () => ({
  useTauriStore: () => [false],
}));

vi.mock("@/hooks/window/useWindowSize", () => ({
  useWindowSize: () => ({ isBreak: () => true }),
}));

afterEach(() => {
  cleanup();
  mocks.inventoryLoadFailed = false;
  vi.clearAllMocks();
});

describe.each([
  ["CPUInfo", CPUInfo],
  ["MemoryInfo", MemoryInfo],
  ["GPUInfo", GPUInfo],
])("%s with an inventory that has not loaded", (_name, Card) => {
  it("keeps the skeleton while the read is still pending", () => {
    const { container } = render(<Card />);

    expect(screen.queryByTestId("load-failure")).toBeNull();
    expect(container.querySelector('[class*="animate-pulse"]')).not.toBeNull();
  });

  it("replaces the skeleton with a failure and retries through init", () => {
    mocks.inventoryLoadFailed = true;

    const { container } = render(<Card />);

    expect(screen.getByTestId("load-failure")).toHaveTextContent(
      "pages.dashboard.systemSpecifications.inventoryLoadFailed",
    );
    expect(container.querySelector('[class*="animate-pulse"]')).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "shared.retry" }));
    expect(mocks.init).toHaveBeenCalledOnce();
  });
});
