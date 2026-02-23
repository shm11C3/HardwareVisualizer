import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useStickyObserver } from "@/hooks/useStickyObserver";

// Mock for IntersectionObserver that captures the callback so tests can invoke it
const mockObserve = vi.fn();
const mockUnobserve = vi.fn();
const mockDisconnect = vi.fn();
let capturedCallback: IntersectionObserverCallback | null = null;

global.IntersectionObserver = class MockIntersectionObserver {
  constructor(callback: IntersectionObserverCallback) {
    capturedCallback = callback;
  }
  observe = mockObserve;
  unobserve = mockUnobserve;
  disconnect = mockDisconnect;
} as unknown as typeof IntersectionObserver;

describe("useStickyObserver", () => {
  beforeEach(() => {
    mockObserve.mockClear();
    mockUnobserve.mockClear();
    mockDisconnect.mockClear();
    capturedCallback = null;
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

  it("sets isStuck to true when entry is not intersecting", async () => {
    const { result } = renderHook(() => useStickyObserver());

    expect(capturedCallback).not.toBeNull();

    await act(async () => {
      capturedCallback?.(
        [{ isIntersecting: false } as IntersectionObserverEntry],
        {} as IntersectionObserver,
      );
    });

    expect(result.current.isStuck).toBe(true);
  });

  it("sets isStuck to false when entry is intersecting", async () => {
    const { result } = renderHook(() => useStickyObserver());

    // First make it stuck
    await act(async () => {
      capturedCallback?.(
        [{ isIntersecting: false } as IntersectionObserverEntry],
        {} as IntersectionObserver,
      );
    });
    expect(result.current.isStuck).toBe(true);

    // Then un-stick
    await act(async () => {
      capturedCallback?.(
        [{ isIntersecting: true } as IntersectionObserverEntry],
        {} as IntersectionObserver,
      );
    });
    expect(result.current.isStuck).toBe(false);
  });
});
