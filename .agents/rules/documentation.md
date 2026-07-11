---
scope: "README.md,README.ja.md,CONTRIBUTING.md,CODE_SIGNING_POLICY.md,SECURITY.md,CONTEXT.md,docs/**,.github/**/*.md"
---

# Documentation Instructions

Follow `docs/documentation-guide.md` and the scoped `docs/AGENTS.md`.

## Ownership

- Root README files own the user-facing product, installation, and distribution
  entry points.
- `docs/design-principles.md` owns the cross-cutting product/design lens.
- `CONTEXT.md` owns product terms and avoided aliases only.
- `docs/adr/**` owns specific decisions, status, rationale, and consequences.
- `docs/architecture/**` and owner README files describe current structure.
- `docs/agents/lessons/**` records evidence and provenance; it is not canonical
  until a lesson is promoted to its owning surface.
- `.agents/rules/**` contains concise path-specific AI rules. Link to the
  reason instead of duplicating it.

## Writing And Verification

- Be factual and technical. Prefer short paragraphs, lists, and exact commands.
- Verify behavior in current code, tests, PR diffs, logs, or runtime evidence.
- Keep temporary branch, PR, check, and version state out of durable rules unless
  it is historical evidence with a revalidation condition.
- Update `docs/README.md` when adding a discoverable document.
- Keep English/Japanese user-facing README headings and feature claims aligned.
  Do not duplicate internal architecture changes into user docs unless they
  affect the product contract.
- Preserve clean-room provenance and role boundaries in sensor specs. Those
  rules override general documentation convenience.
