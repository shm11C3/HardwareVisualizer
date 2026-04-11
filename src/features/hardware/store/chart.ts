import { atom } from "jotai";
import type { NameValues } from "@/features/hardware/types/hardwareDataType";

export const cpuUsageHistoryAtom = atom<(number | null)[]>([]);
export const processorsUsageHistoryAtom = atom<number[][]>([]);
export const memoryUsageHistoryAtom = atom<(number | null)[]>([]);

// ── Multi-GPU state ──

/** Per-GPU usage histories keyed by gpuId */
export const gpuUsageHistoriesAtom = atom<Record<string, (number | null)[]>>(
  {},
);

/** Currently selected GPU ID for dashboard/usage view */
export const selectedGpuIdAtom = atom<string | null>(null);

/** Per-GPU usage source keyed by gpuId */
export const gpuUsageSourcesAtom = atom<Record<string, string | null>>({});

/** Per-GPU dedicated memory (KB) keyed by gpuId */
export const gpuDedicatedMemoryKbMapAtom = atom<Record<string, number | null>>(
  {},
);

/** Per-GPU temperature keyed by gpuId */
export const gpuTempMapAtom = atom<
  Record<string, { name: string; value: number }>
>({});

/** Per-GPU fan speed keyed by gpuId */
export const gpuFanSpeedMapAtom = atom<
  Record<string, { name: string; value: number }>
>({});

export const cpuTempAtom = atom<NameValues>([]);
export const cpuFanSpeedAtom = atom<NameValues>([]);

/** All GPUs temperature as NameValues (read-write: write clears the map) */
export const gpuTempAtom = atom<NameValues, [NameValues], void>(
  (get) => Object.values(get(gpuTempMapAtom)),
  (_get, set, _update) => {
    set(gpuTempMapAtom, {});
  },
);

/** All GPUs fan speed as NameValues */
export const gpuFanSpeedAtom = atom<NameValues>((get) =>
  Object.values(get(gpuFanSpeedMapAtom)),
);

// ── Derived atoms for backward compatibility ──

/** Resolves to the selected (or first) GPU's usage history */
export const graphicUsageHistoryAtom = atom<(number | null)[]>((get) => {
  const selected = get(selectedGpuIdAtom);
  const histories = get(gpuUsageHistoriesAtom);
  if (selected && histories[selected]) return histories[selected];
  const keys = Object.keys(histories);
  return keys.length > 0 ? histories[keys[0]] : [];
});

/** Resolves to the selected (or first) GPU's usage source */
export const gpuUsageSourceAtom = atom<string | null>((get) => {
  const selected = get(selectedGpuIdAtom);
  const sources = get(gpuUsageSourcesAtom);
  if (selected && selected in sources) return sources[selected];
  const keys = Object.keys(sources);
  return keys.length > 0 ? (sources[keys[0]] ?? null) : null;
});

/** Resolves to the selected (or first) GPU's dedicated memory (KB) */
export const gpuDedicatedMemoryKbAtom = atom<number | null>((get) => {
  const selected = get(selectedGpuIdAtom);
  const map = get(gpuDedicatedMemoryKbMapAtom);
  if (selected && selected in map) return map[selected];
  const keys = Object.keys(map);
  return keys.length > 0 ? (map[keys[0]] ?? null) : null;
});
