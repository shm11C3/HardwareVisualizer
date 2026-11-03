import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import { SensorServiceStartupNotice } from "./SensorServiceStartupNotice";

const mocks = vi.hoisted(() => ({
  status: vi.fn(),
  open: vi.fn(),
  settings: { elevatedStartupMode: true },
  elevated: true as boolean | null,
}));

vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => "windows" }));
vi.mock("@/rspc/bindings", () => ({
  commands: { getSensorServiceStatus: mocks.status },
}));
vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({ settings: mocks.settings }),
}));
vi.mock("@/hooks/settings/useElevationAvailability", () => ({
  useProcessElevated: () => mocks.elevated,
}));
vi.mock("@/hooks/navigation/useSensorServiceSettings", () => ({
  useSensorServiceSettings: () => mocks.open,
}));

describe("SensorServiceStartupNotice", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.settings.elevatedStartupMode = true;
    mocks.elevated = true;
    mocks.status.mockResolvedValue({
      status: "ok",
      data: { connected: true, hasReadings: true },
    });
  });
  afterEach(cleanup);

  it("offers settings for working service readings without changing the saved preference", async () => {
    render(
      <SensorServiceStartupNotice settingsLoaded displayTarget="performance" />,
    );
    await userEvent.click(
      await screen.findByRole("button", { name: "Open sensor settings" }),
    );
    expect(mocks.open).toHaveBeenCalledOnce();
    expect(mocks.settings.elevatedStartupMode).toBe(true);
    await userEvent.click(
      screen.getByRole("button", {
        name: "Hide startup guidance for this session",
      }),
    );
    expect(screen.queryByRole("complementary")).toBeNull();
  });

  it.each([
    { connected: false, hasReadings: false },
    { connected: true, hasReadings: false },
  ])("does not claim sensors are ready with status %j", async (data) => {
    mocks.status.mockResolvedValue({ status: "ok", data });
    render(
      <SensorServiceStartupNotice settingsLoaded displayTarget="performance" />,
    );
    await waitFor(() => expect(mocks.status).toHaveBeenCalledOnce());
    expect(screen.queryByRole("complementary")).toBeNull();
  });

  it("does not probe or prompt when ordinary startup was already chosen", () => {
    mocks.settings.elevatedStartupMode = false;
    render(
      <SensorServiceStartupNotice settingsLoaded displayTarget="performance" />,
    );
    expect(mocks.status).not.toHaveBeenCalled();
    expect(screen.queryByRole("complementary")).toBeNull();
  });
});
