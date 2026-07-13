// src/features/hardware/hooks/useProcessInfo.test.ts
import { renderHook, waitFor } from "@testing-library/react";
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

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getProcessList: vi.fn(),
  },
}));

/**
 * Import hook to test
 */
import { useProcessInfo } from "@/features/hardware/hooks/useProcessInfo";
import { commands } from "@/rspc/bindings";

/**
 * Test execution
 */
describe("useProcessInfo", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns process list on successful fetch", async () => {
    const processData = [
      { pid: 1, name: "process1", cpuUsage: 10, memoryUsage: 200 },
      { pid: 2, name: "process2", cpuUsage: 5, memoryUsage: 100 },
    ];
    (commands.getProcessList as Mock).mockResolvedValue(processData);

    const { result } = renderHook(() => useProcessInfo(), {
      wrapper: Provider,
    });

    await waitFor(() => {
      expect(result.current).toEqual(processData);
    });
  });

  it("starts with empty array before fetch resolves", () => {
    (commands.getProcessList as Mock).mockReturnValue(new Promise(() => {}));

    const { result } = renderHook(() => useProcessInfo(), {
      wrapper: Provider,
    });

    expect(result.current).toEqual([]);
  });

  it("does not fetch or subscribe to shared process state when disabled", () => {
    const { result } = renderHook(() => useProcessInfo({ enabled: false }), {
      wrapper: Provider,
    });

    expect(result.current).toEqual([]);
    expect(commands.getProcessList).not.toHaveBeenCalled();
  });

  it("calls error() and console.error on fetch failure", async () => {
    const errorMsg = "Failed to fetch processes";
    (commands.getProcessList as Mock).mockRejectedValue(errorMsg);

    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    const { result } = renderHook(() => useProcessInfo(), {
      wrapper: Provider,
    });

    await waitFor(() => {
      expect(errorMock).toHaveBeenCalledWith(errorMsg);
    });

    expect(result.current).toEqual([]);
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      "Failed to fetch processes:",
      errorMsg,
    );
    consoleErrorSpy.mockRestore();
  });

  it("refetches processes via setInterval every 3000ms", async () => {
    const firstBatch = [
      { pid: 1, name: "proc1", cpuUsage: 10, memoryUsage: 100 },
    ];
    const secondBatch = [
      { pid: 2, name: "proc2", cpuUsage: 20, memoryUsage: 200 },
    ];

    (commands.getProcessList as Mock)
      .mockResolvedValueOnce(firstBatch)
      .mockResolvedValueOnce(secondBatch);

    vi.useFakeTimers({ shouldAdvanceTime: true });

    try {
      const { result } = renderHook(() => useProcessInfo(), {
        wrapper: Provider,
      });

      // Wait for initial fetch
      await waitFor(() => {
        expect(result.current).toEqual(firstBatch);
      });

      // Advance timer by 3000ms to trigger the interval callback
      vi.advanceTimersByTime(3000);

      await waitFor(() => {
        expect(result.current).toEqual(secondBatch);
      });

      expect(commands.getProcessList).toHaveBeenCalledTimes(2);
    } finally {
      vi.useRealTimers();
    }
  });

  it("clears interval on unmount (no further fetches)", async () => {
    (commands.getProcessList as Mock).mockResolvedValue([]);

    vi.useFakeTimers({ shouldAdvanceTime: true });

    try {
      const { unmount } = renderHook(() => useProcessInfo(), {
        wrapper: Provider,
      });

      await waitFor(() => {
        expect(commands.getProcessList).toHaveBeenCalledTimes(1);
      });

      unmount();

      // Advance 9 seconds - no further calls expected after unmount
      vi.advanceTimersByTime(9000);

      expect(commands.getProcessList).toHaveBeenCalledTimes(1);
    } finally {
      vi.useRealTimers();
    }
  });
});
