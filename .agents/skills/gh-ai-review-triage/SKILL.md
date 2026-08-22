---
name: gh-ai-review-triage
description: Triage and optionally address AI-generated GitHub PR review feedback from Copilot, CodeRabbit, or similar bots. Use when the user asks to check AI review comments, decide whether comments require action, ignore non-actionable suggestions, fix required/should-fix/worth-fixing feedback, or summarize bot review findings on a pull request.
---

# GitHub AI Review Triage

Separate AI review noise from useful feedback, and end the review with an
explicit shippability verdict rather than with silence.

## Workflow

1. Resolve the target PR (user-given number/URL, else the current branch) and
   collect threads, comments, and review submissions —
   see [references/retrieval.md](references/retrieval.md).
2. Extract binding constraints from the user request, maintainer corrections,
   and canonical decisions; they are the classification boundary, not inputs
   equal to bot feedback.
3. Classify each comment as `Required` / `Should Fix` / `Worth Fixing` /
   `Optional` / `Ignore`, verifying before classifying —
   definitions and verification rules in
   [references/classification.md](references/classification.md).
4. Act: fix `Required`, `Should Fix`, and `Worth Fixing` by default ("only
   required" from the user narrows this); leave `Optional`/`Ignore` alone.
   Keep fixes scoped to the comment; run the smallest relevant checks first.
5. No GitHub writes — replies, thread resolution, review submission, issue
   filing — without the user's explicit request.

## Convergence

Bot reviewers are generators, not gates: they can always produce another
finding, so "respond until silent" never terminates (see lesson
`bound-bot-review-loops`).

1. Split findings by evidence class. Reproducible (code, tests, types, CI,
   deterministic guidance checks): fix what reproduces — defects are finite.
   Subjective (wording, naming, structure): fix only self-contradictions and
   factual errors; decline the rest once, with reasoning.
2. Do not restructure to satisfy a subjective finding — file an issue or ask
   the maintainer. A verified `Required` defect needing structural repair is
   fixed, in any round.
3. A finding that targets the previous response: verify it first (responses
   can introduce real regressions); if it does not reproduce, stop
   forward-fixing — revert the accumulated response-structure or freeze.
4. Stop-loss backstop: after two response rounds, stop editing for subjective
   findings; summarize the remainder as declined-with-evidence or filed
   issues and hand the trade-off to the maintainer.

## Sufficiency Judgment

Processing a round means deciding fix / decline / file-an-issue for each
valid finding, with reasoning — processing is not fixing. Then judge the
change as a whole:

| Verdict | Meaning |
| --- | --- |
| `SUFFICIENT` | purpose achieved; nothing remaining justifies blocking the merge |
| `INSUFFICIENT` | a correctness, safety, or acceptance-criteria problem remains |
| `UNCERTAIN` | agent-available evidence cannot settle it; a human decides |

Sufficient does not mean zero findings: purpose and acceptance criteria met,
no credible correctness/security/regression concern, required verification
passing, and no remaining finding worth fixing in this PR rather than filing.

Before the verdict, answer: purpose met? credible defect open? CI green?
must any remainder land in this PR? is a proposed addition a fix or taste?
and — the loop-ender — **what concretely breaks if this merges now?** A
finding that cannot answer the last question does not block, however
reasonable it sounds. `SUFFICIENT`: end the iteration, state the rationale.
`INSUFFICIENT`: fix and re-judge. `UNCERTAIN`: ask the maintainer.

## Reviewer Commands

- `@coderabbitai review` is not a routine step: it starts a full re-review
  over unchanged code — fuel for the loop. Reserve it for a push that
  genuinely changes direction or scope.
- After `SUFFICIENT`, clear a stale `CHANGES_REQUESTED` with
  `@coderabbitai approve` (explicit write authorization as always). Approval
  is the outcome of the judgment, never the goal; do not use it to bypass
  `INSUFFICIENT`.

## Output

Report per class: addressed, declined and why, filed issues, validation
results, whether GitHub writes were performed — and the sufficiency verdict
with its rationale, anchored on what would concretely break by merging now.
Respond in the user's language.
