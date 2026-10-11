import { atom } from "jotai";
import type { SensorSupport } from "@/rspc/bindings";

/** Whether this runtime has produced at least one power reading. */
export const powerDrawAvailableAtom = atom(false);

/** Hardware support for CPU package-power collection. */
export const cpuPowerSupportAtom = atom<SensorSupport>("unknown");
