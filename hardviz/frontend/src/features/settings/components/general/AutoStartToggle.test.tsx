import {
  cleanup,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import { AutoStartToggle } from "@/features/settings/components/general/AutoStartToggle";

const mocks = vi.hoisted(() => ({
  enable: vi.fn(),
  disable: vi.fn(),
  isEnabled: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-autostart", () => ({
  enable: mocks.enable,
  disable: mocks.disable,
  isEnabled: mocks.isEnabled,
}));

describe("AutoStartToggle", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  // Protects: a rejected read must not leave the skeleton on screen forever.
  it("shows a failure with retry when the read fails and recovers to the switch on retry", async () => {
    const user = userEvent.setup();
    mocks.isEnabled
      .mockRejectedValueOnce(new Error("plugin unavailable"))
      .mockResolvedValueOnce(true);

    render(<AutoStartToggle />);

    const failure = await screen.findByTestId("load-failure");
    expect(failure).toHaveTextContent("Could not load the data.");
    // The backend detail stays in the log.
    expect(failure).not.toHaveTextContent("plugin unavailable");
    expect(screen.queryByRole("switch")).toBeNull();

    await user.click(within(failure).getByRole("button"));

    expect(await screen.findByRole("switch")).toBeChecked();
    expect(screen.queryByTestId("load-failure")).toBeNull();
  });

  // Protects: a failed write leaves the switch at the value the OS has, and
  // the failure line is the only trace.
  it("returns the switch to the confirmed value and shows the failure line when the write fails", async () => {
    const user = userEvent.setup();
    mocks.isEnabled.mockResolvedValue(false);
    mocks.enable.mockRejectedValueOnce(new Error("registry denied"));

    render(<AutoStartToggle />);
    await user.click(await screen.findByRole("switch"));

    const failure = await screen.findByTestId("action-failure");
    expect(failure).toHaveTextContent(
      "Could not change the automatic startup setting.",
    );
    expect(failure).not.toHaveTextContent("registry denied");
    await waitFor(() => expect(screen.getByRole("switch")).not.toBeChecked());
    expect(screen.getByRole("switch")).toBeEnabled();
  });

  // Protects: the optimistic value, so the switch does not lag behind the
  // click while the OS call runs, and blocks overlapping writes.
  it("shows the new value immediately while the write is pending and keeps it once the write succeeds", async () => {
    const user = userEvent.setup();
    let finishWrite: () => void = () => {};
    mocks.isEnabled.mockResolvedValue(false);
    mocks.enable.mockReturnValueOnce(
      new Promise<void>((resolve) => {
        finishWrite = resolve;
      }),
    );

    render(<AutoStartToggle />);
    await user.click(await screen.findByRole("switch"));

    await waitFor(() => expect(screen.getByRole("switch")).toBeChecked());
    expect(screen.getByRole("switch")).toBeDisabled();
    expect(screen.queryByTestId("action-failure")).toBeNull();

    finishWrite();

    await waitFor(() => expect(screen.getByRole("switch")).toBeEnabled());
    expect(screen.getByRole("switch")).toBeChecked();
    expect(mocks.enable).toHaveBeenCalledTimes(1);
  });

  // Protects: the failure line belongs to the attempt that failed, so a
  // later successful write clears it.
  it("clears the failure line after the next write succeeds", async () => {
    const user = userEvent.setup();
    mocks.isEnabled.mockResolvedValue(false);
    mocks.enable
      .mockRejectedValueOnce(new Error("denied"))
      .mockResolvedValueOnce(undefined);

    render(<AutoStartToggle />);
    await user.click(await screen.findByRole("switch"));
    await screen.findByTestId("action-failure");
    await waitFor(() => expect(screen.getByRole("switch")).toBeEnabled());

    await user.click(screen.getByRole("switch"));

    await waitFor(() => expect(screen.getByRole("switch")).toBeChecked());
    await waitFor(() => expect(screen.getByRole("switch")).toBeEnabled());
    expect(screen.queryByTestId("action-failure")).toBeNull();
  });

  // Protects: StrictMode runs the mount read twice; a superseded read that
  // rejects after the newer one succeeded must not replace the switch with
  // the failure state.
  it("ignores a superseded read that rejects after a newer read succeeded", async () => {
    let rejectFirst: (reason: Error) => void = () => {};
    let resolveSecond: (value: boolean) => void = () => {};
    mocks.isEnabled
      .mockReturnValueOnce(
        new Promise<boolean>((_resolve, reject) => {
          rejectFirst = reject;
        }),
      )
      .mockReturnValueOnce(
        new Promise<boolean>((resolve) => {
          resolveSecond = resolve;
        }),
      );

    render(
      <StrictMode>
        <AutoStartToggle />
      </StrictMode>,
    );
    await waitFor(() => expect(mocks.isEnabled).toHaveBeenCalledTimes(2));

    resolveSecond(true);
    expect(await screen.findByRole("switch")).toBeChecked();

    rejectFirst(new Error("stale failure"));
    await waitFor(() => expect(console.error).toHaveBeenCalled());

    expect(screen.getByRole("switch")).toBeChecked();
    expect(screen.queryByTestId("load-failure")).toBeNull();
  });
});
