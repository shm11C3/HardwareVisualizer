import { atom } from "jotai";
import type { LiveGpuId } from "@/features/hardware/gpuIdentity";

/** Currently selected GPU ID for dashboard/usage view */
export const selectedGpuIdAtom = atom<LiveGpuId | null>(null);

/** Currently selected storage device id for the Storage Health Display */
export const selectedStorageDeviceIdAtom = atom<string | null>(null);
