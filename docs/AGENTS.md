# Documentation Instructions

These instructions add to the repository root `AGENTS.md` for work under
`docs/`.

- `docs/design-principles.md` owns the cross-cutting product and design lens.
- `CONTEXT.md` owns product vocabulary and avoided aliases; do not turn it into
  an architecture or workflow document.
- `docs/adr/**` owns specific decisions and consequences. Check ADR status before
  treating a proposed decision as shipped behavior.
- `docs/architecture/**` and owner README files describe the current structure.
- `docs/agents/lessons/**` records evidence and provenance; a lesson does not
  override canonical product or architecture docs.
- Follow `docs/documentation-guide.md` for placement and naming, and update
  `docs/README.md` when adding a discoverable document.
- Verify behavior in current code, tests, PR diffs, or runtime evidence before
  documenting it. Keep temporary branch/check state out of durable guidance.

For `docs/specs/sensors/**`, the clean-room role, source, provenance, status,
and PR gates are mandatory. Read the local spec README and
`.agents/rules/clean-room-sensors.md` before any work.

Read `.agents/rules/documentation.md` for documentation changes and
`.agents/rules/design.md` when documentation records a product or architecture
decision.
