import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import { SensorServiceSection } from "./SensorServiceSection";

const mocks = vi.hoisted(() => ({
  status: vi.fn(),
  configure: vi.fn(),
  update: vi.fn(),
  settings: { elevatedStartupMode: true },
}));
vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => "windows" }));
vi.mock("@/rspc/bindings", () => ({
  commands: {
    getSensorServiceStatus: mocks.status,
    configureSensorService: mocks.configure,
  },
}));
vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: mocks.settings,
    updateSettingAtom: mocks.update,
  }),
}));
vi.mock("@/hooks/settings/useElevationAvailability", () => ({
  useProcessElevated: () => true,
}));

describe("SensorServiceSection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.status.mockResolvedValue({
      status: "ok",
      data: {
        connected: true,
        hasReadings: true,
        canConfigure: true,
        detail: null,
      },
    });
    mocks.configure.mockResolvedValue({ status: "ok", data: null });
    mocks.update.mockResolvedValue(true);
  });
  afterEach(cleanup);

  it("repairs the bundled service without duplicating the startup preference control", async () => {
    const user = userEvent.setup();
    render(<SensorServiceSection />);
    await screen.findByText("Service connected");
    expect(mocks.configure).not.toHaveBeenCalled();
    expect(mocks.update).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Repair service" }));
    expect(mocks.configure).toHaveBeenCalledOnce();
    expect(mocks.update).not.toHaveBeenCalled();
    expect(screen.queryByRole("switch")).toBeNull();
    expect(
      screen.queryByText("Use normal permissions on next startup"),
    ).toBeNull();
  });

  it("does not offer writable-install setup or migrate intent when the service is absent", async () => {
    mocks.status.mockResolvedValue({
      status: "ok",
      data: {
        connected: false,
        hasReadings: false,
        canConfigure: false,
        detail: "missing",
      },
    });
    render(<SensorServiceSection />);
    await screen.findByText("Service unavailable; other monitoring continues.");
    expect(
      screen.getByRole("button", { name: "Repair service" }),
    ).toBeDisabled();
    expect(mocks.update).not.toHaveBeenCalled();
  });

  it("shows cancellation as an action failure and preserves the preference", async () => {
    const error = vi.spyOn(console, "error").mockImplementation(() => {});
    mocks.configure.mockResolvedValue({ status: "error", error: "cancelled" });
    const user = userEvent.setup();
    render(<SensorServiceSection />);
    await screen.findByText("Service connected");
    await user.click(screen.getByRole("button", { name: "Repair service" }));
    expect(
      await screen.findByText(/The operation failed or was cancelled/),
    ).toBeInTheDocument();
    expect(mocks.update).not.toHaveBeenCalled();
    error.mockRestore();
  });

  it.each(["rejected", "error result"])(
    "offers retry after an initial status read failure (%s)",
    async (failure) => {
      const error = vi.spyOn(console, "error").mockImplementation(() => {});

      if (failure === "rejected") {
        mocks.status.mockRejectedValueOnce(new Error("read failed"));
      } else {
        mocks.status.mockResolvedValueOnce({
          status: "error",
          error: "read failed",
        });
      }

      const user = userEvent.setup();
      render(<SensorServiceSection />);

      expect(await screen.findByTestId("load-failure")).toHaveTextContent(
        "Failed to check the sensor service.",
      );
      expect(
        screen.queryByText("Checking the sensor service…"),
      ).not.toBeInTheDocument();
      expect(
        screen.queryByText(/The operation failed or was cancelled/),
      ).not.toBeInTheDocument();

      await user.click(screen.getByRole("button", { name: "Try again" }));
      await screen.findByText("Service connected");
      expect(screen.queryByTestId("load-failure")).not.toBeInTheDocument();
      expect(mocks.configure).not.toHaveBeenCalled();
      error.mockRestore();
    },
  );

  it("clears the previous connected state when a refresh fails", async () => {
    const error = vi.spyOn(console, "error").mockImplementation(() => {});
    const user = userEvent.setup();
    render(<SensorServiceSection />);
    await screen.findByText("Service connected");

    mocks.status.mockRejectedValueOnce(new Error("refresh failed"));
    await user.click(screen.getByRole("button", { name: "Check again" }));

    await screen.findByTestId("load-failure");
    expect(screen.queryByText("Service connected")).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Repair service" }),
    ).toBeDisabled();
    error.mockRestore();
  });
});
