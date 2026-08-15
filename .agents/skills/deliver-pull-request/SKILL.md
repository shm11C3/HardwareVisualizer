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

After publication, inspect the PR's current CI results and review feedback. Do
not repeatedly request new reviews after each correction.

For review feedback, read and use
[gh-ai-review-triage](../gh-ai-review-triage/SKILL.md):

- Treat a comment as evidence of a possible problem, not as a prescribed
  patch.
- Verify the root cause, current-scope risk, and owning boundary before editing.
- Accept only feedback needed for the current requirement. Otherwise reply
  with the reason for declining it.
- Keep accepted corrections narrow, run the relevant checks, reply with the
  decision and evidence, and resolve the thread.
- Never request a Codex review manually. Codex decides when to review.
- Rely on configured automatic reviewers. Request another review only when a
  human explicitly asks or a material unreviewed change makes the existing
  review obsolete.

For a failing check, inspect the failing leaf job and exact error before
editing. Separate product regressions from test and environment failures, and
fix only an in-scope cause.

## Completion Gate

Stop when:

- the requested change is complete and contains no unrelated work;
- relevant local checks and CI pass;
- all existing actionable feedback is fixed or declined with evidence, and its
  threads are resolved; and
- the PR is in the requested publication state without a merge conflict.

Do not seek more findings merely for additional certainty. If permission or an
external service prevents completion, report the concrete blocker and the
evidence already completed.
