import { describe, expect, it } from "vitest";

/**
 * "Which GPU is effective" must be answered in exactly one place. Rounds 5-8
 * of the PR #1944 review were one mechanism repeating: a surface re-derived
 * the selection from `selectedGpuIdAtom` on its own and drifted from the
 * others, labelling one adapter while rendering another's values.
 *
 * This test enumerates the files allowed to touch the atom. A new surface
 * that needs the selection imports `useGpuAdapters` (or the derived atoms in
 * `store/chart.ts`) instead of joining the atom itself. Extending the
 * allowlist is a deliberate act reviewed with `verify-identity-contracts`.
 */
const ALLOWED_CONSUMERS = new Set([
  // Defines the atom and the derived resolution every read-only surface uses.
  "/src/features/hardware/store/chart.ts",
  // The one resolution owner components consume.
  "/src/features/hardware/hooks/useGpuAdapters.ts",
  // Restores, persists, and migrates the stored intent.
  "/src/features/hardware/hooks/useSelectedGpuPersistence.ts",
  // Writes the auto-selection when nothing is selected yet.
  "/src/features/hardware/hooks/useHardwareEventListener.ts",
  // The classic card resolves through findInventoryGpu/toLiveGpuId.
  "/src/features/hardware/dashboard/components/DashboardItems.tsx",
]);

const sources = import.meta.glob("/src/**/*.{ts,tsx}", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

const importsAtom = (source: string) =>
  /import[^;]*\bselectedGpuIdAtom\b[^;]*from/s.test(source);

describe("selectedGpuIdAtom ownership", () => {
  it("is consumed only by the allowlisted resolution owners", () => {
    const offenders = Object.entries(sources)
      .filter(([path]) => !/\.test\.tsx?$/.test(path))
      .filter(([, source]) => importsAtom(source))
      .map(([path]) => path)
      .filter((path) => !ALLOWED_CONSUMERS.has(path));

    expect(offenders).toEqual([]);
  });

  it("keeps every allowlisted consumer real, so the list cannot rot", () => {
    for (const path of ALLOWED_CONSUMERS) {
      const source = sources[path];
      expect(source, `${path} does not exist`).toBeDefined();
      const touchesAtom =
        importsAtom(source) || /export const selectedGpuIdAtom\b/.test(source);
      expect(
        touchesAtom,
        `${path} no longer defines or imports selectedGpuIdAtom; remove it from the allowlist`,
      ).toBe(true);
    }
  });
});
