import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useFullScreenMode } from "@/hooks/useFullScreenMode";

const hoisted = vi.hoisted(() => ({
  setDecoration: vi.fn().mockResolvedValue(undefined),
  setIsFullScreen: vi.fn(),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    setDecoration: hoisted.setDecoration,
  },
}));

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: vi.fn().mockReturnValue([false, hoisted.setIsFullScreen]),
}));

describe("useFullScreenMode", () => {
  it("should return fullscreen state and toggle function", () => {
    const { result } = renderHook(() => useFullScreenMode());

    expect(result.current.isFullScreen).toBe(false);
    expect(typeof result.current.toggleFullScreen).toBe("function");
  });

  it("toggleFullScreen: calls setDecoration and toggles state", async () => {
    const { result } = renderHook(() => useFullScreenMode());

    await act(async () => {
      await result.current.toggleFullScreen();
    });

    expect(hoisted.setDecoration).toHaveBeenCalledWith(false);
    expect(hoisted.setIsFullScreen).toHaveBeenCalledWith(true);
  });
});
