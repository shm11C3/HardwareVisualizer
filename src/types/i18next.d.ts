import "i18next";
import type en from "@/lang/en.json";

/**
 * The runtime registers one namespace, `translation`, per language
 * (`src/lib/i18n.ts`), so the types do the same with `en` as the key source.
 * Declaring one namespace per language tripled the key union and pushed
 * `t()` call sites past TypeScript's instantiation limit as keys were added.
 */
declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: "translation";
    resources: {
      translation: typeof en;
    };
  }
}
