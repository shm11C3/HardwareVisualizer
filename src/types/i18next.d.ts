import "i18next";
import type en from "@/lang/en.json";
import type ja from "@/lang/ja.json";

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
