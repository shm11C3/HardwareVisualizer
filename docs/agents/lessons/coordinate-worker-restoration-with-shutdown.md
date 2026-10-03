---
id: LRN-20261003-coordinate-worker-restoration-with-shutdown
status: promoted
cause_status: confirmed
scope: Tauri App worker controller start, restart, and recovery during process shutdown
trigger: adding or reviewing code that installs a worker into WorkersState or restarts one after a pause, recovery, cancellation, panic, or handoff
failure_signature: a restoration path could install a producer after terminate_all had taken its slot, leaving shutdown unable to await that worker
root_cause: restoration did not synchronize its shutdown decision with the slot mutex used by take_for_shutdown, so shutdown could transfer the slot between an earlier flag check and insertion
guardrail: src-tauri/AGENTS.md worker-slot rule and the producer_resumers_do_not_start_workers_during_shutdown regression test
canonical_refs: src-tauri/AGENTS.md, src-tauri/src/app/native_conversion.rs
verification: producer_resumers_do_not_start_workers_during_shutdown verifies closures are skipped after shutdown begins, and shutdown_drains_a_producer_started_while_shutdown_begins verifies shutdown collects a controller whose construction overlaps its flag transition; restoration checks the flag while holding the same mutex used by shutdown to take each slot
evidence: src-tauri/src/workers/mod.rs::WorkersState::terminate_all sets shutting_down before take_for_shutdown; src-tauri/src/app/native_conversion.rs::start_missing_producers and its regression test cover restoration
revalidate_when: WorkersState shutdown or slot ownership changes, or a new path starts worker controllers after pausing them
---

# Coordinate Worker Restoration With Shutdown

When `WorkersState::terminate_all` starts, it sets `shutting_down` and then
moves the controller handles out of their slots under each slot's mutex. A
restart that checks the flag outside that mutex can race: shutdown may take the
empty slot, after which the restart inserts a live controller that shutdown
never awaits.

Check `shutting_down` while holding the same slot mutex before constructing a
controller. If restoration already holds the slot when shutdown sets the flag,
shutdown waits and then takes the newly installed controller. If shutdown has
already set the flag, restoration leaves the slot empty. Keep a regression test
that asserts restoration closures are not called after shutdown begins.
