---
id: LRN-20260815-deliver-pull-requests-through-review
status: promoted
cause_status: confirmed
scope: pull request creation, CI follow-through, and automated review handling
trigger: an agent is asked to create or publish a HardwareVisualizer pull request
failure_signature: work stopped at PR creation, or review follow-through expanded a simple workflow into speculative GitHub state handling
root_cause: PR publication and completion were separated, then individual review findings were encoded as general workflow requirements
guardrail: .agents/skills/deliver-pull-request/SKILL.md owns a small PR delivery workflow; review comments do not expand its scope without evidence from the current requirement
canonical_refs: AGENTS.md, .agents/skills/deliver-pull-request/SKILL.md
verification: confirm the skill publishes a focused PR, performs one collected review and correction cycle, validates it with CI, and does not wait for another automated review
evidence: "maintainer corrections during HardwareVisualizer PR #1934"
revalidate_when: repository PR completion criteria, automatic review behavior, or sandbox execution policy changes
---

# Deliver Pull Requests Without Growing The Workflow

PR creation is not completion: finish the current CI and review work that
belongs to the requested change. However, that follow-through is not an
invitation to model every GitHub state or encode every reviewer suggestion.

Treat feedback as evidence, make only changes required by the current scope,
and stop after one collected review and correction cycle has been validated by
CI. Do not wait for a review of the correction commit or recursively reopen the
cycle when another automated review arrives. Never request Codex review
manually; Codex decides when review is needed.
