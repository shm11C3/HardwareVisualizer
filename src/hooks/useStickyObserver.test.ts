import { act, renderHook } from "@testing-library/react";
import type React from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useStickyObserver } from "@/hooks/useStickyObserver";

// Mock for IntersectionObserver that captures the callback so tests can invoke it
const mockObserve = vi.fn();
const mockUnobserve = vi.fn();
const mockDisconnect = vi.fn();
let capturedCallback: IntersectionObserverCallback | null = null;

globalThis.IntersectionObserver = class MockIntersectionObserver {
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

  it("calls observe when sentinelRef is attached to a DOM element", () => {
    // Render a wrapper component that assigns sentinelRef to a real element,
    // triggering the `if (el) observer.observe(el)` branch (lines 19-20).
    const el = document.createElement("div");

    // Override the constructor so sentinelRef.current is set before useEffect fires
    globalThis.IntersectionObserver = class MockIntersectionObserver {
      constructor(callback: IntersectionObserverCallback) {
        capturedCallback = callback;
      }
      observe = mockObserve;
      unobserve = mockUnobserve;
      disconnect = mockDisconnect;
    } as unknown as typeof IntersectionObserver;

    const { unmount } = renderHook(() => {
      const hook = useStickyObserver();
      // Manually point the ref at a real element before the effect runs
      // (useRef object is mutable so we can set .current directly)
      (hook.sentinelRef as React.MutableRefObject<HTMLDivElement>).current = el;
      return hook;
    });

    // Force the effect to re-run by unmounting and remounting isn't needed –
    // the initial render's useEffect runs after the ref is set.
    // We verify observe was called with the element.
    expect(mockObserve).toHaveBeenCalledWith(el);

    unmount();
    // unobserve should be called during cleanup
    expect(mockUnobserve).toHaveBeenCalledWith(el);
  });
});
