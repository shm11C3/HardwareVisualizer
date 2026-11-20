import { atom } from "jotai";
import type {
  ProcessGpuRead,
  ProcessStat,
} from "@/features/hardware/insights/types/processStats";

export const processStatsAtom = atom<ProcessStat[] | null>(null);
export const processGpuReadAtom = atom<ProcessGpuRead | null>(null);
