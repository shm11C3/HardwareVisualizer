import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TrayWidgetFlyout } from "@/features/tray/TrayWidgetFlyout";

type TrayWidgetFrameEvent = {
  payload: {
    items: Array<{
      metric: "cpu" | "gpu" | "gpu-temp";
      value: string;
      state: "normal" | "warning" | "critical";
    }>;
    tooltip: string;
  };
};

const mocks = vi.hoisted(() => ({
  emit: vi.fn(),
  listen: vi.fn(),
  listeners: new Map<string, (event: TrayWidgetFrameEvent) => void>(),
  translations: {
    "trayWidgetFlyout.openMainWindow": "Open main window",
    "trayWidgetFlyout.close": "Close tray widget",
    "trayWidgetFlyout.noMetrics": "No tray metrics available",
    "trayWidgetFlyout.metric.cpu": "CPU usage",
    "trayWidgetFlyout.metric.gpu": "GPU usage",
    "trayWidgetFlyout.metric.gpu-temp": "GPU temperature",
  } as Record<string, string>,
}));

vi.mock("@tauri-apps/api/event", () => ({
  emit: mocks.emit,
  listen: mocks.listen,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => mocks.translations[key] ?? key,
  }),
}));

const emitFrame = (event: TrayWidgetFrameEvent) => {
  act(() => {
    mocks.listeners.get("tray-widget-frame")?.(event);
  });
};

describe("TrayWidgetFlyout", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.listeners.clear();
    mocks.emit.mockResolvedValue(undefined);
    mocks.listen.mockImplementation((eventName, handler) => {
      mocks.listeners.set(eventName, handler);

      return Promise.resolve(() => {
        mocks.listeners.delete(eventName);
      });
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("renders a localized empty state before the first tray frame", () => {
    render(<TrayWidgetFlyout />);

    expect(screen.getByText("No tray metrics available")).toBeInTheDocument();
  });

  it("listens for tray frames and renders localized metric rows", async () => {
    render(<TrayWidgetFlyout />);

    await waitFor(() => expect(mocks.listen).toHaveBeenCalled());
    emitFrame({
      payload: {
        items: [
          { metric: "cpu", value: "42%", state: "normal" },
          { metric: "gpu-temp", value: "84C", state: "warning" },
        ],
        tooltip: "CPU usage: 42%",
      },
    });

    expect(screen.getByText("CPU usage")).toBeInTheDocument();
    expect(screen.getByText("42%")).toBeInTheDocument();
    expect(screen.getByText("GPU temperature")).toBeInTheDocument();
    expect(screen.getByText("84C")).toBeInTheDocument();
  });

  it("emits window action events from the header buttons", async () => {
    const user = userEvent.setup();
    render(<TrayWidgetFlyout />);

    await user.click(screen.getByRole("button", { name: "Open main window" }));
    await user.click(screen.getByRole("button", { name: "Close tray widget" }));

    expect(mocks.emit).toHaveBeenCalledWith("tray-widget-open-main-requested");
    expect(mocks.emit).toHaveBeenCalledWith("tray-widget-hide-requested");
  });
});
