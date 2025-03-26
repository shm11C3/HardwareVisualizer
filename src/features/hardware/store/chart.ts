import type { NameValues } from "@/features/hardware/types/hardwareDataType";
import { atom } from "jotai";

export const cpuUsageHistoryAtom = atom<number[]>([]);
export const memoryUsageHistoryAtom = atom<number[]>([]);
export const graphicUsageHistoryAtom = atom<number[]>([]);
export const cpuTempAtom = atom<NameValues>([]);
export const cpuFanSpeedAtom = atom<NameValues>([]);
export const gpuTempAtom = atom<NameValues>([]);
export const gpuFanSpeedAtom = atom<NameValues>([]);
