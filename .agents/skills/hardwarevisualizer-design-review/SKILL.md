---
name: hardwarevisualizer-design-review
description: Review or shape HardwareVisualizer product and architecture changes against the maintainer's design principles. Use when planning, implementing, or reviewing behavior that affects hardware availability, collection cost, live/history semantics, settings, persistence, user selection, Core/App ownership, optional components, privacy, or user-visible evidence.
---

# HardwareVisualizer Design Review

## Load The Decision Context

Read only the context relevant to the change:

1. `docs/design-principles.md`.
2. The applicable terms in `CONTEXT.md`.
3. The nearest scoped `AGENTS.md` and owner README.
4. Relevant ADRs, including their status.
5. Current code/tests/runtime evidence for the claimed behavior.

Treat lessons, handoffs, issue comments, and AI memory as leads. Verify them
against the current branch and canonical sources before using them as facts.

## Review Workflow

### 1. Frame The Product Claim

State the user-visible outcome in one sentence. Name the issue or explicit
request that anchors scope. Separate adjacent cleanup or features unless they
are necessary for the same claim.

### 2. Map Ownership

Identify who owns each part:

- hardware fact and collection;
- product policy and fallback;
- presentation and wire conversion;
- interaction/view state;
- lifecycle and background work;
- persisted value and migration.

Reject designs that move Tauri into Core, OS access into commands/frontend, or
Application Preferences into frontend Tauri Store for convenience.

### 3. Check Semantics

Ask:

- Is the value live, short-window, archived, daily-record, or UI-local?
- Is the subject automatic focus, current data, or explicit user selection?
- Are the availability/validity states defined by this domain preserved rather
  than flattened into a generic status model?
- Does a partial provider/device failure preserve useful results?
- Does uncertainty avoid destructive deactivation or data loss?

Use existing `CONTEXT.md` terms. Propose a glossary change before inventing a
new synonym for an existing concept.

### 4. Check Cost And Optionality

Confirm collection/rendering cost follows visible or explicit background value.
Do not stop legitimate archive/tray work merely because the main window is
hidden.

For an optional component, verify it was attempted and all useful fallbacks are
insufficient before showing guidance. Do not label unsupported hardware as a
missing installation.

### 5. Check User Intent And Privacy

Explicit user choices must survive automatic refresh while they remain valid.
If a selected subject is temporarily absent, use a coherent fallback without
unnecessarily deleting the stored intent. Only choices classified as
Application Preferences are required to survive restart as app configuration;
UI-local state may be reset.

Persist only the local identifiers needed for the feature and do not introduce
telemetry or portable identity without an explicit product decision.

### 6. Define Proof Before Editing

Choose evidence that proves the actual claim:

- focused unit/integration contract;
- runtime log or SQLite state;
- web/mock UI behavior;
- native Tauri integration;
- rendered screenshot at relevant viewports;
- measured performance/CI timing;
- release artifact/signature inspection.

Do not substitute one evidence class for another.

## Decision

Return one of these outcomes before or alongside implementation:

- `Aligned`: existing principles and ownership cover the change.
- `Aligned with guardrail`: proceed with a named test, rule, or migration.
- `ADR required`: a specific trade-off or exception needs a recorded decision.
- `Clarification required`: canonical sources conflict or product intent is not
  discoverable.
- `Not aligned`: explain the violated principle and the smallest viable design
  correction.

When implementation is requested and the decision is aligned, continue through
the change and validation. Do not stop at the review summary.
