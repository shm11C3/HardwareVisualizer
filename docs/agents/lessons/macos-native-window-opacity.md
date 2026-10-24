---
id: LRN-20261016-macos-native-window-opacity
status: promoted
cause_status: confirmed
scope: macOS main-window appearance and transparency preferences
trigger: diagnosing WindowServer GPU cost or changing Transparent UI on macOS
failure_signature: disabling Transparent UI clears vibrancy but leaves the native window non-opaque
root_cause: the main window is configured transparent at creation and clearing vibrancy does not change NSWindow opacity or backing color
guardrail: apply native opacity and backing color from the explicit transparency preference on startup and runtime changes
canonical_refs: src-tauri/src/services/window_appearance_service.rs, src-tauri/src/commands/settings.rs, src-tauri/src/lib.rs
verification: native getters after startup and ON/OFF transitions; ordinary-window alternating GPU-counter comparison
evidence: AppKit runtime probes and local release-profile measurements on 2026-10-16
revalidate_when: macOS, Tauri/Wry window creation, native vibrancy, or main-window transparency behavior changes
---

# Match macOS native opacity to Transparent UI

Clearing native vibrancy does not make a transparent `NSWindow` opaque. With
Transparent UI disabled, apply both `isOpaque = true` and an opaque backing
color. Restore a clear backing and `isOpaque = false` before applying vibrancy
when transparency is enabled. A zero blur level with Transparent UI enabled
must remain transparent.

The App-owned window appearance service is the canonical implementation. It
uses the same path at startup and after preference changes, reads current
preferences on the main thread, and leaves the WebView transparent so switching
back to transparency does not require replacing it.

## Evidence limits and revalidation

Creation-time configuration alone does not prove runtime switching. Check
native opacity and background alpha after ON/OFF, ON with blur zero, and restart.
Inspect rendered light/dark and transparent output separately from GPU timing.

Compare fully on-screen ordinary windows with the same viewport and live
updates. Record absence of dialogs during timing: a diagnostic version of
`0.0.0` can open the automatic updater dialog. Keep computer-use capture out of
timing intervals; it can itself raise GPU cost. Sample animation activity more
frequently than the metric update period to avoid phase-locked false zeros.

GPU results are local evidence, not a cross-machine percentage or battery-life
claim. The WindowServer measurement uses deltas of the private
`AGXDeviceUserClient` `AppUsage.accumulatedGPUTime` counter divided by elapsed
time; it is not Activity Monitor GPU percent.

## Local runtime result (2026-10-16)

An Apple M4 on macOS 26.6.2 was tested with a release-profile build and default
features, an ordinary 1800 x 1002 window (1800 x 970 WebView), and live Panels.
The baseline fixture restored the previous clear/non-opaque native state while
keeping Transparent UI off. The candidate was measured after actual ON -> OFF
IPC transitions in the same process. Each cell is a 30-sample mean in counter
units per second divided by 1,000,000.

| Pair | Previous native state | Runtime opaque state | Reduction |
| --- | ---: | ---: | ---: |
| 1 | 677.04 | 152.30 | 77.50% |
| 2 | 566.31 | 129.50 | 77.13% |

The aggregate reduction was 77.34%. Native getters confirmed opaque=true and
background alpha=1 after disabling transparency, and opaque=false/alpha=0 when
enabling it, including blur zero. WKWebView remained non-opaque. Saved OFF state
was also applied after restart. All timing reports had no dialogs, changing live
values and active metric transitions; bounds were checked before and after each
phase. There were no replayd capture-health records during the measured interval.
