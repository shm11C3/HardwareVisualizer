---
id: LRN-20261110-distinguish-dxgi-display-aliases
status: promoted
cause_status: confirmed
scope: Windows GPU inventory, PDH fallback and DXGI adapter identity
trigger: Windows GPU results list two adapters with the same name or hardware identifiers
failure_signature: the backend inventory and PDH-only sampling both returned two RTX 5080 entries for one present PCI graphics device
root_cause: DXGI enumerated a physical renderer and an indirect display-only adapter with distinct LUIDs and one shared physical hardware key
guardrail: filter only OS-identified indirect display aliases of a separately enumerated renderer
canonical_refs: core/src/infrastructure/providers/windows/directx.rs, core/src/platform/windows/gpu.rs, docs/adr/0016-gpu-attribution-on-the-performance-screen.md
verification: inventory and live discovery fixtures preserve the renderer identity, separate renderers, an unmatched indirect adapter, and an adapter whose physical identity is unavailable
evidence: "Issue #1990 follow-up. Live DXGI and PnP enumeration, read-only D3DKMT identity queries, and a live PDH-only probe before and after the fix"
revalidate_when: Windows GPU inventory or PDH discovery changes its enumeration API or alias identity evidence
---

# Distinguish Indirect Display Aliases From Physical GPUs

On the reproduction machine, two DXGI descriptors had distinct LUIDs and
identical model and subsystem identifiers. The kernel reported one
render-capable adapter and one indirect display-only adapter. Each resolved to
the same single physical hardware key. The physical NVIDIA device appeared
once in the present PnP display inventory.

Compare the OS adapter role and physical key before suppressing an inventory
row or a live fallback entry.
Two physical GPUs of the same model can share the DXGI descriptive fields.
Keep the existing identifier assignment when hiding the extra row.
The [inventory decision](../../adr/0016-gpu-attribution-on-the-performance-screen.md)
preserves separate rows when identity is incomplete. The
[Microsoft adapter type](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d3dkmthk/ns-d3dkmthk-_d3dkmt_adaptertype)
and [physical PnP key](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d3dkmthk/ns-d3dkmthk-_d3dkmt_query_physical_adapter_pnp_key)
describe the two pieces of evidence.
