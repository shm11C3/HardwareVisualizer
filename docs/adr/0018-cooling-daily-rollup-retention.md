# Cooling Daily Rollup Retention

Status: accepted

The cooling daily rollup (`cooling_daily_summary`) is stored and retained separately from the Hardware Archive. Hardware Archive rows summarize one-minute windows and are kept for `hardwareArchive.retentionDays` (default 30 days); the daily rollup derives one row per completed local day from those rows and keeps its own fixed retention window of about 400 days, defined as a Core constant rather than a user-configurable setting.

This lets Cooling Insight show 90-day and 1-year CPU temperature trends without loading a year of per-minute archive rows and without extending how long the much larger one-minute Hardware Archive history has to be kept. It follows the same separation already established for Storage Health history (ADR 0004): a derived, long-lived summary can outlive the shorter-window raw data it is computed from.

The rollup's own cleanup runs from the same `scheduledDataDeletion`-gated startup site as the Hardware Archive cleanup (`persistence::archive::cleanup_old_data`), so there is still exactly one place that decides whether startup deletion runs at all - only the retention window differs.

For planned chunked storage, [ADR 0019](0019-lossless-chunked-hardware-archive.md)
refines the startup-only cleanup trigger to cover long-lived sessions. The
independent Retention Period and rollup-before-deletion dependency remain;
this refinement does not claim that recurring cleanup is already implemented.
