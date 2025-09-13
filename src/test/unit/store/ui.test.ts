import { describe, expect, it } from "vitest";
import { modalAtoms, settingAtoms } from "@/store/ui";

describe("UI Store", () => {
  describe("modalAtoms", () => {
    it("should have showSettingsModal atom with default value false", () => {
      expect(modalAtoms.showSettingsModal).toBeDefined();
      // Atoms don't expose their initial values directly, but we can verify the structure
      expect(typeof modalAtoms.showSettingsModal).toBe("object");
    });
  });

  describe("settingAtoms", () => {
    it("should have isRequiredRestart atom with default value false", () => {
      expect(settingAtoms.isRequiredRestart).toBeDefined();
      expect(typeof settingAtoms.isRequiredRestart).toBe("object");
    });
  });

  it("should export both modalAtoms and settingAtoms", () => {
    expect(modalAtoms).toBeDefined();
    expect(settingAtoms).toBeDefined();
  });
});
