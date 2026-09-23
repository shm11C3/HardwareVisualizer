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
const mockSetNoticeShown = vi.fn();
const mockSetHardwareArchiveRetentionDays = vi.fn();

let mockState: DatabaseConversionState = { kind: "notSupported" };
let mockError: string | null = null;
let mockJustCompleted = false;
let mockNoticeShown: boolean | null = false;

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

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: () => [mockNoticeShown, mockSetNoticeShown, false],
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: { hardwareArchive: { retentionDays: 30 } },
    setHardwareArchiveRetentionDays: mockSetHardwareArchiveRetentionDays,
  }),
}));

import { DatabaseConversionSettings } from "./DatabaseConversionSettings";

describe("DatabaseConversionSettings", () => {
  afterEach(cleanup);

  beforeEach(() => {
    vi.clearAllMocks();
    mockState = { kind: "notSupported" };
    mockError = null;
    mockJustCompleted = false;
    mockNoticeShown = false;
  });

  it("renders nothing when the build does not support conversion", () => {
    const { container } = render(<DatabaseConversionSettings />);
    expect(container).toBeEmptyDOMElement();
  });

  it("offers a convert action while SQLite is authoritative", () => {
    mockState = { kind: "sqliteAuthoritative" };
    render(<DatabaseConversionSettings />);

    const button = screen.getByText(
      "pages.settings.insights.databaseConversion.convert",
    );
    fireEvent.click(button);
    expect(mockStart).toHaveBeenCalledTimes(1);
  });

  it("shows progress and a cancel action while converting", () => {
    mockState = { kind: "converting", step: "reconciling" };
    render(<DatabaseConversionSettings />);

    expect(
      screen.getByText(
        "pages.settings.insights.databaseConversion.converting:pages.settings.insights.databaseConversion.step.reconciling",
      ),
    ).toBeInTheDocument();

    fireEvent.click(
      screen.getByText("pages.settings.insights.databaseConversion.cancel"),
    );
    expect(mockCancel).toHaveBeenCalledTimes(1);
  });

  it("shows an actionable message and a details section on failure", () => {
    mockState = {
      kind: "actionRequired",
      reason: "conversionFailed",
      diagnostic: "ConversionFailed { step: Reconciling }",
    };
    render(<DatabaseConversionSettings />);

    expect(
      screen.getByText(
        "pages.settings.insights.databaseConversion.actionRequired.conversionFailed",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("ConversionFailed { step: Reconciling }"),
    ).toBeInTheDocument();
  });

  it.each(["conversionFailed", "conversionCancelled"] as const)(
    "offers a retry action for %s, which re-invokes start()",
    (reason) => {
      mockState = {
        kind: "actionRequired",
        reason,
        diagnostic: "detail",
      };
      render(<DatabaseConversionSettings />);

      const retryButton = screen.getByText(
        "pages.settings.insights.databaseConversion.retry",
      );
      fireEvent.click(retryButton);
      expect(mockStart).toHaveBeenCalledTimes(1);
    },
  );

  it("does not offer a retry action for a non-retryable issue", () => {
    mockState = {
      kind: "actionRequired",
      reason: "authorityDisagreement",
      diagnostic: "detail",
    };
    render(<DatabaseConversionSettings />);

    expect(
      screen.queryByText("pages.settings.insights.databaseConversion.retry"),
    ).not.toBeInTheDocument();
  });

  it("shows the one-time notice after a completed conversion and hides it once dismissed", async () => {
    mockState = { kind: "nativeAuthoritative" };
    mockJustCompleted = true;
    mockNoticeShown = false;

    render(<DatabaseConversionSettings />);

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
    expect(mockSetNoticeShown).toHaveBeenCalledWith(true);
    expect(mockAcknowledgeCompletion).toHaveBeenCalledTimes(1);
  });

  it("dismisses the notice for good once 'set to 1 year' is confirmed saved", async () => {
    mockState = { kind: "nativeAuthoritative" };
    mockJustCompleted = true;
    mockNoticeShown = false;
    mockSetHardwareArchiveRetentionDays.mockResolvedValue(true);

    render(<DatabaseConversionSettings />);

    await act(async () => {
      fireEvent.click(
        screen.getByText(
          "pages.settings.insights.databaseConversion.notice.setToOneYear",
        ),
      );
    });

    expect(mockSetHardwareArchiveRetentionDays).toHaveBeenCalledWith(365);
    expect(mockSetNoticeShown).toHaveBeenCalledWith(true);
    expect(mockAcknowledgeCompletion).toHaveBeenCalledTimes(1);
  });

  it("keeps the notice showing when 'set to 1 year' fails to save, so the choice is not lost", async () => {
    mockState = { kind: "nativeAuthoritative" };
    mockJustCompleted = true;
    mockNoticeShown = false;
    mockSetHardwareArchiveRetentionDays.mockResolvedValue(false);

    render(<DatabaseConversionSettings />);

    await act(async () => {
      fireEvent.click(
        screen.getByText(
          "pages.settings.insights.databaseConversion.notice.setToOneYear",
        ),
      );
    });

    expect(mockSetHardwareArchiveRetentionDays).toHaveBeenCalledWith(365);
    expect(mockSetNoticeShown).not.toHaveBeenCalled();
    expect(mockAcknowledgeCompletion).not.toHaveBeenCalled();
    expect(
      screen.getByText(
        "pages.settings.insights.databaseConversion.notice.title",
      ),
    ).toBeInTheDocument();
  });

  it("never shows the notice again once it was already shown", () => {
    mockState = { kind: "nativeAuthoritative" };
    mockJustCompleted = true;
    mockNoticeShown = true;

    render(<DatabaseConversionSettings />);

    expect(
      screen.queryByText(
        "pages.settings.insights.databaseConversion.notice.title",
      ),
    ).not.toBeInTheDocument();
  });
});
