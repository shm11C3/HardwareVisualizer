import { act, renderHook } from "@testing-library/react";
import { Provider } from "jotai";
import { beforeEach, describe, expect, it, vi } from "vitest";

const mockSetMenuOpen = vi.fn();
const mockSetDisplayTarget = vi.fn();

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: vi
    .fn()
    .mockImplementation((key: string, defaultValue: unknown) => {
      if (key === "sideMenuOpen") return [defaultValue, mockSetMenuOpen];
      if (key === "display") return [defaultValue, mockSetDisplayTarget];
      return [defaultValue, vi.fn()];
    }),
}));

import { useMenu } from "@/features/menu/hooks/useMenu";

describe("useMenu", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns initial state with isOpen=false and displayTarget=dashboard", () => {
    const { result } = renderHook(() => useMenu(), { wrapper: Provider });

    expect(result.current.isOpen).toBe(false);
    expect(result.current.displayTarget).toBe("dashboard");
    expect(typeof result.current.toggleMenu).toBe("function");
    expect(typeof result.current.handleMenuClick).toBe("function");
  });

  it("toggleMenu: calls setMenuOpen with toggled value", () => {
    const { result } = renderHook(() => useMenu(), { wrapper: Provider });

    act(() => {
      result.current.toggleMenu();
    });

    expect(mockSetMenuOpen).toHaveBeenCalledWith(true);
  });

  it("handleMenuClick: updates display target store and atom", () => {
    const { result } = renderHook(() => useMenu(), { wrapper: Provider });

    act(() => {
      result.current.handleMenuClick("settings");
    });

    expect(mockSetDisplayTarget).toHaveBeenCalledWith("settings");
  });

  it("handleMenuClick: can switch between different display targets", () => {
    const { result } = renderHook(() => useMenu(), { wrapper: Provider });

    act(() => {
      result.current.handleMenuClick("usage");
    });
    expect(mockSetDisplayTarget).toHaveBeenCalledWith("usage");

    act(() => {
      result.current.handleMenuClick("insights");
    });
    expect(mockSetDisplayTarget).toHaveBeenCalledWith("insights");
  });
});
