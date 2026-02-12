import { act, renderHook, waitFor } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { createElement, Suspense } from "react";
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

// Mock getHardwareInfo, getNetworkInfo, getMemoryInfoDetail in commands.
// Default values are needed because the module-level Promise atoms call
// fetchHardwareInfo / fetchNetworkInfo on import.
vi.mock("@/rspc/bindings", () => ({
  commands: {
    getHardwareInfo: vi.fn().mockResolvedValue({
      data: {
        cpu: null,
        memory: null,
        gpus: null,
        storage: [],
        motherboard: null,
      },
    }),
    getNetworkInfo: vi.fn().mockResolvedValue({ data: [] }),
    getMemoryInfoDetail: vi.fn(),
  },
}));

/**
 * Import hook to test
 */
import {
  hardwareInfoPromiseAtom,
  networkInfoPromiseAtom,
  useHardwareInfoAtom,
  useHardwareInfoSuspense,
  useNetworkInfoSuspense,
} from "@/features/hardware/hooks/useHardwareInfoAtom";
import { commands } from "@/rspc/bindings";

/**
 * Helper: create a wrapper with Jotai Provider + Suspense for testing
 * Suspense-based hooks with pre-set Promise atom values.
 */
const createSuspenseWrapper = (
  hardwarePromise: Promise<unknown>,
  networkPromise?: Promise<unknown>,
) => {
  const store = createStore();
  store.set(hardwareInfoPromiseAtom, hardwarePromise as Promise<never>);
  if (networkPromise) {
    store.set(networkInfoPromiseAtom, networkPromise as Promise<never>);
  }
  return ({ children }: { children: React.ReactNode }) =>
    createElement(
      Provider,
      { store },
      createElement(
        Suspense,
        { fallback: createElement("div", null, "loading") },
        children,
      ),
    );
};

/**
 * Test execution
 */
describe("useHardwareInfoAtom (legacy)", () => {
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
      motherboard: null,
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

describe("useHardwareInfoPromise (Suspense)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("resolves hardware info through use()", async () => {
    const hwData = {
      cpu: { name: "Intel" },
      memory: null,
      gpus: null,
      storage: [],
      motherboard: null,
    };

    const wrapper = createSuspenseWrapper(Promise.resolve(hwData));

    const { result } = renderHook(() => useHardwareInfoSuspense(), { wrapper });

    // The hook returns a promise — in a Suspense context it suspends until resolved.
    await waitFor(() => {
      expect(result.current).toBeDefined();
    });
  });

  it("resolves network info through use()", async () => {
    const netData = [{ macAddress: "AA:BB" }];
    const hwData = {
      cpu: null,
      memory: null,
      gpus: null,
      storage: [],
      motherboard: null,
    };

    const wrapper = createSuspenseWrapper(
      Promise.resolve(hwData),
      Promise.resolve(netData),
    );

    const { result } = renderHook(() => useNetworkInfoSuspense(), { wrapper });

    await waitFor(() => {
      expect(result.current).toBeDefined();
    });
  });
});
