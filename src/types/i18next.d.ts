import "i18next";
import type en from "@/lang/en.json";
import type ja from "@/lang/ja.json";
import type ru from "@/lang/ru.json";

type Resources = {
  en: typeof en;
  ja: typeof ja;
  ru: typeof ru;
};

declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: keyof Resources;
    resources: Resources;
  }
}
