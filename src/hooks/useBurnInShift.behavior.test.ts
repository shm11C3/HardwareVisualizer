import { renderHook } from "@testing-library/react";
import type { RefObject } from "react";
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  type Mock,
  vi,
} from "vitest";

// Mock settings atom with a configurable mock function
vi.mock("@/features/settings/hooks/useSettingsAtom", () => {
  return {
    useSettingsAtom: vi.fn(() => ({
      settings: {
        burnInShift: true,
        burnInShiftMode: "jump",
        burnInShiftPreset: "balanced",
        burnInShiftIdleOnly: false,
      },
    })),
  };
});

// Deterministic randInt: always return the min bound
vi.mock("@/lib/math", () => ({
  randInt: vi.fn((min: number) => min),
}));

import { useSettingsAtom as useSettingsAtomMocked } from "@/features/settings/hooks/useSettingsAtom";
// Import after mocks
import { useBurnInShift } from "@/hooks/useBurnInShift";

describe("useBurnInShift (Behavior)", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("applies preset-based CSS variables on mount", () => {
    const el = document.createElement("div");
    const ref: RefObject<HTMLElement | null> = { current: el };

    renderHook(() => useBurnInShift(ref, true));

    // Balanced preset: driftSec = 24, ampPx lower bound = 4 (due to mocked randInt)
    expect(el.style.getPropertyValue("--drift-duration")).toBe("24s");
    expect(el.style.getPropertyValue("--shift-x-start")).toBe("-4px");
    expect(el.style.getPropertyValue("--shift-x-mid")).toBe("4px");
    expect(el.style.getPropertyValue("--shift-x-end")).toBe("-4px");
    expect(el.style.getPropertyValue("--shift-y-start")).toBe("-4px");
    expect(el.style.getPropertyValue("--shift-y-mid")).toBe("4px");
    expect(el.style.getPropertyValue("--shift-y-end")).toBe("-4px");
  });

  it("updates shift variables on interval in jump mode", () => {
    const el = document.createElement("div");
    const ref: RefObject<HTMLElement | null> = { current: el };

    renderHook(() =>
      useBurnInShift(ref, true, {
        // Make timer short and amplitudes deterministic
        intervalMs: 10,
        amplitudePx: [3, 7],
        driftDurationSec: 12,
        idleThresholdMs: null,
      }),
    );

    // First tick (10ms): randInt returns min => -amp for both axes
    vi.advanceTimersByTime(10);
    expect(el.style.getPropertyValue("--shift-x")).toBe("-3px");
    expect(el.style.getPropertyValue("--shift-y")).toBe("-7px");
  });

  it("respects idleOnly: jumps only after idle threshold", () => {
    const el = document.createElement("div");
    const ref: RefObject<HTMLElement | null> = { current: el };

    // Configure settings to enable idle-only behavior
    (useSettingsAtomMocked as Mock).mockReturnValue({
      settings: {
        burnInShift: true,
        burnInShiftMode: "jump",
        burnInShiftPreset: "balanced",
        burnInShiftIdleOnly: true,
      },
    });

    renderHook(() =>
      useBurnInShift(ref, true, {
        intervalMs: 10,
        amplitudePx: [2, 2],
        idleThresholdMs: 100,
        driftDurationSec: null,
      }),
    );

    // Before idle threshold: no movement
    vi.advanceTimersByTime(10);
    expect(el.style.getPropertyValue("--shift-x")).toBe("");
    expect(el.style.getPropertyValue("--shift-y")).toBe("");

    // Cross idle threshold (100ms), then next tick should move
    vi.advanceTimersByTime(100);
    vi.advanceTimersByTime(10);
    expect(el.style.getPropertyValue("--shift-x")).toBe("-2px");
    expect(el.style.getPropertyValue("--shift-y")).toBe("-2px");
  });
});
