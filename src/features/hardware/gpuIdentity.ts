declare const liveGpuIdBrand: unique symbol;

/**
 * An id from the monitor stream — the namespace every live map and the
 * shared selection are keyed by.
 *
 * The inventory (`GraphicInfo.id`) uses a different namespace on every
 * platform (see ADR 0016), so it is deliberately NOT this type: the compiler
 * rejects writing an inventory id into the selection or indexing a live map
 * with one — the bug class behind most of PR #1944's review rounds.
 */
export type LiveGpuId = string & { readonly [liveGpuIdBrand]: true };

/**
 * Mint a `LiveGpuId`. Only the monitor payload (`useHardwareEventListener`)
 * and test seeds may call this. An id that is not known to come from the
 * stream (an inventory id, a stored id) is an `unresolved` selection instead;
 * resolve it by lookup through `findLiveGpuId` or `findNamedLiveGpuId`.
 */
export const asLiveGpuId = (id: string): LiveGpuId => id as LiveGpuId;

/**
 * The shared GPU selection: either an adapter the live stream names, or an
 * intent that no live map can resolve yet.
 *
 * An intent is unresolved when it comes from outside the stream: the classic
 * card clicked before the first sample or when its name join is ambiguous,
 * and a stored id restored at startup (shipped versions wrote inventory ids
 * there). Keeping it out of `LiveGpuId` means the compiler rejects indexing a
 * live map with it. Only `getEffectiveGpuId`, `findInventoryGpu`, and the
 * persistence migration read the `unresolved` arm; the migration promotes it
 * to `live` once the stream names the adapter.
 */
export type GpuSelection =
  | { kind: "live"; id: LiveGpuId }
  | { kind: "unresolved"; id: string };

export const liveGpuSelection = (id: LiveGpuId): GpuSelection => ({
  kind: "live",
  id,
});

export const unresolvedGpuSelection = (id: string): GpuSelection => ({
  kind: "unresolved",
  id,
});

/**
 * Build a live-keyed map from entries whose keys are already minted.
 *
 * `Object.fromEntries` returns a `string`-indexed object, and TypeScript
 * accepts that for a `Record<LiveGpuId, T>` — so constructing a live map
 * wholesale would otherwise bypass the brand entirely and let an
 * inventory-keyed dictionary into the atoms. Indexing is checked; only
 * construction needed this gate.
 */
export const liveGpuRecord = <T>(
  entries: readonly (readonly [LiveGpuId, T])[],
): Record<LiveGpuId, T> => Object.fromEntries(entries) as Record<LiveGpuId, T>;

/**
 * One adapter as the Performance screens need it: the stable id every live
 * value is keyed by, the name the platform reported, and a short label for
 * controls that cannot fit the full name.
 */
export type GpuAdapter = {
  id: LiveGpuId;
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
  usageHistories: Record<LiveGpuId, (number | null)[]>;
  temperatures: Record<LiveGpuId, { name: string; value: number }>;
  fanSpeeds: Record<LiveGpuId, { name: string; value: number }>;
  dedicatedMemoryKb: Record<LiveGpuId, number | null>;
};

/** `Object.keys` erases the brand; these two restate what the maps guarantee. */
const liveKeys = (map: Record<LiveGpuId, unknown>) =>
  Object.keys(map) as LiveGpuId[];
const liveEntries = <T>(map: Record<LiveGpuId, T>) =>
  Object.entries(map) as [LiveGpuId, T][];

const liveMapsInOrder = (live: GpuLiveMaps) => [
  live.usageHistories,
  live.temperatures,
  live.fanSpeeds,
  live.dedicatedMemoryKb,
];

