import { act, renderHook, waitFor } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { createElement, type ReactNode } from "react";
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  type Mock,
  vi,
} from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  Channel: class Channel<T> {
    onmessage: ((event: T) => void) | null = null;
  },
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    fetchUpdate: vi.fn(),
    installUpdate: vi.fn(),
    restartApp: vi.fn(),
  },
}));

import {
  UPDATE_CHECK_INTERVAL_MS,
  useUpdater,
} from "@/features/updater/hooks/useAppUpdate";
import { commands } from "@/rspc/bindings";
import {
  updateDialogOpenAtom,
  updateIndicatorVersionAtom,
} from "@/store/appUpdate";

let store = createStore();

const renderUpdater = () =>
  renderHook(() => useUpdater(), {
    wrapper: ({ children }: { children: ReactNode }) =>
      createElement(Provider, { store }, children),
  });

type TestDownloadEvent =
  | { event: "started"; data: { contentLength: string } }
  | { event: "progress"; data: { chunkLength: string } }
  | { event: "finished"; data: Record<string, never> };

type TestChannel = {
  onmessage: ((event: TestDownloadEvent) => void) | null;
};

describe("useUpdater", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    store = createStore();
    (commands.fetchUpdate as Mock).mockResolvedValue({
      status: "ok",
      data: null,
    });
    (commands.restartApp as Mock).mockResolvedValue(undefined);
    (commands.installUpdate as Mock).mockResolvedValue({
      status: "ok",
      data: null,
    });
  });

  it("fetchUpdate: meta is set on success", async () => {
    const meta = {
      version: "2.0.0",
      currentVersion: "1.0.0",
      notes: "release notes",
      pubDate: "2025-12-31",
    };

    (commands.fetchUpdate as Mock).mockResolvedValue({
      status: "ok",
      data: meta,
    });

    const { result } = renderUpdater();

    await act(async () => {
      await waitFor(() => result.current.meta?.version === "2.0.0");
    });

    expect(commands.fetchUpdate).toHaveBeenCalled();
    expect(result.current.meta).toEqual(meta);
  });

  it("install: handles download events and restarts app", async () => {
    (commands.installUpdate as Mock).mockImplementation(async (ch: unknown) => {
      const channel = ch as TestChannel;

      channel.onmessage?.({
        event: "started",
        data: {
          contentLength: "100",
        },
      });

      channel.onmessage?.({
        event: "progress",
        data: {
          chunkLength: "40",
        },
      });

      channel.onmessage?.({
        event: "progress",
        data: {
          chunkLength: "60",
        },
      });

      channel.onmessage?.({
        event: "finished",
        data: {},
      });

      return { status: "ok", data: null };
    });

    const { result } = renderUpdater();

    await act(async () => {
      await result.current.install();
    });

    await waitFor(() => result.current.isFinished === true);

    expect(commands.installUpdate).toHaveBeenCalledTimes(1);
    expect(commands.restartApp).toHaveBeenCalledTimes(1);

    expect(result.current.installing).toBe(true);
    expect(result.current.total).toBe(100n);
    expect(result.current.downloaded).toBe(100n);
    expect(result.current.percent).toBe(100);
    expect(result.current.isFinished).toBe(true);
  });

  it("install: does not restart after a download error and refreshes the pending update", async () => {
    (commands.installUpdate as Mock).mockResolvedValue({
      status: "error",
      error: { Updater: "network failure" },
    });
    (commands.fetchUpdate as Mock).mockResolvedValue({
      status: "ok",
      data: {
        version: "2.0.0",
        currentVersion: "1.0.0",
        notes: null,
        pubDate: null,
      },
    });

    const { result } = renderUpdater();

    await act(async () => {
      await result.current.install();
    });

    expect(commands.restartApp).not.toHaveBeenCalled();
    expect(result.current.installing).toBe(false);
    expect(result.current.installError).toEqual({
      kind: "before-shutdown",
      message: "network failure",
    });
    expect(commands.fetchUpdate).toHaveBeenCalledTimes(2);
    expect(result.current.meta?.version).toBe("2.0.0");
  });

  it("install: requires an explicit restart after a post-shutdown failure", async () => {
    (commands.installUpdate as Mock).mockResolvedValue({
      status: "error",
      error: { RestartRequired: "installer handoff failed" },
    });

    const { result } = renderUpdater();

    await act(async () => {
      await result.current.install();
    });

    expect(commands.restartApp).not.toHaveBeenCalled();
    expect(result.current.installError).toEqual({
      kind: "restart-required",
      message: "installer handoff failed",
    });
  });

  it("install: does not mark a finished download complete when handoff fails", async () => {
    (commands.installUpdate as Mock).mockImplementation(async (ch: unknown) => {
      (ch as TestChannel).onmessage?.({
        event: "finished",
        data: {},
      });
      return {
        status: "error",
        error: { RestartRequired: "installer handoff failed" },
      };
    });

    const { result } = renderUpdater();

    await act(async () => {
      await result.current.install();
    });

    expect(result.current.isFinished).toBe(false);
    expect(result.current.installError).toEqual({
      kind: "restart-required",
      message: "installer handoff failed",
    });
    expect(commands.restartApp).not.toHaveBeenCalled();
  });

  it("percent: null when total is null", () => {
    const { result } = renderUpdater();
    expect(result.current.total).toBeNull();
    expect(result.current.percent).toBeNull();
  });

  it("percent: null when total is 0n", async () => {
    // Edge case: started event sends contentLength "0", total becomes 0n
    (commands.installUpdate as Mock).mockImplementation(async (ch: unknown) => {
      const channel = ch as TestChannel;
      channel.onmessage?.({ event: "started", data: { contentLength: "0" } });
      channel.onmessage?.({ event: "finished", data: {} });
      return { status: "ok", data: null };
    });

    const { result } = renderUpdater();

    await act(async () => {
      await result.current.install();
    });

    await waitFor(() => result.current.isFinished === true);
    // total is 0n, so percent must be null (division by zero guard)
    expect(result.current.percent).toBeNull();
  });

  it("install: total stays null when contentLength is absent", async () => {
    // started event without contentLength → setTotal(null)
    (commands.installUpdate as Mock).mockImplementation(async (ch: unknown) => {
      const channel = ch as TestChannel;
      channel.onmessage?.({
        event: "started",
        // contentLength omitted → falsy
        data: { contentLength: "" },
      });
      channel.onmessage?.({ event: "progress", data: { chunkLength: "50" } });
      channel.onmessage?.({ event: "finished", data: {} });
      return { status: "ok", data: null };
    });

    const { result } = renderUpdater();

    await act(async () => {
      await result.current.install();
    });

    await waitFor(() => result.current.isFinished === true);
    // total should remain null (unknown download size)
    expect(result.current.total).toBeNull();
    expect(result.current.percent).toBeNull();
  });

  it("fetchUpdate: meta stays null when result is not ok", async () => {
    (commands.fetchUpdate as Mock).mockResolvedValue({
      status: "error",
      error: "network failure",
    });

    const { result } = renderUpdater();

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    expect(result.current.meta).toBeNull();
  });

  describe("periodic check", () => {
    const update = (version: string) => ({
      status: "ok",
      data: { version, currentVersion: "1.0.0", notes: null, pubDate: null },
    });

    beforeEach(() => {
      vi.useFakeTimers();
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    const advanceOneInterval = async () => {
      await act(async () => {
        await vi.advanceTimersByTimeAsync(UPDATE_CHECK_INTERVAL_MS);
      });
    };

    it("finds a release published after startup", async () => {
      (commands.fetchUpdate as Mock)
        .mockResolvedValueOnce({ status: "ok", data: null })
        .mockResolvedValueOnce(update("2.0.0"));

      const { result } = renderUpdater();
      await act(async () => {});
      expect(result.current.meta).toBeNull();

      await advanceOneInterval();

      expect(commands.fetchUpdate).toHaveBeenCalledTimes(2);
      expect(result.current.meta?.version).toBe("2.0.0");
    });

    it("opens the update dialog for an update found at startup", async () => {
      (commands.fetchUpdate as Mock).mockResolvedValueOnce(update("2.0.0"));

      renderUpdater();
      await act(async () => {});

      expect(store.get(updateDialogOpenAtom)).toBe(true);
      expect(store.get(updateIndicatorVersionAtom)).toBe("2.0.0");
    });

    it("offers a later update in the side menu without opening the dialog", async () => {
      (commands.fetchUpdate as Mock)
        .mockResolvedValueOnce({ status: "ok", data: null })
        .mockResolvedValueOnce(update("2.0.0"));

      renderUpdater();
      await act(async () => {});
      await advanceOneInterval();

      expect(store.get(updateDialogOpenAtom)).toBe(false);
      expect(store.get(updateIndicatorVersionAtom)).toBe("2.0.0");
    });

    it("hides the side menu entry once an install starts", async () => {
      (commands.fetchUpdate as Mock).mockResolvedValue(update("2.0.0"));
      (commands.installUpdate as Mock).mockReturnValue(new Promise(() => {}));

      const { result } = renderUpdater();
      await act(async () => {});
      act(() => {
        void result.current.install();
      });

      expect(store.get(updateIndicatorVersionAtom)).toBeNull();
    });

    it("replaces the meta when a newer version is published", async () => {
      (commands.fetchUpdate as Mock)
        .mockResolvedValueOnce(update("2.0.0"))
        .mockResolvedValueOnce(update("2.1.0"));

      const { result } = renderUpdater();
      await act(async () => {});

      await advanceOneInterval();

      expect(result.current.meta?.version).toBe("2.1.0");
    });

    it("keeps the known update when a recheck fails", async () => {
      (commands.fetchUpdate as Mock)
        .mockResolvedValueOnce(update("2.0.0"))
        .mockResolvedValueOnce({
          status: "error",
          error: { Updater: "offline" },
        });

      const { result } = renderUpdater();
      await act(async () => {});

      await advanceOneInterval();

      expect(result.current.meta?.version).toBe("2.0.0");
    });

    it("does not recheck while an install is in progress", async () => {
      (commands.fetchUpdate as Mock).mockResolvedValue(update("2.0.0"));
      (commands.installUpdate as Mock).mockReturnValue(new Promise(() => {}));

      const { result } = renderUpdater();
      await act(async () => {});
      act(() => {
        void result.current.install();
      });

      await advanceOneInterval();

      expect(commands.fetchUpdate).toHaveBeenCalledTimes(1);
    });

    it("ignores a check that resolves after an install started", async () => {
      let resolveRecheck: (value: unknown) => void = () => {};
      (commands.fetchUpdate as Mock)
        .mockResolvedValueOnce(update("2.0.0"))
        .mockReturnValueOnce(
          new Promise((resolve) => {
            resolveRecheck = resolve;
          }),
        );
      (commands.installUpdate as Mock).mockReturnValue(new Promise(() => {}));

      const { result } = renderUpdater();
      await act(async () => {});
      await advanceOneInterval();
      act(() => {
        void result.current.install();
      });

      await act(async () => {
        resolveRecheck(update("2.1.0"));
      });

      expect(commands.fetchUpdate).toHaveBeenCalledTimes(2);
      expect(result.current.meta?.version).toBe("2.0.0");
    });

    it("ignores a check that started before a failed install", async () => {
      let resolveStaleCheck: (value: unknown) => void = () => {};
      (commands.fetchUpdate as Mock)
        .mockResolvedValueOnce(update("2.0.0"))
        .mockReturnValueOnce(
          new Promise((resolve) => {
            resolveStaleCheck = resolve;
          }),
        )
        // The recovery refresh after the download failure.
        .mockResolvedValueOnce(update("2.1.0"));
      (commands.installUpdate as Mock).mockResolvedValue({
        status: "error",
        error: { Updater: "network failure" },
      });

      const { result } = renderUpdater();
      await act(async () => {});
      await advanceOneInterval();
      await act(async () => {
        await result.current.install();
      });
      expect(result.current.meta?.version).toBe("2.1.0");

      await act(async () => {
        resolveStaleCheck(update("2.0.0"));
      });

      expect(result.current.meta?.version).toBe("2.1.0");
    });

    it("stops checking after unmount", async () => {
      const { unmount } = renderUpdater();
      await act(async () => {});
      unmount();

      await advanceOneInterval();

      expect(commands.fetchUpdate).toHaveBeenCalledTimes(1);
    });
  });
});
