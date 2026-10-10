# Frontend Architecture Design

Decision: [ADR 0027](../adr/0027-frontend-layer-boundaries-and-state-ownership.md)
(proposed). Migration tracking: [#2325](https://github.com/shm11c3/HardwareVisualizer/issues/2325).

## Problem

The frontend has no enforced dependency direction and no single owner for
Jotai state. Generic layers import features, features import each other, and
atoms are declared wherever they were first needed. Those are the conditions
under which a boundary erodes one convenient import at a time.

## Layers

A layer may import from the layers listed above it, never from the layers
listed below it.

| Layer | Paths | May import |
| --- | --- | --- |
| Foundation | `src/rspc`, `src/types`, `src/consts`, `src/lib` | Foundation only. No React, no Jotai. |
| Store | `src/store`, `src/features/<f>/store` | Foundation, plus the feature's own non-UI modules. No React or Jotai React bindings. |
| Shared hooks | `src/hooks` | Foundation, store |
| UI primitives | `src/components/ui`, `src/components/icons` | Foundation. Never IPC `commands` or `events`. No Jotai. |
| Shared components | `src/components/charts`, `src/components/shared` | All of the above. Charts never import shared components. |
| Feature | `src/features/<f>` | All of the above and its own feature. Never another feature. |
| App | `src/app` (planned), `src/App.tsx`, `src/lazyScreens.tsx`, `src/main*.tsx` | Anything |

Within a feature, `store/` and `hooks/` import their own feature through a
closed allow-list rather than an open exemption:

- `store/` may import the feature's `store`, `types`, `consts`, `utils`, and
  `funcs` modules.
- `hooks/` may import the same modules plus the feature's `hooks`.
- Feature root modules such as `hardware/gpuIdentity` are allowed by name.

An allow-list is used because specifiers carry no file extension, so a deny
pattern cannot tell a screen component from a logic module.

Two rules close the obvious bypasses:

- **No parent-relative imports.** Every import that leaves its own directory
  uses `@/`, because the path rules match the specifier as written.
- **No barrels and no `export *`.** Every dependency stays visible at the
  import site.

## State Ownership

| Where | Declare atoms | Write atoms | Read atoms |
| --- | --- | --- | --- |
| `store/` modules | yes | in derived atoms | yes |
| Hooks | no | yes | yes |
| Components and the app layer | no | no | `useAtomValue` only |

- `src/store` holds cross-feature state, including Application Preferences,
  which live in `src/store/settings.ts` with their hook in
  `src/hooks/useSettingsAtom.ts`. `src/features/<f>/store` holds state owned by
  one feature.
- `getDefaultStore` and `createStore` stay out of production code. State reaches
  React through the Provider-scoped store, which is what lets tests isolate
  each case with their own store.
- Module-scope `let` is allowed only in store modules, because only there is it
  visible as owned state. Anything shared between hook instances becomes an
  atom or store-keyed state, for example the `WeakMap` keyed by the Jotai store
  that `useProcessInfo` uses.
- The persistence boundary does not change. Application Preferences still
  persist through typed Rust settings commands, and Tauri Store remains only
  for UI-local state.

## Enforcement

Root `biome.jsonc` enables `noImportCycles`, `noBarrelFile`, and
`noReExportAll`. It extends `.config/biome/frontend-boundaries.jsonc`, which
holds one `noRestrictedImports` override per layer scope, plus the GritQL
plugin `.config/biome/no-module-scope-let.grit`.

| Rule | Mechanism |
| --- | --- |
| Downward-only imports, no cross-feature imports | `noRestrictedImports` pattern groups per scope |
| Alias-only cross-directory imports | `../**` in every scope's group |
| Jotai API placement | `paths.jotai.importNames` per scope, plus `jotai/**` subpaths banned outside store modules |
| UI primitives never call IPC | `paths["@/rspc/bindings"].importNames` = `commands`, `events` |
| No bypass through namespace imports | `*` is listed wherever specific names are restricted |
| No module-scope `let` outside store modules | GritQL plugin through an override |
| No cycles, barrels, or `export *` | `noImportCycles`, `noBarrelFile`, `noReExportAll` |

The restricted Jotai names follow jotai's root exports. Jotai 3 added
`useAtomValueRaw`, `useAtomValueRawSync`, and `INTERNAL_overrideCreateStore`,
and a new export is allowed until it is listed. Re-check the lists against
the package's exports whenever jotai is upgraded.

Test files are outside the boundary scopes. They legitimately seed a
`createStore` and import across layers to build fixtures.

`src/main*.tsx` and `src/e2e/**` are composition roots and test
infrastructure, so they are also unscoped.

Every feature has three overrides: UI, `hooks/`, and `store/`. A feature
directory with no overrides of its own falls into a catch-all that bans every
`@/features/**` import, including its own. A new feature therefore fails lint
until it gets its own overrides.

### Pending exceptions

A target constraint that the current code still violates is commented out in
the boundary file as `pending Sn (#2325)`. Negated patterns
(`!@/features/settings/**`) exempt one dependency, and commented-out entries
disable one API or pattern.

These rules keep the pending list honest:

- The list shrinks only. Each migration slice deletes the exceptions it
  resolves.
- New code that needs a new exception is a design discussion, not a config
  edit.
- Files that the app-layer slice will move are listed in the last override
  and get app-layer rules now, so their current directory's rules stay strict.

### Biome behaviors this relies on

These behaviors were verified on Biome 2.5.

- **A later override replaces the options of a rule configured earlier for the
  same file. It does not merge them.** Scopes must stay disjoint, every
  override must carry its complete set, and the app override must stay last.
- `noRestrictedImports` group patterns are gitignore-style and support
  negation (`!`).
- Plugin paths in an extended config resolve from the repository root.
- The snippet `let $x = $y` does not match in GritQL, while `const $x = $y`
  does. The plugin therefore matches `JsVariableDeclaration()` by source text.
- Plugin diagnostics are suppressed with `// biome-ignore lint/plugin: <reason>`.
- `biome lint` prints only 20 diagnostics by default. Use `--max-diagnostics`
  when counting.

Changes to the boundary file have to be proven against canaries, because a
mistyped scope does not fail. It silently disables the rule. To prove a
change, place one file per affected scope with one import that each
constraint must reject and one that it must allow, then confirm the
diagnostics line by line.

## Migration

The slices, their order, and the baseline measurements live in #2325. They are
a starting plan. Re-measure and re-slice when 1.12.0 work begins.

A slice is done when its pending exceptions are removed and `npm run lint:ci`,
`npm run build`, and `npm test` pass. A slice that changes 1 Hz atoms also
needs the fan-out regression test that `src/AGENTS.md` requires.

## Open Questions

- Whether presentation components may call IPC `commands` directly, or only
  hooks may. Today only UI primitives are restricted. #2311 changes how
  failed reads are reported and may settle this.
