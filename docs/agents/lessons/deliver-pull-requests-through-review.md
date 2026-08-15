---
id: LRN-20260815-deliver-pull-requests-through-review
status: promoted
cause_status: confirmed
scope: pull request creation, CI follow-through, and automated review handling
trigger: an agent is asked to create or publish a HardwareVisualizer pull request
failure_signature: work stopped after push or PR creation, review comments expanded scope, Codex review was requested manually, decisions were either lost or over-documented, or auto-merge could merge a PR without authorization
root_cause: publication and completion were treated as separate tasks, review comments were treated as prescribed patches, and reviewer, decision-record, and merge-authorization responsibilities were unclear
guardrail: AGENTS.md owns pre-change justification and .agents/skills/deliver-pull-request/SKILL.md owns end-to-end PR delivery
canonical_refs: AGENTS.md, .agents/skills/deliver-pull-request/SKILL.md
verification: invoke the skill for a PR request and confirm it states the minimal change rationale, preserves non-obvious decisions at their smallest durable owner, preserves unrelated work, prevents unauthorized auto-merge, monitors required CI and reviews, replies with evidence, resolves threads, and stops at the documented completion gate
evidence: "maintainer PR workflow corrections, including automatic Codex review timing and proportionate decision records; shm11C3/whowns AGENTS.md review discipline; HardwareVisualizer auto-merge.yml; HardwareVisualizer PR #1915, PR #1930, and PR #1934 review follow-through"
revalidate_when: repository PR completion criteria, automatic review behavior, GitHub tooling, or sandbox execution policy changes
---

# Deliver Pull Requests Through Review

Creating a PR is a publication milestone, not the completion condition. Keep
working until relevant CI and reviews applicable to the intended publication
state pass, every actionable review item has an evidence-backed decision,
inline threads are resolved, and the PR is in its intended Ready or Draft state
and mergeable.

Before implementation, justify the smallest coherent change using Why, What,
How, and Why Not. Review feedback does not expand the task by itself: verify the
problem, root cause, owning boundary, and current-scope risk before editing.
Do not turn those four questions into mandatory paperwork. Preserve only the
non-obvious decisions future maintainers need, using code for How, tests for
What, commit or PR context for a change-local Why, comments for a local Why Not,
and an ADR for an architecturally significant Why and its consequences.

Never request Codex review manually; Codex decides automatically when review is
needed. For other configured automatic reviewers, avoid repeated full reviews
after each correction unless a material unreviewed change makes the prior
review stale or a human explicitly asks. Create new PRs authored by the
repository's auto-merge actors as Draft unless merge is explicitly authorized.
For an existing Ready PR by one of those actors, stop before pushing and ask to
convert it to Draft or authorize merge. Treat pre-existing auto-merge as merge
intent. Keep an existing Draft by one of those actors Draft unless merge is
authorized; requesting Ready alone is insufficient because that event enables
auto-merge. Do not drive the PR to completion until the user authorizes merge
or confirms that auto-merge may be disabled.
