---
scope: "**"
---

# Design And Evidence Instructions

For product, architecture, persistence, monitoring, or UX decisions, read
`docs/design-principles.md`, `CONTEXT.md`, the relevant accepted ADR, and the
nearest scoped `AGENTS.md`.

- State which product claim and design principle the change serves.
- Decide who owns the fact, policy, presentation, lifecycle, and persistence.
- Keep live, short-window, archive, daily-record, and UI-local lifetimes clear.
- Preserve partial capability, explicit unavailable states, and explicit user
  intent.
- Make collection/rendering cost follow visible or explicit background value.
- Match evidence to the claim; inspect rendered output for visual claims.
- Use the issue/request as the scope anchor and separate adjacent work.
- If a specific exception is needed, add or update an ADR rather than silently
  crossing a documented boundary.

Memory, handoffs, lessons, and aggregate CI status are leads. Verify current
code, specs, tests, leaf-job logs, runtime/DB data, or GitHub state before using
them as facts.
