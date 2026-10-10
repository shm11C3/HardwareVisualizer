---
id: LRN-20261003-prove-biome-boundary-config-with-canaries
status: promoted
cause_status: confirmed
scope: biome.jsonc, .config/biome/**
trigger: when adding or editing Biome overrides, noRestrictedImports scopes, or GritQL plugins
failure_signature: a boundary rule reports zero diagnostics because it never ran; overlapping overrides silently drop earlier constraints
root_cause: "a later Biome override replaces, rather than merges, the options of a rule configured earlier for the same file; a mistyped scope, a plugin path that does not resolve the way the author expected, or a GritQL snippet that never matches produces no error, only no diagnostics"
guardrail: the boundary file keeps disjoint scopes with complete constraint sets and documents the replacement semantics; every edit is proven with per-scope canary imports that must be rejected or allowed
canonical_refs: .config/biome/frontend-boundaries.jsonc, docs/design/frontend-architecture.md, .agents/rules/frontend.md
verification: two overrides matching the same file with different noRestrictedImports options report only the later one's diagnostics; the 2026-10-03 canary run passed 107 expected-reject and expected-allow checks across every scope
evidence: "issue #2325; ADR 0027; experiments recorded in docs/design/frontend-architecture.md (Biome behaviors section)"
revalidate_when: Biome changes override merging, noRestrictedImports pattern matching, plugin path resolution in extended configs, or GritQL snippet matching for `let`
---

# Prove Biome boundary config with canaries

## Observation

Designing the frontend layer rules, several configurations reported zero
diagnostics for reasons unrelated to the code being clean:

- An override listed earlier stopped producing diagnostics once a broader
  override also configured `noRestrictedImports` for the same files.
- A GritQL snippet `let $x = $y` matched nothing, while `const $x = $y`
  matched.
- A plugin path written relative to the extended config failed to load. The
  same path written relative to the repository root loaded.
- Counting violations from `biome lint` output undercounted, because only 20
  diagnostics print by default.

## Confirmed cause

Biome applies the last matching override's options for a rule. It does not
combine them with earlier overrides. A configuration mistake never fails
loudly as a rule error. It shows up as an absence of diagnostics, which looks
exactly like compliant code.

## Promotion

The Design Doc records the verified behaviors and the canary procedure. The
boundary file's header states the replacement rule and the path resolution.
The frontend rule requires canaries for any edit to that file.
