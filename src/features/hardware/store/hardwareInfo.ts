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
