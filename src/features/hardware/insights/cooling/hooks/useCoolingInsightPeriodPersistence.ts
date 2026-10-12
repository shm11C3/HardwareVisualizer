import { useAtom } from "jotai";
import { useEffect, useState } from "react";
import type { CoolingInsightPeriod } from "@/features/hardware/insights/cooling/types";
import {
  coolingInsightPeriodAtom,
  coolingInsightPeriodHydratedAtom,
} from "@/features/hardware/store/coolingPeriod";
import { useTauriStore } from "@/hooks/tauri/useTauriStore";

/**
 * UI-local key for the Cooling tab's single period selector. Deliberately
 * separate from the per-chart `period*` Tauri Store keys the CPU/Memory tab
 * still uses (`periodAvgCpuTemperature`, etc.) - those stay owned by that
 * tab, and sharing one key across two tabs was the pre-#2018 bug this
 * dedicated key avoids reintroducing.
 */
export const COOLING_INSIGHT_PERIOD_STORE_KEY = "coolingInsightPeriod";

/**
 * Keep `coolingInsightPeriodAtom` and its Tauri Store key in step: read the
 * stored period when the Cooling tab mounts, write it back when the
 * selection changes. Same lifecycle as `useSelectedGpuPersistence`.
 *
 * `coolingInsightPeriodHydratedAtom` turns true once the stored period has
 * landed in the atom, which is what the view waits for so the default never
 * flashes (and never fetches) before the stored period arrives. A failed
 * read settles on the default with the atom already equal to it, so the
 * write-back stays quiet until the user picks something.
 */
export const useCoolingInsightPeriodPersistence = () => {
  const [stored, setStored, isPending] = useTauriStore<CoolingInsightPeriod>(
    COOLING_INSIGHT_PERIOD_STORE_KEY,
    "24h",
  );
  const [period, setPeriod] = useAtom(coolingInsightPeriodAtom);
  const [atomHydrated, setAtomHydrated] = useAtom(
    coolingInsightPeriodHydratedAtom,
  );
  // State, not a ref: the hydration effect and the write-back effect can run
  // in the same commit, where `period` is still the pre-hydration value.
  const [hydrated, setHydrated] = useState(false);

  useEffect(() => {
    if (isPending || hydrated) return;
    // The atom outlives this tab's mount. A later mount must not overwrite a
    // selection made while its own read was still in flight.
    if (!atomHydrated) {
      setPeriod(stored);
      setAtomHydrated(true);
    }
    setHydrated(true);
  }, [isPending, hydrated, atomHydrated, stored, setPeriod, setAtomHydrated]);

  useEffect(() => {
    if (!hydrated || period === stored) return;
    setStored(period).catch((error: unknown) => {
      console.error("Failed to persist the Cooling Insight period:", error);
    });
  }, [hydrated, period, stored, setStored]);
};
