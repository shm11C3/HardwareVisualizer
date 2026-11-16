---
id: LRN-20261117-single-owner-for-stateful-samplers
status: promoted
cause_status: confirmed
scope: core/src/collector, core/src/persistence, and design reviews that add a background consumer of a sampled fact
trigger: a persistence worker, tray view, or other background consumer needs a fact that a Core collector already samples
failure_signature: "the first Process Insight design for per-process GPU usage gave the archive worker its own sysinfo System, duplicating the refresh state, PID identity handling, and macOS retained-value filtering the Live Process GPU collector already owned; the follow-up correction then overstated the rule as 'persistence must not touch the OS', which the Storage Health Record worker contradicts"
root_cause: "the canonical docs said only that persistence does not share state with the collector; they did not say where a background-only consumer of an already-sampled, delta-based fact gets its readings, and the Storage Health worker's direct device read looked like a precedent even though that read is stateless"
guardrail: docs/architecture/backend.md Design Rule 8 assigns one collector owner to each stateful fact and routes other consumers through declared demand and EventBus; the design-review skill rejects a second refresh state
canonical_refs: docs/architecture/backend.md, core/README.md, .agents/skills/hardwarevisualizer-design-review/SKILL.md
verification: "in a design that adds a consumer of a sampled fact, name the collector that owns the refresh state and how the consumer declares demand; confirm no second sysinfo System or delta baseline is created for the same fact"
evidence: "issue #2720 design discussion; core/src/collector/live_process_gpu.rs (own System, delta between refreshes, (pid, start_time) identity, drop_retained_readings); core/src/persistence/storage_health.rs (stateless device health read); core/src/persistence/archive.rs (EventBus subscriber)"
revalidate_when: sysinfo stops computing usage as a delta between refreshes, the archive stops consuming EventBus snapshots, or a fact gains a cumulative counter that makes independent samplers cheap and consistent
---

# One Owner For Each Stateful Sampler

## Observation

Persisting per-process GPU usage for Process Insight (#2720) needs the same
fact the Live Process Table already reads. The first design put a dedicated
sampler inside the archive worker. That would have doubled the refresh cost
while both ran and duplicated the identity and platform-quirk handling of the
existing collector.

The correction first cited a rule that persistence must not access the OS.
The Storage Health Record worker reads device health directly from
`core/src/persistence/`, so that rule does not describe the codebase.

## Cause

What matters is whether the reading is stateful. sysinfo computes CPU and
per-process GPU usage as a delta between two refreshes of one `System`, so
each independent sampler carries its own baseline, cost, and filtering. A
stateless read has no such state and can run in each consumer's lifecycle.

## Rule

A stateful fact has one owner in `core/src/collector/`. Other consumers
declare demand to that owner and receive readings through `EventBus`
snapshots. Separate facts still keep separate samplers (see
[Keep CPU power sampling independent](keep-independent-cpu-power-sampler.md)).
