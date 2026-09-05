---
id: LRN-20260905-preserve-sqlite-timestamp-membership
status: promoted
cause_status: confirmed
scope: core/examples/archive_format_benchmark, core/src/infrastructure/database/archive_queries.rs
trigger: replacing SQLite timestamp predicates with decoded archive reads
failure_signature: a submillisecond row crosses an inclusive range endpoint after Rust timestamp conversion
root_cause: SQLite strftime fractional-second conversion and chrono timestamp_millis do not use identical rounding
guardrail: differential endpoint regression in the G1 database contract tests
canonical_refs: core/examples/archive_format_benchmark/database_contract_tests.rs
verification: cargo test -p hardviz-core --example archive_format_benchmark database::contract_tests::ambient_endpoint_uses_sqlite_submillisecond_rounding_and_offsets
evidence: core/src/infrastructure/database/archive_queries.rs, core/examples/archive_format_benchmark/database.rs
revalidate_when: SQLite or chrono changes, or an endpoint intentionally changes timestamp membership
---

# Preserve SQLite Timestamp Membership

The G1 archive experiment initially converted decoded timestamps with
`chrono::DateTime::timestamp_millis` while its relational oracle used the
existing SQLite `strftime` expression. For
`2026-01-01T00:00:00.1239+00:00`, the SQLite expression produces epoch millisecond
`1767225600124`; the Rust conversion produces `1767225600123`. A range containing
only the former millisecond includes the source row but excludes the decoded
row under the replacement predicate. The equivalent `+09:00` spelling must
have the same membership.

The executable regression compares the relational and decoded paths at that
endpoint and checks the expected two included records. The experiment applies
the existing SQLite predicate to decoded timestamps in bounded batches backed
by a temporary table. Its measured query cost includes that work; this is not
a production query-performance claim.

The durable contract is endpoint equivalence, not a new universal timestamp
normalization rule. Preserve stored timestamp bytes and verify fractional
precision, offsets, duplicate instants, and backward clock steps against the
actual owning predicate before replacing it. Re-run the regression when the
SQL expression, SQLite, chrono, or the storage reader changes. A future faster
conversion needs the same differential evidence.
