import { act, renderHook } from "@testing-library/react";
import { Provider } from "jotai";
import React from "react";
import { describe, expect, it } from "vitest";
import { useTitleIconVisualSelector } from "@/hooks/useTitleIconVisualSelector";

// Wrap each renderHook with a fresh Jotai Provider to isolate atom state
const wrapper = ({ children }: { children: React.ReactNode }) =>
  React.createElement(Provider, null, children);

describe("useTitleIconVisualSelector", () => {
  it("returns default visible types including all initial display types", () => {
    const { result } = renderHook(() => useTitleIconVisualSelector(), {
      wrapper,
    });

    expect(result.current.visibleTypes).toContain("dashboard");
    expect(result.current.visibleTypes).toContain("cpuDetail");
    expect(result.current.visibleTypes).toContain("insights");
    expect(result.current.visibleTypes).toContain("settings");
  });

  it("isTitleIconVisible returns true for visible types", () => {
    const { result } = renderHook(() => useTitleIconVisualSelector(), {
      wrapper,
    });

    expect(result.current.isTitleIconVisible("dashboard")).toBe(true);
    expect(result.current.isTitleIconVisible("cpuDetail")).toBe(true);
  });

  it("isTitleIconVisible returns false for hidden types", () => {
    const { result } = renderHook(() => useTitleIconVisualSelector(), {
      wrapper,
    });

    // Hide 'dashboard'
    act(() => {
      result.current.toggleTitleIconVisibility("dashboard", false);
    });

    expect(result.current.isTitleIconVisible("dashboard")).toBe(false);
  });

  it("toggleTitleIconVisibility(type, false) removes the type", () => {
    const { result } = renderHook(() => useTitleIconVisualSelector(), {
      wrapper,
    });

    act(() => {
      result.current.toggleTitleIconVisibility("insights", false);
    });

    expect(result.current.visibleTypes).not.toContain("insights");
  });

  it("toggleTitleIconVisibility(type, true) adds the type back", () => {
    const { result } = renderHook(() => useTitleIconVisualSelector(), {
      wrapper,
    });

    act(() => {
      result.current.toggleTitleIconVisibility("settings", false);
    });
    expect(result.current.visibleTypes).not.toContain("settings");

    act(() => {
      result.current.toggleTitleIconVisibility("settings", true);
    });
    expect(result.current.visibleTypes).toContain("settings");
  });

  it("toggleTitleIconVisibility(type, true) does not duplicate an already-visible type", () => {
    const { result } = renderHook(() => useTitleIconVisualSelector(), {
      wrapper,
    });
    const beforeCount = result.current.visibleTypes.filter(
      (t) => t === "dashboard",
    ).length;

    act(() => {
      result.current.toggleTitleIconVisibility("dashboard", true);
    });

    const afterCount = result.current.visibleTypes.filter(
      (t) => t === "dashboard",
    ).length;
    expect(afterCount).toBe(beforeCount);
  });

  it("toggleTitleIconVisibility(type, false) keeps state identity when the type is already hidden", () => {
    const { result } = renderHook(() => useTitleIconVisualSelector(), {
      wrapper,
    });

    act(() => {
      result.current.toggleTitleIconVisibility("usage", false);
    });

    const hiddenVisibleTypes = result.current.visibleTypes;

    act(() => {
      result.current.toggleTitleIconVisibility("usage", false);
    });

    expect(result.current.visibleTypes).toBe(hiddenVisibleTypes);
  });
});
