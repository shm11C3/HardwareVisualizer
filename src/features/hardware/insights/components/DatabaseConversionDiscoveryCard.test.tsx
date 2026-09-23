import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { DatabaseConversionState } from "@/rspc/bindings";

const mockSetDisplayTargetAtom = vi.fn();
const mockSetStoredDisplayTarget = vi.fn();
const mockSetDismissed = vi.fn();

let mockState: DatabaseConversionState = { kind: "notSupported" };
let mockDismissed: boolean | null = false;
let mockDismissedPending = false;

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("jotai", async (importOriginal) => ({
  ...(await importOriginal<typeof import("jotai")>()),
  useSetAtom: () => mockSetDisplayTargetAtom,
}));

vi.mock("@/features/menu/hooks/useMenu", () => ({
  displayTargetAtom: {},
  DEFAULT_DISPLAY_TARGET: "dashboard",
}));

vi.mock("@/features/settings/hooks/useDatabaseConversion", () => ({
  useDatabaseConversion: () => ({ state: mockState }),
}));

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: (key: string) => {
    if (key === "databaseConversionDiscoveryCardDismissed") {
      return [mockDismissed, mockSetDismissed, mockDismissedPending];
    }
    // The "display" key, used to navigate to Settings.
    return [null, mockSetStoredDisplayTarget, false];
  },
}));

import { DatabaseConversionDiscoveryCard } from "./DatabaseConversionDiscoveryCard";

describe("DatabaseConversionDiscoveryCard", () => {
  afterEach(cleanup);

  beforeEach(() => {
    vi.clearAllMocks();
    mockState = { kind: "notSupported" };
    mockDismissed = false;
    mockDismissedPending = false;
  });

  it.each([
    { kind: "sqliteAuthoritative" },
    { kind: "conversionRecoverable", resumable: true },
  ] as DatabaseConversionState[])("shows the card for %o", (state) => {
    mockState = state;
    render(<DatabaseConversionDiscoveryCard />);

    expect(
      screen.getByTestId("database-conversion-discovery-card"),
    ).toBeInTheDocument();
  });

  it.each([
    { kind: "notSupported" },
    { kind: "converting", step: "reconciling" },
    { kind: "nativeAuthoritative" },
    {
      kind: "actionRequired",
      reason: "conversionFailed",
      diagnostic: "detail",
    },
  ] as DatabaseConversionState[])("hides the card for %o", (state) => {
    mockState = state;
    render(<DatabaseConversionDiscoveryCard />);

    expect(
      screen.queryByTestId("database-conversion-discovery-card"),
    ).not.toBeInTheDocument();
  });

  it("hides the card once already dismissed", () => {
    mockState = { kind: "sqliteAuthoritative" };
    mockDismissed = true;
    render(<DatabaseConversionDiscoveryCard />);

    expect(
      screen.queryByTestId("database-conversion-discovery-card"),
    ).not.toBeInTheDocument();
  });

  it("hides the card while the dismissed flag is still loading", () => {
    mockState = { kind: "sqliteAuthoritative" };
    mockDismissedPending = true;
    render(<DatabaseConversionDiscoveryCard />);

    expect(
      screen.queryByTestId("database-conversion-discovery-card"),
    ).not.toBeInTheDocument();
  });

  it("navigates to Settings when the primary action is clicked", async () => {
    mockState = { kind: "sqliteAuthoritative" };
    const user = userEvent.setup();
    render(<DatabaseConversionDiscoveryCard />);

    await user.click(
      screen.getByText("pages.insights.databaseConversionDiscovery.action"),
    );

    expect(mockSetDisplayTargetAtom).toHaveBeenCalledWith("settings");
    expect(mockSetStoredDisplayTarget).toHaveBeenCalledWith("settings");
  });

  it("dismisses the card for good when Later is clicked", async () => {
    mockState = { kind: "sqliteAuthoritative" };
    const user = userEvent.setup();
    render(<DatabaseConversionDiscoveryCard />);

    await user.click(
      screen.getByText("pages.insights.databaseConversionDiscovery.later"),
    );

    expect(mockSetDismissed).toHaveBeenCalledWith(true);
  });
});
