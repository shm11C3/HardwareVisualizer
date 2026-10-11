# Frontend Architecture Design

Decision: [ADR 0027](../adr/0027-frontend-layer-boundaries-and-state-ownership.md)
(accepted). Migration tracking: [#2325](https://github.com/shm11c3/HardwareVisualizer/issues/2325).

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
| Shared components | `src/components/charts`, `src/components/*.tsx` | All of the above. Charts never import shared components. |
| Feature | `src/features/<f>` | All of the above and its own feature. Never another feature. |
| App | `src/app`, `src/main*.tsx` | Anything |

The root directories (`types`, `consts`, `lib`, `store`, `hooks`, `components`)
are the shared layer, organized by kind. Grouping by concern happens in
sub-folders inside them (for example `src/hooks/window`), never as new
top-level directories, so the path scopes above stay fixed.

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
  `src/hooks/settings/useSettingsAtom.ts`. `src/features/<f>/store` holds state owned by
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
| No OS error dialogs from the hardware feature | `paths["@/hooks/tauri/useTauriDialog"]` banned in the `features/hardware` overrides |
| No module-scope `let` outside store modules | GritQL plugin through an override |
| No cycles, barrels, or `export *` | `noImportCycles`, `noBarrelFile`, `noReExportAll` |
| File name matches its export: components PascalCase `.tsx`, other modules camelCase `.ts`, `src/components/ui` kebab-case, tests either of the two | `useFilenamingConvention` overrides in root `biome.jsonc` |

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

A target constraint that the code still violates is recorded in the boundary
file as a pending exception tagged with the #2325 slice that removes it.
Negated patterns (`!@/features/settings/**`) exempt one dependency, and
commented-out entries disable one API or pattern.

No pending exceptions remain as of the state-ownership slice (S5): every
constraint in the boundary file is enforced.

These rules keep the pending list honest:

- The list shrinks only. Each migration slice deletes the exceptions it
  resolves, and it is empty now.
- New code that needs a new exception is a design discussion, not a config
  edit.
- The app override covers `src/app/**` and must stay last, so the app layer
  keeps its own rules wherever other scopes overlap it.

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

## Failure Reporting

A failed read or action is reported on two axes. Blast radius decides the
surface; who started the work decides when it may interrupt.

| Tier | Surface | When | Examples |
| --- | --- | --- | --- |
| App cannot continue | Blocking dialog (`useTauriDialog`, backend `error_event`) | Immediately | Database cannot open, conversion failed |
| One panel's read | Failure state where the data would appear, with retry | Never interrupts; automatic refreshes keep retrying | Insight charts, Cooling panels, Storage Health, process table |
| Secondary lane | Nothing in the UI; log only | Never | Fan and ambient lanes next to a primary series |
| User action failed | Near the control that was used | Immediately, once | Settings toggles, restart, tray actions |

Rules that follow from the axes:

- A read hook exposes `hasError` (and `hasLoaded` when the panel needs to
  tell "not yet" from "empty"), and a `retry` function. It never opens a
  dialog. The panel renders the shared failure component
  (`src/components/LoadFailure.tsx`) in place of the data.
- A failed read is a distinct state from an empty result (DP-02). Empty data
  renders the panel's empty state; a failure renders the failure state.
- Failure copy is translated and does not embed Rust error strings. The
  technical detail goes to `console.error` and to the App's log at `warn`
  with the query arguments, which is what makes a report reproducible.
- Once a failure is a state, interval refreshes cannot stack notices, so no
  per-streak dedup is needed for the UI. Logging may dedup per streak.
- The failure state is only for a failed query. Domain states the Cooling
  Insight already models (establishing baseline, recording coverage, sensor
  unsupported) stay their own states and are never rendered as failures.

Enforcement: `features/hardware` may not import `useTauriDialog`
(`noRestrictedImports` in its three overrides). The user-action tier is
review-checked; it has no in-app notice primitive yet, and choosing one is a
separate decision.

### Call-site classification

| Call site | Tier | Status |
| --- | --- | --- |
| Insight main chart hook, Cooling hooks, process stats, snapshot | One panel's read | Done in #2311 |
| GPU archive names, hardware inventory, process list polling | One panel's read, or log only when the screen renders without them | Done in #2311 |
| Dashboard Storage Health reads | One panel's read | Done in #2311 |
| Settings toggles and actions (`useSettingsAtom`, tray, autostart, elevation) | User action failed | Pending: needs a near-control notice; tracked in #2388 |
| License page reads | One panel's read | Pending: tracked in #2388 |
| Startup dialogs in `src/app` | Already inside a dialog flow | Review-checked |
| Backend `error_event` (`useErrorModalListener`) | App cannot continue | Unchanged |

## Migration

The migration is complete. It landed as five stacked slices tracked in #2325:

| Slice | Change | PR |
| --- | --- | --- |
| S1 | `@/` alias instead of parent-relative imports | #2374 |
| S2 | Shared chart vocabulary in foundation; generated settings types | #2375 |
| S3 | Application Preference state in `src/store` and `src/hooks` | #2376 |
| S4 | `src/app/` composition layer | #2377 |
| S5 | Atoms only in store modules; `chart.ts` split by subject | #2380 |

A future structural change to these boundaries follows the same pattern:
- Record a pending exception only for code that already exists.
- Remove the exception in the change that fixes that code.
- Run `npm run lint:ci`, `npm run build`, and `npm test` before merging.
- If the change alters live event fan-out, add the focused regression test
  that `src/AGENTS.md` requires.

## Open Questions

- Whether presentation components may call IPC `commands` directly, or only
  hooks may. Today only UI primitives are restricted. Read hooks now own the
  failure state, which pulls reads into hooks; the remaining question is
  operation commands called from components.
- Which in-app notice surface the user-action tier uses. The repository has
  no toast primitive; adding one is a dependency decision.
