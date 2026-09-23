---
id: LRN-20260924-keep-tray-flyout-geometry-transient
status: promoted
cause_status: confirmed
scope: Windows tray flyout window state and left-click restore
trigger: when changing tray flyout geometry, window-state persistence, or close-to-tray restore
failure_signature: a tray left click opens an unusably small flyout and its Open button cannot be reached
root_cause: the window-state plugin persisted and restored the transient flyout's shrunken size; the event that first shrank it remains unconfirmed
guardrail: exclude the flyout from window-state persistence and reapply its intended size on every show after selecting the target monitor
canonical_refs: src-tauri/src/lib.rs, src-tauri/src/tray/surface/windows.rs
verification: a 27x13 saved flyout state produced an 18x8 client area in the release app; the patched app opened at 300x148, and recovered to 300x148 after an induced same-session resize and tray left click
evidence: src-tauri/src/lib.rs; src-tauri/src/tray/surface/windows.rs; local Windows native-window measurements and a rendered flyout capture
revalidate_when: the tray flyout becomes resizable, its intended dimensions change, or Tauri window-state restoration behavior changes
---

# Keep tray flyout geometry transient

## Observation

The Windows tray flyout could open as a tiny window. With close-to-tray enabled,
the tray icon's left click opened this flyout, but its Open button was outside
the usable client area.

## Confirmed cause and limit

The window-state plugin included the flyout in its default persistence and
restoration. The affected installation's saved state had a 27x13 flyout, and
the live release window had an 18x8 client area. The initial resize event is
unknown; a display or DPI change is a possible trigger, not a confirmed cause.

## Promotion

The App excludes the transient flyout from window-state persistence. Its
Windows show path reapplies the intended size after selecting the target
monitor, which also repairs a resize during the current process. The code
beside those operations owns the invariant; this record preserves the evidence
boundary for future lifecycle changes.
