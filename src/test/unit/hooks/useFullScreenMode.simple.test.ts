import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useFullScreenMode } from "@/hooks/useFullScreenMode";

// Mock dependencies
vi.mock("@/rspc/bindings", () => ({
  commands: {
    setDecoration: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: vi.fn().mockReturnValue([false, vi.fn()]),
}));

describe("useFullScreenMode (Simple)", () => {
  it("should return fullscreen state and toggle function", () => {
    const { result } = renderHook(() => useFullScreenMode());
    
    expect(result.current.isFullScreen).toBe(false);
    expect(typeof result.current.toggleFullScreen).toBe("function");
  });
});