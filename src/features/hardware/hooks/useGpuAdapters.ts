import { useAtomValue, useSetAtom, useStore } from "jotai";
import { useCallback } from "react";
import {
  type LiveGpuId,
  liveGpuSelection,
  selectionForInventoryGpu,
} from "@/features/hardware/gpuIdentity";
import {
  effectiveGpuAdapterAtom,
  effectiveGpuIdAtom,
  gpuAdaptersAtom,
  gpuHasNoReadingsAtom,
  gpuNamesAtom,
} from "@/features/hardware/store/gpu";
import { selectedGpuIdAtom } from "@/features/hardware/store/selection";

/**
 * The one place the GPU surfaces agree on which adapter they are describing.
 *
 * Every view that renders a GPU reading needs the same three answers — which
 * adapters exist, which one is effective, and whether it is reporting — and
 * they have to be the same answers, or two views would attribute the same
 * numbers to different devices.
 *
 * It returns identity only, never a reading. The adapter list, the effective
 * id and the "no readings" flag are derived atoms that stay referentially
 * stable between samples, so a component that calls this hook does not
 * re-render once a second. A surface that renders the effective adapter's
 * numbers reads them from the per-adapter atoms in `store/gpu.ts`
 * (`graphicUsageHistoryAtom`, `gpuTemperatureValueAtom`, ...) in the smallest
 * component that shows them.
 *
 * `selection` may be an unresolved intent. Render from `effectiveGpuId`; the
 * raw selection is only for the classic card's pre-sample inventory match.
 */
export const useGpuAdapters = () => {
  const store = useStore();
  const adapters = useAtomValue(gpuAdaptersAtom);
  const effectiveGpuId = useAtomValue(effectiveGpuIdAtom);
  const effectiveAdapter = useAtomValue(effectiveGpuAdapterAtom);
  const hasNoReadings = useAtomValue(gpuHasNoReadingsAtom);
  const selection = useAtomValue(selectedGpuIdAtom);
  const setSelection = useSetAtom(selectedGpuIdAtom);

  /** Select an adapter the live stream names. */
  const selectGpu = useCallback(
    (gpuId: LiveGpuId) => setSelection(liveGpuSelection(gpuId)),
    [setSelection],
  );

  /**
   * Select an inventory entry. The live names are read at click time rather
   * than subscribed to, so holding this function costs no re-renders.
   */
  const selectInventoryGpu = useCallback(
    (gpu: { id: string; name: string }) =>
      setSelection(selectionForInventoryGpu(gpu, store.get(gpuNamesAtom))),
    [setSelection, store],
  );

  return {
    adapters,
    selection,
    effectiveGpuId,
    effectiveAdapter,
    hasNoReadings,
    selectGpu,
    selectInventoryGpu,
  };
};
