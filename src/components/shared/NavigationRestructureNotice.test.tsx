import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mockSetDisplayTargetAtom = vi.fn();
const mockSetStoredDisplayTarget = vi.fn();
const mockAcknowledge = vi.fn();
const mockRequestNavigationLayoutFocus = vi.fn();
let mockMenuOpen = false;

let mockSettings = {
  navigationLayout: "grouped" as "grouped" | "classic",
  uiAnnouncementVersion: 0,
  currentUiAnnouncementVersion: 1,
};

vi.mock("jotai", async (importOriginal) => ({
  ...(await importOriginal<typeof import("jotai")>()),
  useAtomValue: () => mockMenuOpen,
  useSetAtom: () => (value: unknown) => {
    if (value === true) {
      mockRequestNavigationLayoutFocus(value);
      return;
    }
    mockSetDisplayTargetAtom(value);
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  navigationMutationPendingAtom: {},
  useSettingsAtom: () => ({
    settings: mockSettings,
    acknowledgeNavigationRestructureAnnouncementAtom: mockAcknowledge,
  }),
}));

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: () => ["performance", mockSetStoredDisplayTarget, false],
}));

import { NavigationRestructureNotice } from "./NavigationRestructureNotice";

describe("NavigationRestructureNotice", () => {
  afterEach(cleanup);

  beforeEach(() => {
    vi.clearAllMocks();
    mockSettings = {
      navigationLayout: "grouped",
      uiAnnouncementVersion: 0,
      currentUiAnnouncementVersion: 1,
    };
    mockMenuOpen = false;
  });

  it("appears only after settings load in grouped navigation", () => {
    const { rerender } = render(
      <NavigationRestructureNotice settingsLoaded={false} />,
    );
    expect(
      screen.queryByLabelText("navigation.notice.title"),
    ).not.toBeInTheDocument();

    rerender(<NavigationRestructureNotice settingsLoaded />);
    expect(
      screen.getByLabelText("navigation.notice.title"),
    ).toBeInTheDocument();
  });

  it("does not appear in classic navigation or after acknowledgement", () => {
    mockSettings.navigationLayout = "classic";
    const { rerender } = render(<NavigationRestructureNotice settingsLoaded />);
    expect(
      screen.queryByLabelText("navigation.notice.title"),
    ).not.toBeInTheDocument();

    mockSettings = {
      navigationLayout: "grouped",
      uiAnnouncementVersion: 1,
      currentUiAnnouncementVersion: 1,
    };
    rerender(<NavigationRestructureNotice settingsLoaded />);
    expect(
      screen.queryByLabelText("navigation.notice.title"),
    ).not.toBeInTheDocument();
  });

  it("opens Settings through the shared UI-local screen selection", () => {
    render(<NavigationRestructureNotice settingsLoaded />);

    fireEvent.click(screen.getByText("navigation.notice.openSettings"));

    expect(mockSetDisplayTargetAtom).toHaveBeenCalledWith("settings");
    expect(mockSetStoredDisplayTarget).toHaveBeenCalledWith("settings");
    expect(mockRequestNavigationLayoutFocus).toHaveBeenCalledWith(true);
  });

  it("moves clear of the expanded sidebar", () => {
    mockMenuOpen = true;

    render(<NavigationRestructureNotice settingsLoaded />);

    expect(screen.getByLabelText("navigation.notice.title")).toHaveClass(
      "left-[17rem]",
    );
  });

  it("persists acknowledgement when dismissed", () => {
    render(<NavigationRestructureNotice settingsLoaded />);

    fireEvent.click(screen.getByLabelText("navigation.notice.dismiss"));

    expect(mockAcknowledge).toHaveBeenCalledOnce();
  });
});
