import { useAtom } from "jotai";
import { useEffect, useState } from "react";
import { selectedGpuIdAtom } from "@/features/hardware/store/chart";
import { useTauriStore } from "@/hooks/useTauriStore";

const STORE_KEY = "selectedGpuId";

/**
 * Persist the currently selected GPU id via Tauri Store so it survives app
 * restarts.
 *
 * The stored id is restored as-is rather than validated against the
 * `getHardwareInfo` inventory: the inventory keys GPUs in a different
 * namespace than the monitor stream the selection comes from (see
 * `gpuNamesAtom`), so validating there would reject every valid id. It also
 * discarded the user's intent permanently the first time they launched
 * without that adapter, which DP-06 forbids.
 *
 * A stored id that no longer matches any reporting adapter costs nothing:
 * `getEffectiveGpuId` falls back at display time, and the intent stays on
 * disk so it applies again when the adapter comes back.
 */
export const useSelectedGpuPersistence = () => {
  const [storedId, setStoredId, isPending] = useTauriStore<string | null>(
    STORE_KEY,
    null,
  );
  const [selectedGpuId, setSelectedGpuId] = useAtom(selectedGpuIdAtom);
  // State, not a ref: a ref flipped inside the hydration effect is already
  // true when the write-back effect runs later in the *same* commit, where
  // `selectedGpuId` is still the pre-hydration value — so it would persist
  // that stale value straight over the preference just restored.
  const [hydrated, setHydrated] = useState(false);

  useEffect(() => {
    if (isPending || hydrated) return;
    if (storedId != null) {
      // Explicit intent outranks the event listener's auto-selection of the
      // first reporting adapter, whichever of the two lands first.
      setSelectedGpuId(storedId);
    }
    setHydrated(true);
  }, [isPending, hydrated, storedId, setSelectedGpuId]);

  useEffect(() => {
    if (!hydrated) return;
    if (selectedGpuId === storedId) return;
    setStoredId(selectedGpuId);
  }, [hydrated, selectedGpuId, storedId, setStoredId]);
};
