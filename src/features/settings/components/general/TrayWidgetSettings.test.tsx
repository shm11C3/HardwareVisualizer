import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type React from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  normalizeMetricOrder,
  normalizeSettings,
  normalizeVisibleMetrics,
  TrayWidgetSettings,
  type TrayWidgetStore,
} from "@/features/settings/components/general/TrayWidgetSettings";
import { useTauriStore } from "@/hooks/useTauriStore";
import { commands } from "@/rspc/bindings";

const mocks = vi.hoisted(() => ({
  latestDragEnd: undefined as
    | ((event: {
        active: { id: string };
        over: { id: string } | null;
      }) => Promise<void>)
    | undefined,
  platform: vi.fn(() => "windows"),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      const translations: Record<string, string> = {
        "pages.settings.general.trayWidget.name": "Tray widget",
        "pages.settings.general.trayWidget.description":
          "Show live metrics in the tray.",
        "pages.settings.general.trayWidget.unavailable":
          "Tray widget is unavailable.",
        "pages.settings.general.trayWidget.metrics": "Metrics",
        "pages.settings.general.trayWidget.updateInterval": "Update interval",
        "pages.settings.general.trayWidget.metric.cpu": "CPU",
        "pages.settings.general.trayWidget.metric.gpu": "GPU",
        "pages.settings.general.trayWidget.metric.gpu-temp": "GPU temperature",
        "pages.settings.general.trayWidget.dragMetric": `Drag ${params?.metric}`,
        "pages.settings.general.trayWidget.seconds": `${params?.count}s`,
      };

      return translations[key] || key;
    },
  }),
}));

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-os", () => ({
  platform: mocks.platform,
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    isCloseToTrayAvailable: vi.fn(),
  },
}));

vi.mock("@/components/ui/label", () => ({
  Label: ({
    children,
    className,
    htmlFor,
  }: {
    children: React.ReactNode;
    className?: string;
    htmlFor?: string;
  }) => (
    <label className={className} htmlFor={htmlFor}>
      {children}
    </label>
  ),
}));

vi.mock("@/components/ui/switch", () => ({
  Switch: ({
    checked,
    disabled,
    id,
    onCheckedChange,
  }: {
    checked: boolean;
    disabled?: boolean;
    id?: string;
    onCheckedChange: (checked: boolean) => void;
  }) => (
    <input
      aria-checked={checked}
      checked={checked}
      disabled={disabled}
      id={id}
      onChange={(event) => onCheckedChange(event.currentTarget.checked)}
      role="switch"
      type="checkbox"
    />
  ),
}));

vi.mock("@/components/ui/checkbox", () => ({
  Checkbox: ({
    checked,
    disabled,
    id,
    onCheckedChange,
  }: {
    checked: boolean;
    disabled?: boolean;
    id?: string;
    onCheckedChange: (checked: boolean) => void;
  }) => (
    <input
      checked={checked}
      disabled={disabled}
      id={id}
      onChange={(event) => onCheckedChange(event.currentTarget.checked)}
      type="checkbox"
    />
  ),
}));

vi.mock("@/components/ui/select", () => ({
  Select: ({
    disabled,
    onValueChange,
    value,
  }: {
    children: React.ReactNode;
    disabled?: boolean;
    onValueChange: (value: string) => void;
    value: string;
  }) => (
    <select
      aria-label="Update interval"
      disabled={disabled}
      onChange={(event) => onValueChange(event.currentTarget.value)}
      value={value}
    >
      <option value="1">1s</option>
      <option value="2">2s</option>
      <option value="5">5s</option>
    </select>
  ),
  SelectContent: ({ children }: { children: React.ReactNode }) => children,
  SelectItem: ({
    children,
    value,
  }: {
    children: React.ReactNode;
    value: string;
  }) => <option value={value}>{children}</option>,
  SelectTrigger: ({ children }: { children: React.ReactNode }) => children,
  SelectValue: () => null,
}));

vi.mock("@dnd-kit/core", () => ({
  closestCenter: vi.fn(),
  DndContext: ({
    children,
    onDragEnd,
  }: {
    children: React.ReactNode;
    onDragEnd: (event: {
      active: { id: string };
      over: { id: string } | null;
    }) => Promise<void>;
  }) => {
    mocks.latestDragEnd = onDragEnd;
    return <div data-testid="dnd-context">{children}</div>;
  },
  KeyboardSensor: vi.fn(),
  PointerSensor: vi.fn(),
  useSensor: vi.fn((sensor, options) => ({ options, sensor })),
  useSensors: vi.fn((...sensors) => sensors),
}));

