import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Provider } from "jotai";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import { CloseToTrayFirstRunDialog } from "./CloseToTrayFirstRunDialog";

const mocks = vi.hoisted(() => ({
  error: vi.fn(),
  hide: vi.fn(),
  isCloseToTrayAvailable: vi.fn(),
  listen: vi.fn(),
  markCloseToTrayListenerReady: vi.fn(),
  off: vi.fn(),
  quitApp: vi.fn(),
  setCloseToTrayPreference: vi.fn(),
  setTrayWidgetSettings: vi.fn(),
  closeHandler: undefined as (() => void) | undefined,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: mocks.listen,
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    hide: mocks.hide,
  }),
}));

vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({
    error: mocks.error,
  }),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    isCloseToTrayAvailable: mocks.isCloseToTrayAvailable,
    markCloseToTrayListenerReady: mocks.markCloseToTrayListenerReady,
    quitApp: mocks.quitApp,
    setCloseToTrayPreference: mocks.setCloseToTrayPreference,
    setTrayWidgetSettings: mocks.setTrayWidgetSettings,
  },
}));

const renderDialog = (ui = <CloseToTrayFirstRunDialog />) =>
  render(<Provider>{ui}</Provider>);

describe("CloseToTrayFirstRunDialog", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mocks.closeHandler = undefined;
    mocks.listen.mockImplementation(
      (_eventName: string, handler: () => void) => {
        mocks.closeHandler = handler;
        return Promise.resolve(mocks.off);
      },
    );
    mocks.markCloseToTrayListenerReady.mockResolvedValue({
      status: "ok",
      data: null,
    });
    mocks.isCloseToTrayAvailable.mockResolvedValue({
      status: "ok",
      data: true,
    });
    mocks.setCloseToTrayPreference.mockResolvedValue({
      status: "ok",
      data: null,
    });
    mocks.setTrayWidgetSettings.mockResolvedValue({
      status: "ok",
      data: null,
    });
    mocks.quitApp.mockResolvedValue({
      status: "ok",
      data: null,
    });
  });

  it("prompts on startup when the close-to-tray choice has not been made", async () => {
    const user = userEvent.setup();

    renderDialog();

    expect(
      await screen.findByText("Try system tray mode?"),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Use tray mode" }));

    expect(mocks.setCloseToTrayPreference).toHaveBeenCalledWith(true);
    expect(mocks.setTrayWidgetSettings).toHaveBeenCalledWith({
      enabled: true,
      metricOrder: ["cpu", "gpu", "gpu-temp"],
      visibleMetrics: ["cpu", "gpu", "gpu-temp"],
      updateIntervalSecs: 1,
    });
    expect(mocks.hide).not.toHaveBeenCalled();
    expect(mocks.quitApp).not.toHaveBeenCalled();
  });

  it("dismisses the startup prompt from the close icon without saving a preference", async () => {
    const user = userEvent.setup();

    renderDialog();

    expect(
      await screen.findByText("Try system tray mode?"),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Close dialog" }));

    expect(mocks.setCloseToTrayPreference).not.toHaveBeenCalled();
    expect(mocks.setTrayWidgetSettings).not.toHaveBeenCalled();
    expect(screen.queryByText("Try system tray mode?")).not.toBeInTheDocument();
  });

  it("does not show a second error dialog when accepting tray mode fails", async () => {
    const user = userEvent.setup();
    mocks.setCloseToTrayPreference.mockResolvedValue({
      status: "error",
      error: "save failed",
    });

    renderDialog();

    await user.click(
      await screen.findByRole("button", { name: "Use tray mode" }),
    );

    expect(mocks.error).toHaveBeenCalledTimes(1);
    expect(mocks.error).toHaveBeenCalledWith("save failed");
    expect(mocks.error).not.toHaveBeenCalledWith(
      "Failed to keep the app running in the background.",
    );
  });

  it("does not prompt on startup after the choice is already saved", async () => {
    renderDialog(<CloseToTrayFirstRunDialog closeToTrayChoiceMade />);

    await waitFor(() =>
      expect(mocks.markCloseToTrayListenerReady).toHaveBeenCalled(),
    );

    expect(screen.queryByText("Try system tray mode?")).not.toBeInTheDocument();
  });

  it("keeps the existing close-request flow and hides the window when accepted", async () => {
    const user = userEvent.setup();

    renderDialog(<CloseToTrayFirstRunDialog closeToTrayChoiceMade />);

    await waitFor(() => expect(mocks.closeHandler).toBeDefined());
    act(() => {
      mocks.closeHandler?.();
    });

    expect(
      await screen.findByText("Keep HardwareVisualizer running?"),
    ).toBeInTheDocument();

    await user.click(
      screen.getByRole("button", { name: "Continue in background" }),
    );

    expect(mocks.setCloseToTrayPreference).toHaveBeenCalledWith(true);
    expect(mocks.setTrayWidgetSettings).toHaveBeenCalledWith({
      enabled: true,
      metricOrder: ["cpu", "gpu", "gpu-temp"],
      visibleMetrics: ["cpu", "gpu", "gpu-temp"],
      updateIntervalSecs: 1,
    });
    expect(mocks.hide).toHaveBeenCalled();
    expect(mocks.quitApp).not.toHaveBeenCalled();
  });

  it("does not show a second error dialog when rejecting tray mode fails", async () => {
    const user = userEvent.setup();
    mocks.setCloseToTrayPreference.mockResolvedValue({
      status: "error",
      error: "save failed",
    });

    renderDialog(<CloseToTrayFirstRunDialog closeToTrayChoiceMade />);

    await waitFor(() => expect(mocks.closeHandler).toBeDefined());
    act(() => {
      mocks.closeHandler?.();
    });

    await screen.findByText("Keep HardwareVisualizer running?");
    await user.click(screen.getAllByRole("button", { name: "Quit app" })[1]);

    expect(mocks.error).toHaveBeenCalledTimes(1);
    expect(mocks.error).toHaveBeenCalledWith("save failed");
    expect(mocks.error).not.toHaveBeenCalledWith("Failed to quit the app.");
    expect(mocks.quitApp).not.toHaveBeenCalled();
  });
});
