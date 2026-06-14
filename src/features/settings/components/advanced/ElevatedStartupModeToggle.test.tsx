import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ElevatedStartupModeToggle } from "./ElevatedStartupModeToggle";

const mocks = vi.hoisted(() => ({
  platform: vi.fn(() => "windows"),
  updateSettingAtom: vi.fn(),
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
      })[key] ?? key,
  }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: {
      elevatedStartupMode: false,
    },
    updateSettingAtom: mocks.updateSettingAtom,
  }),
}));

describe("ElevatedStartupModeToggle", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.platform.mockReturnValue("windows");
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
});
