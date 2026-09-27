---
id: LRN-20260927-claim-windows-open-before-database
status: promoted
cause_status: confirmed
scope: Windows App startup, single-instance activation, and database ownership
trigger: changing process startup order, elevated reopen, or single-instance notification
failure_signature: a secondary launch can inspect or open the database before the single-instance plugin decides it should exit
root_cause: App run startup performed native authority resolution and SQLite preflight before Tauri initialized the single-instance plugin
guardrail: claim the Windows Open event before settings and database startup, and let a secondary process signal and exit before opening an authoritative database
canonical_refs: docs/adr/0007-elevated-startup-mode.md, docs/architecture/backend.md, src-tauri/src/lib.rs
verification: inspect the first operation in App run and exercise the Windows activation test with a primary and a second launch
evidence: "src-tauri/src/lib.rs startup ordering before Issue #2291; tauri-plugin-single-instance 2.4.5 Windows setup; same-machine medium-to-elevated legacy WM_COPYDATA probe returned WIN32_ERROR(5)"
revalidate_when: Tauri plugin startup ordering changes, the Windows activation channel changes, or database authority startup moves
---

# Claim Windows Open before database startup

The old startup sequence resolved database authority and could apply SQLite
migrations before Tauri initialized the single-instance plugin. That ordering
is confirmed by the code; the exact contribution of early database work to the
observed reopen failure is not established. A live medium-to-elevated launch
returned `WIN32_ERROR(5)` for the legacy plugin's `WM_COPYDATA`, confirming
that Windows blocked the notification in the reproduced case.

Claim an activation channel before any settings write or database open, so a
second launch signals the existing process and exits without becoming another
database owner. Keep the channel's payload limited to Open because the
receiver may be elevated. The App lifecycle remains the owner of window
restore and hidden WebView resume.
