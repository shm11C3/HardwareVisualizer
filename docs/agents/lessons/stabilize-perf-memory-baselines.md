---
id: LRN-20260822-stabilize-perf-memory-baselines
status: promoted
cause_status: confirmed
scope: perf-monitor startup stabilization and process-tree memory growth checks
trigger: when changing perf-monitor warmup, helper discovery, sampling, or memory growth calculations
failure_signature: the Windows performance workflow exceeded process-tree growth while average and P95 stayed within threshold, then passed on the same commit when rerun
root_cause: a fixed warmup could end before WebView helper RSS settled, and last-minus-first growth treated delayed initialization and endpoint noise as steady-state growth
guardrail: perf-monitor waits for a stable helper PID set and component RSS before measurement, then computes growth from endpoint-window medians
canonical_refs: perf-monitor/src/monitor.rs, perf-monitor/src/config.rs, perf-monitor/README.md
verification: cargo test --manifest-path perf-monitor/Cargo.toml
evidence: GitHub Actions run 32559708158 attempts 1 and 2; perf-monitor/src/monitor.rs regression tests
revalidate_when: the WebView process model, sampling interval, growth thresholds, or performance runner platform changes
---

# Stabilize performance memory baselines

## Observation

The Windows performance gate reported 386.9 MiB of process-tree growth even
though its average and P95 remained within threshold. WebView helpers accounted
for 360.7 MiB of that growth. A rerun of the same commit reported -8.3 MiB of
tree growth, while the P95 differed by only 7.9 MiB.

The previous gate waited a fixed 15 seconds and defined growth as the last RSS
sample minus the first. On a slower startup, the first sample therefore
represented a partially initialized WebView process tree rather than the
steady-state baseline.

## Confirmed cause

The measurement boundary did not prove that startup had settled. Increasing
the measurement sample count could improve average and percentile estimates,
but it could not make a two-endpoint growth calculation robust.

## Promotion

The monitor now treats the configured warmup as a minimum, then requires the
same helper PID set and stable component RSS for a rolling window before
measurement. It fails explicitly if no stable baseline is found. Growth uses
the medians of short endpoint windows so one allocation or reclamation sample
cannot decide the gate. Focused unit tests own both invariants.
