import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Provider } from "jotai";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import { TextSelectionToggle } from "@/features/settings/components/general/TextSelectionToggle";

const mocks = vi.hoisted(() => ({
  setTextSelectable: vi.fn(),
}));

// A failed write is reported under the row, never as an OS dialog.
vi.mock("@/hooks/tauri/useTauriDialog", () => {
  throw new Error("settings rows must not open a dialog");
});

vi.mock("@/rspc/bindings", () => ({
  commands: {
    setTextSelectable: mocks.setTextSelectable,
  },
}));

describe("TextSelectionToggle failure line", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("shows a translated line under the row after a failed write and clears it after a successful one", async () => {
    const user = userEvent.setup();
    mocks.setTextSelectable
      .mockResolvedValueOnce({ status: "error", error: "disk is read-only" })
      .mockResolvedValueOnce({ status: "ok", data: null });

    render(
      <Provider>
        <TextSelectionToggle />
      </Provider>,
    );

    expect(screen.queryByTestId("action-failure")).toBeNull();

    await user.click(screen.getByRole("switch"));

    const failure = await screen.findByTestId("action-failure");
    expect(failure).toHaveTextContent(
      "Could not save this setting. The previous value was restored.",
    );
    // The backend detail stays in the log.
    expect(failure).not.toHaveTextContent("disk is read-only");
    // The optimistic update was rolled back.
    expect(screen.getByRole("switch")).not.toBeChecked();

    await user.click(screen.getByRole("switch"));

    await waitFor(() =>
      expect(screen.queryByTestId("action-failure")).toBeNull(),
    );
    expect(screen.getByRole("switch")).toBeChecked();
  });
});
