# Retire the SQLite Conversion Path in v2.0.0

Status: accepted

Status update (2026-09-23): the "fresh install starts on SQLite" precondition
below is closed — a fresh install creates and selects a native database
directly on `develop` (#2203, merged). PR #2224 (open) enables the
`duckdb-archive` feature for shipped builds via `build.features` in
`src-tauri/tauri.conf.json`. The rest of this record, including the removal
plan and its remaining preconditions, is unchanged.

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

- **The updater always jumps to the latest release.** Its one endpoint
  (`src-tauri/tauri.conf.json`) is the fixed URL
  `releases/latest/download/latest.json`, a static GitHub release asset with no
  version variable in it. A static file cannot answer differently per client, so
  any old install can land on any later release in one step.
- **Adoption cannot be measured.** DP-01 keeps hardware data local and rules out
  outbound telemetry, so there is no way to learn how many archives are
  converted.
- **Conversion is planned to start only from explicit user intent.** Nothing in
  the application can start one today; the conversion driver has no production
  caller. [#2136](https://github.com/shm11C3/HardwareVisualizer/issues/2136)
  requires the flow to begin from the user's choice (DP-06). If it ships that
  way, a user who never chooses it never converts.
- **A fresh install starts on SQLite.** No path creates a native database except
  by converting one.
- **Native writers still use SQLite.** They derive each row's epoch key from an
  in-memory SQLite oracle (Design Doc, "Finalization and the measured
  compatibility boundary"), so the `sqlx` SQLite dependency outlives the SQLite
  backend.

Issue #2191 implements the fresh-profile path only behind the existing
`duckdb-archive` feature. It must remain excluded from release defaults until
#2137 qualifies native storage for production; that qualification is the gate
for changing the production default, not this implementation alone.

Minor releases have shipped every four to six weeks (v1.8.0 on 2026-05-06,
v1.9.0 on 2026-06-21, v1.10.0 on 2026-08-22, per the project's
[GitHub releases](https://github.com/shm11C3/HardwareVisualizer/releases)). The
cadence only translates the window below into a number of releases; it is not
what the window is based on.

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
4. **Every install older than the bridge release reaches 2.x through it.** The
   existing updater endpoint keeps serving the bridge release for good. 2.x is
   published on a second endpoint that only the bridge release and later know,
   and the bridge release offers 2.x once nothing is left to convert: the
   archive is converted, or there is no SQLite archive at all. The in-app
   updater therefore never moves an unconverted archive onto 2.x, and the
   recovery path never depends on a downgrade.
5. **v2.0.0 keeps a minimal detector as the last line of defense**, for an
   installer downloaded by hand and run over an old version. It recognizes an
   unconverted archive (a SQLite source with no selected native database) and
   stops with guidance that names the bridge release. It never creates an empty
   database and never deletes or renames the SQLite file. When it fires, v2.0.0
   also writes nothing else: no settings migration, no store changes. Installing
   the bridge release over a v2.0.0 that refused to start is therefore a
   supported and tested path, the one narrow downgrade this project guarantees.
   This extends the guarantees of
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
- The two-endpoint distribution exists and has been exercised before v2.0.0:
  the old URL still resolves to the bridge release after a 2.x release is
  published, and the bridge release updates to 2.x from the new one. How the
  old URL is pinned is a release-engineering choice; the simplest is never
  marking a 2.x release as GitHub's latest, which also leaves the repository's
  "Latest release" badge on the bridge release.
- A test installs the bridge release over a v2.0.0 that stopped at the detector
  and converts from there.

Replacing the SQLite epoch oracle is a separate decision. Until it is made,
"removing SQLite" means removing the backend, not the dependency.

## Alternatives and trade-offs

| Alternative | Why not selected |
| --- | --- |
| Remove in a 1.x minor once most users have converted | "Most" cannot be measured without telemetry, and the updater can move an unconverted install straight onto that release. |
| Keep both engines indefinitely | Every new family is implemented twice and proven bit-identical forever, and it turns the temporary topology ADR 0022 accepted into the permanent split it rejected. |
| Convert silently at startup in a late 1.x, then remove | A multi-gigabyte conversion needs disk space, time and a visible failure path; starting it without the user's choice breaks DP-06. Requiring conversion remains possible, but as an explicit step. |
| Keep a read-only SQLite importer in 2.x | An importer is the candidate builder, finalization and reconciliation under another name, so most of the machinery and its tests would stay. |
| Keep one updater endpoint and rely on the detector alone | Every unconverted install that updates lands on v2.0.0, and the only remedy is installing 1.x over it: a downgrade nobody tested, as the routine path for an unknown number of users. |

## Consequences

Both engines are maintained through the rest of 1.x, and anything persisted
before v2.0.0 needs a SQLite implementation, a native implementation and a
differential test. Every install older than the bridge release takes two updates
to reach 2.x, including users with nothing to convert, who are offered 2.x as
soon as they are on the bridge release. Only a hand-run v2.0.0 installer can
still meet the detector; that user installs the bridge release, converts, and
updates again, with their data untouched throughout. Downgrading from 2.x to 1.x
otherwise stays unsupported, as
[#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052) already
deferred it. The bridge release, the second updater endpoint and the pinned old
URL become permanent release-engineering obligations, and visitors to the
repository see the bridge release labelled as the latest one.
