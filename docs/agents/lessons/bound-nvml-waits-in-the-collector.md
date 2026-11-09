---
id: LRN-20261110-bound-nvml-waits-in-the-collector
status: promoted
cause_status: confirmed
scope: Linux NVML sampling and the periodic collector
trigger: an optional driver call is added to a shared sampling cycle
failure_signature: a delayed NVML call prevented the collector from publishing CPU and AMD readings
root_cause: the GPU sampling future waited for every native NVML call before the shared snapshot could be published
guardrail: bound the wait and keep the single-pass permit until the actual worker returns
canonical_refs: core/src/infrastructure/providers/linux/nvml_provider.rs
verification: the deadline fixture preserves completed devices, discards late results, and refuses a concurrent retry
evidence: "Issue #1924. Synthetic libnvidia-ml delayed initialization, PCI lookup, or temperature on the normal EventBus path"
revalidate_when: NVML sampling ownership, the collector cadence, or the driver call interface changes
---

# Bound NVML Waits in the Collector

A synthetic NVML library delayed initialization, PCI lookup, or temperature
readback by 1.5 seconds. On the original collector path, each delay prevented
the first snapshot from reaching the EventBus within 900 milliseconds. The
same fixture delivered AMD data and any completed NVIDIA device readings
after the bounded wait was added.

The deadline bounds how long the caller waits. The native call can continue
after the deadline, so the worker keeps the pass permit until it exits. Each
pass has its own receiver, which prevents late readings from entering a later
snapshot. NVIDIA has also acknowledged a driver defect that can block its
management query on [affected devices](https://github.com/NVIDIA/open-gpu-kernel-modules/issues/971).