const hasCurrentGpuReading = (gpuId: LiveGpuId, live: GpuLiveMaps) =>
  live.usageHistories[gpuId]?.at(-1) != null ||
  live.temperatures[gpuId] != null ||
  live.fanSpeeds[gpuId] != null ||
  live.dedicatedMemoryKb[gpuId] != null;

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
  gpuNames: Record<LiveGpuId, string>,
  live: GpuLiveMaps,
): GpuAdapter[] => {
  const named = new Map<LiveGpuId, string>();

  // Every adapter the stream named, in payload order — including one that
  // named itself and reported nothing else, which is the case the unavailable
  // state exists for.
  for (const [id, name] of liveEntries(gpuNames)) {
    named.set(id, name);
  }
  for (const map of liveMapsInOrder(live)) {
    for (const id of liveKeys(map)) {
      if (!named.has(id)) {
        named.set(id, id);
      }
    }
  }
  // The sensor maps carry a name too, so they can still identify an adapter
  // if the name map has not caught up with them.
  for (const map of [live.temperatures, live.fanSpeeds]) {
    for (const [id, value] of liveEntries(map)) {
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
 * The live id the stream named under exactly this raw id, if any.
 *
 * A lookup rather than a cast: the value returned is the key the stream
 * minted, so resolving a stored id never manufactures a live id the stream
 * did not report. A stored id written from a live selection resolves this way
 * on the first sample after a restart.
 */
export const findNamedLiveGpuId = (
  rawId: string,
  gpuNames: Record<LiveGpuId, string>,
): LiveGpuId | undefined => liveKeys(gpuNames).find((id) => id === rawId);

/**
 * The inventory entry a selection describes.
 *
 * `selectedGpuIdAtom` is shared by every GPU surface, but the inventory and
 * the monitor stream use different id namespaces (see `gpuNamesAtom`), so a
 * live id — which is what the Performance selector and the event listener's
 * auto-selection both write — never matches an inventory id directly. Joining
 * on the name both sources report is what keeps the classic card from pairing
 * one adapter's name with another adapter's readings.
 *
 * An unresolved selection can only match an inventory id directly: it names
 * no live adapter, so there is no live name to join on.
 *
 * The join is refused when the name picks out more than one entry; the caller
 * then falls back rather than guessing.
 */
export const findInventoryGpu = <T extends { id: string; name: string }>(
  gpus: readonly T[],
  gpuNames: Record<LiveGpuId, string>,
  selection: GpuSelection | null,
): T | undefined => {
  if (selection == null) {
    return undefined;
  }

  const byId = gpus.find((gpu) => gpu.id === selection.id);
  if (byId != null || selection.kind === "unresolved") {
    return byId;
  }

  const liveName = gpuNames[selection.id];
  if (liveName == null) {
    return undefined;
  }

  const matches = gpus.filter((gpu) => gpu.name === liveName);
  return matches.length === 1 ? matches[0] : undefined;
};

/**
 * The live id that names the same adapter as an inventory entry, joined on
 * the name both sources report. `undefined` when no live adapter reports that
 * name, or when more than one does: the name cannot pick one of them.
 */
export const findLiveGpuId = (
  gpu: { id: string; name: string },
  gpuNames: Record<LiveGpuId, string>,
): LiveGpuId | undefined => {
  const matches = liveEntries(gpuNames).filter(([, name]) => name === gpu.name);
  return matches.length === 1 ? matches[0][0] : undefined;
};

/**
 * The selection a surface holding inventory entries writes for one of them,
 * in the namespace every reading is keyed by.
 *
 * When the join fails, the click is kept as unresolved intent rather than
 * discarded: the persistence migration resolves it once the stream names the
 * adapter.
 */
export const selectionForInventoryGpu = (
  gpu: { id: string; name: string },
  gpuNames: Record<LiveGpuId, string>,
): GpuSelection => {
  const liveId = findLiveGpuId(gpu, gpuNames);
  return liveId != null
    ? liveGpuSelection(liveId)
    : unresolvedGpuSelection(gpu.id);
};

const CURRENT_PDH_GPU_ID = /^pdh:(?:instance:.+|-?\d+:\d+)$/;

/**
 * Translate the pre-LUID Intel PDH id when the current live stream can name
 * exactly one matching PDH adapter. An old name cannot identify either
 * adapter when two same-model devices exist, so ambiguity deliberately keeps
 * the stored intent unresolved instead of guessing.
 */
export const migrateLegacyPdhGpuId = (
  storedGpuId: string,
  gpuNames: Record<LiveGpuId, string>,
): LiveGpuId | undefined => {
  if (!storedGpuId.startsWith("pdh:") || CURRENT_PDH_GPU_ID.test(storedGpuId)) {
    return undefined;
  }

  const legacyName = storedGpuId.slice("pdh:".length);
  const matches = liveEntries(gpuNames).filter(
    ([id, name]) => CURRENT_PDH_GPU_ID.test(id) && name === legacyName,
  );
  return matches.length === 1 ? matches[0][0] : undefined;
};

/**
 * The live id the stream currently reports for a selection, found by lookup
 * among the detected ids and the live map keys. An unresolved selection
 * resolves here only when the stream reports that exact id, so the value
 * returned is always one the stream minted.
 */
const findReportedLiveId = (
  selection: GpuSelection,
  live: GpuLiveMaps,
  detectedGpuIds: readonly LiveGpuId[],
): LiveGpuId | undefined => {
  const isSelected = (id: LiveGpuId) => id === selection.id;
  const detected = detectedGpuIds.find(isSelected);
  if (detected != null) {
    return detected;
  }
  for (const map of liveMapsInOrder(live)) {
    const keyed = liveKeys(map).find(isSelected);
    if (keyed != null) {
      return keyed;
    }
  }
  return undefined;
};

/**
 * The GPU both Performance views agree on.
 *
 * An explicit selection wins while the adapter is still detected, even when it
 * reports nothing: the honest answer there is "this adapter has no readings",
 * not another adapter's numbers. Only a selection pointing at an adapter that
 * no longer exists, or an unresolved intent the stream does not report under
 * that id, falls back to the first GPU that reports anything.
 */
export const getEffectiveGpuId = (
  selection: GpuSelection | null,
  live: GpuLiveMaps,
  detectedGpuIds: readonly LiveGpuId[] = [],
): LiveGpuId | undefined => {
  const selected =
    selection != null
      ? findReportedLiveId(selection, live, detectedGpuIds)
      : undefined;
  if (selected != null) {
    return selected;
  }

  for (const map of liveMapsInOrder(live)) {
    for (const gpuId of liveKeys(map)) {
      if (hasCurrentGpuReading(gpuId, live)) {
        return gpuId;
      }
    }
  }
  return undefined;
};

/**
 * Whether the screen may state that an adapter has no live readings.
 *
 * Empty maps before the first sample mean "not measured yet", so the claim is
 * only allowed once sampling has actually happened and this adapter still has
 * no value — in any of the maps, since a fan speed alone is still a live
 * reading.
 *
 * A detected adapter is itself proof that a sample arrived: a machine whose
 * only GPU reports its name and nothing else leaves every value map empty
 * forever, and treating that as "not measured yet" would suppress the
 * explanation for exactly the user who most needs it.
 */
export const hasNoLiveGpuReadings = (
  gpuId: LiveGpuId | undefined,
  live: GpuLiveMaps,
  detectedGpuIds: readonly LiveGpuId[] = [],
) => {
  if (gpuId == null) {
    return false;
  }

  const maps = liveMapsInOrder(live);
  const sampled =
    detectedGpuIds.length > 0 || maps.some((map) => liveKeys(map).length > 0);

  return sampled && !hasCurrentGpuReading(gpuId, live);
};
