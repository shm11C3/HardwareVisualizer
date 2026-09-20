import { createStore } from "jotai";
import { describe, expect, it } from "vitest";
import { modalAtoms, settingAtoms } from "@/store/ui";

describe("UI Store", () => {
  describe("modalAtoms", () => {
    it("should have showSettingsModal atom with default value false", () => {
      expect(createStore().get(modalAtoms.showSettingsModal)).toBe(false);
    });
  });

  describe("settingAtoms", () => {
    it("should have isRequiredRestart atom with default value false", () => {
      expect(createStore().get(settingAtoms.isRequiredRestart)).toBe(false);
    });
  });

  it("should export both modalAtoms and settingAtoms", () => {
    expect(modalAtoms).toBeDefined();
    expect(settingAtoms).toBeDefined();
  });
});
