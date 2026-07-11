# Core Instructions

These instructions add to the repository root `AGENTS.md` for work under
`core/`.

## Ownership

- `hardviz-core` owns Tauri-independent collection, realtime history, the
  EventBus, platform traits and factory, OS/vendor providers, Core persistence,
  raw models, and Core-consumed settings.
- Do not add a `tauri`, `specta`, or `tauri-specta` dependency to Core. Do not
  emit Tauri window events from Core.
- Core publishes raw facts; `src-tauri` adapters own presentation conversion and
  Tauri delivery.
- Shared hardware access belongs behind `core/src/platform/**`. Low-level OS or
  vendor access belongs under `core/src/infrastructure/providers/**`.
- Use `PlatformFactory`; do not instantiate an OS platform from App code.

Read [`core/README.md`](README.md),
[`docs/architecture/backend.md`](../docs/architecture/backend.md), and
[`docs/design-principles.md`](../docs/design-principles.md), and the relevant
shared rule: [`rust.md`](../.agents/rules/rust.md) or
[`clean-room-sensors.md`](../.agents/rules/clean-room-sensors.md).

## Data Behavior

- Preserve partial results and explicit availability. A missing vendor metric is
  usually `None`/empty plus availability context, not an aggregate error.
- Keep raw units and hardware facts in Core. Presentation units and wire policy
  belong at the App boundary.
- Treat temporary enumeration failure as uncertainty. Do not deactivate or
  delete persisted subjects without positive evidence.
- Keep continuous collection independent from slow persistence consumers; use
  the EventBus rather than reaching through App state.
- Core settings writes must preserve App-owned keys in the shared settings
  object.

## Validation

Use focused checks before workspace-wide aliases:

```bash
cargo fmt --all -- --check
cargo clippy -p hardviz-core --all-targets -- -D warnings
cargo test -p hardviz-core
```

Keep pure/helper tests close to the Core module. Add integration tests when the
claim crosses collector, EventBus, persistence, or provider boundaries.

For PawnIO CPU or Super I/O files, the root clean-room gate is mandatory. Stop
when the required implementation-ready spec does not answer the implementation
question.
