import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useBurnInShift } from "@/hooks/useBurnInShift";
import type { RefObject } from "react";

// Simple mock setup
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

describe("useBurnInShift (Simple)", () => {
  it("should not throw when called with null ref", () => {
    const nullRef: RefObject<HTMLElement> = { current: null };
    
    expect(() => {
      renderHook(() => useBurnInShift(nullRef, true));
    }).not.toThrow();
  });

  it("should not throw when called with disabled state", () => {
    const nullRef: RefObject<HTMLElement> = { current: null };
    
    expect(() => {
      renderHook(() => useBurnInShift(nullRef, false));
    }).not.toThrow();
  });
});