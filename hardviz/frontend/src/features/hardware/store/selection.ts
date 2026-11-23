import { atom } from "jotai";
import type { GpuSelection } from "@/features/hardware/gpuIdentity";

/**
 * The GPU the user (or the first-sample auto-selection) chose, shared by every
 * GPU surface. It may hold an unresolved intent; read it through
 * `effectiveGpuIdAtom` or `useGpuAdapters`, never index a live map with it.
 */
export const selectedGpuIdAtom = atom<GpuSelection | null>(null);

/** Currently selected storage device id for the Storage Health Display */
export const selectedStorageDeviceIdAtom = atom<string | null>(null);
