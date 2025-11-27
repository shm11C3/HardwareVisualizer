import { act, renderHook } from "@testing-library/react";
import { Provider } from "jotai";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";

/**
 * Mock setup
 */
const errorMock = vi.fn();
vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({
    error: errorMock,
  }),
}));

// Mock getHardwareInfo, getNetworkInfo in commands
vi.mock("@/rspc/bindings", () => ({
  commands: {
    getHardwareInfo: vi.fn(),
    getNetworkInfo: vi.fn(),
  },
}));

/**
 * Import hook to test
 */
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import { commands } from "@/rspc/bindings";

/**
 * Test execution
 */
describe("useHardwareInfoAtom", () => {
  beforeEach(() => {
    // Reset mock state before each test execution
    vi.clearAllMocks();
  });

  it("init: hardwareInfo is updated on success", async () => {
    // Mock data returned from commands
    const hardwareData = {
      cpu: "Intel",
      memory: "16GB",
      gpus: "NVIDIA",
      storage: ["SSD"],
    };
    (commands.getHardwareInfo as Mock).mockResolvedValue({
      data: hardwareData,
    });

    // Render hook wrapped with Provider
    const { result } = renderHook(() => useHardwareInfoAtom(), {
      wrapper: Provider,
    });

    // Execute init() using act() in async
    await act(async () => {
      await result.current.init();
    });

    // Verify that hardwareInfo is updated
    expect(result.current.hardwareInfo).toEqual(hardwareData);
  });

  it("init: error() is called on error and hardwareInfo remains at initial value", async () => {
    const errorMsg = "Failed to fetch hardware info";
    (commands.getHardwareInfo as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    const { result } = renderHook(() => useHardwareInfoAtom(), {
      wrapper: Provider,
    });

    await act(async () => {
      await result.current.init();
    });

    // Verify that error() was called
    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    // Initial state (cpu, memory, gpus are null, storage is empty array) remains
    expect(result.current.hardwareInfo).toEqual({
      cpu: null,
      memory: null,
      gpus: null,
      storage: [],
    });
    expect(consoleErrorSpy).toHaveBeenCalled();
    consoleErrorSpy.mockRestore();
  });

  it("initNetwork: networkInfo is updated on success", async () => {
    const networkData = [{ name: "eth0", ip: "192.168.1.2" }];
    (commands.getNetworkInfo as Mock).mockResolvedValue({ data: networkData });

    const { result } = renderHook(() => useHardwareInfoAtom(), {
      wrapper: Provider,
    });

    await act(async () => {
      await result.current.initNetwork();
    });

    expect(result.current.networkInfo).toEqual(networkData);
  });

  it("initNetwork: error() is called on error and networkInfo remains at initial value", async () => {
    const errorMsg = "Failed to fetch network info";
    (commands.getNetworkInfo as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    const { result } = renderHook(() => useHardwareInfoAtom(), {
      wrapper: Provider,
    });

    await act(async () => {
      await result.current.initNetwork();
    });

    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    expect(result.current.networkInfo).toEqual([]);
    expect(consoleErrorSpy).toHaveBeenCalled();
    consoleErrorSpy.mockRestore();
  });
});
