import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const hoisted = vi.hoisted(() => ({
  errorMock: vi.fn(),
  getGpuArchiveNamesMock: vi.fn(),
}));

vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({ error: hoisted.errorMock }),
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
      expect(result.current).toEqual([
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
      expect(result.current).toEqual([]);
    });
  });

  it("starts with an empty array before the query resolves", () => {
    hoisted.getGpuArchiveNamesMock.mockReturnValue(new Promise(() => {})); // never resolves

    const { result } = renderHook(() => useGpuNames());

    expect(result.current).toEqual([]);
  });

  it("returns empty array and shows an error when the command returns an error result", async () => {
    hoisted.getGpuArchiveNamesMock.mockResolvedValue({
      status: "error",
      error: "database unavailable",
    });

    const { result } = renderHook(() => useGpuNames());

    await waitFor(() => {
      expect(hoisted.errorMock).toHaveBeenCalledWith(
        "Failed to fetch archived GPU names: database unavailable",
      );
    });
    expect(result.current).toEqual([]);
  });

  it("returns empty array and shows an error when the command rejects", async () => {
    hoisted.getGpuArchiveNamesMock.mockRejectedValue(new Error("transport"));

    const { result } = renderHook(() => useGpuNames());

    await waitFor(() => {
      expect(hoisted.errorMock).toHaveBeenCalledWith(
        "Failed to fetch archived GPU names: Error: transport",
      );
    });
    expect(result.current).toEqual([]);
  });
});
