---
name: change-kind-naming
description: Decide the correct change kind and align branch names, PR titles, and commit prefixes. Use when choosing Conventional Commit prefixes, branch prefixes, PR titles, or PR template change types, especially when a change could be fix/refactor/chore/docs/feat.
---

# Change Kind Naming

## Quick Start

Before committing, pushing, or opening a PR, classify the actual change type and make these four surfaces agree:

- commit subject prefix
- PR title prefix
- branch prefix
- PR template "Type of Change"

If they disagree, stop and fix the naming before publishing.

## Classification Rules

Use `fix` only when the change corrects externally observable broken behavior, user-facing wrong output, a failing workflow, data loss, security exposure, or a regression.

Use `refactor` when the change preserves behavior but improves internal structure, terminology, names, boundaries, generated bindings, or code organization.

Use `feat` when the change adds a new user-visible capability or supported workflow.

Use `docs` when the shipped code behavior does not change and the diff is documentation-only.

Use `test` when the diff only adds or changes tests.

Use `ci` when the diff only changes CI/CD automation.

Use `chore` for maintenance that is not product behavior, architecture, docs, tests, or CI.

## Tie-Breakers

Prefer `fix` over `refactor` when the old state caused a real user or integration failure.

Prefer `refactor` over `fix` when the old state was confusing, inconsistent, or poorly named but still worked.

Prefer `feat` over `refactor` when users can now do something they could not do before.

Prefer `docs` over `refactor` when only docs changed, even if the docs clarify architecture.

Prefer the highest-impact type when one PR intentionally mixes types. If the mixed change is accidental, split it.

## Naming Alignment

Use branch prefixes that match the final classification:

- `fix/...`
- `refactor/...`
- `feat/...`
- `docs/...`
- `test/...`
- `ci/...`
- `chore/...`

Use the same prefix in the Conventional Commit subject and PR title:

```text
refactor: rename hardware archive retention setting
```

In the PR template, check the matching Type of Change. Do not leave `Bug fix` checked for a `refactor:` PR.

## Pre-Publish Checklist

Run this before `git commit`, `git push`, or `gh pr create`:

- Does the change fix broken behavior? If no, avoid `fix`.
- Does the change preserve behavior while renaming or reorganizing? Use `refactor`.
- Do branch, commit, PR title, and PR template all agree?
- Does the PR body explain compatibility separately from behavior?
- If the branch prefix is wrong and the PR is already open, prefer renaming before review starts. If renaming would close or disrupt the PR, weigh whether updating title/body/commit is enough.

## Example

Renaming `refreshIntervalDays` to `retentionDays` while preserving compatibility is `refactor`, not `fix`, because the old setting worked but used misleading terminology.

Use:

- branch: `refactor/hardware-archive-retention-naming`
- commit: `refactor: rename hardware archive retention setting`
- PR title: `refactor: rename hardware archive retention setting`
- PR type: `Refactoring`
