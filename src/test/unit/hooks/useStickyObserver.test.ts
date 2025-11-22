import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useStickyObserver } from "@/hooks/useStickyObserver";

// Simple mock for IntersectionObserver
const mockObserve = vi.fn();
const mockUnobserve = vi.fn();
const mockDisconnect = vi.fn();

global.IntersectionObserver = class IntersectionObserver {
  observe = mockObserve;
  unobserve = mockUnobserve;
  disconnect = mockDisconnect;
} as unknown as typeof IntersectionObserver;

describe("useStickyObserver", () => {
  beforeEach(() => {
    mockObserve.mockClear();
    mockUnobserve.mockClear();
    mockDisconnect.mockClear();
  });

  it("should return sentinelRef and isStuck initially false", () => {
    const { result } = renderHook(() => useStickyObserver());

    expect(result.current.isStuck).toBe(false);
    expect(result.current.sentinelRef).toBeDefined();
    expect(result.current.sentinelRef.current).toBeNull();
  });

  it("should create IntersectionObserver and not call observe when ref is null", () => {
    renderHook(() => useStickyObserver());

    // observe should not be called when sentinelRef.current is null
    expect(mockObserve).not.toHaveBeenCalled();
  });

  it("should not throw on unmount", () => {
    const { unmount } = renderHook(() => useStickyObserver());

    expect(() => unmount()).not.toThrow();
  });
});
