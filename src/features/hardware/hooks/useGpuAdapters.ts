import { useAtom, useAtomValue } from "jotai";
import {
  effectiveGpuAdapterAtom,
  effectiveGpuIdAtom,
  gpuAdaptersAtom,
  gpuHasNoReadingsAtom,
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
 */
export const useGpuAdapters = () => {
  const adapters = useAtomValue(gpuAdaptersAtom);
  const effectiveGpuId = useAtomValue(effectiveGpuIdAtom);
  const effectiveAdapter = useAtomValue(effectiveGpuAdapterAtom);
  const hasNoReadings = useAtomValue(gpuHasNoReadingsAtom);
  const [selectedGpuId, setSelectedGpuId] = useAtom(selectedGpuIdAtom);

  return {
    adapters,
    selectedGpuId,
    effectiveGpuId,
    effectiveAdapter,
    hasNoReadings,
    selectGpu: setSelectedGpuId,
  };
};
