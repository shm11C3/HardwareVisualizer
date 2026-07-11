---
id: LRN-20260711-evidence-before-conclusions
status: promoted
cause_status: confirmed
scope: diagnosis, CI, runtime verification, and PR completion
trigger: a claim depends on current failure, runtime, release, or merge state
failure_signature: conclusions were drawn from an aggregate check, stale checkout, memory, or handoff before inspecting primary evidence
root_cause: a secondary status was treated as if it identified the failing leaf job or current runtime behavior
guardrail: docs/design-principles.md DP-09 and the evidence rules in AGENTS.md
canonical_refs: docs/design-principles.md, AGENTS.md, .agents/skills/hardwarevisualizer-design-review/SKILL.md
verification: identify the leaf evidence source for the scoped claim and record its current result before concluding
evidence: GitHub Actions leaf-job logs, current code and tests, application logs and SQLite data, rendered artifacts, and current PR checks/review/merged state
revalidate_when: CI topology, release workflow, runtime storage, or PR policy changes
---

# Evidence Before Conclusions

For current-state questions, inspect the evidence surface that can prove the
claim. A Merge Gate failure does not identify the root failing job. A local
build does not prove a release artifact is signed. A successful unit test does
not prove the native UI rendered correctly. A pushed commit does not prove a PR
was merged.

Memory and handoff documents are useful search indexes, but they can describe a
different branch or an older point in time. Resolve the current branch, issue,
specification revision, workflow run, database, or rendered artifact before
deciding.
