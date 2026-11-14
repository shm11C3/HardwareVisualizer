# Spec: AMD Dragon Range CCD temperatures on Windows

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Windows x86-64, AMD CPUID signature `0x00A60F12`, FL1 package field 1, one enumerated physical processor package and two fixed CCD temperature slots through the existing signed RyzenSMU read interface |
| Issue phase | [#2515](https://github.com/shm11C3/HardwareVisualizer/issues/2515) |

## Sources

| ID | Source | Evidence |
| --- | --- | --- |
| S1 | AMD [Ryzen 9 7945HX specifications](https://www.amd.com/en/products/processors/laptop/ryzen/7000-series/amd-ryzen-9-7945hx.html) and [Family 19h Model 61h PPR, 56713-B1 Rev 3.05, March 8, 2023](https://www.amd.com/content/dam/amd/en/documents/processor-tech-docs/programmer-references/56713-B1_3_05.zip), §1.7.1 Table 12 (p. 33), CPUID (pp. 68, 86–87), Definitions (p. 486) | Primary product codename and FL1 package, plus the platform-specific numeric package assignment. The generic package enumeration disagrees. The PPR title identifies B1 silicon. |
| S2 | [CPU-Z Validator rfdx4g](https://valid.x86.fr/rfdx4g), Processor and Motherboard, March 22, 2026 | Independent Ryzen 9 7945HX observation on FL1, RPL-B2, family/model/stepping 19h/61h/2 and one CPU, 16 cores, 32 threads. Gives decoded identity and socket only. |
| S3 | [First-hand ZenTimings debug report](https://github.com/irusanov/ZenTimings/issues/57#issuecomment-2507152441), November 29, 2024, and [ZenTimings v1.33 release](https://github.com/irusanov/ZenTimings/releases/tag/v1.33), asset SHA-256 `a97644fc3088e0e3b471fe87f7df4ae25125c99bd0b3444e0992f0782903dbb1`. [Matching library version in public source](https://github.com/irusanov/ZenStates-Core/blob/92fca8b7547a45d858af48bb8aa27dc4fc553caa/ZenStates-Core.csproj), [CPUID field extraction](https://github.com/irusanov/ZenStates-Core/blob/92fca8b7547a45d858af48bb8aa27dc4fc553caa/Cpu.cs#L331-L337) and [classifier](https://github.com/irusanov/ZenStates-Core/blob/92fca8b7547a45d858af48bb8aa27dc4fc553caa/Cpu.cs#L613-L619) | The report identifies the exact CPU as `00A60F12` and Dragon Range and gives application version 1.33.1535 and core version 1.72.0. The official archive assemblies have metadata versions 1.33.1535.0 and 1.72.0.0. Its library SHA-256 is `a8a7a6a301d3117c1d832e659c640973a8c3e5de84b65acc47d704cf325b53b2`, matching the [binary pinned by the release tag](https://github.com/irusanov/ZenTimings/blob/8ecdcadcc4e166668ee3964026c68e609836a54d/Common/ZenStates-Core.dll). The public 1.72 classifier associates this output with package field 1, **lead-only (copyleft)**. The diagnostic reports a decoded classification without raw extended-leaf EBX. The release provides no source-commit attestation for the library. |
| S4 | Daisuke Yanase, [Minisforum BD790i first-hand Debian record](https://qiita.com/daisuke_yanase/items/9cbde9ca321083d67f1f), Ryzen 9 7945HX and Linux `6.1.0-32-amd64`; independent [Ryzen 9 7945HX PCI enumeration and boot record](https://www.spinics.net/lists/amd-gfx/msg136405.html) | The Debian record identifies family/model/stepping 25/97/2 and gives converted `Tccd1` 63.9 °C and `Tccd2` 60.2 °C. The separate boot record combines the same family/model/stepping and CPU name with AMD `1022:14d8` at `0000:00:00.0`. Neither gives raw CCD words. |
| S5 | AMD-authored [CCD-offset submission](https://lists.openwall.net/linux-kernel/2022/07/19/1998) and Linux v6.1 [CCD word fields](https://github.com/torvalds/linux/blob/v6.1/drivers/hwmon/k10temp.c#L69-L74), [conversion](https://github.com/torvalds/linux/blob/v6.1/drivers/hwmon/k10temp.c#L211-L218), [discovery validity](https://github.com/torvalds/linux/blob/v6.1/drivers/hwmon/k10temp.c#L370-L378), [model mapping](https://github.com/torvalds/linux/blob/v6.1/drivers/hwmon/k10temp.c#L452-L456) | **Lead-only (copyleft)** for address, slot order, validity indication and scaling in the model range containing 61h. S4 corroborates the first two channel positions on Dragon Range itself. |
| S6 | [HardwareVisualizer PawnIO interface rev 7](pawnio-interface.md), RyzenSMU and mutex sections. Signed [PawnIO.Modules release 0.2.11](https://github.com/namazso/PawnIO.Modules/releases/tag/0.2.11) at `52a7e536dff3e53c96917a28caac5e0fa6510696`, [classification](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/RyzenSMU.p#L149-L150), [host bridge access](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/RyzenSMU.p#L247-L283), [allow-list](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/RyzenSMU.p#L614-L628), [read interface](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/RyzenSMU.p#L793-L814) | Upstream signed-module read interface, allowable address window and caller mutex. The fixed host-bridge route and internal hardware classification are **lead-only (copyleft)**. |
| S7 | Microsoft [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex) and [PROCESSOR_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_relationship) | Primary Windows physical package enumeration and active processor-group masks. |

## Detection

| Condition before sampling | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64 and `AuthenticAMD`, CPUID leaf 1 EAX exactly `0x00A60F12` | mechanism | S1–S5 |
| Extended CPUID leaf `0x80000001` present with EBX bits 31:28 equal to 1 | mechanism | S1–S3, S6. S3 and S6 contain lead-only classification facts. |
| Complete OS topology identifies exactly one nonempty physical processor package with valid group masks | safety (fixed root without socket selection) | S4, S6 (lead-only root), S7 |
| Approved signed RyzenSMU read interface is available | safety | S6 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact signature and FL1 field above, one physical package, signed read route | S3 supplies lead-only corroboration of the numerical package classification. S5 supplies the lead-only map, validity and scaling. S6 pins the read interface, with lead-only internal CPU classification and fixed host route. The Ryzen 9 7945HX and the observed AMD host bridge are evidence identities (S1–S4). | Experimental under [ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md). |
| AM5 field 0, unknown package field or topology, adjacent signature, multiple physical packages | Outside the Dragon Range candidate. | Unsupported by this scope. |

## Register map (facts)

| Fixed slot | Address | Validity and temperature |
| --- | --- | --- |
| 1 | SMN `0x00059B08` | 32-bit result. Bit 11 indicates validity at discovery. Bits 10:0 encode 0.125 °C per count with a −49 °C reference offset. S5, lead-only. |
| 2 | SMN `0x00059B0C` | Same field layout for this slot. S5, lead-only. |

## Read procedure and decode

1. After admission, sample on demand through the existing Windows acquisition path. For each fixed slot, independently acquire `Global\Access_PCI` with the existing timeout, hold it across the signed read, and release it when that request ends (S6). If acquisition is denied or times out, leave only that slot unavailable and continue with the other slot. Make at most one attempt per slot per sample.
2. Reject a failed read, zero, all ones, or clear bit 11 for the affected slot. Decode bits 10:0 as 0.125 °C per count minus 49 °C (S5). Accept only −40 °C through +120 °C, inclusive. The driver in S5 checks validity at discovery. Per-read validity and the plausibility bounds are defensive project policy shared with the [Raphael scope](cpu-amd-raphael-ccd-smn.md).
3. Keep an unavailable slot at its original one-based position in the sole package. Preserve an independently valid sibling, the separate package temperature result, and its fallbacks.

## Quirks

- Slot numbers identify fixed register positions. The two attributed Linux channels support these positions (S4, S5). The number of populated CCDs may vary and slot identifiers have no OS-core mapping.

## Safety notes

- Both scoped addresses are within the signed module's `0x56000`–`0x5AFFF` window. The module uses its fixed host bridge's selector/data interface, with a selector write intrinsic to each read. The module offers no selector-restoration guarantee (S6, lead-only for the internal route). The app calls the approved read interface and performs no direct PCI/SMN operations, data writes, mailbox commands, or PM-table requests.
- Missing identity, malformed package topology, or a missing signed module prevents admission. A mutex acquisition failure prevents only that slot's read. Do not scan additional addresses or try another package map after a failure.

## Synthetic acceptance cases

| Case | Required result |
| --- | --- |
| Exact signature and package field, one valid package, both requests succeed with `0x0AC8` and `0x0B38` | Slots 1 and 2 report 40 and 54 °C. Exactly the two fixed addresses are requested. CPU brand has no effect on admission. |
| AM5 package field 0, unknown field, leaf missing, wrong stepping, wrong vendor, duplicate or malformed package records, or multiple packages | No Dragon Range target and no Dragon Range read. Separately admitted AM5 and package-temperature paths retain their own checks. |
| Either slot faults or returns zero, all ones, clear validity despite a plausible field, or a value beyond the supported bounds | That fixed slot is unavailable. A valid sibling and independent package result survive. Exercise a later invalid result after a valid result. |
| Valid words `0x0988`, `0x0848` and `0x0D48`, with one-increment values outside the bounds | Accept 0, −40 and +120 °C. Reject the adjacent out-of-range values. |
| Signed module missing | No Dragon Range target or read. |
| Either slot fails to acquire the mutex and the other slot succeeds | Do not issue or retry the failed slot's read. Preserve the successful sibling in its fixed position. Exercise both slot orders and acquisition denial and timeout. |

## Open questions

- Non-blocking for the Experimental scope: the exact FL1 gate is supported by the platform-specific definition and the observed classifier result.
  S1 Table 12 and Definitions associate FL1 with field 1. Its generic p. 87 enumeration reserves fields 0–3. S2 independently observes an exact retail FL1 B2 CPU. S3 corroborates field 1 through its exact first-hand report, matching release metadata, and the public classifier for that library version. The cited report gives the decoded classification. Raw extended-leaf EBX remains unobserved.
- Non-blocking for the Experimental scope: the approved Windows read transport and the exact observed Linux channels have independent evidence.
  The CCD mapping and field definitions rely on lead-only S5, corroborated by the converted readings in S4. The cited observations do not include raw CCD words or a HardwareVisualizer Windows run. Verified status requires primary register documentation or maintainer-accepted hardware evidence.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-13 | Initial specification. Propose transition to Implementation-ready for the Experimental scope, pending maintainer sign-off. |
