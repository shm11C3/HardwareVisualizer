import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const hoisted = vi.hoisted(() => ({
  loadMock: vi.fn(),
}));

vi.mock("@/lib/sqlite", () => ({
  sqlitePromise: Promise.resolve({
    load: hoisted.loadMock,
    save: vi.fn(),
  }),
}));

import { useGpuNames } from "@/features/hardware/hooks/useGpuNames";

describe("useGpuNames", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns GPU names fetched from the database", async () => {
    hoisted.loadMock.mockResolvedValue([
      { gpu_name: "NVIDIA GeForce RTX 4090" },
      { gpu_name: "AMD Radeon RX 7900 XTX" },
    ]);

    const { result } = renderHook(() => useGpuNames());

    await waitFor(() => {
      expect(result.current).toEqual([
        "NVIDIA GeForce RTX 4090",
        "AMD Radeon RX 7900 XTX",
      ]);
    });

    expect(hoisted.loadMock).toHaveBeenCalledWith(
      expect.stringContaining("SELECT DISTINCT gpu_name FROM GPU_DATA_ARCHIVE"),
    );
  });

  it("returns empty array when no GPU names are in the database", async () => {
    hoisted.loadMock.mockResolvedValue([]);

    const { result } = renderHook(() => useGpuNames());

    await waitFor(() => {
      expect(result.current).toEqual([]);
    });
  });

  it("starts with an empty array before the query resolves", () => {
    hoisted.loadMock.mockReturnValue(new Promise(() => {})); // never resolves

    const { result } = renderHook(() => useGpuNames());

    expect(result.current).toEqual([]);
  });
});
