import { useAtom, useAtomValue } from "jotai";
import { useEffect, useRef, useState } from "react";
import { asLiveGpuId, toLiveGpuId } from "@/features/hardware/gpuIdentity";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import {
  gpuNamesAtom,
  selectedGpuIdAtom,
} from "@/features/hardware/store/chart";
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
  // Stored as a plain string on disk. Minted as intent on restore: shipped
  // versions wrote inventory ids into this key, which is exactly what the
  // migration effect below translates once the stream names the adapter.
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
  const inventoryRequestedRef = useRef(false);
  const gpuNames = useAtomValue(gpuNamesAtom);
  const { hardwareInfo, init } = useHardwareInfoAtom();

  useEffect(() => {
    if (isPending || hydrated) return;
    if (storedId != null) {
      // Explicit intent outranks the event listener's auto-selection of the
      // first reporting adapter, whichever of the two lands first.
      setSelectedGpuId(asLiveGpuId(storedId));
    }
    setHydrated(true);
  }, [isPending, hydrated, storedId, setSelectedGpuId]);

  useEffect(() => {
    if (!hydrated) return;
    if (selectedGpuId === storedId) return;
    setStoredId(selectedGpuId);
  }, [hydrated, selectedGpuId, storedId, setStoredId]);

  // Selections written before this hook existed — and any made in the classic
  // card before the first sample — are inventory ids, which address no
  // readings. Translating them here rather than inside a GPU screen matters:
  // grouped navigation never mounts the classic card, so a stored choice would
  // otherwise stay inert for the whole life of the installation.
  useEffect(() => {
    if (!hydrated || selectedGpuId == null) return;
    // Before the first sample every id looks unresolved; there is nothing to
    // decide yet, so wait rather than fetch an inventory that may never be
    // needed.
    if (Object.keys(gpuNames).length === 0) return;
    if (gpuNames[selectedGpuId] != null) return;

    // A restart can land on a view that never fetches the inventory (Monitor,
    // Compact), and the migration cannot read what nothing has fetched. Only
    // an unresolved id triggers the fetch, so steady-state startups where the
    // stored id is already live cost no extra IPC. The ref keeps the request
    // single-shot: `init` is a new function every render, so it re-triggers
    // this effect while the fetch is still in flight.
    if (hardwareInfo.gpus == null) {
      if (!inventoryRequestedRef.current) {
        inventoryRequestedRef.current = true;
        void init();
      }
      return;
    }

    const inventoryEntry = hardwareInfo.gpus.find(
      (gpu) => gpu.id === selectedGpuId,
    );
    if (inventoryEntry == null) return;

    const liveId = toLiveGpuId(inventoryEntry, gpuNames);
    if (liveId !== selectedGpuId) {
      setSelectedGpuId(liveId);
    }
  }, [
    hydrated,
    selectedGpuId,
    gpuNames,
    hardwareInfo.gpus,
    setSelectedGpuId,
    init,
  ]);
};
