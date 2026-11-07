# Native-only Persistence for New Families Before v2.0.0

Status: accepted

Decided on 2026-11-08. This amends one consequence of
[ADR 0025](0025-retire-sqlite-conversion-path-in-v2.md): that anything persisted
before v2.0.0 needs a SQLite implementation, a native implementation and a
differential test. The removal version, the six-month window, the bridge release
and every other precondition in ADR 0025 are unchanged.

Prompted by [#2543](https://github.com/shm11C3/HardwareVisualizer/issues/2543),
whose Hardware Change records and new per-band projection would each have needed
both engines.

## Context

ADR 0025 keeps the SQLite backend until v2.0.0, so that no release can strand an
unconverted archive. As a side effect, every new persisted family added during
the rest of 1.x has to be written for both engines and proven equivalent: a
SQLite migration, writer and query; a native schema, writer and query; routing
in `infrastructure/database/dispatch.rs`; and a differential test. Each cooling
projection added so far has a SQLite module and a `native_database` module beside
it. That doubles the persistence work of every new feature, and SQLite is the
engine already scheduled for removal.

Removing the SQLite backend sooner, in the next minor release, was considered
and rejected for the reasons ADR 0025 already records: the updater moves any
install straight to the latest release, so an unconverted archive would land on
a build that cannot read it.

Two facts limit who an engine-specific feature reaches:

- A fresh install creates and selects a native database directly and never
  starts on SQLite.
- An existing install stays on SQLite until its user starts the conversion from
  Settings (#2136). Nothing converts it automatically.

## Decision

1. **New families are native-only.** A persisted family introduced from now on,
   whether a new table, projection or user-entered record, is implemented only
   on the native database. It gets no SQLite migration, writer or query, and no
   differential test.
2. **Only existing families keep both engines.** Decision 1 holds even when an
   existing feature reads the new family. ADR 0025's two-engine requirement now
   covers only the families that already exist: a change to an existing table
   is made on both engines until v2.0.0. When an existing feature starts using
   a new family, that new behavior is native-only, and on SQLite the feature
   keeps its existing behavior without it.
3. **Native-only data gets its own tables.** Before v2.0.0, existing tables gain
   no native-only columns; data that extends an existing family goes in its own
   native-only table. Conversion plans every native column from the SQLite
   candidate (`plan_columns`, used by both finalization and reconciliation), so
   a column without a SQLite source would make every conversion fail.
4. **Unavailable is its own state.** While SQLite is authoritative, a
   native-only feature reports that it needs the native database. This state is
   distinct from empty data, from a read failure and from
   `DispatchError::NativeUnavailable` (DP-02). The UI may point to the existing
   Settings conversion action, but it never starts a conversion (DP-06).
5. **Conversion learns which tables have no SQLite source.** Finalization and
   reconciliation walk every table in `NativeSchemaDefinition.tables` and refuse
   a candidate that lacks one. The native schema definition therefore marks
   native-only tables. Conversion creates them empty and leaves them out of the
   copy and the reconciliation, and it still refuses a candidate that lacks any
   table that does have a SQLite source. After conversion, a projection derived
   from the Hardware Archive backfills up to the archive's Retention Period, and
   a user-entered family starts empty. This change lands with the first
   native-only table or before it.
6. **Builds without the `duckdb-archive` feature** report the same unavailable
   state for native-only features.
7. **Existing native databases need a schema upgrade first.** The native schema
   is at version 1, and opening a database at any other version fails with
   `IncompatibleSchema`; there is no upgrade step. The first native-only table
   therefore needs a versioned native schema upgrade for databases already at
   version 1. A new table would need this whichever engine owned it.

## Consequences

- New persistence work is written once. The work v2.0.0 has to remove does not
  grow.
- The first native-only feature carries two one-time prerequisites: the native
  schema upgrade (decision 7) and the conversion change (decision 5). Later
  native-only tables reuse both.
- Existing installs that have not converted do not get native-only features
  until their user converts. Because conversion only starts from a Settings
  action, this may be most installs that upgraded from v1.10 or earlier. The
  unavailable state should say why the feature is missing and where to convert.
- New features become a reason to convert. That supports ADR 0025's removal plan
  without making conversion automatic.
- The dispatch boundary gains an answer meaning "this family exists only on the
  native database". A native-only family's consumers have to handle it.
- An existing feature that gains a native-only part keeps its current behavior
  on SQLite (decision 2). A new feature that combines an existing family with a
  native-only one either degrades to the existing part on SQLite or reports
  unavailable as a whole; each such feature decides which and states it.

## Alternatives Considered

| Alternative | Why not selected |
| --- | --- |
| Keep both engines for every new family (ADR 0025 as written) | Doubles the persistence work of every 1.x feature for an engine that is already scheduled for removal. |
| Remove the SQLite backend in the next minor release | ADR 0025's reasons still hold: the updater can move an unconverted archive onto a release that cannot read it. |
| Give SQLite a reduced fallback for each new feature | It is still a second implementation to write and test, and its results would differ by engine. |
| Convert automatically so every install gets new features | ADR 0025 rejects silent conversion: it needs disk space, time and a visible failure path, and it overrides the user's choice (DP-06). |
