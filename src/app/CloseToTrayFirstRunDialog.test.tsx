import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Provider } from "jotai";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import { CloseToTrayFirstRunDialog } from "./CloseToTrayFirstRunDialog";

const mocks = vi.hoisted(() => ({
  error: vi.fn(),
  hideMainWindowToTray: vi.fn(),
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

vi.mock("@/hooks/tauri/useTauriDialog", () => ({
  useTauriDialog: () => ({
    error: mocks.error,
  }),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    hideMainWindowToTray: mocks.hideMainWindowToTray,
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
    mocks.hideMainWindowToTray.mockResolvedValue({
      status: "ok",
      data: null,
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

  it("does not prompt or check availability on startup", async () => {
    renderDialog();
    await waitFor(() =>
      expect(mocks.markCloseToTrayListenerReady).toHaveBeenCalled(),
    );
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(mocks.isCloseToTrayAvailable).not.toHaveBeenCalled();
    expect(mocks.setCloseToTrayPreference).not.toHaveBeenCalled();
  });

  it("queues a close request until settings and the active dialog settle", async () => {
    const onRequestChange = vi.fn();
    const { rerender } = renderDialog(
      <CloseToTrayFirstRunDialog
        settingsSettled={false}
        deferred
        onRequestChange={onRequestChange}
      />,
    );
    await waitFor(() => expect(mocks.closeHandler).toBeDefined());
    act(() => mocks.closeHandler?.());
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(onRequestChange).toHaveBeenLastCalledWith(false);
    rerender(
      <Provider>
        <CloseToTrayFirstRunDialog deferred onRequestChange={onRequestChange} />
      </Provider>,
    );
    expect(onRequestChange).toHaveBeenLastCalledWith(true);
    expect(screen.queryByRole("alertdialog")).toBeNull();
    rerender(
      <Provider>
        <CloseToTrayFirstRunDialog onRequestChange={onRequestChange} />
      </Provider>,
    );
    expect(await screen.findByRole("alertdialog")).toBeVisible();
  });

  it("keeps the existing close-request flow and hides the window when accepted", async () => {
    const user = userEvent.setup();

    renderDialog(<CloseToTrayFirstRunDialog />);

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
    expect(mocks.hideMainWindowToTray).toHaveBeenCalledTimes(1);
    expect(mocks.quitApp).not.toHaveBeenCalled();
  });

  it("reports an error when the App-owned hide transition fails", async () => {
    const user = userEvent.setup();
    mocks.hideMainWindowToTray.mockResolvedValue({
      status: "error",
      error: "hide failed",
    });

    renderDialog(<CloseToTrayFirstRunDialog />);

    await waitFor(() => expect(mocks.closeHandler).toBeDefined());
    act(() => {
      mocks.closeHandler?.();
    });

    await user.click(
      await screen.findByRole("button", { name: "Continue in background" }),
    );

    expect(mocks.error).toHaveBeenCalledWith(
      "Failed to keep the app running in the background.",
    );
  });

  it("shows one translated error dialog, not the backend error, when rejecting tray mode fails", async () => {
    const user = userEvent.setup();
    mocks.setCloseToTrayPreference.mockResolvedValue({
      status: "error",
      error: "save failed",
    });

    renderDialog(<CloseToTrayFirstRunDialog />);

    await waitFor(() => expect(mocks.closeHandler).toBeDefined());
    act(() => {
      mocks.closeHandler?.();
    });

    await screen.findByText("Keep HardwareVisualizer running?");
    await user.click(screen.getAllByRole("button", { name: "Quit app" })[1]);

    await waitFor(() => expect(mocks.error).toHaveBeenCalledTimes(1));
    expect(mocks.error).toHaveBeenCalledWith(
      "Failed to save the close-to-tray setting.",
    );
    expect(mocks.error).not.toHaveBeenCalledWith("save failed");
    expect(mocks.error).not.toHaveBeenCalledWith("Failed to quit the app.");
    expect(mocks.quitApp).not.toHaveBeenCalled();
  });
  it("saves Quit and exits only after the preference is saved", async () => {
    const user = userEvent.setup();
    renderDialog();
    await waitFor(() => expect(mocks.closeHandler).toBeDefined());
    act(() => mocks.closeHandler?.());
    await user.click(
      (await screen.findAllByRole("button", { name: "Quit app" }))[1],
    );
    expect(mocks.setCloseToTrayPreference).toHaveBeenCalledWith(false);
    expect(mocks.quitApp).toHaveBeenCalledTimes(1);
    expect(
      mocks.setCloseToTrayPreference.mock.invocationCallOrder[0],
    ).toBeLessThan(mocks.quitApp.mock.invocationCallOrder[0] ?? 0);
    expect(mocks.hideMainWindowToTray).not.toHaveBeenCalled();
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });
});
