# Group Shipped Components Under One Source Root

Status: accepted

Implementation status: all four migration slices land in
[#2769](https://github.com/shm11C3/HardwareVisualizer/pull/2769). The Context
section describes the layout before that change.

## Context

The repository root mixes the four components that ship in the product with
everything that does not:

- `src/` (Frontend), `src-tauri/` (Tauri App, crate `hardware_visualizer`),
  `core/` (Core, crate `hardviz-core`) and `sensor-service/` (crate
  `hardviz-sensor-service`) ship.
- `docs/`, `scripts/`, `infra/`, `e2e/`, `e2e-native/` and `perf-monitor/`
  are tests, tooling and records. Next to them sit about thirty tool and
  configuration files.

Nothing in the layout says which directories are product source. `src/` reads
as "the source" of the repository although it holds only the Frontend, and
the three Rust components are siblings of developer tools such as
`perf-monitor/`. Contributors and agents have to learn the split from
`AGENTS.md` instead of reading it from the tree.

The architecture vocabulary already names the components: Frontend, App
(the Tauri App boundary, [ADR 0002](0002-core-app-split.md)) and Core. The
directory names do not use those terms.

## Decision

1. **All shipped components live under `hardviz/`.** Nothing outside
   `hardviz/` is compiled into a release artifact. Tests, tooling, CI and
   records stay outside it.

   ```text
   hardviz/
   ├── frontend/        # Frontend (today src/, index.html, Vite/TS/shadcn config)
   ├── app/             # Tauri App, crate hardware_visualizer (today src-tauri/)
   ├── core/            # Core, crate hardviz-core (today core/)
   └── sensor-service/  # Windows sensor service (today sensor-service/)
   ```

2. **Directory names follow the architecture terms.** `hardviz/app/` is the
   App of the Core / App split and `hardviz/core/` is Core. The Tauri crate
   directory is not named `src-tauri/` or `tauri/`: the `src-` prefix carries
   no meaning inside a source root, and "App" is the term every architecture
   document already uses for this boundary.
3. **Each component directory is a self-contained project root.** Its
   manifest and configuration sit at its top and its source in `src/`, as
   the Rust crates already do. The Frontend gets the same shape: `index.html`,
   `vite.config.ts`, `vitest.config.ts`, `tsconfig*.json` and `components.json`
   move into `hardviz/frontend/` with the code in `hardviz/frontend/src/`.
   The root `package.json` stays the single npm project; it also hosts
   repository tooling scripts and drives the Frontend through explicit paths.
4. **Repository-wide roots stay at the root.** The Cargo workspace
   (`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`), `package.json` and its
   lockfile, Biome, Git and GitHub configuration, and community files are
   tool or platform conventions and keep their location. `target/` stays the
   workspace target directory.
5. **Scoped guidance moves with its component.** `src/AGENTS.md`,
   `src-tauri/AGENTS.md` and `core/AGENTS.md` move into their component
   directories, and every `.agents/rules` scope, guidance checker path, CI
   change filter, CodeQL path and generated-file guard follows them in the
   same change.
6. **Historical records keep their paths.** Accepted ADRs and dated evidence
   describe the tree as it was. Current docs, owner READMEs, rules, lesson
   `canonical_refs` and verification commands are updated.

## Migration

Each slice is one PR that leaves CI green. Merge or rebase open PRs that touch
a moving directory before its slice.

1. `core/` and `sensor-service/` to `hardviz/`: Cargo workspace members,
   crate-relative paths, `sensor-service/build-bundle.ps1` repository-root
   resolution, WiX and MSI checks.
2. `src-tauri/` to `hardviz/app/`: `tauri.conf.json` `frontendDist` and
   `beforeDevCommand`/`beforeBuildCommand` working directory, the
   `export_bindings` output path, `include_bytes!` paths, Tauri CLI discovery
   of the config from the repository root, `publish.yml` and `tauri-action`
   project path.
3. The Frontend to `hardviz/frontend/`: Vite root and config path, the `@`
   alias, `tsconfig` includes, Vitest, shadcn aliases, Biome scopes, the
   App's `export_bindings` output path and the generated `rspc/bindings.ts`
   guard in `.agents/tooling/agent-hook.mjs` (both point at the Frontend
   again after it moves), Playwright `webServer` commands, coverage paths.
4. Remaining single-owner root files: Playwright and E2E configs into
   `e2e/` and `e2e-native/`, `perf-thresholds.toml` into `perf-monitor/`,
   `tsconfig.ci-telemetry.json` into `.github/`, and the tray logo from
   `assets/` into `hardviz/app/`.

Every slice must verify, not assume, how its tool locates the moved
directory: Tauri CLI config discovery, `tauri-action`'s project path and the
`beforeDevCommand` working directory in slice 2, and the Vite, Vitest and
shadcn roots in slice 3.

## Alternatives Considered

- **`crates/core/` and `crates/sensor-service/`, Frontend and `src-tauri/`
  unchanged.** Groups the Rust crates but leaves `src/` meaning "Frontend" at
  the root, which is the confusion this record addresses.
- **`apps/desktop/` for Frontend and Tauri, `crates/` for the rest.** Common
  in JavaScript monorepos, but it splits shipped code across two top-level
  directories, and `apps/` suggests several applications where there is one.
- **`app/` as the source root.** Rejected because "App" already means the
  Tauri App boundary; `app/core/` would read as Core belonging to App, the
  opposite of [ADR 0002](0002-core-app-split.md).
- **`src/` as the source root (`src/frontend/`, `src/app/`, ...).** The usual
  meaning of `src/`, but it changes what an existing path means: every
  historical reference to `src/...` would silently point at the wrong
  component, and the tree would be ambiguous during migration.
- **Other names for the source root.** `product/` states the intent but is an
  unusual directory name, `hardware-visualizer/` is long, `desktop/` suggests
  UI-only code although the sensor service has none, and `hv/` is cryptic.
  `components/`, `platform/` and `runtime/` collide with existing terms
  (External Component Setup, the Platform Layer, runtime sensor gates).
  `hardviz` is already the short name of the component crates.
- **Keep the layout and document it.** Costs nothing, but the tree keeps
  contradicting the architecture vocabulary that documentation has to
  explain.

## Consequences

- The root lists one product directory, so "is this shipped code?" is a path
  check.
- Paths diverge from Tauri's default `src-tauri/` layout, so commands from
  Tauri documentation need the App path or must run where the Tauri CLI
  finds the config.
- Links to branch paths in issues, PRs and external pages break; links pinned
  to a commit keep working. `git log --follow` and rename detection keep file
  history.
- Every open branch that touches a moved directory needs a rebase, which
  the slice order is meant to bound.
