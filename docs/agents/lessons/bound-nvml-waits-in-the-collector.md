---
id: LRN-20261110-bound-nvml-waits-in-the-collector
status: promoted
cause_status: confirmed
scope: Linux NVML sampling and the periodic collector
trigger: an optional driver call is added to a shared sampling cycle
failure_signature: a delayed NVML call prevented the collector from publishing CPU and AMD readings
root_cause: the GPU sampling future waited for every native NVML call before the shared snapshot could be published
guardrail: bound the wait and keep the worker slot and PCI claim until the native call returns
canonical_refs: core/src/infrastructure/providers/linux/nvml_provider.rs
verification: the deadline fixture preserves later devices after a stalled first lookup, discards late results, and refuses a retry on the same PCI address
evidence: "Issue #1924. Synthetic libnvidia-ml delayed initialization, PCI lookup, or temperature on the normal EventBus path"
revalidate_when: NVML sampling ownership, the collector cadence, or the driver call interface changes
---

# Bound NVML Waits in the Collector

A synthetic NVML library delayed initialization, PCI lookup, or temperature
readback by 1.5 seconds. On the original collector path, each delay prevented
the first snapshot from reaching the EventBus within 900 milliseconds. The
same fixture delivered AMD data after the bounded wait was added. Stalling
the first GPU still hid later NVIDIA results until the read work was split
across two bounded workers.

The deadline bounds how long the caller waits. The native call can continue
after the deadline, so its worker slot and PCI claim remain held until it
returns. The other worker can sample healthy PCI addresses. Each pass has its
own receiver, which prevents late readings from entering a later snapshot.
NVIDIA documents [concurrent NVML calls](https://docs.nvidia.com/deploy/nvml-api/nvml-api-reference.html)
and has acknowledged a driver defect that can block management queries on
[affected devices](https://github.com/NVIDIA/open-gpu-kernel-modules/issues/971).
