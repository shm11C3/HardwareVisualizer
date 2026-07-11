# Backend Architecture

This document describes the current backend shape after the Core / App split.
The backend is now split across two Rust crates:

- `core/` (`hardviz-core`): Tauri-independent hardware collection, platform
  access, persistence primitives, settings consumed by Core, and shared data
  models.
- `src-tauri/` (`hardware_visualizer`): Tauri app boundary, commands, App-side
  services, event adapters, UI-only settings, lifecycle, plugins, and Tauri
  runtime wiring.

Use this document together with:

- [`core/README.md`](../../core/README.md)
- [`src-tauri/README.md`](../../src-tauri/README.md)

## High-Level Flow

The main dependency direction is:

```text
Frontend
  -> Tauri commands (`src-tauri/src/commands`)
  -> App services / Core APIs (`src-tauri/src/services`, `hardviz_core::*`)
  -> Core collector / platform / persistence (`core/src/*`)
  -> OS APIs, system providers, SQLite
```

Realtime sensor updates use a separate event flow:

```text
Core collector
  -> `hardviz_core::event_bus::EventBus`
  -> App adapters (`src-tauri/src/adapters`)
  -> Tauri events
  -> Frontend listeners
```

## Workspace Layout

```text
core/src/
├── collector/             # Sampling loop, history store, monitor controller
├── enums/                 # Core-level error, hardware, and settings enums
├── event_bus.rs           # In-process broadcast bus for MetricsSnapshot
├── infrastructure/
│   ├── database/          # Core DB initialization and access
│   └── providers/         # OS / vendor / system providers
├── models/                # Core data models, including MetricsSnapshot
├── monitoring/            # Monitoring state types
├── persistence/           # Archive, preflight, cleanup, storage health workers
├── platform/              # Platform traits, factory, and OS implementations
├── settings/              # Core-consumed settings subset
└── utils/                 # Core helpers and logging macros

src-tauri/src/
├── adapters/              # Core EventBus subscribers -> Tauri event/tray output
├── app/                   # App startup helpers
├── commands/              # Tauri command handlers and tauri-specta bindings
├── enums/                 # App-side enums exposed at the Tauri boundary
├── infrastructure/        # App-owned infrastructure, including SQL migrations
├── lifecycle.rs           # Window close / second instance / run event handling
├── models/                # App-side DTOs with specta/tauri-specta types
├── services/              # App-side business logic and Core API wrappers
├── tray/                  # Tray widget UI/runtime code
├── utils/                 # Tauri-aware helpers
└── workers/               # Handles for background controllers/adapters
```

## Crate Responsibilities

### `hardviz-core` (`core/`)

Core owns code that does not need a Tauri context:

- sampling CPU, memory, GPU, process, and related hardware metrics;
- keeping realtime history in `collector::HistoryStore`;
- publishing `MetricsSnapshot` values through `event_bus::EventBus`;
- defining platform traits and OS-specific platform implementations;
- using infrastructure providers for OS, vendor, kernel, and system data;
- running persistence workers that do not need Tauri objects;
- reading and writing settings that affect Core behavior.

Core must not depend on Tauri. This boundary is enforced by
`core/Cargo.toml`: adding `tauri` there is a design violation.

### `hardware_visualizer` (`src-tauri/`)

The App crate owns code that needs Tauri, frontend bindings, or UI-level state:

- registering Tauri commands and generating TypeScript bindings through
  `tauri-specta`;
- holding Tauri `State`, `AppHandle`, windows, plugins, and lifecycle hooks;
- converting Core events into frontend-visible Tauri events;
- applying presentation settings at the App boundary, such as temperature unit
  conversion;
- owning UI-only settings such as theme, language, graph display options,
  background image choices, tray widget settings, and close-to-tray behavior;
- defining the ordered SQL migration set supplied to Core at startup;
- holding worker/controller handles so shutdown can terminate background work.

When Close to Tray is enabled, closing the main window and exiting the process
are different lifecycle events. Code that starts, stops, flushes, or cleans up
workers must key off the process/App shutdown path when it needs finalization,
not merely the window-close path.

Elevated Startup Mode is also App-owned lifecycle behavior. It restarts the
whole Tauri process with Windows administrator privileges when enabled and the
current process is not already elevated. Core cannot be elevated independently
because it is linked into the App process; a Core-only elevation model would
require a separate helper or service process and IPC boundary.

## Layer Responsibilities

### Commands (`src-tauri/src/commands/`)

Commands are the frontend-facing IPC boundary.

Responsibilities:

- validate and normalize command inputs;
- call App services or Core APIs;
- return frontend-friendly DTOs and errors;
- participate in `tauri-specta` command collection for generated TypeScript
  bindings.

Commands should not contain sensor collection logic, OS access, long-running
worker logic, or database schema decisions.

### App Services (`src-tauri/src/services/`)

App services coordinate command-level use cases.

Responsibilities:

- wrap Tauri plugins or App-only state;
- call Core APIs where hardware data or persisted Core behavior is needed;
- keep Tauri-specific DTO conversion out of Core;
- handle UI-owned settings and presentation behavior.
- contain a mix of thin Core wrappers and App-owned services. See
  [`src-tauri/README.md`](../../src-tauri/README.md) for App crate details.

### App Adapters (`src-tauri/src/adapters/`)

Adapters subscribe to Core outputs and translate them into App outputs.

Known adapters:

- `window.rs`: receives `MetricsSnapshot` from `EventBus`, converts it into
  `HardwareMonitorUpdate`, applies the user's temperature unit, and emits the
  Tauri event.
- `tray.rs`: subscribes to the same Core event bus for tray-related output.

Core should publish data; adapters decide how that data is exposed to Tauri.

### Core Collector (`core/src/collector/`)

The collector owns realtime sampling and history.

Responsibilities:

- periodically sample system and GPU metrics;
- update `HistoryStore`;
- publish snapshots through `EventBus`;
- run under a Tokio runtime handle supplied by the App crate.

The collector does not emit Tauri events directly.

### Core Platform (`core/src/platform/`)

The platform layer abstracts OS-specific hardware access.

Responsibilities:

- define common hardware access traits in `traits.rs`;
- create the current OS implementation through `PlatformFactory`;
- keep OS-specific differences under `windows/`, `linux/`, and `macos/`;
- call infrastructure providers or OS APIs as needed.

Current platform traits include memory, GPU, network, and motherboard access.

### Core Infrastructure (`core/src/infrastructure/`)

Core infrastructure contains lower-level external access used by Core:

- `database/`: Core database initialization and DB access used by persistence;
- `providers/`: OS, vendor, and system data providers. See
  [`core/README.md`](../../core/README.md) for Core-specific provider details.

Windows sensor providers that depend on external runtime components, such as
PawnIO and its CPU-specific module blobs, are documented in
[`windows-sensor-external-components.md`](windows-sensor-external-components.md).

### Persistence (`core/src/persistence/`, `src-tauri/src/infrastructure/`)

Persistence is split:

- Core owns persistence workers and DB operations that are independent of Tauri.
- App owns the ordered migration definitions and passes the resolved SQLite
  path and migration set to Core during startup.
- Core owns the database pool and executes the App-supplied migration set
  through `hardviz_core::infrastructure::database::migrate`.

Startup flow:

1. App resolves the SQLite database path.
2. App initializes Core's DB location.
3. App checks schema compatibility through Core preflight.
4. App supplies the ordered migration definitions to Core's migrator when the
   DB is compatible.
5. DB-dependent Core workers start only when startup preflight allows it.

The Hardware Archive Retention Period is controlled by
`hardwareArchive.retentionDays`. The `scheduledDataDeletion` flag controls
whether cleanup for records older than the Retention Period runs at startup; it
does not create a continuously scheduled deletion task. This was a deliberate
simplification from the period before close-to-tray/background execution was a
supported app behavior, when the app was not expected to stay running
continuously.

Process Insight data is a sampled and ranked summary derived from realtime
process observations. It is not a complete process audit log, and persistence
code should preserve that expectation unless a new feature explicitly changes
the product contract.

Storage Health Records are intentionally retained separately from the
Hardware Archive. Dashboard display can use the latest record and recent
changes, while future historical views can keep following long-term storage
health even if short-window utilization archive settings are reduced.

## Settings Ownership

Settings are split by consumer:

- Core settings affect sampling, persistence, retention, or other Core behavior.
- App settings affect UI presentation, language/theme, graph options, tray
  behavior, window behavior, process launch behavior, and Tauri-only features.
- Both sides share a single top-level `settings.json` object. Core deserializes
  only Core-owned keys and ignores App-owned keys; App deserializes App-owned
  keys and preserves existing unknown keys when writing.
- Settings writes must merge into the existing JSON object so the writer does
  not drop keys owned by the other crate.

When adding a setting:

1. Decide whether Core or App consumes the value.
2. Persist user-facing preferences through the Rust settings service and typed
   Tauri commands.
3. Do not write user-facing application preferences directly from the frontend
   through Tauri Store.
4. Regenerate `src/rspc/bindings.ts` by running `npm run tauri:dev` when command
   types change.

## Design Rules

1. **Core has no Tauri dependency.** No `tauri` crate and no `use tauri::*`
   under `core/src/`.
2. **Commands stay thin.** Put business logic in App services or Core.
3. **Core publishes snapshots, App emits events.** `window.emit(...)` belongs in
   App adapters, not in Core.
4. **Use `PlatformFactory` for platform access.** Do not instantiate OS platform
   implementations directly from App command code.
5. **Presentation stays in App.** Core stores raw values; UI-specific conversion
   happens at the App boundary.
6. **Generated bindings are not edited by hand.** Update Rust commands and
   regenerate `src/rspc/bindings.ts`.
7. **Respect settings ownership.** Core-owned and App-owned settings may share
   the same file, but each side should preserve the other side's keys.

## Common Change Paths

### Add a New Frontend-Callable Backend Command

1. Add or update the App service in `src-tauri/src/services/`.
2. Add the command handler in `src-tauri/src/commands/`.
3. Register the command in `collect_commands![...]` in `src-tauri/src/lib.rs`.
4. Run `npm run tauri:dev` to regenerate TypeScript bindings.
5. Use the generated binding from frontend code.

### Add a New Hardware Data Source

1. Add or update the relevant Core platform trait if the capability is shared.
2. Implement OS-specific behavior under `core/src/platform/{windows,linux,macos}`.
3. Add provider code under `core/src/infrastructure/providers/` if low-level OS
   or vendor access is needed.
4. Surface the data through Core models and collector snapshots if it is
   realtime.
5. Translate Core data to App DTOs in `src-tauri/src/adapters/` or App services.

Missing or unsupported sensor data should be treated as best-effort where the
existing command/service contract allows it: prefer partial results, `None`, or
empty lists over failing an aggregate response. Check the target service before
choosing the fallback shape, because some commands still return an error for
platform initialization or required provider failures.

For vendor- or OS-dependent metrics, absence is usually a data-availability
condition rather than a fault. Preserve the distinction in new models and DTOs:
use nullable fields for unavailable per-metric values, keep source information
when it helps explain partial support, and avoid converting a partially
available device into an all-or-nothing failure unless the caller cannot
produce a useful result without that data.

External Component Guidance follows the same best-effort rule. Core may produce
structured guidance candidates when an optional runtime component such as
PawnIO or `smartctl` was actually attempted, could not be used, and fallback
collection still leaves user-visible hardware data unavailable. Those
candidates are diagnostic side data only: they must not change collection
results, fallback order, or aggregate success/failure behavior.

Core candidates should describe stable facts such as component, usage,
reason-kind, missing important signals, and optional diagnostic detail. App owns
the user-facing policy: it checks `externalComponentGuidance.acknowledgedKeys`
in `settings.json`, holds unshown candidates only in session memory, maps
guidance keys to detail URLs, returns candidates only while a relevant view can
show them, and persists acknowledgements through typed settings commands.

The initial candidate shape should stay minimal: a stable guidance key,
component, usage, reason kind, missing signal names, optional affected device
count, and optional diagnostic detail. Raw provider errors can be carried as
diagnostic detail for logs or expandable UI, but translated user-facing copy
should be driven by component, usage, and reason kind.

The initial implementation scope is limited to PawnIO for Windows CPU package
temperature and `smartctl` for Storage Health. Other vendor libraries, drivers,
or OS APIs should not be folded into this guidance path until their component,
usage, fallback behavior, and user action are explicit.
The first implementation slice should wire both initial guidance keys through
the shared candidate, settings, command, and dialog path rather than shipping
only one component first.

The minimum validation should cover both included components. PawnIO guidance
appears only when PawnIO cannot provide CPU package temperature and no ACPI CPU
temperature fallback is available. `smartctl` guidance appears only when
`smartctl` cannot be used and fallback Storage Health collection still lacks an
important health signal for at least one device. In both cases, tests should
also cover the inverse path where the optional component is unavailable but
fallback collection provides the user-visible data, so no guidance candidate is
produced.

App should aggregate unacknowledged guidance candidates by stable key. Repeated
detections of the same key replace the session-held candidate instead of
growing a queue, and already acknowledged keys are discarded before display.
If more than one unacknowledged key is relevant to the current view, the
frontend should show them one at a time.

Relevant frontend views should request pending guidance through a typed command
when they mount or become visible, rather than relying on a backend push event.
That command should filter candidates by view and return only guidance that can
be shown in the current UI context. Acknowledgement is a separate typed command
that records the guidance key in App-owned settings.

The frontend's "later" action should suppress the same guidance key only for
the current app session. It must not write an acknowledgement to settings, so a
future app session can show the guidance again if the same collection gap still
exists.

### Add or Change Persisted Data

1. Keep the App-owned ordered migration definitions in
   `src-tauri/src/infrastructure/database/`.
2. Keep Tauri-independent persistence code in `core/src/persistence/` or
   `core/src/infrastructure/database/`.
3. Update startup compatibility checks if schema compatibility changes.
4. Add tests for migration or preflight behavior where practical.

## Testing

Use the root Cargo aliases for CI parity:

```bash
cargo tauri-fmt
cargo tauri-lint
cargo tauri-test
```

For Core-only checks:

```bash
cargo build -p hardviz-core
cargo test -p hardviz-core
```

Frontend and TypeScript checks are run from the repository root:

```bash
npm run lint
npm test
```

Test placement follows the ownership boundary:

- Core pure/helper tests should live near the Core module they exercise.
- App command/service tests should live either next to the module or under
  `src-tauri/src/_tests/` when they need shared App test setup.
- Frontend tests should stay co-located under `src/**` with the component,
  hook, or feature they cover.
