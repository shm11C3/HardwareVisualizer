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
2. **The consumer decides.** Data that only a native-only feature reads is
   native-only, even when it extends an existing family. Data that an existing
   feature reads keeps both engines until v2.0.0, as ADR 0025 requires.
3. **Unavailable is its own state.** While SQLite is authoritative, a
   native-only feature reports that it needs the native database. This state is
   distinct from empty data, from a read failure and from
   `DispatchError::NativeUnavailable` (DP-02). The UI may point to the existing
   Settings conversion action, but it never starts a conversion (DP-06).
4. **Conversion does not change.** A native-only family has no SQLite source to
   convert. The native schema creates its tables on a fresh database and on a
   converted one alike. A projection derived from the Hardware Archive backfills
   after conversion, up to the archive's Retention Period; a user-entered family
   starts empty.
5. **Builds without the `duckdb-archive` feature** report the same unavailable
   state for native-only features.

## Consequences

- New persistence work is written once. The work v2.0.0 has to remove does not
  grow.
- Existing installs that have not converted do not get native-only features
  until their user converts. Because conversion only starts from a Settings
  action, this may be most installs that upgraded from v1.10 or earlier. The
  unavailable state should say why the feature is missing and where to convert.
- New features become a reason to convert. That supports ADR 0025's removal plan
  without making conversion automatic.
- The dispatch boundary gains an answer meaning "this family exists only on the
  native database". A native-only family's consumers have to handle it.
- Features that span both kinds of data, such as one combining an existing
  family with a native-only one, degrade to the existing part on SQLite, or
  report unavailable as a whole. Each feature decides which and states it.

## Alternatives Considered

| Alternative | Why not selected |
| --- | --- |
| Keep both engines for every new family (ADR 0025 as written) | Doubles the persistence work of every 1.x feature for an engine that is already scheduled for removal. |
| Remove the SQLite backend in the next minor release | ADR 0025's reasons still hold: the updater can move an unconverted archive onto a release that cannot read it. |
| Give SQLite a reduced fallback for each new feature | It is still a second implementation to write and test, and its results would differ by engine. |
| Convert automatically so every install gets new features | ADR 0025 rejects silent conversion: it needs disk space, time and a visible failure path, and it overrides the user's choice (DP-06). |
