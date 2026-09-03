---
id: LRN-20260904-separate-sensor-support-from-recording-coverage
status: promoted
cause_status: confirmed
scope: user-facing sensor availability and historical coverage
trigger: explaining why a live or historical sensor lane has no values
failure_signature: a recorded period with no values was presented as proof that the current hardware does not support the sensor
root_cause: historical value presence and hardware support were collapsed into one frontend capability flag
guardrail: carry hardware support as an explicit Core-owned fact and combine it with period-specific recording presence only at presentation time
canonical_refs: docs/design-principles.md, core/src/models/metrics.rs, src/features/hardware/insights/cooling/utils/sensorNotice.ts
verification: test unsupported, supported-but-not-collected, unknown-support, present, and wholly unrecorded states independently
evidence: maintainer correction on 2026-09-04; src/features/hardware/insights/cooling/utils/thermalTimeline.ts; src/features/hardware/insights/cooling/utils/fanTimeline.ts
revalidate_when: the archive persists sensor support transitions alongside readings
---

# Separate Sensor Support from Recording Coverage

A historical period containing other metrics but no power or fan values proves
only that the target metric was not recorded in that period. It does not prove
that the current hardware is unsupported.

Keep these facts separate:

- Core owns whether the current hardware path is supported, unsupported, or
  still unknown.
- Archive and rollup queries own whether the selected period contains values.
- Presentation combines both facts: say "not supported by current hardware"
  only from explicit support evidence, "not collected for this period" for a
  supported path with no values, and avoid guessing when support is unknown.

Do not derive hardware support from an empty series, elapsed time, platform
name, or frontend inspection of sensor labels.
