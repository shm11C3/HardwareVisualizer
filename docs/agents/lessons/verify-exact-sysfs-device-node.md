---
id: LRN-20261110-verify-exact-sysfs-device-node
status: promoted
cause_status: confirmed
scope: Linux sensor endpoint ownership and review
trigger: a review proposes changing the subsystem expected for a canonical sysfs device
failure_signature: "the review of PR #2564 expected the native I²C adapter to use a class subsystem, while the provider required the I²C bus subsystem"
root_cause: the proposed check conflated the canonical bus adapter with the separate character-device class entry
guardrail: the approved DIMM specification distinguishes the nodes, and the provider fixture exercises their different subsystem targets
canonical_refs: docs/specs/sensors/memory-ddr5-odroid-h4-spd5118.md, core/src/infrastructure/providers/linux/dimm_temperature.rs
verification: run the Linux adapter_and_client_require_the_i2c_bus_subsystem test and check that the positive bus links and independent class-link substitutions exercise the same canonical nodes as the production lookup
evidence: "PRs #2564, #2577, and #2604"
revalidate_when: the kernel ABI, endpoint resolution path, or identity predicate changes
---

# Verify the exact sysfs device node

Trace the provider's canonical path before assigning a subsystem. A bus
adapter, its client, and an optional character-device child can expose
similar names while having different ownership. Confirm the final node and
its subsystem target against the pinned ABI evidence, then test the expected
target and substitute each misleading class link independently.

In the ODROID-H4 DIMM case, the specification clarified an existing
predicate in [#2577](https://github.com/shm11C3/HardwareVisualizer/pull/2577).
[#2604](https://github.com/shm11C3/HardwareVisualizer/pull/2604) added the
regression fixture. The production predicate stayed the same.
