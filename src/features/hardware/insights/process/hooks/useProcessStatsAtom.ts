import { useAtom } from "jotai";
import type { ProcessStat } from "@/features/hardware/insights/types/processStats";
import { processStatsAtom } from "@/features/hardware/store/processStats";

export const useProcessStatsAtom = () => {
  const [processStats, setProcessStats] = useAtom(processStatsAtom);

  const setProcessStatsAtom = (processes: ProcessStat[]) => {
    setProcessStats(processes);
  };

  return { processStats, setProcessStatsAtom };
};
