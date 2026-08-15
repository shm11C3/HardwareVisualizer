# Centralized Live Process Polling

Status: accepted

## Context

The Live Process Table is populated through the `get_process_list` Tauri
command. The frontend stores the latest result in a shared Jotai atom, but
`useProcessInfo` also starts an initial request and a three-second polling
interval for every hook consumer.

The default Hardware Dashboard has multiple consumers: CPU information uses the
process count, the Live Process Table uses the full list, and the Hardware
Report uses the process count. Sharing the atom therefore shares data but does
not share request ownership. Mounting these consumers can create multiple
polling loops that perform the same IPC request and write to the same atom.

The process list is live, window-only data. Hardware Archive process sampling
has a separate Core-owned lifetime and must not depend on whether the frontend
is polling. Issue
[#1931](https://github.com/shm11C3/HardwareVisualizer/issues/1931) owns the
implementation scope for this decision.

## Decision

The frontend will have one polling coordinator for the shared live process
resource per Jotai store.

- Components consume process state without creating their own timers.
- A consumer is active only while it is mounted and its `enabled` input is
  `true`. It joins the coordinator when it mounts enabled or `enabled` changes
  from `false` to `true`, and leaves when it unmounts or `enabled` changes from
  `true` to `false`. Disabled consumers do not keep polling active.
- The first active consumer starts polling, and polling stops after the last
  active consumer leaves.
- Polling pauses while the document is hidden and performs an immediate refresh
  when the document becomes visible again.
- Only one `get_process_list` request may be in flight. A periodic three-second
  tick that occurs while a request is in flight is skipped and is not queued,
  so steady-state polling starts at most one request every three seconds.
- If consumer demand starts or the document becomes visible while a request
  from an earlier inactive or hidden lifecycle is still in flight, its result
  is ignored and those demand signals are coalesced into one immediate refresh
  after it settles. A stopped or superseded request must not replace current
  process state when it completes.
- The last successful process list remains available during a transient request
  failure. One failed polling attempt follows one shared error path.
- The existing three-second cadence remains the initial policy. Changing the
  cadence requires separate runtime evidence.

The Live Process Table remains command-backed. The unbounded process list will
not be added to the one-second `HardwareMonitorUpdate` event. Core continues to
own process collection, App continues to own the typed command boundary, and
the frontend owns polling demand and view state.

The coordinator may use Jotai lifecycle support, a small feature-owned provider,
or an equivalent local mechanism. This decision defines ownership and lifecycle
semantics rather than prescribing one React implementation.

## Rationale

- Request cost should follow active frontend value instead of the number of
  mounted consumers.
- A single owner prevents duplicate IPC, full-list conversion, error reporting,
  and out-of-order writes.
- Keeping the process list out of the one-second event avoids increasing
  window IPC payloads for consumers that only need other live metrics.
- A feature-owned coordinator preserves the existing Core / App split without
  adding a general-purpose data-fetching dependency.

## Consequences

- Multiple process-data consumers observe one refresh schedule and one shared
  result.
- Hidden or inactive live views no longer require process-list IPC.
- Polling lifecycle, mounted `enabled` changes, visibility changes, slow
  requests, and late responses need focused frontend tests.
- The displayed list may remain at the last successful sample during a
  transient failure; the existing error surface still reports the failed
  refresh.
- Process count extraction, list ranking or capping, backend collection cadence,
  and Hardware Archive fan-out remain separate decisions.

## Rejected Alternatives

### Keep polling inside every consumer hook

This keeps components locally simple but makes request count depend on component
composition. The shared atom does not deduplicate timers or IPC.

### Add the full process list to HardwareMonitorUpdate

The event is emitted every second and serves live metrics that do not need the
unbounded process list. Sending the list through that path would trade duplicate
polling for a larger high-frequency payload.

### Add a general-purpose frontend query dependency

The required behavior is one feature-owned polling lifecycle. A new dependency
is not justified unless broader product requirements need its caching and
coordination model.
