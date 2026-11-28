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
[gh-ai-review-triage](../gh-ai-review-triage/SKILL.md). It never merges,
approves its own specification, or edits `.claude/settings*`, permission modes
or CLAUDE.md.

A specification is implementable only after the maintainer approves the PR
that changes its status, and that status reads exactly
`Implementation-ready (rev N)`
([specification gate](../../../docs/specs/sensors/README.md#status-transition-draft--implementation-ready)).
The status string alone is not a sign-off. Implementation may start earlier
when the maintainer asks for it. In that case, open the implementation as a
Draft PR stacked on the specification PR, and mark it ready only after the
specification PR is approved or merged.

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
| Rust implementation | 3 | `sensor-clean-room-implementer` |

Each Issue has at most one active lane. One PR covers one scope item
(one specification or one implementation layer). Stack dependent PRs.

## Worktrees

Only one session may write to a worktree. Follow this rule in every tool. The
repository hooks do not enforce it. Claude Code's worktree isolation does
reject Edit and Write calls on any worktree other than the session's own.

1. Before spawning, put the orchestrator's worktree on a branch at
   `origin/develop`, because isolated worktrees fork from its HEAD.
2. Launch every agent that writes files with isolated worktrees. A worktree
   the orchestrator creates with `git worktree add` is not writable by a
   subagent.
3. For stacked work, the agent's first command merges the base,
   `git merge --no-edit origin/<base>`, and the agent never creates or switches
   branches.
4. The orchestrator commits if the agent could not. It pushes with
   `git -C <wt> push origin HEAD:refs/heads/<branch>`. Use the
   [change-kind-naming](../change-kind-naming/SKILL.md) skill to pick the
   branch name.
5. Never copy files into another session's worktree with shell commands.
   Send follow-up edits to the owning agent.
6. After a parent PR is squash-merged, rebuild each stacked branch from
   `origin/develop` by cherry-picking its own commits. Do not rebase a stack
   onto the squash.

## Tick

1. **Inventory.** Collect the #2510 checklist and child Issues, open sensor
   PRs with their checks and threads, running agents, and specs whose PRs were
   merged since the last tick.
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
     the pinning statement and attestation the implementer reports.
3. **Handle review feedback.** Use triage, not blind application. Send each
   fix to the owning role, and verify the fix before you reply and resolve the
   thread.
4. **Start implementations.** Fill free Rust lanes from implementable
   specifications. Brief the implementer with the pinned specs only.
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
