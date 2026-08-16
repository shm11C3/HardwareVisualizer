import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { Provider } from "jotai";
import { createElement, type PropsWithChildren, StrictMode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

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

import { useProcessInfo } from "@/features/hardware/hooks/useProcessInfo";
import { commands, type ProcessInfo } from "@/rspc/bindings";

const getProcessListMock = vi.mocked(commands.getProcessList);

const setDocumentHidden = (hidden: boolean) => {
  Object.defineProperty(document, "hidden", {
    configurable: true,
    value: hidden,
  });
};

const setDocumentVisibility = (hidden: boolean) => {
  setDocumentHidden(hidden);
  document.dispatchEvent(new Event("visibilitychange"));
};

const flushMicrotasks = async () => {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
};

const deferred = <T>() => {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
};

const StrictProvider = ({ children }: PropsWithChildren) =>
  createElement(StrictMode, null, createElement(Provider, null, children));

describe("useProcessInfo", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    setDocumentHidden(false);
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.restoreAllMocks();
    setDocumentHidden(false);
  });

  it("returns process list on successful fetch", async () => {
    const processData = [
      { pid: 1, name: "process1", cpuUsage: "10", memoryUsage: "200" },
      { pid: 2, name: "process2", cpuUsage: "5", memoryUsage: "100" },
    ];
    getProcessListMock.mockResolvedValue(processData);

    const { result } = renderHook(() => useProcessInfo(), {
      wrapper: Provider,
    });

    await waitFor(() => {
      expect(result.current).toEqual(processData);
    });
  });

  it("starts with empty array before fetch resolves", () => {
    getProcessListMock.mockReturnValue(new Promise(() => {}));

    const { result } = renderHook(() => useProcessInfo(), {
      wrapper: Provider,
    });

    expect(result.current).toEqual([]);
  });

  it("does not fetch or subscribe to shared process state when disabled", async () => {
    const { result } = renderHook(() => useProcessInfo({ enabled: false }), {
      wrapper: Provider,
    });

    await flushMicrotasks();

    expect(result.current).toEqual([]);
    expect(getProcessListMock).not.toHaveBeenCalled();
  });

  it("shares one initial request and polling schedule across consumers", async () => {
    getProcessListMock.mockResolvedValue([]);
    vi.useFakeTimers();

    renderHook(
      () => {
        const first = useProcessInfo();
        const second = useProcessInfo();
        return [first, second];
      },
      { wrapper: StrictProvider },
    );

    await flushMicrotasks();
    expect(getProcessListMock).toHaveBeenCalledTimes(1);

    await act(async () => {
      vi.advanceTimersByTime(3000);
      await Promise.resolve();
    });

    expect(getProcessListMock).toHaveBeenCalledTimes(2);
  });

  it("registers and removes demand when enabled changes", async () => {
    getProcessListMock.mockResolvedValue([]);
    vi.useFakeTimers();

    const { rerender } = renderHook(
      ({ enabled }) => useProcessInfo({ enabled }),
      {
        initialProps: { enabled: true },
        wrapper: Provider,
      },
    );

    await flushMicrotasks();
    expect(getProcessListMock).toHaveBeenCalledTimes(1);

    rerender({ enabled: false });
    vi.advanceTimersByTime(9000);
    expect(getProcessListMock).toHaveBeenCalledTimes(1);

    rerender({ enabled: true });
    await flushMicrotasks();
    expect(getProcessListMock).toHaveBeenCalledTimes(2);
  });

  it("keeps polling while another consumer remains active", async () => {
    getProcessListMock.mockResolvedValue([]);
    vi.useFakeTimers();

    const { rerender } = renderHook(
      ({ secondEnabled }) => {
        useProcessInfo();
        useProcessInfo({ enabled: secondEnabled });
      },
      {
        initialProps: { secondEnabled: true },
        wrapper: Provider,
      },
    );

    await flushMicrotasks();
    expect(getProcessListMock).toHaveBeenCalledTimes(1);

    rerender({ secondEnabled: false });
    await act(async () => {
      vi.advanceTimersByTime(3000);
      await Promise.resolve();
    });

    expect(getProcessListMock).toHaveBeenCalledTimes(2);
  });

  it("pauses while hidden and refreshes immediately when visible", async () => {
    getProcessListMock.mockResolvedValue([]);
    vi.useFakeTimers();

    renderHook(() => useProcessInfo(), {
      wrapper: Provider,
    });

    await flushMicrotasks();
    expect(getProcessListMock).toHaveBeenCalledTimes(1);

    act(() => setDocumentVisibility(true));
    vi.advanceTimersByTime(9000);
    expect(getProcessListMock).toHaveBeenCalledTimes(1);

    act(() => setDocumentVisibility(false));
    await flushMicrotasks();
    expect(getProcessListMock).toHaveBeenCalledTimes(2);
  });

  it("skips periodic ticks while a request is in flight", async () => {
    const slowRequest = deferred<ProcessInfo[]>();
    getProcessListMock
      .mockReturnValueOnce(slowRequest.promise)
      .mockResolvedValue([]);
    vi.useFakeTimers();

    renderHook(() => useProcessInfo(), {
      wrapper: Provider,
    });

    await flushMicrotasks();
    expect(getProcessListMock).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(9000);
    expect(getProcessListMock).toHaveBeenCalledTimes(1);

    slowRequest.resolve([]);
    await flushMicrotasks();
    expect(getProcessListMock).toHaveBeenCalledTimes(1);

    await act(async () => {
      vi.advanceTimersByTime(3000);
      await Promise.resolve();
    });
    expect(getProcessListMock).toHaveBeenCalledTimes(2);
  });

  it("coalesces resumed demand and ignores a stale response", async () => {
    const staleRequest = deferred<ProcessInfo[]>();
    const resumedRequest = deferred<ProcessInfo[]>();
    const staleData = [
      { pid: 1, name: "stale", cpuUsage: "10", memoryUsage: "100" },
    ];
    const currentData = [
      { pid: 2, name: "current", cpuUsage: "20", memoryUsage: "200" },
    ];
    getProcessListMock
      .mockReturnValueOnce(staleRequest.promise)
      .mockReturnValueOnce(resumedRequest.promise);

    const { result } = renderHook(() => useProcessInfo(), {
      wrapper: Provider,
    });

    await flushMicrotasks();
    expect(getProcessListMock).toHaveBeenCalledTimes(1);

    act(() => {
      setDocumentVisibility(true);
      setDocumentVisibility(false);
      setDocumentVisibility(false);
    });
    await flushMicrotasks();
    expect(getProcessListMock).toHaveBeenCalledTimes(1);

    staleRequest.resolve(staleData);
    await flushMicrotasks();
    expect(getProcessListMock).toHaveBeenCalledTimes(2);
    expect(result.current).toEqual([]);

    resumedRequest.resolve(currentData);
    await waitFor(() => {
      expect(result.current).toEqual(currentData);
    });
    expect(getProcessListMock).toHaveBeenCalledTimes(2);
  });

  it("preserves the last successful result and reports one shared error", async () => {
    const processData = [
      { pid: 1, name: "process1", cpuUsage: "10", memoryUsage: "200" },
    ];
    const pollingError = "Failed to fetch processes";
    getProcessListMock
      .mockResolvedValueOnce(processData)
      .mockRejectedValueOnce(pollingError);
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    const { result } = renderHook(
      () => {
        useProcessInfo();
        return useProcessInfo();
      },
      { wrapper: Provider },
    );

    await waitFor(() => {
      expect(result.current).toEqual(processData);
    });

    act(() => setDocumentVisibility(true));
    act(() => setDocumentVisibility(false));

    await waitFor(() => {
      expect(errorMock).toHaveBeenCalledWith(pollingError);
    });

    expect(result.current).toEqual(processData);
    expect(errorMock).toHaveBeenCalledTimes(1);
    expect(consoleErrorSpy).toHaveBeenCalledTimes(1);
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      "Failed to fetch processes:",
      pollingError,
    );
  });

  it("stops polling after the final consumer unmounts", async () => {
    getProcessListMock.mockResolvedValue([]);
    vi.useFakeTimers();

    const { unmount } = renderHook(() => useProcessInfo(), {
      wrapper: Provider,
    });

    await flushMicrotasks();
    expect(getProcessListMock).toHaveBeenCalledTimes(1);

    unmount();
    vi.advanceTimersByTime(9000);

    expect(getProcessListMock).toHaveBeenCalledTimes(1);
  });
});
