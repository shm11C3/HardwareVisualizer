import { atom } from "jotai";
import {
  getEffectiveGpuId,
  type LiveGpuId,
} from "@/features/hardware/gpuIdentity";
import { selectedGpuIdAtom } from "@/features/hardware/store/selection";
import type { NameValues } from "@/features/hardware/types/hardwareDataType";

// ── Multi-GPU state ──

/** Per-GPU usage histories keyed by gpuId */
export const gpuUsageHistoriesAtom = atom<Record<LiveGpuId, (number | null)[]>>(
  {},
);

/**
 * Per-GPU name keyed by the live sampling id.
 *
 * The monitor stream and the one-shot `getHardwareInfo` inventory key their
 * GPUs in different namespaces on every platform — Windows NVIDIA reports the
 * raw NVAPI id as `GraphicInfo.id` but samples as `nvapi:<id>`, macOS pairs
 * `0x<registry_id>` with `iokit:<name>`, Linux pairs `card<n>` with the PCI
 * BDF. So a live id cannot be resolved against the inventory, and every
 * sample carries its own name for exactly that reason.
 */
export const gpuNamesAtom = atom<Record<LiveGpuId, string>>({});

/** Per-GPU usage source keyed by gpuId */
export const gpuUsageSourcesAtom = atom<Record<LiveGpuId, string | null>>({});

/** Per-GPU dedicated memory (KB) keyed by gpuId */
export const gpuDedicatedMemoryKbMapAtom = atom<
  Record<LiveGpuId, number | null>
>({});

/** Per-GPU temperature keyed by gpuId */
export const gpuTempMapAtom = atom<
  Record<LiveGpuId, { name: string; value: number }>
>({});

/** Per-GPU fan speed keyed by gpuId */
export const gpuFanSpeedMapAtom = atom<
  Record<LiveGpuId, { name: string; value: number }>
>({});

/** All GPUs temperature as NameValues */
export const gpuTempAtom = atom<NameValues>((get) =>
  Object.values(get(gpuTempMapAtom)),
);

/** All GPUs fan speed as NameValues */
export const gpuFanSpeedAtom = atom<NameValues>((get) =>
  Object.values(get(gpuFanSpeedMapAtom)),
);

// ── Derived atoms for backward compatibility ──

/**
 * The adapter every derived atom below describes.
 *
 * It has to be the same answer the GPU selectors show, or a surface would
 * label one adapter and render another's numbers. In particular an explicit
 * selection that reports no usage resolves to itself, so the consumers below
 * return nothing rather than borrowing the first adapter's values.
 */
const effectiveGpuIdAtom = atom<LiveGpuId | undefined>((get) =>
  getEffectiveGpuId(
    get(selectedGpuIdAtom),
    {
      usageHistories: get(gpuUsageHistoriesAtom),
      temperatures: get(gpuTempMapAtom),
      fanSpeeds: get(gpuFanSpeedMapAtom),
      dedicatedMemoryKb: get(gpuDedicatedMemoryKbMapAtom),
    },
    Object.keys(get(gpuNamesAtom)) as LiveGpuId[],
  ),
);

/** Resolves to the effective GPU's usage history */
export const graphicUsageHistoryAtom = atom<(number | null)[]>((get) => {
  const effective = get(effectiveGpuIdAtom);
  return effective != null ? (get(gpuUsageHistoriesAtom)[effective] ?? []) : [];
});

/** Resolves to the effective GPU's usage source */
export const gpuUsageSourceAtom = atom<string | null>((get) => {
  const effective = get(effectiveGpuIdAtom);
  return effective != null
    ? (get(gpuUsageSourcesAtom)[effective] ?? null)
    : null;
});

/** Resolves to the effective GPU's dedicated memory (KB) */
export const gpuDedicatedMemoryKbAtom = atom<number | null>((get) => {
  const effective = get(effectiveGpuIdAtom);
  return effective != null
    ? (get(gpuDedicatedMemoryKbMapAtom)[effective] ?? null)
    : null;
});
