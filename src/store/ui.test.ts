import { createStore } from "jotai";
import { describe, expect, it } from "vitest";
import { isRestartRequiredAtom } from "@/store/ui";

describe("UI Store", () => {
  describe("isRestartRequiredAtom", () => {
    it("should default to false", () => {
      expect(createStore().get(isRestartRequiredAtom)).toBe(false);
    });
  });
});
