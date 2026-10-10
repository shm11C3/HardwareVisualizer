import { atom } from "jotai";

export const cpuUsageHistoryAtom = atom<(number | null)[]>([]);
export const processorsUsageHistoryAtom = atom<number[][]>([]);
export const memoryUsageHistoryAtom = atom<(number | null)[]>([]);

/** Stand-in for `processorsUsageHistoryAtom` while runtime stats are disabled. */
export const disabledProcessorsUsageHistoryAtom = atom<number[][]>([]);
