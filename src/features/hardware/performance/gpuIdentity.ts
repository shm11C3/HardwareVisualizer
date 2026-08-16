/**
 * One adapter as the Performance screens need it: the stable id every live
 * value is keyed by, the name the platform reported, and a short label for
 * controls that cannot fit the full name.
 */
export type GpuAdapter = {
  id: string;
  /** The raw platform name. Two identical cards report the same one. */
  name: string;
  /** Always unique across the list, so a control can never be ambiguous. */
  label: string;
  /**
   * Whether another adapter reports the same `name`. Any join that keys on
   * the name — the inventory's VRAM total, for one — has to refuse when this
   * is true, because the name cannot pick out one of the two.
   */
  isNameAmbiguous: boolean;
};

/**
 * The live per-GPU maps, together.
 *
 * They are populated independently — an adapter can report a fan speed with no
 * usage, or VRAM with no temperature — so anything that asks "did this adapter
 * report" has to ask all four. Asking a subset is what turns a partially
 * reporting adapter into a silent one.
 */
export type GpuLiveMaps = {
  usageHistories: Record<string, (number | null)[]>;
  temperatures: Record<string, { name: string; value: number }>;
  fanSpeeds: Record<string, { name: string; value: number }>;
  dedicatedMemoryKb: Record<string, number | null>;
};

const liveMapsInOrder = (live: GpuLiveMaps) => [
  live.usageHistories,
  live.temperatures,
  live.fanSpeeds,
  live.dedicatedMemoryKb,
];

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
 * Every adapter the Performance screens can attribute a reading to.
 *
 * The list is built from the live maps alone, never unioned with the
 * `getHardwareInfo` inventory: the two key their GPUs in different namespaces
 * on every platform (see `gpuNamesAtom`), so unioning by id renders one
 * physical GPU as two adapters — one of which reports nothing and is then
 * declared silent. The inventory's job is the System Specifications sheet;
 * this list's job is attribution, and only ids that carry readings can be
 * attributed to.
 *
 * Names come from the sample that carried the reading. Labels are shortened
 * only while they stay unique; when two adapters shorten to the same string
 * every label falls back to the full name rather than showing two controls
 * that read identically.
 */
export const listGpuAdapters = (
  gpuNames: Record<string, string>,
  live: GpuLiveMaps,
): GpuAdapter[] => {
  const named = new Map<string, string>();

  // Every adapter the stream named, in payload order — including one that
  // named itself and reported nothing else, which is the case the unavailable
  // state exists for.
  for (const [id, name] of Object.entries(gpuNames)) {
    named.set(id, name);
  }
  for (const map of liveMapsInOrder(live)) {
    for (const id of Object.keys(map)) {
      if (!named.has(id)) {
        named.set(id, id);
      }
    }
  }
  // The sensor maps carry a name too, so they can still identify an adapter
  // if the name map has not caught up with them.
  for (const map of [live.temperatures, live.fanSpeeds]) {
    for (const [id, value] of Object.entries(map)) {
      if (named.get(id) === id) {
        named.set(id, value.name);
      }
    }
  }

  const entries = [...named.entries()];
  const shortened = dropSharedPrefixWords(
    entries.map(([, name]) => shortenGpuName(name)),
  );
  const nameCounts = new Map<string, number>();
  for (const [, name] of entries) {
    nameCounts.set(name, (nameCounts.get(name) ?? 0) + 1);
  }

  // An ordinal for the adapters the platform reports under one name — two
  // identical cards — then, only if labels still collide, the full names.
  // A control the user cannot tell from its neighbour is not a choice.
  const withOrdinals = (bases: string[]) => {
    const seen = new Map<string, number>();
    return bases.map((base, index) => {
      const name = entries[index][1];
      if ((nameCounts.get(name) ?? 0) < 2) {
        return base;
      }
      const ordinal = (seen.get(name) ?? 0) + 1;
      seen.set(name, ordinal);
      return `${base} #${ordinal}`;
    });
  };

  const shortLabels = withOrdinals(shortened);
  const labels =
    new Set(shortLabels).size === shortLabels.length
      ? shortLabels
      : withOrdinals(entries.map(([, name]) => name));

  return entries.map(([id, name], index) => ({
    id,
    name,
    label: labels[index],
    isNameAmbiguous: (nameCounts.get(name) ?? 0) > 1,
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
  live: GpuLiveMaps,
  detectedGpuIds: readonly string[] = [],
) => {
  if (
    selectedGpuId != null &&
    (detectedGpuIds.includes(selectedGpuId) ||
      liveMapsInOrder(live).some((map) => Object.hasOwn(map, selectedGpuId)))
  ) {
    return selectedGpuId;
  }

  for (const map of liveMapsInOrder(live)) {
    const [first] = Object.keys(map);
    if (first != null) {
      return first;
    }
  }
  return undefined;
};

/**
 * Whether the screen may state that an adapter has no live readings.
 *
 * Empty maps before the first sample mean "not measured yet", so the claim is
 * only allowed once some adapter has reported and this one still has not — in
 * any of the maps, since a fan speed alone is still a live reading.
 */
export const hasNoLiveGpuReadings = (
  gpuId: string | undefined,
  live: GpuLiveMaps,
) => {
  if (gpuId == null) {
    return false;
  }

  const maps = liveMapsInOrder(live);
  const anyAdapterReported = maps.some((map) => Object.keys(map).length > 0);

  return anyAdapterReported && !maps.some((map) => Object.hasOwn(map, gpuId));
};
