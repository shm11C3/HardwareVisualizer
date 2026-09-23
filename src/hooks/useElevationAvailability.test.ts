import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useElevationAvailability } from "./useElevationAvailability";

const mocks = vi.hoisted(() => ({
  getElevationAvailability: vi.fn(),
  platform: vi.fn(() => "windows"),
}));

vi.mock("@tauri-apps/plugin-os", () => ({
  platform: mocks.platform,
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getElevationAvailability: mocks.getElevationAvailability,
  },
}));

describe("useElevationAvailability", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.platform.mockReturnValue("windows");
  });

  it("reports the backend answer on Windows", async () => {
    mocks.getElevationAvailability.mockResolvedValue("unprotectedLocation");
    const { result } = renderHook(() => useElevationAvailability());
    expect(result.current).toBeNull();
    await waitFor(() => expect(result.current).toBe("unprotectedLocation"));
  });

  it("is unsupported outside Windows without asking the backend", async () => {
    mocks.platform.mockReturnValue("linux");
    const { result } = renderHook(() => useElevationAvailability());
    await waitFor(() => expect(result.current).toBe("unsupported"));
    expect(mocks.getElevationAvailability).not.toHaveBeenCalled();
  });

  it("fails closed when the backend cannot answer", async () => {
    mocks.getElevationAvailability.mockRejectedValue(new Error("ipc"));
    vi.spyOn(console, "error").mockImplementation(() => {});
    const { result } = renderHook(() => useElevationAvailability());
    await waitFor(() => expect(result.current).toBe("unprotectedLocation"));
  });
});
