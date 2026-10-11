import { act, renderHook } from "@testing-library/react";
import { Provider } from "jotai";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";

/**
 * Mock setup
 */
// A failed inventory read is state the specification sheet renders, and a
// failed user-triggered detail read is reported by its caller. Neither is a
// native dialog.
const dialogMessageMock = vi.fn();
vi.mock("@tauri-apps/plugin-dialog", () => ({
  message: dialogMessageMock,
  ask: dialogMessageMock,
  confirm: dialogMessageMock,
}));

// Mock getHardwareInfo, getNetworkInfo, getMemoryInfoDetail in commands
vi.mock("@/rspc/bindings", () => ({
  commands: {
    getHardwareInfo: vi.fn(),
    getNetworkInfo: vi.fn(),
    getMemoryInfoDetail: vi.fn(),
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

  it("init: a failed read sets inventoryLoadFailed, opens no dialog, and a later success clears it", async () => {
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

    expect(result.current.inventoryLoadFailed).toBe(true);
    expect(dialogMessageMock).not.toHaveBeenCalled();
    // Initial state (cpu, memory, gpus are null, storage is empty array) remains
    expect(result.current.hardwareInfo).toEqual({
      cpu: null,
      memory: null,
      gpus: null,
      storage: [],
      motherboard: null,
    });
    expect(consoleErrorSpy).toHaveBeenCalled();

    // The sheet's retry is `init` itself.
    (commands.getHardwareInfo as Mock).mockResolvedValue({
      status: "ok",
      data: {
        cpu: null,
        memory: null,
        gpus: null,
        storage: [],
        motherboard: null,
      },
    });
    await act(async () => {
      await result.current.init();
    });
    expect(result.current.inventoryLoadFailed).toBe(false);
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

  it("initNetwork: a failed read sets networkLoadFailed, opens no dialog, and keeps networkInfo", async () => {
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

    expect(result.current.networkLoadFailed).toBe(true);
    expect(dialogMessageMock).not.toHaveBeenCalled();
    expect(result.current.networkInfo).toEqual([]);
    expect(consoleErrorSpy).toHaveBeenCalled();
    consoleErrorSpy.mockRestore();
  });

  it("fetchMemoryInfoDetail: memory is updated on success", async () => {
    const memoryData = { size: "32GB", memoryType: "DDR5", isDetailed: true };
    (commands.getMemoryInfoDetail as Mock).mockResolvedValue({
      status: "ok",
      data: memoryData,
    });

    const { result } = renderHook(() => useHardwareInfoAtom(), {
      wrapper: Provider,
    });

    let loaded: boolean | undefined;
    await act(async () => {
      loaded = await result.current.fetchMemoryInfoDetail();
    });

    expect(loaded).toBe(true);
    expect(result.current.hardwareInfo.memory).toEqual(memoryData);
  });

  it("fetchMemoryInfoDetail: reports failure through its return value, restores memory, opens no dialog", async () => {
    const seededMemory = {
      size: "16GB",
      memoryType: "DDR4",
      isDetailed: false,
    };
    (commands.getHardwareInfo as Mock).mockResolvedValue({
      status: "ok",
      data: {
        cpu: null,
        memory: seededMemory,
        gpus: null,
        storage: [],
        motherboard: null,
      },
    });
    const errorMsg = "Failed to fetch memory info detail";
    (commands.getMemoryInfoDetail as Mock).mockResolvedValue({
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
    expect(result.current.hardwareInfo.memory).toEqual(seededMemory);

    let loaded: boolean | undefined;
    await act(async () => {
      loaded = await result.current.fetchMemoryInfoDetail();
    });

    expect(loaded).toBe(false);
    expect(dialogMessageMock).not.toHaveBeenCalled();
    expect(result.current.hardwareInfo.memory).toEqual(seededMemory);
    expect(consoleErrorSpy).toHaveBeenCalled();
    consoleErrorSpy.mockRestore();
  });
});