vi.mock("@dnd-kit/sortable", () => ({
  arrayMove: <T,>(items: T[], oldIndex: number, newIndex: number) => {
    const nextItems = [...items];
    const [removed] = nextItems.splice(oldIndex, 1);
    nextItems.splice(newIndex, 0, removed);
    return nextItems;
  },
  SortableContext: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="sortable-context">{children}</div>
  ),
  sortableKeyboardCoordinates: vi.fn(),
  useSortable: vi.fn(({ id, disabled }: { id: string; disabled: boolean }) => ({
    attributes: { "data-sortable-id": id },
    listeners: { "data-sortable-disabled": String(disabled) },
    setNodeRef: vi.fn(),
    transform: null,
    transition: undefined,
  })),
  verticalListSortingStrategy: vi.fn(),
}));

vi.mock("@dnd-kit/utilities", () => ({
  CSS: {
    Transform: {
      toString: vi.fn(() => undefined),
    },
  },
}));

const setSettings = vi.fn();

const mockStore = (
  settings: Partial<TrayWidgetStore> | null,
  isPending = false,
) => {
  vi.mocked(useTauriStore).mockReturnValue([
    settings,
    setSettings,
    isPending,
  ] as never);
};

const mockAvailability = (data: boolean) => {
  vi.mocked(commands.isCloseToTrayAvailable).mockResolvedValue({
    data,
    status: "ok",
  });
};

describe("TrayWidgetSettings normalization", () => {
  it("returns null for pending settings", () => {
    expect(normalizeSettings(null)).toBeNull();
  });

  it("normalizes legacy metrics into order and visibility", () => {
    const settings = normalizeSettings({
      metrics: ["gpu", "gpu-temp", "cpu"],
      updateIntervalSecs: 2,
    });

    expect(settings).toEqual({
      enabled: false,
      metricOrder: ["gpu", "gpu-temp", "cpu"],
      visibleMetrics: ["gpu", "gpu-temp", "cpu"],
      updateIntervalSecs: 2,
    });
  });

  it("normalizes old temperature metric keys", () => {
    const settings = normalizeSettings({
      metricOrder: ["gpu", "temp", "cpu"],
      visibleMetrics: ["temp", "gpu"],
      updateIntervalSecs: 2,
    });

    expect(settings).toEqual({
      enabled: false,
      metricOrder: ["gpu", "gpu-temp", "cpu"],
      visibleMetrics: ["gpu-temp", "gpu"],
      updateIntervalSecs: 2,
    });
  });

  it("keeps metric order and filters visible metrics by that order", () => {
    const settings = normalizeSettings({
      enabled: true,
      metricOrder: ["gpu", "cpu"],
      visibleMetrics: ["cpu", "gpu-temp"],
      updateIntervalSecs: 1,
    });

    expect(settings).toEqual({
      enabled: true,
      metricOrder: ["gpu", "cpu", "gpu-temp"],
      visibleMetrics: ["cpu", "gpu-temp"],
      updateIntervalSecs: 1,
    });
  });

  it("removes unavailable metrics from order and visibility", () => {
    const settings = normalizeSettings(
      {
        enabled: true,
        metricOrder: ["gpu-temp", "gpu", "cpu"],
        visibleMetrics: ["gpu-temp", "gpu"],
        updateIntervalSecs: 1,
      },
      ["gpu-temp"],
    );

    expect(settings).toEqual({
      enabled: true,
      metricOrder: ["gpu", "cpu"],
      visibleMetrics: ["gpu"],
      updateIntervalSecs: 1,
    });
  });

  it("removes duplicate metrics from metric order", () => {
    expect(normalizeMetricOrder(["gpu", "gpu-temp", "gpu", "cpu"])).toEqual([
      "gpu",
      "gpu-temp",
      "cpu",
    ]);
  });

  it("removes unavailable metrics from metric order", () => {
    expect(
      normalizeMetricOrder(["gpu", "gpu-temp", "cpu"], ["gpu-temp"]),
    ).toEqual(["gpu", "cpu"]);
  });

  it("removes duplicate and non-configurable visible metrics", () => {
    expect(
      normalizeVisibleMetrics(
        ["gpu-temp", "gpu", "gpu", "cpu"],
        ["cpu", "gpu"],
      ),
    ).toEqual(["gpu", "cpu"]);
  });

  it("falls back to the default interval when persisted interval is invalid", () => {
    for (const updateIntervalSecs of [3, 4, 99]) {
      const settings = normalizeSettings({
        enabled: true,
        metricOrder: ["gpu", "cpu"],
        visibleMetrics: ["gpu"],
        updateIntervalSecs,
      });

      expect(settings?.updateIntervalSecs).toBe(1);
    }
  });

  it("falls back to metric order when visible metrics normalize to empty", () => {
    expect(normalizeVisibleMetrics(["gpu-temp"], ["gpu", "cpu"])).toEqual([
      "gpu",
      "cpu",
    ]);
  });

  it("handles completely empty persisted objects", () => {
    const settings = normalizeSettings({} as Partial<TrayWidgetStore>);

    expect(settings).toEqual({
      enabled: false,
      metricOrder: ["cpu", "gpu", "gpu-temp"],
      visibleMetrics: ["cpu", "gpu", "gpu-temp"],
      updateIntervalSecs: 1,
    });
  });
});

