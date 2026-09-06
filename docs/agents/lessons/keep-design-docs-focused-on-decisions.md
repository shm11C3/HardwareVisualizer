---
id: LRN-20260906-keep-design-docs-focused-on-decisions
status: promoted
cause_status: confirmed
scope: Design Docs and reference specifications
trigger: implementation detail or measurement procedure grows into a parallel specification
failure_signature: the native DuckDB Design Doc retained detail intended for a custom archive algorithm
root_cause: documentation depth did not follow the transfer of algorithm ownership to the database
guardrail: docs/documentation-guide.md
canonical_refs: docs/documentation-guide.md
verification: "Review the Design Doc for decisions and tradeoffs; run npm run check:agent-guidance and git diff --check."
evidence: "Maintainer corrections during PR #2086 on 2026-09-06; docs/design/hardware-archive-duckdb.md."
revalidate_when: the documentation policy or clean-room source and role requirements change
---

# Keep Design Docs Focused on Decisions

The maintainer clarified that code is the living specification; detailed prose
that restates it becomes stale. The project does not use specification-driven
development. Clean-room
reference specifications serve independent implementation and provenance;
they are not an exception that adopts that development method.

The [documentation guide](../../documentation-guide.md#documentation-depth)
owns the policy. Keep decisions and consequential tradeoffs in the Design Doc,
and link detailed procedures and observations from their implementation owner.
