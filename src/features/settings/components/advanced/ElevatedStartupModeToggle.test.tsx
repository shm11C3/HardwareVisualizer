import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ElevatedStartupModeToggle } from "./ElevatedStartupModeToggle";

const mocks = vi.hoisted(() => ({
  platform: vi.fn(() => "windows"),
  settings: { elevatedStartupMode: false },
  updateSettingAtom: vi.fn(),
  useElevationAvailability: vi.fn(() => "available"),
}));

vi.mock("@/hooks/useElevationAvailability", () => ({
  useElevationAvailability: mocks.useElevationAvailability,
}));

vi.mock("@tauri-apps/plugin-os", () => ({
  platform: mocks.platform,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        "pages.settings.advanced.elevatedStartupMode.name":
          "Run as administrator on startup",
        "pages.settings.advanced.elevatedStartupMode.description":
          "Restart as administrator.",
        "elevationUnavailable.reason": "Not under Program Files.",
        "elevationUnavailable.elevatedStartupModeNotApplied":
          "Saved but not applied.",
      })[key] ?? key,
  }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: mocks.settings,
    updateSettingAtom: mocks.updateSettingAtom,
  }),
}));

describe("ElevatedStartupModeToggle", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.platform.mockReturnValue("windows");
    mocks.settings.elevatedStartupMode = false;
    mocks.useElevationAvailability.mockReturnValue("available");
  });

  afterEach(() => {
    cleanup();
  });

  it("is shown on Windows", () => {
    render(<ElevatedStartupModeToggle />);

    expect(
      screen.getByText("Run as administrator on startup"),
    ).toBeInTheDocument();
  });

  it("is hidden outside Windows", () => {
    mocks.platform.mockReturnValue("linux");

    render(<ElevatedStartupModeToggle />);

    expect(
      screen.queryByText("Run as administrator on startup"),
    ).not.toBeInTheDocument();
  });

  it("updates the elevated startup preference", async () => {
    const user = userEvent.setup();

    render(<ElevatedStartupModeToggle />);

    await user.click(screen.getByRole("switch"));

    expect(mocks.updateSettingAtom).toHaveBeenCalledWith(
      "elevatedStartupMode",
      true,
    );
  });

  it("cannot be turned on outside Program Files and says why", () => {
    mocks.useElevationAvailability.mockReturnValue("unprotectedLocation");

    render(<ElevatedStartupModeToggle />);

    expect(screen.getByRole("switch")).toBeDisabled();
    expect(screen.getByText("Not under Program Files.")).toBeInTheDocument();
  });

  it("keeps a saved on value switchable off and marks it not applied", async () => {
    const user = userEvent.setup();
    mocks.useElevationAvailability.mockReturnValue("unprotectedLocation");
    mocks.settings.elevatedStartupMode = true;

    render(<ElevatedStartupModeToggle />);

    expect(screen.getByText("Saved but not applied.")).toBeInTheDocument();
    await user.click(screen.getByRole("switch"));
    expect(mocks.updateSettingAtom).toHaveBeenCalledWith(
      "elevatedStartupMode",
      false,
    );
  });
});
