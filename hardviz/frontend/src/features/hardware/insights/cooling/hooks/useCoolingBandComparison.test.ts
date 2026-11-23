import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useCoolingBandComparison } from "@/features/hardware/insights/cooling/hooks/useCoolingBandComparison";

const hoisted = vi.hoisted(() => ({
  dialogMessage: vi.fn(),
  getCoolingBandComparison: vi.fn(),
}));

// A failed Cooling read is a panel state, never a native dialog.
vi.mock("@tauri-apps/plugin-dialog", () => ({
  message: hoisted.dialogMessage,
  ask: hoisted.dialogMessage,
  confirm: hoisted.dialogMessage,
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getCoolingBandComparison: hoisted.getCoolingBandComparison,
  },
}));

const established = {
  status: "establishing",
  qualifyingDays: 1,
  requiredDays: 3,
};

describe("useCoolingBandComparison", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  it("returns the comparison and no error on success", async () => {
    hoisted.getCoolingBandComparison.mockResolvedValue({
      status: "ok",
      data: established,
    });

    const { result } = renderHook(() => useCoolingBandComparison());

    await waitFor(() => expect(result.current.data).toEqual(established));
    expect(result.current.hasError).toBe(false);
  });

  it("reports a failed read as hasError with no data and no dialog", async () => {
    hoisted.getCoolingBandComparison.mockResolvedValue({
      status: "error",
      error: "db locked",
    });

    const { result } = renderHook(() => useCoolingBandComparison());

    await waitFor(() => expect(result.current.hasError).toBe(true));
    expect(result.current.data).toBeNull();
    expect(console.error).toHaveBeenCalledWith(
      expect.objectContaining({
        message: "Failed to fetch cooling band comparison: db locked",
      }),
    );
    expect(hoisted.dialogMessage).not.toHaveBeenCalled();
  });

  it("refetches on retry and clears hasError once the read succeeds", async () => {
    hoisted.getCoolingBandComparison
      .mockResolvedValueOnce({ status: "error", error: "db locked" })
      .mockResolvedValue({ status: "ok", data: established });

    const { result } = renderHook(() => useCoolingBandComparison());
    await waitFor(() => expect(result.current.hasError).toBe(true));

    act(() => {
      result.current.retry();
    });

    await waitFor(() => expect(result.current.data).toEqual(established));
    expect(result.current.hasError).toBe(false);
    expect(hoisted.getCoolingBandComparison).toHaveBeenCalledTimes(2);
    expect(hoisted.dialogMessage).not.toHaveBeenCalled();
  });

  it("ignores a late failure from a request that retry superseded", async () => {
    let rejectFirst: (reason: Error) => void = () => {};
    hoisted.getCoolingBandComparison
      .mockImplementationOnce(
        () =>
          new Promise((_, reject) => {
            rejectFirst = reject;
          }),
      )
      .mockResolvedValue({ status: "ok", data: established });

    const { result } = renderHook(() => useCoolingBandComparison());
    act(() => {
      result.current.retry();
    });
    await waitFor(() => expect(result.current.data).toEqual(established));

    await act(async () => {
      rejectFirst(new Error("late failure"));
      await Promise.resolve();
    });

    expect(result.current.hasError).toBe(false);
    expect(result.current.data).toEqual(established);
  });
});
