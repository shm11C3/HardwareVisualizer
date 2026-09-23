import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { DatabaseConversionState } from "@/rspc/bindings";

const mockStart = vi.fn();
const mockCancel = vi.fn();
const mockAcknowledgeCompletion = vi.fn();
const mockSetDismissed = vi.fn();
const mockSetHardwareArchiveRetentionDays = vi.fn();
const mockSetNoticeShown = vi.fn(async () => {});

let mockState: DatabaseConversionState = { kind: "notSupported" };
let mockSettled = true;
let mockError: string | null = null;
let mockJustCompleted = false;
let mockDismissed: boolean | null = false;
let mockDismissedPending = false;
let mockHardwareArchiveEnabled = true;
let mockNoticeShown = false;
let mockNoticeShownPending = false;

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) => {
      if (options && "days" in options) {
        return `${key}:${options["days"]}`;
      }
      if (options && "step" in options) {
        return `${key}:${options["step"]}`;
      }
      return key;
    },
  }),
}));

vi.mock("@/features/settings/hooks/useDatabaseConversion", () => ({
  useDatabaseConversion: () => ({
    state: mockState,
    settled: mockSettled,
    error: mockError,
    start: mockStart,
    cancel: mockCancel,
    justCompleted: mockJustCompleted,
    acknowledgeCompletion: mockAcknowledgeCompletion,
  }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: {
      hardwareArchive: {
        enabled: mockHardwareArchiveEnabled,
        retentionDays: 30,
      },
    },
    setHardwareArchiveRetentionDays: mockSetHardwareArchiveRetentionDays,
  }),
}));

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: (key: string) => {
    if (key === "databaseConversionPromptDismissed") {
      return [mockDismissed, mockSetDismissed, mockDismissedPending];
    }
    return [false, vi.fn(), false];
  },
}));

vi.mock("@/features/settings/hooks/useDatabaseConversionNoticeShown", () => ({
  useDatabaseConversionNoticeShown: () => [
    mockNoticeShown,
    mockSetNoticeShown,
    mockNoticeShownPending,
  ],
}));

import { DatabaseConversionPromptDialog } from "./DatabaseConversionPromptDialog";

