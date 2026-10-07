---
id: LRN-20261003-exclude-major-upgrades-from-deferred-setup
status: promoted
cause_status: confirmed
scope: src-tauri/windows/wix
trigger: changing MSI setup defaults, dialog routing, or commit-time external component refresh
failure_signature: a full-UI major upgrade also ran deferred setup, refreshing modules before the product installation could roll back
root_cause: a major upgrade changes ProductCode, so NOT Installed is true for the incoming package even when an older related product is installed
guardrail: exclude WIX_UPGRADE_DETECTED from the setup default, deferred setup action, and optional-components dialog routes; use only commit-time refresh on upgrades
canonical_refs: docs/design/external-component-setup.md, src-tauri/windows/wix/external-component-setup.wxs, src-tauri/windows/wix/check-external-component-setup.ps1
verification: compile the WiX fragment into an MSI and run check-external-component-setup.ps1; evaluate setup and refresh conditions for fresh installs and major upgrades at UI levels 2 through 5
evidence: "PR #2328 Codex review, Microsoft's Installed property and Major Upgrades documentation, and the MSI condition/table checks"
revalidate_when: the MSI upgrade strategy, Tauri WiX template, or setup consent flow changes
---

# Exclude Major Upgrades From Deferred Setup

`NOT Installed` describes the incoming ProductCode, not the absence of every
earlier version. A major upgrade therefore needs an explicit related-product
guard. Excluding upgrades only from the setup default still permits an
explicit setup property to run the deferred action; guard the executor too.
Skip the setup dialog on upgrades so it cannot offer an action that will not run.

Keep module replacement on upgrades exclusively in the commit action required
by ADR 0026. Verify both paths in the built MSI tables, including the setup
default and dialog conditions, whenever the installer template changes.

Primary sources: [Installed property](https://learn.microsoft.com/en-us/windows/win32/msi/installed)
and [Major upgrades](https://learn.microsoft.com/en-us/windows/win32/msi/major-upgrades).
