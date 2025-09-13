import i18n from "i18next";
import { initReactI18next } from "react-i18next";

const loaders = {
  en: () => import("@/lang/en.json"),
  ja: () => import("@/lang/ja.json"),
} satisfies Record<string, () => Promise<{ default: unknown }>>;

i18n.use(initReactI18next).init({
  lng: "en",
  fallbackLng: "en",
  resources: {},
  interpolation: { escapeValue: false },
});

const loaded = new Set<string>();

export async function ensureLanguage(lang: keyof typeof loaders) {
  if (!loaded.has(lang)) {
    const mod = await loaders[lang]();
    i18n.addResourceBundle(lang, "translation", mod.default, true, true);
    loaded.add(lang);
  }
  if (i18n.language !== lang) await i18n.changeLanguage(lang);
}

// Preload default language
void ensureLanguage("en");

export default i18n;
