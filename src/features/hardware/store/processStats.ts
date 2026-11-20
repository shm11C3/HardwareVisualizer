import { atom } from "jotai";
import type { ProcessStat } from "@/features/hardware/insights/types/processStats";
import type { ProcessGpuStats } from "@/rspc/bindings";

export const processStatsAtom = atom<ProcessStat[] | null>(null);
export const processGpuStatsAtom = atom<ProcessGpuStats | null>(null);
