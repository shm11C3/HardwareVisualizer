import { act, renderHook } from "@testing-library/react";
import { Provider } from "jotai";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTauriStore } from "@/hooks/useTauriStore";

const mockSetMenuOpen = vi.fn();
const mockSetDisplayTarget = vi.fn();

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: vi
    .fn()
    .mockImplementation((key: string, defaultValue: unknown) => {
      if (key === "sideMenuOpen") return [defaultValue, mockSetMenuOpen, false];
      if (key === "display") return [defaultValue, mockSetDisplayTarget, false];
      return [defaultValue, vi.fn(), false];
    }),
}));

import { normalizeDisplayTarget, useMenu } from "@/features/menu/hooks/useMenu";

describe("useMenu", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi
      .mocked(useTauriStore)
      .mockImplementation((key: string, defaultValue: unknown) => {
        if (key === "sideMenuOpen")
          return [defaultValue, mockSetMenuOpen, false];
        if (key === "display")
          return [defaultValue, mockSetDisplayTarget, false];
        return [defaultValue, vi.fn(), false];
      }) as unknown as typeof useTauriStore;
  });

  it("returns initial state with isOpen=false and displayTarget=dashboard", () => {
    const { result } = renderHook(() => useMenu("grouped"), {
      wrapper: Provider,
    });

    expect(result.current.isOpen).toBe(false);
    expect(result.current.displayTarget).toBe("dashboard");
    expect(typeof result.current.toggleMenu).toBe("function");
    expect(typeof result.current.handleMenuClick).toBe("function");
  });

  it("toggleMenu: calls setMenuOpen with toggled value", () => {
    const { result } = renderHook(() => useMenu("grouped"), {
      wrapper: Provider,
    });

    act(() => {
      result.current.toggleMenu();
    });

    expect(mockSetMenuOpen).toHaveBeenCalledWith(true);
  });

  it("handleMenuClick: updates display target store and atom", () => {
    const { result } = renderHook(() => useMenu("grouped"), {
      wrapper: Provider,
    });

    act(() => {
      result.current.handleMenuClick("settings");
    });

    expect(mockSetDisplayTarget).toHaveBeenCalledWith("settings");
  });

  it("useEffect: does not call setDisplayTargetAtom when displayTarget is null", () => {
    vi.mocked(useTauriStore).mockImplementation((key: string) => {
      if (key === "sideMenuOpen") return [false, mockSetMenuOpen, false];
      if (key === "display") return [null, mockSetDisplayTarget, true];
      return [null, vi.fn(), true];
    }) as unknown as typeof useTauriStore;
    const { result } = renderHook(() => useMenu("grouped"), {
      wrapper: Provider,
    });

    expect(result.current.displayTarget).toBeNull();
  });

  it("handleMenuClick: can switch between different display targets", () => {
    const { result } = renderHook(() => useMenu("grouped"), {
      wrapper: Provider,
    });

    act(() => {
      result.current.handleMenuClick("usage");
    });
    expect(mockSetDisplayTarget).toHaveBeenCalledWith("usage");

    act(() => {
      result.current.handleMenuClick("insights");
    });
    expect(mockSetDisplayTarget).toHaveBeenCalledWith("insights");
  });

  it("normalizes classic screens to Performance in grouped navigation", () => {
    expect(normalizeDisplayTarget("usage", "grouped")).toBe("performance");
    expect(normalizeDisplayTarget("dashboard", "grouped")).toBe("performance");
    expect(normalizeDisplayTarget("cpuDetail", "grouped")).toBe("performance");
  });

  it("normalizes Performance to the Hardware Dashboard in classic navigation", () => {
    expect(normalizeDisplayTarget("performance", "classic")).toBe("dashboard");
  });

  it("preserves shared screens in both layouts", () => {
    for (const target of ["insights", "settings"] as const) {
      expect(normalizeDisplayTarget(target, "grouped")).toBe(target);
      expect(normalizeDisplayTarget(target, "classic")).toBe(target);
    }
  });
});
