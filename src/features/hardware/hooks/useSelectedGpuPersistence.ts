import { useAtom } from "jotai";
import { useEffect, useRef } from "react";
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
  const hydratedRef = useRef(false);

  useEffect(() => {
    if (isPending || hydratedRef.current) return;
    if (storedId != null) {
      // Explicit intent outranks the event listener's auto-selection of the
      // first reporting adapter, whichever of the two lands first.
      setSelectedGpuId(storedId);
    }
    hydratedRef.current = true;
  }, [isPending, storedId, setSelectedGpuId]);

  useEffect(() => {
    if (!hydratedRef.current) return;
    if (selectedGpuId === storedId) return;
    setStoredId(selectedGpuId);
  }, [selectedGpuId, storedId, setStoredId]);
};
