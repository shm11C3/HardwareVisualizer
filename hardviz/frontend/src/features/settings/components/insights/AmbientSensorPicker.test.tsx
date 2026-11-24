import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AmbientSensorCandidate } from "@/rspc/bindings";
import { AmbientSensorPicker } from "./AmbientSensorPicker";

const mocks = vi.hoisted(() => ({
  getAmbientSensorCandidates: vi.fn(),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getAmbientSensorCandidates: mocks.getAmbientSensorCandidates,
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) =>
      ({
        "pages.settings.insights.ambientSensor.picker.title": "Device to read",
        "pages.settings.insights.ambientSensor.picker.searching":
          "Listening for nearby SwitchBot devices…",
        "pages.settings.insights.ambientSensor.picker.device": `${params?.["shortId"]} — ${params?.["temperature"]} ${params?.["unit"]} / ${params?.["humidity"]} %`,
      })[key] ?? key,
  }),
}));

const candidate = (
  overrides: Partial<AmbientSensorCandidate> = {},
): AmbientSensorCandidate => ({
  deviceId: "d051fa0f2cd0",
  shortId: "2cd0",
  temperature: 25.2,
  temperatureUnit: "C",
  humidityPercent: 54,
  selected: false,
  ...overrides,
});

describe("AmbientSensorPicker", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getAmbientSensorCandidates.mockResolvedValue({
      status: "ok",
      data: [],
    });
  });

  afterEach(() => {
    cleanup();
  });

  /**
   * The reading is what tells two devices in one room apart, so it is
   * shown in the unit the user reads everything else in - and the unit
   * is named, because a bare number could be either scale.
   */
  it("shows each reading in the unit the backend converted it to", async () => {
    mocks.getAmbientSensorCandidates.mockResolvedValue({
      status: "ok",
      data: [candidate({ temperature: 77.4, temperatureUnit: "F" })],
    });

    render(<AmbientSensorPicker selectedDeviceId={null} onSelect={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("2cd0 — 77.4 °F / 54 %")).toBeInTheDocument();
    });
  });

  it("labels a Celsius reading as such", async () => {
    mocks.getAmbientSensorCandidates.mockResolvedValue({
      status: "ok",
      data: [candidate()],
    });

    render(<AmbientSensorPicker selectedDeviceId={null} onSelect={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("2cd0 — 25.2 °C / 54 %")).toBeInTheDocument();
    });
  });

  /**
   * Humidity is optional on the wire; its absence is shown as absent,
   * not as zero.
   */
  it("shows a missing humidity as a dash rather than a number", async () => {
    mocks.getAmbientSensorCandidates.mockResolvedValue({
      status: "ok",
      data: [candidate({ humidityPercent: null })],
    });

    render(<AmbientSensorPicker selectedDeviceId={null} onSelect={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("2cd0 — 25.2 °C / — %")).toBeInTheDocument();
    });
  });

  it("says it is still listening while nothing has been heard", async () => {
    render(<AmbientSensorPicker selectedDeviceId={null} onSelect={vi.fn()} />);

    await waitFor(() => {
      expect(mocks.getAmbientSensorCandidates).toHaveBeenCalled();
    });
    expect(
      screen.getByText("Listening for nearby SwitchBot devices…"),
    ).toBeInTheDocument();
  });
});
