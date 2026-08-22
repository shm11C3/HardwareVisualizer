---
id: LRN-20260714-navigation-levels-explicit
status: promoted
cause_status: confirmed
scope: product navigation, dashboard and Insights information architecture, issues, ADRs, frontend routes, and E2E
trigger: a requirement groups screens, historical views, tabs, categories, or sections under one user-facing label
failure_signature: grouped UI requirements were flattened at the wrong level, including Dashboard tabs becoming routes and Cooling Insight being treated as mutually exclusive with a CPU/Memory display requested by its source issue
root_cause: the implementation did not classify each node as a route, in-screen tab, or content section before deciding whether an individual metric served one or more distinct analytical views
guardrail: .agents/rules/design.md requires navigation levels to be identified before implementing routes; ADR 0010 and CONTEXT.md own the confirmed hierarchy and terms
canonical_refs: .agents/rules/design.md, docs/adr/0010-grouped-navigation-with-classic-fallback.md, CONTEXT.md
verification: focused menu and Dashboard-tab tests assert one grouped Dashboard destination, peer tabs, no category routes, and unmounting of inactive content
evidence: maintainer corrections on issues 1793 and 1911, Draft PR 1807 route structure, docs/adr/0010-grouped-navigation-with-classic-fallback.md, CONTEXT.md, src/features/menu/SideMenu.tsx, src/features/hardware/insights/Insights.tsx
revalidate_when: the grouped Dashboard hierarchy, Insights Screen hierarchy, Classic Navigation contract, or navigation component model changes
---

# Make Navigation Levels Explicit

Before implementing a navigation redesign, write the intended tree with every
node classified as a sidebar destination, an in-screen tab, or a content
section. A visual grouping label does not imply a route. Confirm ambiguous
interaction models in the owning issue before changing routing or persistence.

For the grouped Dashboard, the durable hierarchy is recorded in ADR 0010 and
`CONTEXT.md`. Tests should protect the visible structure and mounting behavior,
not only internal route names.

For the Insights Screen, preserve every analytical view named by the relevant
requirements. Cooling Insight remains a peer view, while an individual metric
may also appear in a hardware-subject view when it serves a distinct comparison
such as CPU temperature alongside CPU utilization. Share the underlying data;
do not collapse the views or infer that their displays must be mutually
exclusive.
