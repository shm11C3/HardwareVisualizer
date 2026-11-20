import { useAtom } from "jotai";
import { processStatsAtom } from "@/features/hardware/store/processStats";

export const useProcessStatsAtom = () => {
  const [processStats, setProcessStats] = useAtom(processStatsAtom);

  // Jotai's setter is stable. A new wrapper per render would change
  // `useProcessStats`'s `load` every render and refetch in a loop wherever
  // React Compiler does not memoize it (e.g. under Vitest).
  return { processStats, setProcessStatsAtom: setProcessStats };
};
