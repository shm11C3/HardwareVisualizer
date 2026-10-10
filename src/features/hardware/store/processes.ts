import { atom } from "jotai";
import type { ProcessInfo } from "@/rspc/bindings";

export const processesAtom = atom<ProcessInfo[]>([]);
export const disabledProcessesAtom = atom<ProcessInfo[]>([]);
