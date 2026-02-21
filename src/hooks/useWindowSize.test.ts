import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useWindowSize } from "@/hooks/useWindowSize";

describe("useWindowSize", () => {
  const originalInnerWidth = globalThis.innerWidth;

  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    Object.defineProperty(globalThis, "innerWidth", {
      writable: true,
      configurable: true,
      value: originalInnerWidth,
    });
  });

  const setWidth = (width: number) => {
    Object.defineProperty(globalThis, "innerWidth", {
      writable: true,
      configurable: true,
      value: width,
    });
  };

  it("returns xs breakpoint for width < 640", () => {
    setWidth(400);
    const { result } = renderHook(() => useWindowSize());
    expect(result.current.breakpoint).toBe("xs");
  });

  it("returns sm breakpoint for width 640–767", () => {
    setWidth(640);
    const { result } = renderHook(() => useWindowSize());
    expect(result.current.breakpoint).toBe("sm");
  });

  it("returns md breakpoint for width 768–1023", () => {
    setWidth(768);
    const { result } = renderHook(() => useWindowSize());
    expect(result.current.breakpoint).toBe("md");
  });

  it("returns lg breakpoint for width 1024–1279", () => {
    setWidth(1024);
    const { result } = renderHook(() => useWindowSize());
    expect(result.current.breakpoint).toBe("lg");
  });

  it("returns xl breakpoint for width 1280–1535", () => {
    setWidth(1280);
    const { result } = renderHook(() => useWindowSize());
    expect(result.current.breakpoint).toBe("xl");
  });

  it("returns 2xl breakpoint for width >= 1536", () => {
    setWidth(1536);
    const { result } = renderHook(() => useWindowSize());
    expect(result.current.breakpoint).toBe("2xl");
  });

  it("updates breakpoint on window resize after debounce", async () => {
    setWidth(400);
    const { result } = renderHook(() => useWindowSize());
    expect(result.current.breakpoint).toBe("xs");

    setWidth(1024);
    act(() => {
      globalThis.dispatchEvent(new Event("resize"));
    });

    // Before debounce timeout — still old value
    expect(result.current.breakpoint).toBe("xs");

    await act(async () => {
      vi.advanceTimersByTime(100);
    });

    expect(result.current.breakpoint).toBe("lg");
  });

  it("debounces multiple resize events", async () => {
    setWidth(640);
    const { result } = renderHook(() => useWindowSize());

    // Fire several events in quick succession
    for (const width of [768, 1024, 1280]) {
      setWidth(width);
      act(() => {
        globalThis.dispatchEvent(new Event("resize"));
      });
      vi.advanceTimersByTime(50);
    }

    await act(async () => {
      vi.advanceTimersByTime(100);
    });

    // Should settle on the last width (1280 → xl)
    expect(result.current.breakpoint).toBe("xl");
  });

  it("does not update state when breakpoint has not changed", async () => {
    setWidth(1024);
    const { result } = renderHook(() => useWindowSize());
    const initial = result.current.breakpoint;

    // Resize but stay in lg range
    setWidth(1100);
    act(() => {
      globalThis.dispatchEvent(new Event("resize"));
    });
    await act(async () => {
      vi.advanceTimersByTime(100);
    });

    expect(result.current.breakpoint).toBe(initial);
  });

  it("removes resize listener on unmount", () => {
    setWidth(400);
    const removeSpy = vi.spyOn(globalThis, "removeEventListener");
    const { unmount } = renderHook(() => useWindowSize());

    unmount();

    expect(removeSpy).toHaveBeenCalledWith("resize", expect.any(Function));
    removeSpy.mockRestore();
  });

  describe("isBreak", () => {
    it("returns true when current breakpoint >= target", () => {
      setWidth(1024); // lg
      const { result } = renderHook(() => useWindowSize());

      expect(result.current.isBreak("xs")).toBe(true);
      expect(result.current.isBreak("sm")).toBe(true);
      expect(result.current.isBreak("md")).toBe(true);
      expect(result.current.isBreak("lg")).toBe(true);
    });

    it("returns false when current breakpoint < target", () => {
      setWidth(1024); // lg
      const { result } = renderHook(() => useWindowSize());

      expect(result.current.isBreak("xl")).toBe(false);
      expect(result.current.isBreak("2xl")).toBe(false);
    });
  });
});
