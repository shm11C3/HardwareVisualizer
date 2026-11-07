import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "@/rspc/bindings";
import { CpuCoreTemperatures } from "./CpuCoreTemperatures";

vi.mock("@/rspc/bindings", () => ({
  commands: { getCpuCoreTemperatures: vi.fn() },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { processor: number }) =>
      options ? `${key} ${options.processor}` : key,
  }),
}));

const getTemperatures = vi.mocked(commands.getCpuCoreTemperatures);

const advance = async (ms: number) => {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
};

describe("CPU core temperatures", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    getTemperatures.mockReset();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("shows physical core readings and preserves unavailable slots", async () => {
    getTemperatures.mockResolvedValue({
      status: "ok",
      data: {
        temperatureUnit: "F",
        readings: [
          { processorGroup: 0, logicalProcessor: 0, temperature: 104 },
          { processorGroup: 0, logicalProcessor: 2, temperature: null },
        ],
      },
    });

    render(<CpuCoreTemperatures />);
    await advance(0);

    expect(screen.getByText("104 °F")).toBeVisible();
    expect(screen.getByText("shared.notAvailable")).toBeVisible();
    expect(
      screen.getByText("pages.performance.coreTemperatureProcessor 2"),
    ).toBeVisible();
  });

  it("keeps an empty result out of the screen", async () => {
    getTemperatures.mockResolvedValue({
      status: "ok",
      data: { temperatureUnit: "C", readings: [] },
    });

    render(<CpuCoreTemperatures />);
    await advance(0);

    expect(getTemperatures).toHaveBeenCalledOnce();
    expect(screen.queryByRole("heading")).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("retries a failed read from the panel", async () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    getTemperatures
      .mockResolvedValueOnce({ status: "error", error: "fixture failure" })
      .mockResolvedValueOnce({
        status: "ok",
        data: { temperatureUnit: "C", readings: [] },
      });

    render(<CpuCoreTemperatures />);
    await advance(0);
    expect(screen.getByRole("alert")).toBeVisible();
    expect(screen.queryByText("fixture failure")).toBeNull();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "shared.retry" }));
    });

    expect(getTemperatures).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("only polls while mounted and the document is visible", async () => {
    let hidden = true;
    vi.spyOn(document, "hidden", "get").mockImplementation(() => hidden);
    getTemperatures.mockResolvedValue({
      status: "ok",
      data: { temperatureUnit: "C", readings: [] },
    });
    const changeVisibility = async (value: boolean) => {
      await act(async () => {
        hidden = value;
        document.dispatchEvent(new Event("visibilitychange"));
      });
    };

    const { unmount } = render(<CpuCoreTemperatures />);
    await advance(6000);
    expect(getTemperatures).not.toHaveBeenCalled();

    await changeVisibility(false);
    expect(getTemperatures).toHaveBeenCalledOnce();
    await advance(3000);
    expect(getTemperatures).toHaveBeenCalledTimes(2);

    await changeVisibility(true);
    await advance(6000);
    expect(getTemperatures).toHaveBeenCalledTimes(2);
    await changeVisibility(false);
    expect(getTemperatures).toHaveBeenCalledTimes(3);

    unmount();
    await advance(6000);
    expect(getTemperatures).toHaveBeenCalledTimes(3);
  });
});
