---
name: deliver-pull-request
description: Deliver a focused HardwareVisualizer change as a pull request, then address its CI and review feedback to completion. Use when the user explicitly asks to create or publish a PR, or to finish an existing PR.
---

# Deliver Pull Request

Publish one coherent change and finish the CI and review work that belongs to
it. Do not merge unless the user explicitly asks.

## 1. Confirm The Boundary

A request to create or publish a PR authorizes the branch, commit, push, PR,
review replies, and thread resolution needed to deliver that change. It does
not authorize merge, destructive Git operations, or unrelated changes.

Before editing, apply
[AGENTS.md](../../../AGENTS.md#change-justification-and-simplicity) and be able
to explain:

- why the change is needed now;
- what must change;
- how the smallest coherent solution works; and
- why plausible alternatives are unnecessary or worse for this requirement.

Do not create mandatory decision paperwork. Preserve a non-obvious decision
only at the smallest durable owner that will need it.

## 2. Keep The Change Focused

- Inspect the current branch, worktree, base, and associated PR.
- Preserve unrelated user changes. Use an isolated worktree when necessary.
- Implement only the current requirement. Keep adjacent findings separate.
- Add a focused regression test when it can prove the changed contract.
- Allow complexity only when the current requirement or an established
  ownership boundary requires it.

## 3. Publish The Pull Request

Read and use [change-kind-naming](../change-kind-naming/SKILL.md). Follow
`CONTRIBUTING.md` and the PR template.

1. Confirm the branch contains only the intended change.
2. Run focused checks, then broader checks only when the blast radius requires
   them.
3. Review the complete diff and stage only intended paths.
4. Create a Conventional Commit and push the project-prefixed branch.
5. Create or update the PR in the Ready or Draft state requested by the user.
6. Report the PR URL, scope, validation, and any preserved unrelated work.

Prefer an available GitHub connector. The `gh` CLI cannot complete GitHub
operations inside the project sandbox; when it is needed, run the explicit
operation in the permitted environment with the required approval.

## 4. Finish CI And Review

Separate discovery from verification so review converges on approval.

### Primary Review

1. Let the configured automatic reviewers perform one primary review of the
   published change.
2. Collect its findings before editing and keep a working record of each claim,
   evidence, decision, and owning boundary.
3. Triage all accepted findings together and make one focused correction batch.

The primary review is the only broad review. Do not request another full review
after corrections.

For review feedback, read and use
[gh-ai-review-triage](../gh-ai-review-triage/SKILL.md):

- Treat a comment as evidence of a possible problem, not as a prescribed
  patch.
- Implement a suggestion only when all of these are true: the problem is
  supported by code, tests, CI, or a canonical decision; it breaks the current
  requirement's correctness or security, or makes the changed boundary unclear
  or brittle for a presently required case; the PR owns that boundary; and a
  smallest coherent fix is identified.
- Decline when the claim is false, stale, duplicate, outside the current
  requirement without affecting its contract, or owned by another boundary
  that this requirement does not need to change. Reply with the verified
  evidence, scope decision, and responsible boundary.
- Do not use low implementation cost, reviewer preference, or possible future
  value to justify implementation.
- If the evidence is insufficient, or a verified risk is real but outside the
  current PR's responsibility, do not guess, dismiss it, or add a defensive
  patch. Ask the maintainer for a decision.
- Keep accepted corrections narrow, run the relevant checks, reply with the
  decision and evidence, and resolve the thread.
- Never request a Codex review manually. Codex decides when to review.

### Verification Reviews

After the correction batch and relevant CI pass:

1. Request one CodeRabbit incremental review with `@coderabbitai review`.
2. Treat that review as verification of the primary-review decisions and the
   correction diff, not as a new broad review.
3. Accept a new finding only when it proves that a primary finding remains
   unresolved or that the correction introduced a regression. Reply and defer
   unrelated discovery instead of expanding the PR.
4. If another correction is required, repeat the focused validation and one
   incremental review. Never use `@coderabbitai full review`.
5. Once threads are resolved and CI passes, use `@coderabbitai approve` when
   needed and confirm GitHub records approval.

If two consecutive verification reviews fail to reach approval, stop automatic
correction. Report the unresolved claims, evidence, and decisions to the
maintainer instead of starting a third implementation cycle.

For a failing check, inspect the failing leaf job and exact error before
editing. Separate product regressions from test and environment failures, and
fix only an in-scope cause.

## Completion Gate

Stop when:

- the requested change is complete and contains no unrelated work;
- relevant local checks and CI pass;
- primary-review feedback is fixed or declined with evidence, and its threads
  are resolved;
- GitHub has no outstanding change request and records approval from the
  approval-capable reviewer; and
- the PR is in the requested publication state without a merge conflict.

Do not seek more findings merely for additional certainty. A verification
review does not reopen broad discovery. If permission, approval, a required
gate, or an external service prevents completion, report the concrete blocker
and the evidence already completed.
