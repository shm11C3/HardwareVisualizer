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
verification: "review responses split findings by evidence class, process every valid finding as fix, decline, or filed issue with reasoning, and end with an explicit sufficiency verdict anchored on what concretely breaks by merging now"
evidence: "PR #1944 (11 rounds), #1957 (4 restructures of one skill), #1958; the oscillation narrow-wide-narrow on verify-identity-contracts SKILL.md"
revalidate_when: "review bots gain a converging notion of sufficiency, or the repository changes its bot-review tooling"
---

# Bound Bot Review Loops

A bot reviewer re-reviews every push and can always produce another finding,
so "respond until silent" is not a terminating strategy. Reproducible findings
— anything a deterministic check can verify: code, tests, types, CI, guidance
validators — converge because defects are finite; verify by reproducing and
fix what reproduces, in any round. Subjective findings (wording, naming,
document or design shape) do not converge, because no experiment can prove
the bot right; fix only self-contradictions and factual errors, decline the
rest once, with reasoning.

The loop signature is a finding that targets the previous response. Verify it
first — a response can introduce a real regression, which gets fixed — and
when it does not reproduce, stop forward-fixing: revert accumulated
response-structure or freeze. Do not restructure a document, type model, or
module boundary to satisfy a subjective finding — file an issue or ask the
maintainer; a verified defect that requires structural repair is fixed.

The missing piece was a positive termination condition, not only limits: the
loop ends when the change is judged shippable, not when findings run out.
Every valid finding is processed — fixed, declined, or filed, with reasoning —
and then the change gets a sufficiency verdict (`SUFFICIENT` /
`INSUFFICIENT` / `UNCERTAIN`) anchored on one question: what concretely
breaks if this merges now? A finding that cannot answer it does not block,
however reasonable it sounds.

The [`gh-ai-review-triage`](../../../.agents/skills/gh-ai-review-triage/SKILL.md)
skill carries the rules (Convergence and Sufficiency Judgment sections). It must actually be loaded when
responding to bot reviews — the #1944 cycle happened with it sitting unused.
