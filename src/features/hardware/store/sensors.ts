import { atom } from "jotai";
import type {
  MotherboardFanSpeedValues,
  MotherboardTemperatureValues,
  NameValues,
} from "@/features/hardware/types/hardwareDataType";
import type { SensorSupport } from "@/rspc/bindings";

export const cpuTempAtom = atom<NameValues>([]);
export const cpuFanSpeedAtom = atom<NameValues>([]);

/** All named temperature sensors (thermal zones), Windows only for now */
export const sensorTempsAtom = atom<NameValues>([]);

/** Live motherboard temperature sensors from the Super I/O provider */
export const motherboardTempsAtom = atom<MotherboardTemperatureValues>([]);

/** Live motherboard fan speeds from the Super I/O provider */
export const motherboardFanSpeedsAtom = atom<MotherboardFanSpeedValues>([]);

/** Hardware support for motherboard fan-speed collection. */
export const motherboardFanSupportAtom = atom<SensorSupport>("unknown");

/**
 * Whether any Super I/O reading is present. A flag, so the card that decides
 * whether to show the sensor section does not re-render with every sample.
 */
export const hasMotherboardSensorsAtom = atom(
  (get) =>
    get(motherboardTempsAtom).length > 0 ||
    get(motherboardFanSpeedsAtom).length > 0,
);
