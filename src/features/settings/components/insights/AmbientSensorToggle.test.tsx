import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AmbientSensorToggle } from "./AmbientSensorToggle";

const mocks = vi.hoisted(() => ({
  platform: vi.fn(() => "windows"),
  toggleSwitchbotMeterAtom: vi.fn(async (_value: boolean) => true),
  switchbotMeterEnabled: false,
}));

vi.mock("@tauri-apps/plugin-os", () => ({
  platform: mocks.platform,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        "pages.settings.insights.ambientSensor.name":
          "Read room temperature from a SwitchBot Meter",
        "pages.settings.insights.ambientSensor.description":
          "Records room temperature and humidity from a meter nearby.",
        "pages.settings.insights.ambientSensor.placement":
          "Place the meter near the PC's air intake, and away from exhaust airflow, direct sunlight, and other heat sources.",
      })[key] ?? key,
  }),
}));

vi.mock("@/components/shared/System", () => ({
  NeedRestart: ({ alertOpen }: { alertOpen: boolean }) =>
    alertOpen ? <div>Restart Required</div> : null,
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: {
      environmentalSensors: {
        switchbotMeterEnabled: mocks.switchbotMeterEnabled,
      },
    },
    toggleSwitchbotMeterAtom: mocks.toggleSwitchbotMeterAtom,
  }),
}));

describe("AmbientSensorToggle", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.platform.mockReturnValue("windows");
    mocks.toggleSwitchbotMeterAtom.mockResolvedValue(true);
    mocks.switchbotMeterEnabled = false;
  });

  afterEach(() => {
    cleanup();
  });

  it("is off until the user turns it on, so no scan starts unasked", () => {
    render(<AmbientSensorToggle />);

    expect(screen.getByRole("switch")).not.toBeChecked();
  });

  it("reflects an enabled ambient source", () => {
    mocks.switchbotMeterEnabled = true;

    render(<AmbientSensorToggle />);

    expect(screen.getByRole("switch")).toBeChecked();
  });

  /**
   * Where the meter sits decides whether its readings mean anything, so
   * the guidance has to be on the screen where the user turns the sensor
   * on — not behind a tooltip or a docs link.
   */
  it("shows the sensor placement guidance next to the switch", () => {
    render(<AmbientSensorToggle />);

    expect(
      screen.getByText(
        "Place the meter near the PC's air intake, and away from exhaust airflow, direct sunlight, and other heat sources.",
      ),
    ).toBeInTheDocument();
  });

  it("turns the ambient source on", async () => {
    const user = userEvent.setup();

    render(<AmbientSensorToggle />);
    await user.click(screen.getByRole("switch"));

    expect(mocks.toggleSwitchbotMeterAtom).toHaveBeenCalledWith(true);
  });

  /**
   * The registry is built once at startup, so the scan only starts or
   * stops with the process. The user has to be told.
   */
  it("tells the user a restart is needed after toggling", async () => {
    const user = userEvent.setup();

    render(<AmbientSensorToggle />);
    await user.click(screen.getByRole("switch"));

    expect(screen.getByText("Restart Required")).toBeInTheDocument();
  });

  /**
   * A refused write (corrupted settings.json, read-only directory)
   * leaves the scan exactly as it was, so telling the user to restart to
   * apply it would name a change that never happened — on top of the
   * error dialog the failed write already raised.
   */
  it("does not ask for a restart when the preference could not be saved", async () => {
    const user = userEvent.setup();
    mocks.toggleSwitchbotMeterAtom.mockResolvedValue(false);

    render(<AmbientSensorToggle />);
    await user.click(screen.getByRole("switch"));

    expect(mocks.toggleSwitchbotMeterAtom).toHaveBeenCalledWith(true);
    expect(screen.queryByText("Restart Required")).not.toBeInTheDocument();
  });

  it("is hidden outside Windows, where no scan can run", () => {
    mocks.platform.mockReturnValue("linux");

    render(<AmbientSensorToggle />);

    expect(screen.queryByRole("switch")).not.toBeInTheDocument();
  });
});
