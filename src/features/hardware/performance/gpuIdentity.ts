import type { GraphicInfo } from "@/rspc/bindings";

/**
 * One adapter as the Performance screens need it: the stable id every live
 * value is keyed by, the name the platform reported, and a short label for
 * controls that cannot fit the full name.
 */
export type GpuAdapter = {
  id: string;
  name: string;
  label: string;
};

/**
 * Vendor words that every adapter of a given brand repeats, so they carry no
 * information in a control that exists to tell two adapters apart. Anything
 * else in the name is kept: shortening must not invent a distinction or drop
 * the part the user recognizes the card by.
 */
const VENDOR_PREFIX =
  /^(nvidia|advanced micro devices,?\s*inc\.?|amd|intel(?: corporation)?|apple)\s+/i;

const shortenGpuName = (name: string) => {
  const cleaned = name
    .replace(/\((?:r|tm)\)/gi, "")
    .replace(/[®™]/g, "")
    .replace(/\s+/g, " ")
    .trim();
  const withoutVendor = cleaned.replace(VENDOR_PREFIX, "").trim();

  if (withoutVendor.length > 0) {
    return withoutVendor;
  }
  return cleaned.length > 0 ? cleaned : name;
};

/**
 * Drop a leading run of words every adapter repeats.
 *
 * A switcher exists to tell two adapters apart, and a shared prefix is both
 * the part that cannot do that and the part a narrow card keeps while
 * truncating away the model: "GeForce RTX 4080" and "GeForce RTX 4060" become
 * "GeForce RTX 40…" twice. The full name stays on the control's tooltip.
 */
const dropSharedPrefixWords = (labels: string[]) => {
  if (labels.length < 2) {
    return labels;
  }

  const wordLists = labels.map((label) => label.split(" "));
  const shortest = Math.min(...wordLists.map((words) => words.length));
  let shared = 0;
  while (
    // Never consume a whole name: at least one word has to survive.
    shared < shortest - 1 &&
    wordLists.every(
      (words) =>
        words[shared].toLowerCase() === wordLists[0][shared].toLowerCase(),
    )
  ) {
    shared += 1;
  }

  if (shared === 0) {
    return labels;
  }

  const trimmed = wordLists.map((words) => words.slice(shared).join(" "));
  return new Set(trimmed).size === trimmed.length ? trimmed : labels;
};

/**
 * Every adapter the Performance screens can attribute a reading to: the ones
 * the one-shot hardware fetch detected, plus any id that only ever shows up in
 * the live maps. The second part matters because a reading rendered without an
 * owner is exactly the misattribution this list exists to prevent.
 *
 * Labels are shortened only while they stay unique; when two adapters shorten
 * to the same string every label falls back to the full name rather than
 * showing two controls that read identically.
 */
export const listGpuAdapters = (
  gpus: GraphicInfo[] | null | undefined,
  liveNamesById: Record<string, { name: string }>,
  liveIds: readonly string[] = [],
): GpuAdapter[] => {
  const named = new Map<string, string>();

  for (const gpu of gpus ?? []) {
    named.set(gpu.id, gpu.name);
  }
  for (const id of liveIds) {
    if (!named.has(id)) {
      named.set(id, liveNamesById[id]?.name ?? id);
    }
  }
  for (const [id, value] of Object.entries(liveNamesById)) {
    if (!named.has(id)) {
      named.set(id, value.name);
    }
  }

  const entries = [...named.entries()];
  const labels = dropSharedPrefixWords(
    entries.map(([, name]) => shortenGpuName(name)),
  );
  const isUnique = new Set(labels).size === labels.length;

  return entries.map(([id, name], index) => ({
    id,
    name,
    label: isUnique ? labels[index] : name,
  }));
};

/**
 * The GPU both Performance views agree on.
 *
 * An explicit selection wins while the adapter is still detected, even when it
 * reports nothing: the honest answer there is "this adapter has no readings",
 * not another adapter's numbers. Only a selection pointing at an adapter that
 * no longer exists falls back to the first GPU that reports anything.
 */
export const getEffectiveGpuId = (
  selectedGpuId: string | null,
  gpuUsageHistories: Record<string, (number | null)[]>,
  gpuTemperatureMap: Record<string, { name: string; value: number }>,
  detectedGpuIds: readonly string[] = [],
) => {
  if (
    selectedGpuId != null &&
    (Object.hasOwn(gpuUsageHistories, selectedGpuId) ||
      detectedGpuIds.includes(selectedGpuId))
  ) {
    return selectedGpuId;
  }

  return Object.keys(gpuUsageHistories)[0] ?? Object.keys(gpuTemperatureMap)[0];
};

/**
 * Whether the screen may state that an adapter has no live readings.
 *
 * Empty maps before the first sample mean "not measured yet", so the claim is
 * only allowed once some adapter has reported and this one still has not.
 */
export const hasNoLiveGpuReadings = (
  gpuId: string | undefined,
  gpuUsageHistories: Record<string, (number | null)[]>,
  gpuTemperatureMap: Record<string, { name: string; value: number }>,
) => {
  if (gpuId == null) {
    return false;
  }

  const anyAdapterReported =
    Object.keys(gpuUsageHistories).length > 0 ||
    Object.keys(gpuTemperatureMap).length > 0;

  return (
    anyAdapterReported &&
    !Object.hasOwn(gpuUsageHistories, gpuId) &&
    !Object.hasOwn(gpuTemperatureMap, gpuId)
  );
};
