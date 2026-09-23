import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  elevationUnavailableReasonKey,
  useElevationAvailability,
  useProcessElevated,
} from "./useElevationAvailability";

const mocks = vi.hoisted(() => ({
  getElevationAvailability: vi.fn(),
  isProcessElevated: vi.fn(),
  platform: vi.fn(() => "windows"),
}));

vi.mock("@tauri-apps/plugin-os", () => ({
  platform: mocks.platform,
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getElevationAvailability: mocks.getElevationAvailability,
    isProcessElevated: mocks.isProcessElevated,
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

  it("reports unknown, not an install location, when the backend cannot answer", async () => {
    mocks.getElevationAvailability.mockRejectedValue(new Error("ipc"));
    vi.spyOn(console, "error").mockImplementation(() => {});
    const { result } = renderHook(() => useElevationAvailability());
    await waitFor(() => expect(result.current).toBe("unknown"));
  });

  it("reads whether the process is already elevated", async () => {
    mocks.isProcessElevated.mockResolvedValue({ status: "ok", data: true });
    const { result } = renderHook(() => useProcessElevated());
    await waitFor(() => expect(result.current).toBe(true));
  });

  it("maps availability to the explanation to show", () => {
    expect(elevationUnavailableReasonKey("unprotectedLocation")).toBe(
      "elevationUnavailable.reason",
    );
    expect(elevationUnavailableReasonKey("unknown")).toBe(
      "elevationUnavailable.unknown",
    );
    expect(elevationUnavailableReasonKey("available")).toBeNull();
    expect(elevationUnavailableReasonKey(null)).toBeNull();
  });
});
