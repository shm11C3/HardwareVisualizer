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
]);

const sources = import.meta.glob("/src/**/*.{ts,tsx}", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

// `$` is not a word character, so a bare \b would let the distinct
// identifier `$selectedGpuIdAtom` count as a reference.
const ATOM = /(?<![$\w])selectedGpuIdAtom\b/;

/**
 * Blank out comments, strings, templates, and regex literals so only code
 * remains.
 *
 * A lexer over JavaScript's five literal token classes, because anything less
 * has a bypass: a `//` inside a URL string eats the rest of the line, a quote
 * inside a regex swallows the rest of the file, and blanking a whole template
 * hides a real reference inside its `${}`. The classes are finite, so this is
 * written once and pinned by tests rather than patched per counterexample.
 * Whether `/` starts a regex or is division uses the standard lexer
 * heuristic: a regex can only follow an operator, opener, keyword boundary,
 * or start of input.
 */
const codeOnly = (source: string): string => {
  let out = "";
  let i = 0;
  let lastCode = "";
  const remember = (ch: string) => {
    if (!/\s/.test(ch)) {
      lastCode = ch;
    }
  };
  const regexCanStart = () =>
    lastCode === "" || "([{,;=:!&|?+-*%<>~^".includes(lastCode);

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
    if (ch === '"' || ch === "'") {
      i += 1;
      while (i < source.length && source[i] !== ch) {
        i += source[i] === "\\" ? 2 : 1;
      }
      i += 1;
      remember('"');
      continue;
    }
    if (ch === "`") {
      // Blank the text but recurse into ${}: interpolations are code, and a
      // real reference inside one must not be hidden.
      i += 1;
      while (i < source.length && source[i] !== "`") {
        if (source.slice(i, i + 2) === "${") {
          let depth = 1;
          const exprStart = i + 2;
          i += 2;
          while (i < source.length && depth > 0) {
            if (source[i] === "{") depth += 1;
            if (source[i] === "}") depth -= 1;
            i += 1;
          }
          out += ` ${codeOnly(source.slice(exprStart, i - 1))} `;
          continue;
        }
        i += source[i] === "\\" ? 2 : 1;
      }
      i += 1;
      remember('"');
      continue;
    }
    if (ch === "/" && regexCanStart()) {
      i += 1;
      let inClass = false;
      while (i < source.length && (inClass || source[i] !== "/")) {
        if (source[i] === "\\") i += 1;
        else if (source[i] === "[") inClass = true;
        else if (source[i] === "]") inClass = false;
        i += 1;
      }
      i += 1;
      remember('"');
      continue;
    }
    out += ch;
    remember(ch);
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
 * Comments and literals name it without consuming it — `gpuIdentity.ts`
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

  it("keeps a reference after a regex literal containing a quote", () => {
    expect(
      codeOnly(`const p = /["']/; use(chart.selectedGpuIdAtom);`),
    ).toContain("selectedGpuIdAtom");
  });

  it("keeps a reference inside a template interpolation", () => {
    // eslint-style template: the text is a literal, the ${} is code.
    // Concatenated so the ${} is data here, not a lint-visible placeholder.
    const source = "const s = `atom: $" + "{chart.selectedGpuIdAtom}`;";
    expect(codeOnly(source)).toContain("selectedGpuIdAtom");
  });

  it("does not count the distinct identifier $selectedGpuIdAtom", () => {
    expect(codeOnly("const $selectedGpuIdAtom = 1;")).toContain(
      "$selectedGpuIdAtom",
    );
    expect(ATOM.test(codeOnly("const $selectedGpuIdAtom = 1;"))).toBe(false);
  });

  it("treats division as code, not as a regex opener", () => {
    expect(codeOnly("const x = a / b; use(selectedGpuIdAtom);")).toContain(
      "selectedGpuIdAtom",
    );
  });

  it.each([
    ["line comment", "// selectedGpuIdAtom"],
    ["block comment", "/* selectedGpuIdAtom */"],
    ["string literal", 'const s = "selectedGpuIdAtom";'],
    ["template literal text", "const s = `selectedGpuIdAtom`;"],
    ["regex literal", "const p = /selectedGpuIdAtom/;"],
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
