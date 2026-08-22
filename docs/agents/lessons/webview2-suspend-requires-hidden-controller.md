---
id: LRN-20260822-webview2-suspend-requires-hidden-controller
status: promoted
cause_status: confirmed
scope: Windows Tauri WebView2 hide, suspend, and resume lifecycle
trigger: when implementing or changing WebView2 suspension for a Tauri window
failure_signature: TrySuspend fails with HRESULT 0x8007139F after the native Tauri window is hidden
root_cause: hiding the native Tauri window does not make the embedded WebView2 controller invisible, but TrySuspend requires the controller IsVisible property to be false
guardrail: the owning Windows WebView lifecycle explicitly hides the controller before TrySuspend and restores it before Resume and native show
canonical_refs: docs/adr/0017-suspend-hidden-windows-webviews.md, src-tauri/src/webview_memory.rs, src-tauri/src/lib.rs
verification: the experiment failed before SetIsVisible(false); the corrected production path logged one suspension per flyout hide transition, held the stable tree Private Working Set at 31.9 MiB, restored the main window in 190 ms, and restored the flyout in 28 ms on real Windows hardware
evidence: "issue #1962; src-tauri/src/webview_memory.rs; Microsoft CoreWebView2.TrySuspendAsync documentation"
revalidate_when: Tauri or wry changes WebView2 controller visibility behavior, the WebView2 suspend API contract changes, or the Windows window lifecycle is redesigned
---

# WebView2 suspension requires hiding the controller

## Observation

Calling Tauri's native window `hide` operation was not sufficient preparation
for WebView2 suspension. `TrySuspend` failed with `ERROR_INVALID_STATE` even
though the containing Tauri window was no longer visible.

## Confirmed cause

WebView2 requires its controller's `IsVisible` property to be `false` when
`TrySuspend` is called. The Tauri native-window hide path did not change that
embedded controller property in the tested lifecycle.

The experiment began suspending successfully only after it explicitly called
`ICoreWebView2Controller::SetIsVisible(false)` before `TrySuspend`. On restore,
controller visibility must be restored before the WebView is shown so a failed
versioned-interface cast cannot leave a visible native window with a hidden
WebView controller.

An explicit flyout hide also emits `Focused(false)`. A focus-loss handler that
unconditionally hides and suspends therefore attempts the same transition a
second time. The focus-loss path must first verify that the flyout is still
visible so each hide transition has exactly one suspension owner.

## Promotion

ADR 0017 owns the suspend-over-destroy decision. The Windows WebView lifecycle
preserves the required visibility ordering next to the COM calls because a
mocked unit test would not prove the Tauri/WebView2 integration behavior.
Revalidate on real Windows hardware instead of assuming the behavior if Tauri
or wry begins synchronizing native-window visibility with the WebView2
controller.
