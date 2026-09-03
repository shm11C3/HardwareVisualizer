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

  it("should fall back to English for untranslated Russian keys", async () => {
    await i18n.changeLanguage("ru");
    try {
      expect(i18n.t("pages.settings.general.trayWidget.name")).toBe(
        "Tray widget",
      );
    } finally {
      // Reset to English for other tests
      await i18n.changeLanguage("en");
    }
  });

  it("should have escapeValue disabled for interpolation", () => {
    expect(i18n.options.interpolation?.escapeValue).toBe(false);
  });

  it("should fall back to English for empty translations", () => {
    expect(i18n.options.returnEmptyString).toBe(false);
  });

  it("distinguishes unsupported hardware from uncollected history in every locale", async () => {
    const expected = {
      en: [
        "Your current hardware does not support power collection.",
        "Power data has not been collected for this period yet.",
      ],
      ja: [
        "現在ご利用のハードウェアでは、電力の取得に対応していません。",
        "この期間の電力データはまだ収集されていません。",
      ],
      ru: [
        "Текущее оборудование не поддерживает сбор данных о мощности.",
        "Данные о мощности за этот период ещё не собраны.",
      ],
    } as const;
    const keys = [
      "pages.insights.cooling.sensorStatusNote.unsupported.power",
      "pages.insights.cooling.sensorStatusNote.notCollected.power",
    ] as const;

    try {
      for (const [language, translations] of Object.entries(expected)) {
        await i18n.changeLanguage(language);
        expect(keys.map((key) => i18n.t(key))).toEqual(translations);
      }
    } finally {
      await i18n.changeLanguage("en");
    }
  });

  it("should use react-i18next plugin", () => {
    expect(i18n.isInitialized).toBe(true);
  });
});
