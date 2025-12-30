import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";

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

import { useUpdater } from "@/features/updater/hooks/useAppUpdate";
import { commands } from "@/rspc/bindings";

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

    const { result } = renderHook(() => useUpdater());

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

    const { result } = renderHook(() => useUpdater());

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

  it("percent: null when total is null", () => {
    const { result } = renderHook(() => useUpdater());
    expect(result.current.total).toBeNull();
    expect(result.current.percent).toBeNull();
  });
});
