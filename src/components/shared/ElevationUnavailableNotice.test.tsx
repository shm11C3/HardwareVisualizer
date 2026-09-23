import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import { ElevationUnavailableNotice } from "./ElevationUnavailableNotice";

const mocks = vi.hoisted(() => ({
  settings: { elevatedStartupMode: true },
  updateSettingAtom: vi.fn(),
  useElevationAvailability: vi.fn(() => "unprotectedLocation"),
}));

vi.mock("@/hooks/useElevationAvailability", () => ({
  useElevationAvailability: mocks.useElevationAvailability,
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: mocks.settings,
    updateSettingAtom: mocks.updateSettingAtom,
  }),
}));

const title = "Run as administrator on startup was not applied";

describe("ElevationUnavailableNotice", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mocks.settings.elevatedStartupMode = true;
    mocks.useElevationAvailability.mockReturnValue("unprotectedLocation");
  });

  it("explains a skipped elevated startup outside Program Files", () => {
    render(<ElevationUnavailableNotice settingsLoaded />);
    expect(
      screen.getByRole("complementary", { name: title }),
    ).toBeInTheDocument();
  });

  it.each([
    ["the setting is off", false, "unprotectedLocation"],
    ["elevation is available", true, "available"],
    ["the platform cannot elevate", true, "unsupported"],
  ])("stays hidden when %s", (_case, enabled, availability) => {
    mocks.settings.elevatedStartupMode = enabled;
    mocks.useElevationAvailability.mockReturnValue(availability);
    render(<ElevationUnavailableNotice settingsLoaded />);
    expect(screen.queryByRole("complementary")).not.toBeInTheDocument();
  });

  it("stays hidden until settings load", () => {
    render(<ElevationUnavailableNotice settingsLoaded={false} />);
    expect(screen.queryByRole("complementary")).not.toBeInTheDocument();
  });

  it("turns the setting off on request", async () => {
    const user = userEvent.setup();
    render(<ElevationUnavailableNotice settingsLoaded />);
    await user.click(
      screen.getByRole("button", { name: "Turn off the setting" }),
    );
    expect(mocks.updateSettingAtom).toHaveBeenCalledWith(
      "elevatedStartupMode",
      false,
    );
  });

  it("can be dismissed for this launch without changing the setting", async () => {
    const user = userEvent.setup();
    render(<ElevationUnavailableNotice settingsLoaded />);
    await user.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(screen.queryByRole("complementary")).not.toBeInTheDocument();
    expect(mocks.updateSettingAtom).not.toHaveBeenCalled();
  });
});
