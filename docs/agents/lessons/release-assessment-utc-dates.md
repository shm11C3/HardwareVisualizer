---
id: LRN-20261029-release-assessment-utc-dates
status: promoted
cause_status: confirmed
scope: release OSV exposure assessments
trigger: an assessment is authored in a timezone whose calendar date is ahead of UTC
failure_signature: the release evaluator reports a newly reviewed assessment as future and leaves the finding in triage_required
root_cause: local calendar dates were stored in fields compared against the evaluator's UTC calendar date
guardrail: use UTC calendar dates for assessment validity fields
canonical_refs: docs/security/release-vulnerability-response.md, .github/scripts/evaluate-release-vulnerabilities.mjs
verification: the existing future-assessment regression case in .github/scripts/test-evaluate-release-vulnerabilities.mjs; evaluate the same finding before and after correcting reviewed_on
evidence: PR 2483 review comment 4404195689; the v1.11.3 Zod-only replay returned future before the correction and active afterwards
revalidate_when: the evaluator's date source or assessment validity semantics change
---

# Use UTC Dates In Release Assessments

At 2026-10-28 17:57 UTC, the author's local date was already October 29.
An assessment with `reviewed_on: 2026-10-29` was therefore future-dated for
the evaluator, which defaults to `new Date().toISOString().slice(0, 10)`.
The High Zod finding still required triage despite the completed review.

Record validity dates in UTC. Correct the assessment date rather than
overriding the workflow clock or weakening the future-assessment check.
The existing evaluator tests already assert rejection of future assessments.
