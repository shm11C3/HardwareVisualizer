---
name: sensor-coverage-loop
description: Continuously advance the read-only hardware coverage epic (#2510) by orchestrating sensor spec-author and clean-room implementer agents in parallel lanes, one tick at a time. Use when asked to keep working through sensor coverage Issues, run the coverage loop, or resume it in a new session.
---

# Sensor Coverage Loop

Advance [#2510](https://github.com/shm11C3/HardwareVisualizer/issues/2510)
without a fixed goal. Each tick reconstructs state from GitHub and the running
agents, harvests finished work into pull requests, and refills idle lanes with
the next most valuable Issue. The loop has no state file.

## Running It

- Claude Code: `/loop Follow .agents/skills/sensor-coverage-loop/SKILL.md`.
  Leave out the interval so the loop paces itself. Agent completion
  notifications wake the loop, so use a 20–30 minute fallback wakeup.
- Other agents: run one tick per invocation and repeat.
- Stop when the maintainer asks, or when every lane is waiting on the
  maintainer. In that case, report what is waiting.

## Boundary

The loop may create branches, commits, pushes and pull requests. It may also
reply to and resolve review threads, following
[deliver-pull-request](../deliver-pull-request/SKILL.md) and
[gh-ai-review-triage](../gh-ai-review-triage/SKILL.md). It never edits
`.claude/settings*`, permission modes or CLAUDE.md.

Merging follows the maintainer's policy. The loop may merge a
documentation-only PR, such as a specification, a skill or guidance, once
all of the following hold:

- every thread is resolved;
- CI is green;
- CodeRabbit's review state is APPROVED. Request it with
  `@coderabbitai approve` after the feedback is addressed.

Merge with a squash, which is the repository convention. The loop never
merges a PR that changes code or tests.

A specification is implementable only after its status-change PR has been
signed off and its status reads exactly `Implementation-ready (rev N)`
([specification gate](../../../docs/specs/sensors/README.md#status-transition-draft--implementation-ready)).
The status string alone is not a sign-off. Do not dispatch an implementer and
do not open any implementation PR, Draft or ready, before that sign-off
([implementation PR requirements](../../rules/clean-room-sensors.md#implementation-pr-requirements)).
While a specification waits for sign-off, use its Rust lane for another
specification that is already implementable.

## Roles

Follow [clean-room-sensors](../../rules/clean-room-sensors.md).

- The orchestrator (the loop session) dispatches, verifies with repository
  tools, pushes and opens PRs. It never reads prohibited sources, so it stays
  able to review implementation PRs.
- Spec work runs in the `sensor-spec-author` agent and implementation in the
  `sensor-clean-room-implementer` agent (`.claude/agents/`). Never give one
  agent both roles.
- Review findings about hardware facts go back to a spec-author agent.
  Findings about code go back to an implementer agent.

## Lanes

| Lane | Limit | Agent |
| --- | --- | --- |
| Specification research | 5 | `sensor-spec-author` |
| Rust implementation (running implementers plus open implementation PRs) | 5 | `sensor-clean-room-implementer` |
| Documentation PRs opened per day | 15 | orchestrator |

Open documentation PRs have no concurrency limit, but open at most 15 per
day. When the daily limit is reached, keep finished spec work in its agent
worktree until the next day. Do not refill research lanes while finished
but unpublished spec work is waiting.

Each Issue has at most one active lane. One PR covers one scope item
(one specification or one implementation layer). Stack dependent PRs.

## Worktrees

Only one session may write to a worktree. Follow this rule in every tool. The
repository hooks do not enforce it. Claude Code's worktree isolation does
reject Edit and Write calls on any worktree other than the session's own.

Run `git fetch origin` before any step below that reads an `origin/` ref.

1. Before spawning, put the orchestrator's worktree on a branch at
   `origin/develop`, because isolated worktrees fork from its HEAD.
2. Launch every agent that writes files with isolated worktrees. A worktree
   the orchestrator creates with `git worktree add` is not writable by a
   subagent.
3. For stacked work, the agent first runs
   `git fetch origin && git merge --no-edit origin/<base>`, and never creates
   or switches branches.
4. The orchestrator commits if the agent could not. It pushes with
   `git -C <wt> push origin HEAD:refs/heads/<branch>`. Use the
   [change-kind-naming](../change-kind-naming/SKILL.md) skill to pick the
   branch name.
5. Never copy files into another session's worktree with shell commands.
   Send follow-up edits to the owning agent.
6. After a parent PR is squash-merged, fetch, then rebuild each stacked
   branch from the refreshed `origin/develop` by cherry-picking its own
   commits. Do not rebase a stack onto the squash.

## Tick

1. **Inventory.** Collect the #2510 checklist and child Issues, open sensor
   PRs with their checks and threads, running agents, and every implementable
   specification that has no implementation lane yet, not only the ones whose
   PRs merged since the last tick. Also collect finished work that has no open
   PR: commits ahead of `origin` in agent worktrees (`git worktree list`),
   uncommitted changes left in an agent worktree whose agent has stopped
   (for a spec, commit them yourself after the harvest checks pass, as
   Worktrees rule 4 says; for an implementation, resume the owning
   implementer to commit with its pinning statement and attestation in the
   commit message, and if it cannot be resumed, do not harvest that work but
   start a fresh implementation lane from the pinned spec, because no other
   agent can attest to it), and pushed branches without a PR. Skip a candidate only when its change itself is already merged,
   judged per change rather than per branch: a commit is merged when its
   branch PR merged with no later commits (`gh pr list --state merged --head
   <branch>`) or the same change is already on `origin/develop`, and
   uncommitted changes are merged only when that diff is already on
   `origin/develop`. Squash-merged commits still look ahead of `origin`, and
   new work on a merged branch must still be harvested. Harvest the rest in
   step 2 when the daily documentation PR limit or the implementation lane
   limit allows, before refilling any lane.
2. **Harvest finished agents.**
   - For a spec: run `git diff --check`, check that there are no
     `TODO(provenance)` markers, check that the status, revision history and
     README rows were updated together, and run
     `node .agents/tooling/check-agent-guidance.mjs`.
   - For an implementation: rerun the CI-equivalent fmt, clippy with
     `--all-targets`, and the focused tests.
   - Push, then open a PR. Spec PRs use the standard template and list the
     points that need maintainer review. Implementation PRs use
     `.github/PULL_REQUEST_TEMPLATE/clean-room-sensor-implementation.md` with
     the pinning statement and attestation from the feature commit message.
     If a harvested branch's commits do not carry them, send the branch back
     to an implementer instead of opening the PR.
3. **Handle review feedback.** Use triage, not blind application. Send each
   fix to the owning role, and verify the fix before you reply and resolve the
   thread.
4. **Start implementations.** Fill free Rust lanes from implementable
   specifications. Brief the implementer with the pinned specs only, and
   require it to write the pinning statement ("Implemented from
   docs/specs/sensors/<file> revision N (commit <sha>)" lines) and the
   implementer attestation in the feature commit message body, so they stay
   with the branch.
5. **Refill research lanes.** Pick by the priority rubric below. Brief with
   the Issue, the related specifications and ADRs 0011, 0023 and 0031. Ask for
   the smallest implementable slice, or a precise Draft with blockers, and an
   explicit disposition for every target in the Issue.
6. **Report.** State what was harvested and opened, what is running, and what
   waits on the maintainer.

## Priority Rubric

The wave labels in #2510 are an engineering starting order, not a demand
estimate. Rank open Issues and follow-ups by:

1. **Installed base.** For example: mainstream board vendors and Super I/O
   families, current CPU generations, and popular coolers.
2. **User-visible value.** A new sensor class ranks above an incremental
   channel.
3. **Feasibility.** Consider primary-source availability, whether an existing
   signed transport can be reused, and whether known hazards have safety gates.
4. **Cost to deliver an initial slice.** A follow-on that reuses a mechanism
   that has already been specified is cheap.

An Issue is consumed when its initial slice merges and a linked follow-up
Issue tracks the remaining targets. Never claim LibreHardwareMonitor parity.

## Known Pitfalls

- Auto mode can refuse branch creation inside an agent. The orchestrator then
  runs the Git operation.
- A spec-author's web research does not authorize the orchestrator to read the
  same sources.
- Documentation that tells users what works, such as
  `docs/user/hardware-compatibility.md`, changes with the implementation PR,
  not with the spec PR.
