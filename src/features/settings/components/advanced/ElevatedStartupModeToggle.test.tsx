import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ElevatedStartupModeToggle } from "./ElevatedStartupModeToggle";

const mocks = vi.hoisted(() => ({
  platform: vi.fn(() => "windows"),
  settings: { elevatedStartupMode: false },
  updateSettingAtom: vi.fn(),
  useElevationAvailability: vi.fn((): string | null => "available"),
  useProcessElevated: vi.fn((): boolean | null => false),
}));

vi.mock("@/hooks/useElevationAvailability", async (importOriginal) => ({
  ...(await importOriginal<
    typeof import("@/hooks/useElevationAvailability")
  >()),
  useElevationAvailability: mocks.useElevationAvailability,
  useProcessElevated: mocks.useProcessElevated,
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
        "elevationUnavailable.unknown": "Could not verify.",
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
    mocks.useProcessElevated.mockReturnValue(false);
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

  it.each([
    ["still loading", null, null],
    ["unknown", "unknown", "Could not verify."],
    ["unsupported", "unsupported", "Could not verify."],
  ])(
    "cannot be turned on while availability is %s",
    (_case, availability, note) => {
      mocks.useElevationAvailability.mockReturnValue(availability);

      render(<ElevatedStartupModeToggle />);

      expect(screen.getByRole("switch")).toBeDisabled();
      if (note) {
        expect(screen.getByText(note)).toBeInTheDocument();
      } else {
        expect(screen.queryByText("Not under Program Files.")).toBeNull();
      }
    },
  );

  it("does not claim not applied when the user started it as administrator", () => {
    mocks.useElevationAvailability.mockReturnValue("unprotectedLocation");
    mocks.useProcessElevated.mockReturnValue(true);
    mocks.settings.elevatedStartupMode = true;

    render(<ElevatedStartupModeToggle />);

    expect(screen.queryByText("Saved but not applied.")).toBeNull();
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
