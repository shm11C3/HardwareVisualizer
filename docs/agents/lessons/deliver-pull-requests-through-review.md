---
id: LRN-20260815-deliver-pull-requests-through-review
status: promoted
cause_status: confirmed
scope: pull request creation, CI follow-through, and automated review handling
trigger: an agent is asked to create or publish a HardwareVisualizer pull request
failure_signature: work stopped after push or PR creation, or repeated full automated reviews expanded scope after each feedback commit
root_cause: publication and completion were treated as separate tasks, while review comments were treated as prescribed patches instead of scoped evidence
guardrail: AGENTS.md owns pre-change justification and .agents/skills/deliver-pull-request/SKILL.md owns end-to-end PR delivery
canonical_refs: AGENTS.md, .agents/skills/deliver-pull-request/SKILL.md
verification: invoke the skill for a PR request and confirm it states the minimal change rationale, preserves unrelated work, monitors required CI and reviews, replies with evidence, resolves threads, and stops at the documented completion gate
evidence: "maintainer PR workflow correction; shm11C3/whowns AGENTS.md review discipline; HardwareVisualizer PR #1915 and PR #1930 review follow-through"
revalidate_when: repository PR completion criteria, automatic review behavior, GitHub tooling, or sandbox execution policy changes
---

# Deliver Pull Requests Through Review

Creating a PR is a publication milestone, not the completion condition. Keep
working until relevant CI and configured reviews pass, every actionable thread
has an evidence-backed decision and is resolved, and the PR is Ready and
mergeable.

Before implementation, justify the smallest coherent change using Why, What,
How, and Why Not. Review feedback does not expand the task by itself: verify the
problem, root cause, owning boundary, and current-scope risk before editing.
Rely on configured automatic review instead of repeatedly requesting new full
reviews after each correction.