describe("DatabaseConversionPromptDialog", () => {
  afterEach(cleanup);

  beforeEach(() => {
    vi.clearAllMocks();
    mockState = { kind: "notSupported" };
    mockError = null;
    mockJustCompleted = false;
    mockDismissed = false;
    mockDismissedPending = false;
    mockHardwareArchiveEnabled = true;
    mockNoticeShown = false;
    mockNoticeShownPending = false;
    mockSetNoticeShown.mockImplementation(async () => {});
  });

  it("stays hidden when the build does not support conversion", () => {
    mockState = { kind: "notSupported" };
    render(<DatabaseConversionPromptDialog />);

    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("stays hidden once native is already authoritative", () => {
    mockState = { kind: "nativeAuthoritative" };
    render(<DatabaseConversionPromptDialog />);

    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("stays hidden while Insights recording is disabled", () => {
    mockState = { kind: "sqliteAuthoritative" };
    mockHardwareArchiveEnabled = false;
    render(<DatabaseConversionPromptDialog />);

    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("stays hidden once already dismissed", () => {
    mockState = { kind: "sqliteAuthoritative" };
    mockDismissed = true;
    render(<DatabaseConversionPromptDialog />);

    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("stays hidden while the dismissed flag is still loading", () => {
    mockState = { kind: "sqliteAuthoritative" };
    mockDismissedPending = true;
    render(<DatabaseConversionPromptDialog />);

    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it.each([
    { kind: "sqliteAuthoritative" },
    { kind: "conversionRecoverable", resumable: true },
  ] as DatabaseConversionState[])(
    "shows the prompt with a Convert action for %o",
    (state) => {
      mockState = state;
      render(<DatabaseConversionPromptDialog />);

      expect(screen.getByRole("alertdialog")).toBeInTheDocument();
      expect(
        screen.getByText("databaseConversionPrompt.title"),
      ).toBeInTheDocument();
      expect(
        screen.getByText("pages.settings.insights.databaseConversion.convert"),
      ).toBeInTheDocument();
      expect(
        screen.getByText("databaseConversionPrompt.later"),
      ).toBeInTheDocument();
    },
  );

  it("reports when the startup prompt opens and closes", () => {
    mockState = { kind: "sqliteAuthoritative" };
    const onOpenChange = vi.fn();

    render(<DatabaseConversionPromptDialog onOpenChange={onOpenChange} />);

    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
    expect(onOpenChange).toHaveBeenLastCalledWith(true);

    fireEvent.click(screen.getByText("databaseConversionPrompt.later"));
    expect(onOpenChange).toHaveBeenLastCalledWith(false);
  });

  it("reports pending until the conversion state and dismissal are known", () => {
    mockSettled = false;
    const onPendingChange = vi.fn();
    const { rerender } = render(
      <DatabaseConversionPromptDialog onPendingChange={onPendingChange} />,
    );
    expect(onPendingChange).toHaveBeenLastCalledWith(true);

    mockSettled = true;
    mockDismissedPending = true;
    rerender(
      <DatabaseConversionPromptDialog onPendingChange={onPendingChange} />,
    );
    expect(onPendingChange).toHaveBeenLastCalledWith(true);

    mockDismissedPending = false;
    rerender(
      <DatabaseConversionPromptDialog onPendingChange={onPendingChange} />,
    );
    expect(onPendingChange).toHaveBeenLastCalledWith(false);
  });

  it("stays pending while it will open but is still held back", () => {
    mockState = { kind: "sqliteAuthoritative" };
    const onPendingChange = vi.fn();
    const { rerender } = render(
      <DatabaseConversionPromptDialog
        deferred
        onPendingChange={onPendingChange}
      />,
    );
    expect(onPendingChange).toHaveBeenLastCalledWith(true);

    rerender(
      <DatabaseConversionPromptDialog
        deferred={false}
        onPendingChange={onPendingChange}
      />,
    );
    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
    expect(onPendingChange).toHaveBeenLastCalledWith(false);
  });

  it("waits while another startup dialog is open and shows after it closes", () => {
    mockState = { kind: "sqliteAuthoritative" };
    const { rerender } = render(<DatabaseConversionPromptDialog deferred />);

    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();

    rerender(<DatabaseConversionPromptDialog deferred={false} />);
    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
  });

  it("stays open once shown even if another dialog opens later", () => {
    mockState = { kind: "sqliteAuthoritative" };
    const { rerender } = render(<DatabaseConversionPromptDialog />);
    expect(screen.getByRole("alertdialog")).toBeInTheDocument();

    rerender(<DatabaseConversionPromptDialog deferred />);
    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
  });

  it("puts Later and Convert Now in the same footer row", () => {
    mockState = { kind: "sqliteAuthoritative" };
    render(<DatabaseConversionPromptDialog />);

    const later = screen.getByText("databaseConversionPrompt.later");
    const convert = screen.getByText(
      "pages.settings.insights.databaseConversion.convert",
    );
    expect(later.closest('[data-slot="alert-dialog-footer"]')).not.toBeNull();
    expect(later.closest('[data-slot="alert-dialog-footer"]')).toBe(
      convert.closest('[data-slot="alert-dialog-footer"]'),
    );
  });

  it("puts Close and Try Again in the same footer row on ActionRequired", () => {
    mockState = { kind: "sqliteAuthoritative" };
    const { rerender } = render(<DatabaseConversionPromptDialog />);

    mockState = {
      kind: "actionRequired",
      reason: "conversionFailed",
      diagnostic: "detail",
    };
    rerender(<DatabaseConversionPromptDialog />);

    const close = screen.getByText("databaseConversionPrompt.close");
    const retry = screen.getByText(
      "pages.settings.insights.databaseConversion.retry",
    );
    expect(close.closest('[data-slot="alert-dialog-footer"]')).toBe(
      retry.closest('[data-slot="alert-dialog-footer"]'),
    );
  });

  it("puts Cancel alone in a footer row while converting", () => {
    mockState = { kind: "sqliteAuthoritative" };
    const { rerender } = render(<DatabaseConversionPromptDialog />);

    mockState = { kind: "converting", step: "preflight" };
    rerender(<DatabaseConversionPromptDialog />);

    const cancel = screen.getByText(
      "pages.settings.insights.databaseConversion.cancel",
    );
    const footer = cancel.closest('[data-slot="alert-dialog-footer"]');
    expect(footer).not.toBeNull();
    expect(
      screen.queryByText("databaseConversionPrompt.later"),
    ).not.toBeInTheDocument();
  });

  it("Later persists the dismissal and closes the dialog", () => {
    mockState = { kind: "sqliteAuthoritative" };
    render(<DatabaseConversionPromptDialog />);

    fireEvent.click(screen.getByText("databaseConversionPrompt.later"));

    expect(mockSetDismissed).toHaveBeenCalledWith(true);
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("Convert now calls start()", () => {
    mockState = { kind: "sqliteAuthoritative" };
    render(<DatabaseConversionPromptDialog />);

    fireEvent.click(
      screen.getByText("pages.settings.insights.databaseConversion.convert"),
    );

    expect(mockStart).toHaveBeenCalledTimes(1);
  });

  it("stays open through converting, showing progress and Cancel instead of Later", () => {
    mockState = { kind: "sqliteAuthoritative" };
    const { rerender } = render(<DatabaseConversionPromptDialog />);
    expect(screen.getByRole("alertdialog")).toBeInTheDocument();

    mockState = { kind: "converting", step: "reconciling" };
    rerender(<DatabaseConversionPromptDialog />);

    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
    expect(
      screen.getByText(
        "pages.settings.insights.databaseConversion.converting:pages.settings.insights.databaseConversion.step.reconciling",
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("databaseConversionPrompt.later"),
    ).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByText("pages.settings.insights.databaseConversion.cancel"),
    );
    expect(mockCancel).toHaveBeenCalledTimes(1);
  });

  it("stays open on ActionRequired, offering retry, details and Close", () => {
    mockState = { kind: "sqliteAuthoritative" };
    const { rerender } = render(<DatabaseConversionPromptDialog />);

    mockState = {
      kind: "actionRequired",
      reason: "conversionFailed",
      diagnostic: "ConversionFailed { step: Reconciling }",
    };
    rerender(<DatabaseConversionPromptDialog />);

    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
    expect(
      screen.getByText(
        "pages.settings.insights.databaseConversion.actionRequired.conversionFailed",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("pages.settings.insights.databaseConversion.retry"),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByText("databaseConversionPrompt.close"));
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    // Closing on ActionRequired must not persist the "dismissed for good"
    // flag - only "Later" does.
    expect(mockSetDismissed).not.toHaveBeenCalled();
  });

  it("stays open on completion, showing the one-time notice, then closes when dismissed", async () => {
    mockState = { kind: "sqliteAuthoritative" };
    const { rerender } = render(<DatabaseConversionPromptDialog />);

    mockState = { kind: "nativeAuthoritative" };
    mockJustCompleted = true;
    rerender(<DatabaseConversionPromptDialog />);

    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
    expect(
      screen.getByText(
        "pages.settings.insights.databaseConversion.notice.title",
      ),
    ).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(
        screen.getByText(
          "pages.settings.insights.databaseConversion.notice.keep",
        ),
      );
    });

    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("closes when 'set to 1 year' is confirmed saved", async () => {
    mockState = { kind: "sqliteAuthoritative" };
    mockSetHardwareArchiveRetentionDays.mockResolvedValue(true);
    const { rerender } = render(<DatabaseConversionPromptDialog />);

    mockState = { kind: "nativeAuthoritative" };
    mockJustCompleted = true;
    rerender(<DatabaseConversionPromptDialog />);

    await act(async () => {
      fireEvent.click(
        screen.getByText(
          "pages.settings.insights.databaseConversion.notice.setToOneYear",
        ),
      );
    });

    expect(mockSetHardwareArchiveRetentionDays).toHaveBeenCalledWith(365);
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("stays open when 'set to 1 year' fails to save, so the choice is not lost", async () => {
    mockState = { kind: "sqliteAuthoritative" };
    mockSetHardwareArchiveRetentionDays.mockResolvedValue(false);
    const { rerender } = render(<DatabaseConversionPromptDialog />);

    mockState = { kind: "nativeAuthoritative" };
    mockJustCompleted = true;
    rerender(<DatabaseConversionPromptDialog />);

    await act(async () => {
      fireEvent.click(
        screen.getByText(
          "pages.settings.insights.databaseConversion.notice.setToOneYear",
        ),
      );
    });

    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
    expect(
      screen.getByText(
        "pages.settings.insights.databaseConversion.notice.title",
      ),
    ).toBeInTheDocument();
  });

  describe("nativeAuthoritative without a visible notice offers a Done fallback", () => {
    // CodeRabbit review finding on e0485f46: the notice is the only other
    // exit from `nativeAuthoritative`, but it does not always render -
    // without a fallback, each of these leaves the dialog with no button
    // at all.

    it("when justCompleted never observed the converting -> native transition (a fast completion this mount's own polling missed)", () => {
      mockState = { kind: "sqliteAuthoritative" };
      const { rerender } = render(<DatabaseConversionPromptDialog />);

      // justCompleted stays false: this mount's hook never saw an
      // intermediate "converting" poll before landing on
      // nativeAuthoritative.
      mockState = { kind: "nativeAuthoritative" };
      rerender(<DatabaseConversionPromptDialog />);

      expect(
        screen.queryByText(
          "pages.settings.insights.databaseConversion.notice.title",
        ),
      ).not.toBeInTheDocument();
      const done = screen.getByText("databaseConversionPrompt.done");
      expect(screen.getByRole("alertdialog")).toBeInTheDocument();

      fireEvent.click(done);
      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    });

    it("when the notice-shown flag is already true from an earlier session", () => {
      mockState = { kind: "sqliteAuthoritative" };
      mockNoticeShown = true;
      const { rerender } = render(<DatabaseConversionPromptDialog />);

      mockState = { kind: "nativeAuthoritative" };
      mockJustCompleted = true;
      rerender(<DatabaseConversionPromptDialog />);

      expect(
        screen.queryByText(
          "pages.settings.insights.databaseConversion.notice.title",
        ),
      ).not.toBeInTheDocument();
      expect(
        screen.getByText("databaseConversionPrompt.done"),
      ).toBeInTheDocument();
    });

    it("when the notice-shown flag is still loading", () => {
      mockState = { kind: "sqliteAuthoritative" };
      mockNoticeShownPending = true;
      const { rerender } = render(<DatabaseConversionPromptDialog />);

      mockState = { kind: "nativeAuthoritative" };
      mockJustCompleted = true;
      rerender(<DatabaseConversionPromptDialog />);

      expect(
        screen.queryByText(
          "pages.settings.insights.databaseConversion.notice.title",
        ),
      ).not.toBeInTheDocument();
      expect(
        screen.getByText("databaseConversionPrompt.done"),
      ).toBeInTheDocument();
    });

    it("does not show Done once the notice itself is visible", () => {
      mockState = { kind: "sqliteAuthoritative" };
      const { rerender } = render(<DatabaseConversionPromptDialog />);

      mockState = { kind: "nativeAuthoritative" };
      mockJustCompleted = true;
      rerender(<DatabaseConversionPromptDialog />);

      expect(
        screen.getByText(
          "pages.settings.insights.databaseConversion.notice.title",
        ),
      ).toBeInTheDocument();
      expect(
        screen.queryByText("databaseConversionPrompt.done"),
      ).not.toBeInTheDocument();
    });
  });
});
