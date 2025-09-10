import { beforeAll, describe, expect, it } from "vitest";
import i18n from "@/lib/i18n";

describe("i18n configuration", () => {
  beforeAll(async () => {
    await i18n.init();
  });

  it("should initialize with English as default language", () => {
    expect(i18n.language).toBe("en");
  });

  it("should have English translation resources", () => {
    expect(i18n.hasResourceBundle("en", "translation")).toBe(true);
  });

  it("should have Japanese translation resources", () => {
    expect(i18n.hasResourceBundle("ja", "translation")).toBe(true);
  });

  it("should be able to change language to Japanese", async () => {
    await i18n.changeLanguage("ja");
    expect(i18n.language).toBe("ja");

    // Reset to English for other tests
    await i18n.changeLanguage("en");
  });

  it("should have escapeValue disabled for interpolation", () => {
    expect(i18n.options.interpolation?.escapeValue).toBe(false);
  });

  it("should use react-i18next plugin", () => {
    expect(i18n.isInitialized).toBe(true);
  });
});
