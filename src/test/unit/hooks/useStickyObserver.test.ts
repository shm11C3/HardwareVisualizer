import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useStickyObserver } from "@/hooks/useStickyObserver";

// Simple mock for IntersectionObserver
global.IntersectionObserver = vi.fn().mockImplementation(() => ({
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn(),
}));

describe("useStickyObserver", () => {
  it("should return sentinelRef and isStuck initially false", () => {
    const { result } = renderHook(() => useStickyObserver());

    expect(result.current.isStuck).toBe(false);
    expect(result.current.sentinelRef).toBeDefined();
    expect(result.current.sentinelRef.current).toBeNull();
  });

  it("should create IntersectionObserver", () => {
    renderHook(() => useStickyObserver());

    expect(IntersectionObserver).toHaveBeenCalled();
  });

  it("should not throw on unmount", () => {
    const { unmount } = renderHook(() => useStickyObserver());

    expect(() => unmount()).not.toThrow();
  });
});
