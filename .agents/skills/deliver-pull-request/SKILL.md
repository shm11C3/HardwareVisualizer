---
name: deliver-pull-request
description: Deliver a HardwareVisualizer change as a focused GitHub pull request and continue through CI and review completion. Use when the user explicitly asks to create, open, or publish a PR, or asks Codex to handle an existing PR's feedback and checks through approval.
---

# Deliver Pull Request

Deliver one coherent requirement from the current worktree to its intended
Ready or Draft pull request state with relevant checks and reviews complete. Do
not merge unless the user explicitly asks.

## Authorization Boundary

Use this workflow only when the current request explicitly authorizes PR
publication or follow-through on an existing PR. A request to create, open, or
publish a PR authorizes the in-scope branch, commit, push, PR creation, review
replies, review reactions, and thread resolution required by this workflow. It
does not authorize merge, destructive Git operations, or unrelated changes.

If the user asks only to prepare PR content, propose a plan, inspect status, or
triage feedback, do not perform Git or GitHub mutations beyond that request
without confirmation. Host approval requirements still apply to every tool
call.

## 1. Establish The Change Contract

Read [AGENTS.md](../../../AGENTS.md#change-justification-and-simplicity), the
issue or explicit request, and the smallest relevant decision sources. Before
editing, state or be able to state:

- Why the change is necessary now.
- What behavior or contract must change.
- How the smallest coherent solution satisfies it.
- Why plausible alternatives are unnecessary or worse for the current scope.

Do not use low cost, reviewer preference, future extensibility, or nearby
cleanup as justification. Allow complexity only when the current requirement,
correctness, platform behavior, or established ownership boundary demonstrates
the need.

## 2. Protect Scope And Existing Work

- Inspect the current branch, base, worktree status, and existing PR before
  editing, including the PR's Ready or Draft state and whether auto-merge is
  already enabled.
- Reuse a PR only when it is open and unmerged, its head repository and branch
  are the branch being pushed, its base repository and branch are the intended
  target, and it represents the current requirement. Otherwise stop and ask
  instead of inferring a replacement or update target.
- If an existing PR has auto-merge enabled and the user did not authorize
  merge, stop before driving its checks or reviews to completion. Ask whether
  to disable auto-merge; do not change that existing intent without
  confirmation.
- If an existing Ready PR is authored by
  `hardwarevisualizerappmanager[bot]` or `dependabot[bot]` and merge is not
  authorized, stop before pushing because synchronization enables auto-merge.
  Ask whether to convert it to Draft. Keep an existing Draft by either author
  Draft without merge authorization; a request to mark it Ready is not merge
  authorization because that event also enables auto-merge.
- Preserve unrelated user changes. Use an isolated worktree when another task
  or dirty worktree would contaminate the PR.
- Implement only the current requirement. Keep adjacent findings separate.
- Prefer direct code with a clear owner. Do not trade simplicity for brittle
  behavior, avoidable duplication, or unclear responsibility.
- Add a focused regression test at the boundary that owns the claim when
  practical. Scale broader validation with blast radius.

## 3. Publish The Focused Pull Request

Read and use
[change-kind-naming](../change-kind-naming/SKILL.md) before publication. Follow
`CONTRIBUTING.md` and the repository PR template.

1. Refresh the intended base and confirm the branch contains only the scoped
   change.
2. Run focused checks first, then any broader checks justified by the changed
   ownership boundaries.
3. Review the complete diff and stage only intended paths.
4. Create a Conventional Commit with a body when the reason or compatibility is
   not self-evident.
5. Push the project-prefixed branch. Create a PR only when no eligible existing
   PR identified above covers the change; use Ready for a new PR unless the user
   asks for Draft. If the selected connector authors a new PR as
   `hardwarevisualizerappmanager[bot]` or `dependabot[bot]`, create it as
   Draft unless merge is authorized. When updating an eligible existing PR,
   preserve its current Ready or Draft state unless the user confirms a change.
6. Report the PR URL, committed scope, validation evidence, and preserved
   unrelated changes.

Prefer an available GitHub connector for GitHub state and mutations. The `gh`
CLI cannot run inside the project sandbox. When `gh` is needed, run the explicit
operation outside the sandbox with the required approval. Do not interpret a
sandbox failure as invalid authentication or a GitHub outage without checking
the same operation in the permitted environment.

## 4. Complete CI And Review

Do not stop at PR creation or push. Monitor the failing leaf jobs, required
checks, review decision, unresolved threads, and mergeability until the
completion gate is satisfied.

For review feedback, read and use
[gh-ai-review-triage](../gh-ai-review-triage/SKILL.md), with these binding
constraints:

- Use review to protect the simplest sufficient design and to assess failure
  observability, ownership clarity, and focused regression coverage. Finding
  count and adopted-suggestion count are not quality metrics. Do not use review
  as the primary defect-discovery mechanism.
- Treat every comment as evidence of a possible problem, not as a requested
  patch.
- Verify the essential issue, root cause, current-scope risk, and owning
  boundary before changing code.
- Accept only feedback needed for the current requirement. Reply with the
  decision and evidence for both accepted and declined comments, then resolve
  the thread.
- When a Codex comment asks whether it was useful, react only after deciding:
  use a positive reaction for an accepted finding and a negative reaction for
  an evidence-backed decline. The reaction does not replace the reply or thread
  resolution.
- Keep each correction narrow and add or update a focused regression test when
  it can prove recurrence.
- Treat each automated review as an independent full review, not merely
  validation of the last correction.
- Rely on Codex to review automatically when it determines review is needed.
  Never request a Codex review manually, including after feedback commits or
  material changes.
- For other configured automatic reviewers, do not manually request another
  review after every feedback commit. Re-request only when a material,
  previously unreviewed change makes the prior review stale, or when a human
  explicitly asks.

For CI failures, capture the exact leaf error and determine whether it is a
product regression, test defect, or environment failure before editing. Fix
only in-scope causes, rerun the focused evidence, push, and continue monitoring.

## Completion Gate

Stop the review loop when all of the following are true:

- the current requirement is satisfied without unrelated changes;
- focused regression tests and relevant CI checks pass;
- required or configured reviews applicable to the intended publication state
  complete without outstanding changes;
- every actionable review item is addressed or explicitly declined with
  evidence; inline threads are resolved, and non-thread items have the required
  reply and decision;
- for publication or follow-through without a merge request, the PR is in the
  intended publication state defined above, is mergeable, has no unresolved
  conflict, and does not have auto-merge enabled;
- when the user explicitly requested merge, GitHub reports the PR as merged and
  provides its merged timestamp.

Do not request more review merely for additional certainty or more findings.
If an external permission, unavailable reviewer, or persistent environment
failure prevents completion, report the exact blocker and the evidence already
completed. Do not weaken the gate or broaden the implementation to escape it.
