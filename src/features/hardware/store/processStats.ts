import { atom } from "jotai";
import type { ProcessStat } from "@/features/hardware/insights/types/processStats";

export const processStatsAtom = atom<ProcessStat[] | null>(null);
