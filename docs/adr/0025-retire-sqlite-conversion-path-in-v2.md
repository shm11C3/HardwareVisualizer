# Retire the SQLite Conversion Path in v2.0.0

Status: accepted

Tracking issue: [#2090](https://github.com/shm11C3/HardwareVisualizer/issues/2090).

This records when the SQLite backend and the SQLite-to-DuckDB conversion are
removed, decided on 2026-09-22. Nothing here is implemented. Release builds
still exclude the `duckdb-archive` feature, and the latest release is v1.10.1.

## Context

[ADR 0022](0022-prioritize-native-duckdb-archive-qualification.md) selects one
authoritative native DuckDB database after a verified conversion. Getting there
means the application carries two engines for a while: the SQLite writers,
queries and migrations, the candidate builder, finalization, reconciliation,
durable selection, the space preflight, the App conversion driver, the dispatch
boundary that routes every consumer, and the differential suites that prove both
engines answer bit for bit. That machinery is temporary by intent, and it has a
continuing cost: every new persisted family or query has to be written twice and
proven identical.

Five facts decide when it can go.

- **The updater always jumps to the latest release.** Its endpoint is the fixed
  `releases/latest/download/latest.json`; the server never sees the client's
  version. Any old install can land on any later release in one step.
- **Adoption cannot be measured.** DP-01 keeps hardware data local and rules out
  outbound telemetry, so there is no way to learn how many archives are
  converted.
- **Conversion starts from explicit user intent** (DP-06,
  [#2136](https://github.com/shm11C3/HardwareVisualizer/issues/2136)). A user
  who never chooses it never converts.
- **A fresh install starts on SQLite.** No path creates a native database except
  by converting one.
- **Native writers still use SQLite.** They derive each row's epoch key from an
  in-memory SQLite oracle (Design Doc, "Finalization and the measured
  compatibility boundary"), so the `sqlx` SQLite dependency outlives the SQLite
  backend.

Minor releases have shipped every four to six weeks (v1.8.0 on 2026-05-06,
v1.9.0 on 2026-06-21, v1.10.0 on 2026-08-22).

## Decision

1. **Remove the conversion path and the SQLite backend in v2.0.0, not in a 1.x
   minor.** A release that can no longer read an unconverted archive breaks
   stored user data, which is what a major version is for.
2. **The last 1.x release is the bridge release.** It is the final version that
   contains the conversion. It stays downloadable indefinitely and is named in
   the v2.0.0 release notes and in the application's own guidance.
3. **v2.0.0 ships no earlier than six months after the first release in which
   conversion is the default path**, meaning new installs start on the native
   database and existing users are prompted to convert. The window is measured
   in time because adoption cannot be measured; six months is four or five
   minor releases at the current cadence.
4. **v2.0.0 keeps a minimal detector.** It recognizes an unconverted archive (a
   SQLite source with no selected native database) and stops with guidance that
   names the bridge release. It never creates an empty database and never
   deletes or renames the SQLite file. This extends the guarantees of
   [ADR 0019](0019-lossless-chunked-hardware-archive.md) and ADR 0022, and DP-02:
   history that cannot be read is reported, not silently replaced.

v2.0.0 removes the candidate builder, finalization, reconciliation, selection
writing, the space preflight, the App conversion driver and its state
vocabulary's conversion steps, the dispatch boundary (consumers call the native
backend directly again), the SQLite writers, queries and migrations, and the
differential suites. It keeps the native schema and its versioning, reading the
authority record far enough to run the detector, and the handling of an existing
`.retired` recovery copy.

## Preconditions

Removal waits for each of these, all in 1.x:

- A fresh install creates a native database directly
  ([#2191](https://github.com/shm11C3/HardwareVisualizer/issues/2191)).
- The "Later" path is closed one way or the other: either conversion becomes
  required in a 1.x release, or the v2.0.0 notes state plainly that unconverted
  history is not carried forward. This is still open and belongs with #2136.
- An explicit **Remove recovery copy** action for the `.retired` SQLite file has
  shipped, because v2.0.0 will no longer explain where that file came from.
- The release process names the bridge release and protects it from deletion.

Replacing the SQLite epoch oracle is a separate decision. Until it is made,
"removing SQLite" means removing the backend, not the dependency. Making the
updater endpoint version-aware, so an old install is offered the bridge release
first, would strengthen the path but is not required: the detector is the guard.

## Alternatives and trade-offs

| Alternative | Why not selected |
| --- | --- |
| Remove in a 1.x minor once most users have converted | "Most" cannot be measured without telemetry, and the updater can move an unconverted install straight onto that release. |
| Keep both engines indefinitely | Every new family is implemented twice and proven bit-identical forever, and it turns the temporary topology ADR 0022 accepted into the permanent split it rejected. |
| Convert silently at startup in a late 1.x, then remove | A multi-gigabyte conversion needs disk space, time and a visible failure path; starting it without the user's choice breaks DP-06. Requiring conversion remains possible, but as an explicit step. |
| Keep a read-only SQLite importer in 2.x | An importer is the candidate builder, finalization and reconciliation under another name, so most of the machinery and its tests would stay. |

## Consequences

Both engines are maintained through the rest of 1.x, and anything persisted
before v2.0.0 needs a SQLite implementation, a native implementation and a
differential test. A user who skips the bridge release meets a blocking guidance
screen in v2.0.0 and has to install the bridge release, convert, and update
again; their data is untouched throughout. Downgrading from 2.x to 1.x stays
unsupported, as [#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052)
already deferred it. The bridge release becomes a permanent release-engineering
obligation.
