import { atom, useAtom } from "jotai";
import type { ProcessStat } from "../types/processStats";

const processStatsAtom = atom<ProcessStat[] | null>(null);

export const useProcessStatsAtom = () => {
  const [processStats, setProcessStats] = useAtom(processStatsAtom);

  const setProcessStatsAtom = (processes: ProcessStat[]) => {
    setProcessStats(processes);
  };

  return { processStats, setProcessStatsAtom };
};
