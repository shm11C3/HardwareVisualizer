import { useStore } from "jotai";
import { useCallback } from "react";
import { isEffectivelyGranted } from "@/lib/productAnalytics";
import { commands, type ProductEvent } from "@/rspc/bindings";
import { settingsAtom } from "@/store/settings";

/**
 * Returns a stable `record(event)` for Product Events (ADR 0028).
 *
 * Consent is read from the store at call time, not through a subscription, so
 * callers never rerender when the consent changes. Skipping the IPC while
 * consent is not effective is a cost optimization only: the backend enforces
 * consent on every event. A failed call is logged and is never a panel state.
 */
export const useProductEvent = () => {
  const store = useStore();

  return useCallback(
    async (event: ProductEvent) => {
      if (!isEffectivelyGranted(store.get(settingsAtom).productAnalytics)) {
        return;
      }
      try {
        await commands.recordProductEvent(event);
      } catch (err) {
        console.error("Failed to record a product event:", err);
      }
    },
    [store],
  );
};
