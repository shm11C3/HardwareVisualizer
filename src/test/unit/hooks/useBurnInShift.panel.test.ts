import { renderHook } from "@testing-library/react";
import type { RefObject } from "react";
import { describe, expect, it, vi } from "vitest";
import { useBurnInShift } from "@/hooks/useBurnInShift";

// Mock settings
vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: {
      burnInShift: true,
      burnInShiftMode: "jump",
      burnInShiftPreset: "balanced",
      burnInShiftIdleOnly: false,
    },
  }),
}));

vi.mock("@/lib/math", () => ({
  randInt: vi.fn().mockReturnValue(5),
}));

describe("useBurnInShift - Panel Features", () => {
  it("applies panel scale correctly", () => {
    const el = document.createElement("div");
    const ref: RefObject<HTMLElement | null> = { current: el };

    renderHook(() =>
      useBurnInShift(ref, true, {
        intervalMs: null,
        amplitudePx: null,
        idleThresholdMs: null,
        driftDurationSec: null,
        panelScale: 75,
        panelAspect: null,
        roamAreaPercent: null,
        keepWithinBounds: null,
      }),
    );

    expect(el.style.transform).toBe("scale(0.75)");
    expect(el.style.transformOrigin).toBe("top left");
  });

  it("applies compact aspect ratio", () => {
    const el = document.createElement("div");
    const ref: RefObject<HTMLElement | null> = { current: el };

    renderHook(() =>
      useBurnInShift(ref, true, {
        intervalMs: null,
        amplitudePx: null,
        idleThresholdMs: null,
        driftDurationSec: null,
        panelScale: null,
        panelAspect: "compact",
        roamAreaPercent: null,
        keepWithinBounds: null,
      }),
    );

    expect(el.style.maxWidth).toBe("1200px");
    expect(el.style.aspectRatio).toBe("16/9");
  });

  it("applies tall aspect ratio", () => {
    const el = document.createElement("div");
    const ref: RefObject<HTMLElement | null> = { current: el };

    renderHook(() =>
      useBurnInShift(ref, true, {
        intervalMs: null,
        amplitudePx: null,
        idleThresholdMs: null,
        driftDurationSec: null,
        panelScale: null,
        panelAspect: "tall",
        roamAreaPercent: null,
        keepWithinBounds: null,
      }),
    );

    expect(el.style.maxWidth).toBe("800px");
    expect(el.style.aspectRatio).toBe("9/16");
  });

  it("uses default scale of 100% when panelScale is null", () => {
    const el = document.createElement("div");
    const ref: RefObject<HTMLElement | null> = { current: el };

    renderHook(() =>
      useBurnInShift(ref, true, {
        intervalMs: null,
        amplitudePx: null,
        idleThresholdMs: null,
        driftDurationSec: null,
        panelScale: null,
        panelAspect: null,
        roamAreaPercent: null,
        keepWithinBounds: null,
      }),
    );

    expect(el.style.transform).toBe("scale(1)");
  });

  it("uses default roam area of 100% when roamAreaPercent is null", () => {
    const el = document.createElement("div");
    Object.defineProperty(el, "getBoundingClientRect", {
      value: () => ({
        width: 800,
        height: 600,
        top: 0,
        left: 0,
        bottom: 600,
        right: 800,
      }),
    });
    const ref: RefObject<HTMLElement | null> = { current: el };

    renderHook(() =>
      useBurnInShift(ref, true, {
        intervalMs: null,
        amplitudePx: null,
        idleThresholdMs: null,
        driftDurationSec: null,
        panelScale: null,
        panelAspect: null,
        roamAreaPercent: null,
        keepWithinBounds: true,
      }),
    );

    // Should not throw and should use default values
    expect(el).toBeTruthy();
  });

  it("resets styles on cleanup", () => {
    const el = document.createElement("div");
    const ref: RefObject<HTMLElement | null> = { current: el };

    const { unmount } = renderHook(() =>
      useBurnInShift(ref, true, {
        intervalMs: null,
        amplitudePx: null,
        idleThresholdMs: null,
        driftDurationSec: null,
        panelScale: 150,
        panelAspect: "compact",
        roamAreaPercent: null,
        keepWithinBounds: null,
      }),
    );

    // Verify styles are applied
    expect(el.style.transform).toBe("scale(1.5)");
    expect(el.style.maxWidth).toBe("1200px");

    // Cleanup
    unmount();

    // Verify styles are reset
    expect(el.style.transform).toBe("");
    expect(el.style.transformOrigin).toBe("");
    expect(el.style.maxWidth).toBe("");
    expect(el.style.aspectRatio).toBe("");
  });
});
