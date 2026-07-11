---
id: LRN-20260711-verify-user-visible-results
status: promoted
cause_status: confirmed
scope: frontend, native UI, screenshots, charts, transparent UI, and E2E artifacts
trigger: a change makes a visual, interaction, layout, or native-window claim
failure_signature: code and selectors were treated as proof even though the requested rendered effect or blocking overlay had not been inspected
root_cause: validation used a lower-level signal than the user-visible claim
guardrail: docs/design-principles.md DP-09 and scoped frontend instructions require rendered verification
canonical_refs: docs/design-principles.md, src/AGENTS.md, .agents/rules/frontend.md
verification: inspect rendered output at the relevant viewport/environment and run the focused interaction or UI test
evidence: browser or native screenshot, relevant viewport, interaction result, console output, and focused UI test
revalidate_when: capture harness, supported viewport, WebView runtime, or visual baseline policy changes
---

# Verify User-visible Results

For visual work, inspect the rendered output at representative desktop and
compact viewports. For interaction failures, inspect the failure screenshot or
artifact before weakening selectors; an overlay or first-run dialog may be the
real blocker.

The current capture harness preserves evidence but is not a visual regression
baseline. A successful capture therefore still needs visual inspection when
the claim is about appearance.
