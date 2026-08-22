# Classification And Verification Detail

## Classes

- `Required`: correctness bug, build failure, reproducing lint warning, test
  failure, security/soundness issue, or behavior conflicting with the user
  request. Only after verification; if verification is infeasible, classify
  `Should Fix` tagged "needs verification".
- `Should Fix`: non-blocking but clearly correct — misleading code,
  unrealistic tests, stale comments, fragile behavior, likely follow-up.
- `Worth Fixing`: small low-risk improvement to clarity, test realism,
  hot-path performance, user-visible behavior, or maintainability.
- `Optional`: nitpick, style preference, suggestion outside project
  requirements, or UX improvement with a fallback already in place.
- `Ignore`: outdated, resolved, duplicate, incorrect after verification,
  unrelated, or conflicting with a binding constraint without new evidence.

## Verifying

- Check the proposed edit against binding constraints (user request, explicit
  maintainer corrections, canonical decisions) before its local correctness:
  a plausible implementation can be the wrong product behavior or scope.
- Reproduce claimed CI-impacting issues locally when feasible; inspect the
  referenced code, not only the bot wording; run the relevant check before
  treating a "warnings-as-errors" style claim as required.
- A comment conflicting with a binding constraint: preserve the constraint
  and report the conflict. New correctness/safety evidence goes back to the
  maintainer rather than silently overriding direction.

## Heuristics

- Bot approvals and overview comments usually need no code change.
- CodeRabbit "Nitpick" sections are optional; promote to `Worth Fixing` only
  when clearly safe and directly improving the PR.
- "Consider ..." wording is usually optional unless it points at a real bug,
  misleading test/comment, measurable hot-path cost, or clear low-risk value.
- A stale or incorrect bot claim: call it out briefly, leave the code alone.
- For Rust, local `cargo clippy` output outweighs an AI claim about it.
