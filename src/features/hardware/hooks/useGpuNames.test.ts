import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const hoisted = vi.hoisted(() => ({
  dialogMessageMock: vi.fn(),
  getGpuArchiveNamesMock: vi.fn(),
}));

// A failed name read is a state the Insights tab list renders, never a dialog.
vi.mock("@tauri-apps/plugin-dialog", () => ({
  message: hoisted.dialogMessageMock,
  ask: hoisted.dialogMessageMock,
  confirm: hoisted.dialogMessageMock,
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getGpuArchiveNames: hoisted.getGpuArchiveNamesMock,
  },
}));

import { useGpuNames } from "@/features/hardware/hooks/useGpuNames";

describe("useGpuNames", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns GPU names fetched from the database", async () => {
    hoisted.getGpuArchiveNamesMock.mockResolvedValue({
      status: "ok",
      data: ["NVIDIA GeForce RTX 4090", "AMD Radeon RX 7900 XTX"],
    });

    const { result } = renderHook(() => useGpuNames());

    await waitFor(() => {
      expect(result.current.gpuNames).toEqual([
        "NVIDIA GeForce RTX 4090",
        "AMD Radeon RX 7900 XTX",
      ]);
    });

    expect(hoisted.getGpuArchiveNamesMock).toHaveBeenCalledOnce();
  });

  it("returns empty array when no GPU names are in the database", async () => {
    hoisted.getGpuArchiveNamesMock.mockResolvedValue({
      status: "ok",
      data: [],
    });

    const { result } = renderHook(() => useGpuNames());

    await waitFor(() => {
      expect(result.current.gpuNames).toEqual([]);
    });
  });

  it("starts with an empty array before the query resolves", () => {
    hoisted.getGpuArchiveNamesMock.mockReturnValue(new Promise(() => {})); // never resolves

    const { result } = renderHook(() => useGpuNames());

    expect(result.current.gpuNames).toEqual([]);
  });

  it("sets hasError, keeps gpuNames empty and opens no dialog when the command returns an error result", async () => {
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    hoisted.getGpuArchiveNamesMock.mockResolvedValue({
      status: "error",
      error: "database unavailable",
    });

    const { result } = renderHook(() => useGpuNames());

    await waitFor(() => expect(result.current.hasError).toBe(true));
    expect(result.current.gpuNames).toEqual([]);
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        message: "Failed to fetch archived GPU names: database unavailable",
      }),
    );
    expect(hoisted.dialogMessageMock).not.toHaveBeenCalled();
    consoleErrorSpy.mockRestore();
  });

  it("sets hasError when the command rejects", async () => {
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    hoisted.getGpuArchiveNamesMock.mockRejectedValue(new Error("transport"));

    const { result } = renderHook(() => useGpuNames());

    await waitFor(() => expect(result.current.hasError).toBe(true));
    expect(result.current.gpuNames).toEqual([]);
    expect(hoisted.dialogMessageMock).not.toHaveBeenCalled();
    consoleErrorSpy.mockRestore();
  });

  it("refetches on retry and clears hasError once the names load", async () => {
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    hoisted.getGpuArchiveNamesMock
      .mockResolvedValueOnce({ status: "error", error: "locked" })
      .mockResolvedValue({ status: "ok", data: ["NVIDIA GeForce RTX 4090"] });

    const { result } = renderHook(() => useGpuNames());
    await waitFor(() => expect(result.current.hasError).toBe(true));

    act(() => {
      result.current.retry();
    });

    await waitFor(() =>
      expect(result.current.gpuNames).toEqual(["NVIDIA GeForce RTX 4090"]),
    );
    expect(result.current.hasError).toBe(false);
    expect(hoisted.getGpuArchiveNamesMock).toHaveBeenCalledTimes(2);
    consoleErrorSpy.mockRestore();
  });
});
