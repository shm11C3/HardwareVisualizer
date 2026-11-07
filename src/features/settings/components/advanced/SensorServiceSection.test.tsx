import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import { SensorServiceSection } from "./SensorServiceSection";

const mocks = vi.hoisted(() => ({
  status: vi.fn(),
  open: vi.fn(),
}));
vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => "windows" }));
vi.mock("@/rspc/bindings", () => ({
  commands: {
    getSensorServiceStatus: mocks.status,
  },
}));
vi.mock("@/lib/openUrl", () => ({ openURL: mocks.open }));

describe("SensorServiceSection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.status.mockResolvedValue({ status: "ok", data: true });
    mocks.open.mockResolvedValue(undefined);
  });
  afterEach(cleanup);

  it("shows compatibility without a recovery action when connected", async () => {
    render(<SensorServiceSection />);
    await screen.findByText("Service connected");
    expect(
      screen.queryByRole("button", {
        name: "Open installation and repair guide",
      }),
    ).toBeNull();
    expect(screen.queryByRole("switch")).toBeNull();
  });

  it("links to the Windows MSI recovery instructions when unavailable", async () => {
    mocks.status.mockResolvedValue({ status: "ok", data: false });
    const user = userEvent.setup();
    render(<SensorServiceSection />);
    await screen.findByText("Service unavailable; other monitoring continues.");
    await user.click(
      screen.getByRole("button", {
        name: "Open installation and repair guide",
      }),
    );
    expect(mocks.open).toHaveBeenCalledWith(
      "https://github.com/shm11C3/HardwareVisualizer/blob/develop/docs/user/external-components.md#windows-sensor-service",
    );
  });

  it("shows a local error when opening the guide fails", async () => {
    const error = vi.spyOn(console, "error").mockImplementation(() => {});
    mocks.status.mockResolvedValue({ status: "ok", data: false });
    mocks.open.mockRejectedValueOnce(new Error("browser unavailable"));
    const user = userEvent.setup();
    render(<SensorServiceSection />);

    await user.click(
      await screen.findByRole("button", {
        name: "Open installation and repair guide",
      }),
    );

    expect(
      await screen.findByText(
        "Failed to open external components documentation.",
      ),
    ).toBeInTheDocument();
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

      await user.click(screen.getByRole("button", { name: "Try again" }));
      await screen.findByText("Service connected");
      expect(screen.queryByTestId("load-failure")).not.toBeInTheDocument();
      expect(mocks.open).not.toHaveBeenCalled();
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
      screen.queryByRole("button", {
        name: "Open installation and repair guide",
      }),
    ).not.toBeInTheDocument();
    error.mockRestore();
  });
});