describe("TrayWidgetSettings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.latestDragEnd = undefined;
    mocks.platform.mockReturnValue("windows");
    mockAvailability(true);
    mockStore({
      enabled: true,
      metricOrder: ["cpu", "gpu", "gpu-temp"],
      visibleMetrics: ["cpu", "gpu", "gpu-temp"],
      updateIntervalSecs: 1,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("hides metric controls while the tray widget is disabled", () => {
    mockStore({
      enabled: false,
      metricOrder: ["cpu", "gpu", "gpu-temp"],
      visibleMetrics: ["cpu", "gpu", "gpu-temp"],
      updateIntervalSecs: 1,
    });

    render(<TrayWidgetSettings />);

    expect(
      screen.getByRole("switch", { name: "Tray widget" }),
    ).not.toBeChecked();
    expect(screen.queryByText("Metrics")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Update interval")).not.toBeInTheDocument();
  });

  it("renders configurable metrics and the persisted update interval", () => {
    mockStore({
      enabled: true,
      metricOrder: ["gpu", "cpu"],
      visibleMetrics: ["gpu"],
      updateIntervalSecs: 2,
    });

    render(<TrayWidgetSettings />);

    expect(screen.getByRole("switch", { name: "Tray widget" })).toBeChecked();
    expect(screen.getByText("Metrics")).toBeInTheDocument();
    expect(screen.getByLabelText("GPU")).toBeChecked();
    expect(screen.getByLabelText("CPU")).not.toBeChecked();
    expect(screen.getByLabelText("GPU temperature")).not.toBeChecked();
    expect(screen.getByLabelText("GPU")).toBeDisabled();
    expect(screen.getByLabelText("Update interval")).toHaveValue("2");
    expect(screen.getByRole("button", { name: "Drag GPU" })).toBeEnabled();
  });

  it("hides GPU temperature metric on macOS", () => {
    mocks.platform.mockReturnValue("macos");

    render(<TrayWidgetSettings />);

    expect(screen.getByLabelText("CPU")).toBeChecked();
    expect(screen.getByLabelText("GPU")).toBeChecked();
    expect(screen.queryByLabelText("GPU temperature")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Drag GPU temperature" }),
    ).not.toBeInTheDocument();
  });

  it("persists enabled changes", async () => {
    const user = userEvent.setup();
    mockStore({
      enabled: false,
      metricOrder: ["cpu", "gpu", "gpu-temp"],
      visibleMetrics: ["cpu", "gpu", "gpu-temp"],
      updateIntervalSecs: 1,
    });

    render(<TrayWidgetSettings />);

    await user.click(screen.getByRole("switch", { name: "Tray widget" }));

    expect(setSettings).toHaveBeenCalledWith({
      enabled: true,
      metricOrder: ["cpu", "gpu", "gpu-temp"],
      visibleMetrics: ["cpu", "gpu", "gpu-temp"],
      updateIntervalSecs: 1,
    });
  });

  it("persists metric visibility without allowing the last visible metric to be removed", async () => {
    const user = userEvent.setup();

    render(<TrayWidgetSettings />);

    await user.click(screen.getByLabelText("GPU"));

    expect(setSettings).toHaveBeenCalledWith({
      enabled: true,
      metricOrder: ["cpu", "gpu", "gpu-temp"],
      visibleMetrics: ["cpu", "gpu-temp"],
      updateIntervalSecs: 1,
    });

    cleanup();
    setSettings.mockClear();
    mockStore({
      enabled: true,
      metricOrder: ["cpu", "gpu", "gpu-temp"],
      visibleMetrics: ["cpu"],
      updateIntervalSecs: 1,
    });

    render(<TrayWidgetSettings />);

    expect(screen.getByLabelText("CPU")).toBeDisabled();
    expect(screen.getByLabelText("GPU")).not.toBeChecked();
  });

  it("persists newly visible metrics without duplicating existing selections", async () => {
    const user = userEvent.setup();
    mockStore({
      enabled: true,
      metricOrder: ["cpu", "gpu", "gpu-temp"],
      visibleMetrics: ["cpu"],
      updateIntervalSecs: 1,
    });

    render(<TrayWidgetSettings />);

    await user.click(screen.getByLabelText("GPU"));

    expect(setSettings).toHaveBeenCalledWith({
      enabled: true,
      metricOrder: ["cpu", "gpu", "gpu-temp"],
      visibleMetrics: ["cpu", "gpu"],
      updateIntervalSecs: 1,
    });
  });

  it("persists interval changes", async () => {
    const user = userEvent.setup();

    render(<TrayWidgetSettings />);

    await user.selectOptions(screen.getByLabelText("Update interval"), "5");

    expect(setSettings).toHaveBeenCalledWith({
      enabled: true,
      metricOrder: ["cpu", "gpu", "gpu-temp"],
      visibleMetrics: ["cpu", "gpu", "gpu-temp"],
      updateIntervalSecs: 5,
    });
  });

  it("persists metric order after drag end", async () => {
    render(<TrayWidgetSettings />);

    await waitFor(() => expect(mocks.latestDragEnd).toBeDefined());
    await mocks.latestDragEnd?.({
      active: { id: "gpu" },
      over: { id: "cpu" },
    });

    expect(setSettings).toHaveBeenCalledWith({
      enabled: true,
      metricOrder: ["gpu", "cpu", "gpu-temp"],
      visibleMetrics: ["cpu", "gpu", "gpu-temp"],
      updateIntervalSecs: 1,
    });
  });

  it("ignores drag end events without a valid target", async () => {
    render(<TrayWidgetSettings />);

    await waitFor(() => expect(mocks.latestDragEnd).toBeDefined());
    await mocks.latestDragEnd?.({
      active: { id: "gpu" },
      over: { id: "gpu" },
    });
    await mocks.latestDragEnd?.({
      active: { id: "gpu" },
      over: null,
    });
    await mocks.latestDragEnd?.({
      active: { id: "memory" },
      over: { id: "cpu" },
    });

    expect(setSettings).not.toHaveBeenCalled();
  });

  it("disables settings when the tray widget is unavailable", async () => {
    mockAvailability(false);

    render(<TrayWidgetSettings />);

    await waitFor(() =>
      expect(
        screen.getByText("Tray widget is unavailable."),
      ).toBeInTheDocument(),
    );

    expect(screen.getByRole("switch", { name: "Tray widget" })).toBeDisabled();
    expect(screen.queryByText("Metrics")).not.toBeInTheDocument();
  });

  it("disables settings when availability check returns an error result", async () => {
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    vi.mocked(commands.isCloseToTrayAvailable).mockResolvedValue({
      error: "not available",
      status: "error",
    });

    render(<TrayWidgetSettings />);

    await waitFor(() =>
      expect(
        screen.getByText("Tray widget is unavailable."),
      ).toBeInTheDocument(),
    );

    expect(consoleError).toHaveBeenCalledWith(
      "Failed to check tray widget availability:",
      "not available",
    );
    consoleError.mockRestore();
  });

  it("disables settings when availability check throws", async () => {
    const err = new Error("boom");
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    vi.mocked(commands.isCloseToTrayAvailable).mockRejectedValue(err);

    render(<TrayWidgetSettings />);

    await waitFor(() =>
      expect(
        screen.getByText("Tray widget is unavailable."),
      ).toBeInTheDocument(),
    );

    expect(consoleError).toHaveBeenCalledWith(
      "Failed to check tray widget availability:",
      err,
    );
    consoleError.mockRestore();
  });

  it("does not persist changes while settings are pending", async () => {
    const user = userEvent.setup();
    mockStore(null, true);

    render(<TrayWidgetSettings />);

    await user.click(screen.getByRole("switch", { name: "Tray widget" }));

    expect(setSettings).not.toHaveBeenCalled();
  });

  it("ignores enabled changes when settings have not loaded yet", async () => {
    const user = userEvent.setup();
    mockStore(null, false);

    render(<TrayWidgetSettings />);

    await user.click(screen.getByRole("switch", { name: "Tray widget" }));

    expect(setSettings).not.toHaveBeenCalled();
  });
});
