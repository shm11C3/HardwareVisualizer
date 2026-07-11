---
name: capture-project-learning
description: Turn a HardwareVisualizer maintainer correction, repeated failure, surprising invariant, or costly investigation into an evidence-backed learning record and the right durable guardrail. Use when asked to record learnings, prevent the same AI mistake, update AGENTS/rules/hooks/skills, or when completed work reveals a reusable repository-specific lesson.
---

# Capture Project Learning

## Goal

Convert experience into a small durable improvement without turning chat
history into always-on context.

The lifecycle is:

```text
observe -> verify -> record -> promote -> enforce -> revalidate
```

## Workflow

### 1. Decide Whether It Is A Learning

Record one when at least one is true:

- the maintainer corrected an assumption or product interpretation;
- the same CI, review, environment, or implementation failure repeated;
- investigation found a non-obvious invariant or evidence path;
- an undocumented design decision materially changed implementation;
- a manual check can become a deterministic regression guard.

Do not record a guess, secret, credential, personal absolute path, temporary
check state, or generic software-engineering advice.

### 2. Search Before Adding

Search `docs/agents/lessons/`, `docs/design-principles.md`, `CONTEXT.md`, ADRs,
architecture docs, scoped instructions, skills, tests, and CI. Update or
supersede an existing lesson instead of creating a duplicate.

### 3. Verify The Cause

Confirm the observation against current evidence. Prefer current code/tests,
leaf-job logs, runtime/SQLite data, rendered artifacts, release assets, and
current GitHub state. If the cause is not confirmed, record a `candidate` and do
not promote it as a rule.

Separate the durable invariant from time-specific evidence such as a dependency
version, PR number, runner timing, or spec revision.

### 4. Add One Learning Record

Create one file under `docs/agents/lessons/` using the required frontmatter in
that directory's README. Use an ID of the form `LRN-YYYYMMDD-short-slug`, update
the records index, and state exactly when the lesson must be revalidated.

### 5. Promote To The Real Owner

Use this routing:

- vocabulary -> `CONTEXT.md`;
- cross-cutting decision lens -> `docs/design-principles.md`;
- specific trade-off -> ADR;
- current ownership/structure -> architecture doc or owner README;
- always-on AI constraint -> root or scoped `AGENTS.md`;
- path-specific AI constraint -> `.agents/rules/**`;
- repeated multi-step procedure -> `.agents/skills/**`;
- deterministic invariant -> test, non-mutating script, hook, or CI;
- expiring/environment fact -> learning record only.

Keep hooks cheap and deterministic. They may validate paths, schemas, links,
generated-file edit attempts, or exact dependency invariants. They must not
infer product meaning, clean-room contamination, or change kind.

### 6. Validate

Run:

```bash
npm run check:agent-guidance
git diff --check
```

Run any new focused regression test or script. Inspect the diff for duplicated
or conflicting guidance.

## Completion Criteria

A learning is complete when:

- its cause is labeled confirmed or candidate honestly;
- its durable rule has one canonical owner;
- AI entry points link to, rather than duplicate, detailed facts;
- deterministic behavior is enforced by a test/script/CI where practical;
- the record says when it can expire or must be revalidated.
