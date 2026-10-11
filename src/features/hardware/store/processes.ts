import { atom } from "jotai";
import type { ProcessInfo } from "@/rspc/bindings";

export const processesAtom = atom<ProcessInfo[]>([]);
export const disabledProcessesAtom = atom<ProcessInfo[]>([]);

/**
 * The last process list poll failed. Cleared by the next successful poll, so
 * consumers show a failure state instead of stale rows while polling keeps
 * retrying on its own.
 */
export const processListLoadFailedAtom = atom(false);
