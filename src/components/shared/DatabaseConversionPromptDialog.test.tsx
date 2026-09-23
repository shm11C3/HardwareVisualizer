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

let mockState: DatabaseConversionState = { kind: "notSupported" };
let mockError: string | null = null;
let mockJustCompleted = false;
let mockDismissed: boolean | null = false;
let mockDismissedPending = false;
let mockHardwareArchiveEnabled = true;

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
    // databaseConversionCompleteNoticeShown, from DatabaseConversionStateBody.
    return [false, vi.fn(), false];
  },
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
});
