import { useAtom, useAtomValue } from "jotai";
import { useEffect, useRef, useState } from "react";
import {
  findLiveGpuId,
  findNamedLiveGpuId,
  liveGpuSelection,
  migrateLegacyPdhGpuId,
  unresolvedGpuSelection,
} from "@/features/hardware/gpuIdentity";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import { gpuNamesAtom } from "@/features/hardware/store/gpu";
import { selectedGpuIdAtom } from "@/features/hardware/store/selection";
import { useTauriStore } from "@/hooks/tauri/useTauriStore";

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
  // Stored as a plain string on disk. Restored as unresolved intent: shipped
  // versions wrote inventory ids into this key, which is exactly what the
  // migration effect below translates once the stream names the adapter.
  const [storedId, setStoredId, isPending, loadFailed] = useTauriStore<
    string | null
  >(STORE_KEY, null);
  const [selection, setSelection] = useAtom(selectedGpuIdAtom);
  const selectedId = selection?.id ?? null;
  // State, not a ref: a ref flipped inside the hydration effect is already
  // true when the write-back effect runs later in the *same* commit, where
  // `selection` is still the pre-hydration value — so it would persist
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
      setSelection(unresolvedGpuSelection(storedId));
    }
    setHydrated(true);
  }, [isPending, hydrated, storedId, setSelection]);

  useEffect(() => {
    // A failed read settles `storedId` to null without knowing what is on
    // disk. Writing the auto-selected adapter back from there would replace
    // the stored intent with a fallback for good, so the write-back stays
    // off for the session (DP-06).
    if (!hydrated || loadFailed) return;
    if (selectedId === storedId) return;
    setStoredId(selectedId);
  }, [hydrated, loadFailed, selectedId, storedId, setStoredId]);

  // Every restored selection — and any made in the classic card before the
  // first sample or through an ambiguous name — starts unresolved; some are
  // inventory ids from shipped versions, which address no readings.
  // Resolving them here rather than inside a GPU screen matters: grouped
  // navigation never mounts the classic card, so a stored choice would
  // otherwise stay inert for the whole life of the installation.
  const unresolvedId = selection?.kind === "unresolved" ? selection.id : null;
  useEffect(() => {
    if (!hydrated || unresolvedId == null) return;
    // Before the first sample every id looks unresolved; there is nothing to
    // decide yet, so wait rather than fetch an inventory that may never be
    // needed.
    if (Object.keys(gpuNames).length === 0) return;

    // The common case: a stored id written from a live selection, which the
    // stream reports again under the same id.
    const namedId = findNamedLiveGpuId(unresolvedId, gpuNames);
    if (namedId != null) {
      setSelection(liveGpuSelection(namedId));
      return;
    }

    const migratedPdhId = migrateLegacyPdhGpuId(unresolvedId, gpuNames);
    if (migratedPdhId != null) {
      setSelection(liveGpuSelection(migratedPdhId));
      return;
    }
    // A PDH id is already in the live namespace. If it is not present, the
    // inventory cannot resolve it, so preserve the intent without an
    // unnecessary inventory fetch.
    if (unresolvedId.startsWith("pdh:")) return;

    // A restart can land on a view that never fetches the inventory (Monitor,
    // Compact), and the migration cannot read what nothing has fetched. Only
    // an unresolved id triggers the fetch, so steady-state startups where the
    // stored id is already live cost no extra IPC. The ref keeps the request
    // single-shot even if the effect inputs change while the fetch is in flight.
    if (hardwareInfo.gpus == null) {
      if (!inventoryRequestedRef.current) {
        inventoryRequestedRef.current = true;
        void init();
      }
      return;
    }

    const inventoryEntry = hardwareInfo.gpus.find(
      (gpu) => gpu.id === unresolvedId,
    );
    if (inventoryEntry == null) return;

    // An ambiguous name stays unresolved: guessing would attribute one
    // adapter's readings to the other.
    const liveId = findLiveGpuId(inventoryEntry, gpuNames);
    if (liveId != null) {
      setSelection(liveGpuSelection(liveId));
    }
  }, [hydrated, unresolvedId, gpuNames, hardwareInfo.gpus, setSelection, init]);
};
