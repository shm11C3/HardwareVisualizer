import { atom } from "jotai";
import type { SensorSupport } from "@/rspc/bindings";

export type PowerDraw = {
  cpuWatts: number | null;
  gpuWatts: number | null;
  aneWatts: number | null;
  packageWatts: number | null;
};

export type PowerDrawHistory = {
  [K in keyof PowerDraw]: (number | null)[];
};

export const powerDrawAtom = atom<PowerDraw>({
  cpuWatts: null,
  gpuWatts: null,
  aneWatts: null,
  packageWatts: null,
});

export const powerDrawHistoryAtom = atom<PowerDrawHistory>({
  cpuWatts: [],
  gpuWatts: [],
  aneWatts: [],
  packageWatts: [],
});

/** Whether this runtime has produced at least one power reading. */
export const powerDrawAvailableAtom = atom(false);

/** Hardware support for CPU package-power collection. */
export const cpuPowerSupportAtom = atom<SensorSupport>("unknown");
