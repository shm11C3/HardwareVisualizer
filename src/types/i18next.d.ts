import "i18next";
import type en from "@/i18n/en.json";
import type ja from "@/i18n/ja.json";

type Resources = {
  en: typeof en;
  ja: typeof ja;
};

declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: keyof Resources;
    resources: Resources;
  }
}
