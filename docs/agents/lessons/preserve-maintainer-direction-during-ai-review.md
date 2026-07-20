---
id: LRN-20260720-preserve-maintainer-direction-during-ai-review
status: promoted
cause_status: confirmed
scope: AI-generated pull request review triage and maintainer-confirmed product constraints
trigger: an AI review proposes a change that conflicts with an explicit maintainer correction, scope boundary, or accepted decision
failure_signature: advisory review feedback was treated as authoritative and risked reintroducing successful-reading UI metadata that the maintainer had explicitly excluded
root_cause: review triage verified local technical plausibility without first binding classification to the maintainer-confirmed product and scope constraints
guardrail: .agents/skills/gh-ai-review-triage/SKILL.md
canonical_refs: .agents/skills/gh-ai-review-triage/SKILL.md
verification: confirm the triage extracts binding constraints, classifies conflicting AI feedback as Ignore, preserves the requested direction, and escalates genuinely new correctness or safety evidence instead of silently overriding it
evidence: "PR #1836 body and review feedback concerning successful-reading verification metadata; maintainer correction recorded in the PR implementation direction"
revalidate_when: review authority, AI review classification, or maintainer-decision handling changes
---

# Preserve Maintainer Direction During AI Review

AI review feedback is advisory. Before deciding whether a comment is a fix
target, extract the binding product and scope constraints from the request,
explicit maintainer corrections, and accepted decision sources. A suggestion
that is technically plausible but reverses one of those constraints is not an
improvement to the requested change.

Classify such feedback as `Ignore` and state the conflict. If it presents new
correctness, safety, or internal-consistency evidence, bring that evidence back
to the maintainer rather than silently changing direction or implementing a
compromise that weakens the confirmed constraint.
