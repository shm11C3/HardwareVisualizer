# Frontend Layer Boundaries and State Ownership

Status: accepted

Tracking issue: [#2325](https://github.com/shm11c3/HardwareVisualizer/issues/2325).

The migration landed in five slices: #2374, #2375, #2376, #2377, and #2380.
The Biome boundary configuration has no pending exceptions left.

Biome checks the following rules:
- import direction and the ban on cross-feature imports;
- alias-only cross-directory imports;
- where each Jotai API may be used;
- module-scope `let` outside store modules.

Review still has to check the parts lint cannot see:
- where Application Preference state lives (decision 4);
- shared mutable state held in a module-scope `const`, such as a `Map`;
- the permanent exception for the Tauri Store handle cache in
  `src/lib/tauriStore.ts`.

Implementation detail and the rule-to-lint mapping live in
[`docs/design/frontend-architecture.md`](../design/frontend-architecture.md).

## Context

The frontend grew without an enforced dependency direction. Generic layers
depend on features: shared components, charts, and shared hooks import
`features/settings`. Features also depend on one another:
`features/hardware` and `features/settings` import each other.

Jotai state has no single home. Atoms are declared in hook files as well as in
store modules. Components write atoms directly, while other state hides behind
facade hooks. Some shared state sits in module-scope variables that a
Provider-scoped store cannot isolate between tests.

Two placements cause most of the cross-layer imports:

- Application Preference state lives inside `features/settings`, although every
  screen reads it.
- Chart vocabulary lives inside `features/hardware`, although charts and
  settings share it.

## Decision

1. **The frontend has ordered layers, and imports point only downward:**
   foundation (`rspc`, `types`, `consts`, `lib`) → store → shared hooks →
   UI primitives → shared components → features → app.
   - A feature never imports another feature.
   - Code that composes features belongs in the app layer.
2. **Cross-directory imports use the `@/` alias.** Parent-relative imports are
   not allowed, because they bypass path-based boundary rules.
3. **State ownership is explicit:**
   - Atoms are declared only in `store/` modules: `src/store` for cross-feature
     state and `src/features/<f>/store` for feature state.
   - Components read atoms. Writes go through named hook functions.
   - Shared mutable state does not live in module-scope variables outside
     store modules.
   - `getDefaultStore` is not used.
4. **Application Preference state is cross-feature state.** It lives in the
   shared store and hook layers, not inside the settings feature.
5. **Biome enforces these rules.**
   - A constraint is enforced as an error as soon as the code satisfies it.
   - A constraint that still has violations is recorded as a pending exception
     with the migration slice that removes it. It is never weakened silently.

## Alternatives Considered

- **Document the layering without lint.** Rejected. The current state is what
  convention-only layering produced. A rule nobody checks drifts the same way
  again.
- **Use dependency-cruiser or eslint-plugin-boundaries.** Rejected for now.
  Either one adds a second lint toolchain beside Biome. Biome's path rules
  plus the alias-only import rule cover the same layer and feature
  boundaries, and keep one lint command.
- **Keep atoms next to the hooks that use them.** Rejected. Co-location hides
  shared state inside hooks that other features then import for the atom
  alone. It also leaves no rule that tells a reader where state lives.
- **Migrate everything in one change.** Rejected. The migration touches most
  frontend files and would conflict with every in-flight change. It lands as
  ordered slices instead, each of which restores specific pending constraints.

## Consequences

- New code cannot add an import that crosses an enforced boundary, a barrel
  file, an import cycle, or module-scope `let` outside store modules.
- During the migration, constraints with pending exceptions stayed
  unenforced until their slice landed. None remain; a new exception is a
  design discussion, not a configuration edit.
- Biome replaces, rather than merges, a rule's options when a later override
  matches the same file. The boundary configuration therefore needs disjoint
  scopes and a complete constraint set in each override.
- The Live Metrics Buffer (#1638) has to live in a store module to satisfy the
  module-state rule. Its module-level store fits that placement.
