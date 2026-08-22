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

const ATOM = /\bselectedGpuIdAtom\b/;

/**
 * Blank out comments and literals so only code remains.
 *
 * Written as a scanner rather than a pair of regexes: stripping `//` to the
 * end of the line silently eats a real reference that follows a URL on the
 * same line, which turns this guard into a bypass instead of a check. The
 * TypeScript 7 parser lives behind `unstable/` entry points, so depending on
 * it here would trade one fragility for another.
 */
const codeOnly = (source: string) => {
  let out = "";
  let i = 0;
  while (i < source.length) {
    const two = source.slice(i, i + 2);
    if (two === "//") {
      while (i < source.length && source[i] !== "\n") i += 1;
      continue;
    }
    if (two === "/*") {
      i += 2;
      while (i < source.length && source.slice(i, i + 2) !== "*/") i += 1;
      i += 2;
      continue;
    }
    const ch = source[i];
    if (ch === '"' || ch === "'" || ch === "`") {
      const quote = ch;
      i += 1;
      while (i < source.length && source[i] !== quote) {
        i += source[i] === "\\" ? 2 : 1;
      }
      i += 1;
      continue;
    }
    out += ch;
    i += 1;
  }
  return out;
};

/**
 * Whether the file actually references the atom.
 *
 * Any reference counts, not just a named import: a file can reach the atom
 * through a namespace import (`import * as chart` then
 * `chart.selectedGpuIdAtom`), and member access still contains the name.
 * Comments and strings name it without consuming it — `gpuIdentity.ts`
 * documents the contract in prose — so they are removed first.
 */
const consumesAtom = (source: string) => ATOM.test(codeOnly(source));

describe("codeOnly", () => {
  // The case that motivated the scanner: a naive line-comment regex eats the
  // rest of the line after a URL's `//`, hiding a real reference — a silent
  // bypass, worse than a false positive.
  it("keeps a reference that follows a URL on the same line", () => {
    expect(
      codeOnly('const u = "https://x/a"; use(selectedGpuIdAtom);'),
    ).toContain("selectedGpuIdAtom");
  });

  it.each([
    ["line comment", "// selectedGpuIdAtom"],
    ["block comment", "/* selectedGpuIdAtom */"],
    ["string literal", 'const s = "selectedGpuIdAtom";'],
    ["template literal", "const s = `selectedGpuIdAtom`;"],
  ])("drops the name inside a %s", (_kind, source) => {
    expect(codeOnly(source)).not.toContain("selectedGpuIdAtom");
  });
});

describe("selectedGpuIdAtom ownership", () => {
  it("is consumed only by the allowlisted resolution owners", () => {
    const offenders = Object.entries(sources)
      .filter(([path]) => !/\.test\.tsx?$/.test(path))
      .filter(([, source]) => consumesAtom(source))
      .map(([path]) => path)
      .filter((path) => !ALLOWED_CONSUMERS.has(path));

    expect(offenders).toEqual([]);
  });

  it("keeps every allowlisted consumer real, so the list cannot rot", () => {
    for (const path of ALLOWED_CONSUMERS) {
      const source = sources[path];
      expect(source, `${path} does not exist`).toBeDefined();
      const touchesAtom = consumesAtom(source);
      expect(
        touchesAtom,
        `${path} no longer defines or imports selectedGpuIdAtom; remove it from the allowlist`,
      ).toBe(true);
    }
  });
});
