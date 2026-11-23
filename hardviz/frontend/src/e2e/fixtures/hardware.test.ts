import { describe, expect, it } from "vitest";
import { GPU_FIXTURES } from "./hardware";

/**
 * A fixture may share one id across two sources only if production does — and
 * no platform does: the inventory and the monitor stream key GPUs in
 * different namespaces everywhere (ADR 0016). A fixture that flattens that
 * split certifies cross-namespace joins that fail on real hardware; the
 * classic GPU selector shipped non-functional exactly this way while its e2e
 * passed.
 */
describe("GPU fixture id namespaces", () => {
  it("keeps the inventory id and the live id distinct, as every platform does", () => {
    for (const gpu of GPU_FIXTURES) {
      expect(gpu.id).not.toBe(gpu.liveId);
    }
  });

  it("keeps ids unique within each namespace", () => {
    const inventoryIds: string[] = GPU_FIXTURES.map((gpu) => gpu.id);
    const liveIds: string[] = GPU_FIXTURES.map((gpu) => gpu.liveId);
    expect(new Set(inventoryIds).size).toBe(inventoryIds.length);
    expect(new Set(liveIds).size).toBe(liveIds.length);
    // The namespaces must not overlap either: an id that exists on both
    // sides would let an id-based join succeed by accident in tests.
    expect(inventoryIds.filter((id) => liveIds.includes(id))).toEqual([]);
  });
});
