import { createStore } from "jotai";
import { describe, expect, it } from "vitest";
import { settingAtoms } from "@/store/ui";

describe("UI Store", () => {
  describe("settingAtoms", () => {
    it("should have isRequiredRestart atom with default value false", () => {
      expect(createStore().get(settingAtoms.isRequiredRestart)).toBe(false);
    });
  });
});
