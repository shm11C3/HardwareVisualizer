import { atom } from "jotai";
import type { DimmTemperature, NetworkInfo, SysInfo } from "@/rspc/bindings";

export const hardInfoAtom = atom<SysInfo>({
  cpu: null,
  memory: null,
  gpus: null,
  storage: [],
  motherboard: null,
});

// The last explicit detail request is scoped to the current UI session.
export const memoryDimmTemperatureAtom = atom<DimmTemperature | null>(null);
export const memoryDetailLoadingAtom = atom(false);

export const networkInfoAtom = atom<NetworkInfo[]>([]);

/**
 * The last hardware inventory read failed. Shared because every screen that
 * needs the inventory calls `init`, and the specification sheet must tell
 * "the read failed" from "this machine has no such component".
 */
export const hardwareInfoLoadFailedAtom = atom(false);

export const networkInfoLoadFailedAtom = atom(false);
