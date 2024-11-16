import en from "@/i18n/en.json";
import ja from "@/i18n/ja.json";
import i18n from "i18next";
import { initReactI18next } from "react-i18next";

const resources = {
  en: {
    translation: en,
  },
  ja: {
    translation: ja,
  },
};

i18n.use(initReactI18next).init({
  resources,
  lng: "ja",
  interpolation: {
    escapeValue: false,
  },
});

export default i18n;
