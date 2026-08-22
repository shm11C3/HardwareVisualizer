# Suspend Hidden Windows WebViews

Status: accepted

Close to Tray keeps HardwareVisualizer's monitoring and tray behavior running
after the main window is hidden. On Windows, that also kept both the main
WebView and the pre-created Tray Widget flyout resident. A real-hardware
experiment for [#1962](https://github.com/shm11C3/HardwareVisualizer/issues/1962)
measured the hidden process tree at 173.7 MiB Private Working Set and 602.9 MiB
total Working Set after the initial transition settled.

WebView2 offers two best-effort memory controls for an inactive WebView. A low
memory-usage target reduced the measured tree Private Working Set to 64.6 MiB,
but it allows scripts and timers to keep running. Suspending both WebViews
reduced it to 36.6 MiB and pauses their script timers and animations. Microsoft
documents these controls as alternatives, so they are not combined.

## Decision

On Windows, the App lifecycle suspends the main WebView when Close to Tray hides
it and suspends the pre-created Tray Widget flyout whenever that window hides.
It resumes each WebView before the containing window is shown again.

The lifecycle must set the WebView2 controller's `IsVisible` property to
`false` before `TrySuspend`. Hiding the containing Tauri window did not update
that controller property in the tested runtime, and WebView2 rejected the
suspend request with `ERROR_INVALID_STATE` until the controller was hidden
explicitly. Restore reverses that controller state before showing the native
window. Suspension remains best-effort: a failure is logged but must not change
the user's Close to Tray choice or stop Core and tray background work.

App-to-WebView updates are not emitted to the hidden flyout because such API
calls can resume a suspended WebView implicitly. The App retains the latest
Tray Widget frame and emits it after resume when the flyout is opened.

This is an App-owned Windows lifecycle policy. It does not add a Core state,
user setting, or frontend responsibility. Other platforms keep their current
window lifecycle.

## Alternatives

- **Keep hiding only.** Rejected because it preserves the measured WebView
  memory cost and lets missed frontend visibility gates run hidden work.
- **Use the low memory-usage target.** Rejected because it reclaimed less
  memory in the gate experiment and deliberately leaves scripts running.
- **Destroy and recreate both WebViews.** Deferred because suspension already
  meets the issue's under-200-MiB target without state restoration, ready
  handshakes, or reopen-latency risk. This preserves the hide-over-destroy
  trade-off chosen in #1408.

## Consequences

- The final production validation kept the Windows tray-resident process tree
  intact while its stable Private Working Set fell from 173.7 MiB to 31.9 MiB
  (81.6%) in the tested environment.
- Hidden WebView scripts, timers, and animations stop without stopping Core
  collection, the native tray, Hardware Archive, or other background behavior.
- Showing either window now has a platform lifecycle transition before WebView
  interaction and must preserve acceptable reopen latency.
- The App directly depends on the WebView2 COM interface version used by wry;
  that compatibility must be revalidated when either dependency changes.
- WebView2 suspension is best-effort and runtime-dependent, so native Windows
  logs and real-window restore checks remain the evidence for this boundary.
- Destroy/recreate and a macOS equivalent remain out of scope unless future
  measurements show that suspension no longer meets the product target.
