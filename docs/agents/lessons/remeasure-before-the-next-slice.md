---
id: LRN-20261012-remeasure-before-the-next-slice
status: promoted
cause_status: confirmed
scope: multi-slice performance or architecture work in src/** and src-tauri/**
trigger: a plan has several slices and the first slice's measurement changes what the remaining slices are for
failure_signature: three reviewed PRs (#2396, #2397, #2399) built a Live Metrics Buffer whose per-tick saving (about 11 µs) turned out to be invisible under React reconciliation; they were closed unmerged
root_cause: "the baseline showed render cost follows subscription shape, not atom-write count, and slice 1 took almost the whole win; the remaining slices were built on the issue's original premise instead of on a new measurement of what they would now save"
guardrail: .agents/rules/design.md requires re-measuring the remaining slices' claim before building them when a slice's result changes the premise (DP-04 work follows visible value, DP-09 match evidence to the claim)
canonical_refs: .agents/rules/design.md, docs/design-principles.md
verification: "jsdom benchmark, A = b23baca7d (slice 1) vs B = b23b36ef6 (slice 4), five real-screen scenarios, 200 warm-up + 5x400 samples, three interleaved runs: listener-only 27.3 -> 16.7 us/sample, Dashboard 232 -> 242 (within noise), Performance panels 452 -> 473 (within noise), CPU detail 160 -> 132 (the transpose removal, which needs no buffer)"
evidence: "issue #1638 (Outcome section); PR #2395 (merged slice 1); PRs #2396, #2397, #2399 (closed with the benchmark in a comment); src/features/hardware/renderFanout.test.tsx"
revalidate_when: #1581 moves chart painting off React, which changes what share of a tick the listener is; or the render fan-out harness starts measuring time instead of counts
---

# Re-measure before the next slice

## Observation

Issue #1638 planned a Live Metrics Buffer in four slices. The baseline taken
before slice 1 showed that one 1 Hz update re-rendered 64 Dashboard components,
and that the cause was subscription shape (whole-map reads, arrays read for a
scalar, static UI under live subscribers), not the 19 atom writes, which batch
into one commit. Slice 1 fixed the shape and reached 14 components without a
buffer.

Slices 2 to 4 then built the buffer anyway, on the issue's original argument
that writing 1 Hz data into Jotai is the cost. They were reviewed and stacked.
When the maintainer asked why the buffer existed, a half-hour jsdom benchmark
showed the listener saving about 11 µs per tick, hidden under 200 to 450 µs of
React reconciliation on every screen. The three PRs were closed.

## Confirmed cause

A multi-slice plan carries its premise forward. When a slice's measurement
contradicts that premise, the remaining slices need a new justification, and
"the issue says so" is not one. The cheap check (benchmark the claim the next
slice makes) was available before slice 2 and was not run.

## Promotion

`.agents/rules/design.md` now requires that, when a slice's result changes the
picture, the remaining slices' claim is re-measured before they are built.
This follows DP-04 (work follows visible value) and DP-09 (match evidence to
the claim). The buffer code stays available in the closed PRs as a reference
implementation if #1581 needs a ring buffer.
