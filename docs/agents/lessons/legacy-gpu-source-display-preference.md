---
id: LRN-20260711-legacy-gpu-source-display-preference
status: candidate
cause_status: confirmed
scope: src/features/settings/components/advanced/AdvancedSettings.tsx and dashboard GPU source display
trigger: adding or changing a Settings-screen display preference or using showGpuUsageSource as an example
failure_signature: showGpuUsageSource is a user-facing Settings toggle persisted directly through frontend Tauri Store
root_cause: the legacy toggle predates the current Application Preference ownership boundary
guardrail: do not copy or opportunistically migrate the exception; use typed Rust settings IPC for new preferences and handle migration as focused work
canonical_refs: pending focused migration decision; temporary exception is documented in src/AGENTS.md and .agents/rules/settings.md
verification: inspect AdvancedSettings.tsx and DashboardItems.tsx; a migration must preserve the existing stored value and add settings-service tests
evidence: src/features/settings/components/advanced/AdvancedSettings.tsx, src/features/hardware/dashboard/components/DashboardItems.tsx, and CONTEXT.md
revalidate_when: showGpuUsageSource moves to settings.json, is removed, or the persistence boundary changes
---

# Legacy GPU Source Display Preference

`showGpuUsageSource` is displayed as an explicit setting and changes dashboard
presentation, but it currently persists through Tauri Store. That makes it a
legacy exception to the current Application Preference rule, not an example for
new settings.

Migration needs a focused compatibility decision: read the existing Store value
once, write it through typed settings IPC without overwriting other settings,
and decide when the legacy key can be removed. Do not mix that behavior change
into unrelated guidance or UI work.
