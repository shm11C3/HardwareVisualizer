import { atom } from "jotai";
import type { NetworkInfo, SysInfo } from "@/rspc/bindings";

export const hardInfoAtom = atom<SysInfo>({
  cpu: null,
  memory: null,
  gpus: null,
  storage: [],
  motherboard: null,
});

export const networkInfoAtom = atom<NetworkInfo[]>([]);

/**
 * The last hardware inventory read failed. Shared because every screen that
 * needs the inventory calls `init`, and the specification sheet must tell
 * "the read failed" from "this machine has no such component".
 */
export const hardwareInfoLoadFailedAtom = atom(false);

export const networkInfoLoadFailedAtom = atom(false);
