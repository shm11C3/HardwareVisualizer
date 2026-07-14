---
id: LRN-20260714-navigation-levels-explicit
status: promoted
cause_status: confirmed
scope: product navigation, dashboard information architecture, issues, ADRs, frontend routes, and E2E
trigger: a requirement groups screens, tabs, categories, or sections under one user-facing label
failure_signature: a single Dashboard with Performance and System Specifications tabs was implemented as a Performance sidebar route plus standalone hardware-category screens
root_cause: the issue and accepted ADR named grouped content without explicitly distinguishing sidebar destinations, in-screen tabs, and content sections
guardrail: .agents/rules/design.md requires navigation levels to be identified before implementing routes; ADR 0010 and CONTEXT.md own the confirmed hierarchy and terms
canonical_refs: .agents/rules/design.md, docs/adr/0010-grouped-navigation-with-classic-fallback.md, CONTEXT.md
verification: focused menu and Dashboard-tab tests assert one grouped Dashboard destination, peer tabs, no category routes, and unmounting of inactive content
evidence: maintainer correction on issue 1793, Draft PR 1807 route structure, docs/adr/0010-grouped-navigation-with-classic-fallback.md, src/features/menu/SideMenu.tsx
revalidate_when: the grouped Dashboard hierarchy, Classic Navigation contract, or navigation component model changes
---

# Make Navigation Levels Explicit

Before implementing a navigation redesign, write the intended tree with every
node classified as a sidebar destination, an in-screen tab, or a content
section. A visual grouping label does not imply a route. Confirm ambiguous
interaction models in the owning issue before changing routing or persistence.

For the grouped Dashboard, the durable hierarchy is recorded in ADR 0010 and
`CONTEXT.md`. Tests should protect the visible structure and mounting behavior,
not only internal route names.
