---
id: LRN-20260822-bound-bot-review-loops
status: promoted
cause_status: confirmed
scope: responding to AI bot review feedback on any PR
trigger: "a bot re-review produces new findings after a response push, especially findings about text or code added in the previous response"
failure_signature: "across PRs #1944-#1958, eleven-plus response rounds each fixed everything the bots raised; several rounds' findings targeted the previous round's fix, and skill prose accumulated conditional structure that outweighed the lesson it carried"
root_cause: "bot reviewers are generators that always produce another finding, and fixing every finding each round - including unfalsifiable prose findings - manufactures the next round's material; the existing triage skill was also not loaded at all during the cycle"
guardrail: .agents/skills/gh-ai-review-triage/SKILL.md
canonical_refs: .agents/skills/gh-ai-review-triage/SKILL.md
verification: "review responses split findings by evidence class, fix only reproduced defects and self-contradictions, never restructure mid-review, and stop editing after two response rounds"
evidence: "PR #1944 (11 rounds), #1957 (4 restructures of one skill), #1958; the oscillation narrow-wide-narrow on verify-identity-contracts SKILL.md"
revalidate_when: "review bots gain a converging notion of sufficiency, or the repository changes its bot-review tooling"
---

# Bound Bot Review Loops

A bot reviewer re-reviews every push and can always produce another finding,
so "respond until silent" is not a terminating strategy. Reproducible findings
(code, tests, types, CI) converge because defects are finite — verify by
reproducing and fix what reproduces. Prose and design-shape findings do not
converge, because no experiment can prove the bot right; fix only
self-contradictions and factual errors, decline the rest once, with reasoning.

The loop signature is a finding that targets the previous response. When it
appears, stop forward-fixing: revert accumulated response-structure or freeze.
Never restructure a document, type model, or module boundary in response to
bot review — file an issue or ask the maintainer.

The [`gh-ai-review-triage`](../../../.agents/skills/gh-ai-review-triage/SKILL.md)
skill carries the rules (Convergence section). It must actually be loaded when
responding to bot reviews — the #1944 cycle happened with it sitting unused.
