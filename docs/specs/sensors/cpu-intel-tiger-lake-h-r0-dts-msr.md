# Spec: Intel Tiger Lake-H R0 core DTS

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Proposed Experimental Windows physical-core DTS on exact Tiger Lake-H R0 `0x000806D1`, with one active processor group and one physical package. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel [11th Generation Core Processor Datasheet, Volume 1](https://cdrdv2-public.intel.com/631121/631121_TGL%20Datasheet%20Volume1of2%20rev012_PUBLIC.pdf), 631121-012, May 2023, §1 and §§4.1.3.1–4.1.3.2 (pp. 74–77) | Primary H-platform thermal contract. The factory-calibrated reference is readable in `0x1A2[23:16]`. The local DTS reports the instantaneous difference from that reference, independently of the TCC activation offset and averaging window. |
| S2 | Intel [11th Generation Core Processor Specification Update](https://cdrdv2-public.intel.com/631123/631123_034.pdf), 631123-034, April 2026, §2.1 Table 2-1 (p. 8), §3.2 (pp. 12–17), §4 TGL014 and TGL018–020 (pp. 21–22), and [Intel microcode release notes](https://github.com/intel/Intel-Linux-Processor-Microcode-Data-Files/blob/bdc92abe5c499c3fd7988b76a128d05c9120a520/releasenote.md#L369-L371) | Primary exact H81 signature and R0 stepping. The specification update distinguishes H81 from UP3/UP4/H35 and Refresh. |
| S3 | Intel [INTEL-SA-01153](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-01153.html), May 2025, Affected Products `806D1` row, [Tiger Lake-H platform catalog](https://www.intel.com/content/www/us/en/products/platforms/details/tiger-lake-h.html), and [Tiger Lake-H launch](https://www.intel.com/content/www/us/en/newsroom/news/11th-gen-core-mobile-h.html), May 2021 | Primary product overlap. The exact signature covers 11th Gen Core and Xeon mobile and embedded products. Intel's H platform catalog includes embedded Core, Celeron 6600HE, and Xeon W-11000E. The launch identifies Xeon W-11000 mobile and Core mobile H as Tiger Lake-H. Product names record evidence and do not select the runtime path. |
| S4 | Intel [SDM Volume 4](https://cdrdv2-public.intel.com/929363/335592-093-sdm-vol-4.pdf), 335592-093US, September 2026, Table 2-2 (pp. 2-21–2-22), §2.17 Table 2-39 (pp. 2-359–2-361), §2.17.4 Table 2-45 (pp. 2-397–2-399) | Primary architectural fields and Core-scope layout applicable to `06_8D`. The Tiger Lake supplemental table does not replace `0x19C`. |
| S5 | InstLatx64 [Core i7-11800H register capture](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00806D1_TigerLakeH_CPUID1.txt), commit `17be9a9793c306cf96f8c286e34e705b780a0226`, blob `d641e2cb9081902ef85fc5a7b7d27c060ed557e6`, lines 1–15, 147–160, 1188–1217 and 1404–1425 | Independent exact CPUID, DTS capability, package and physical-core topology, and paired per-logical-CPU raw MSRs. |
| S6 | Linux v6.18 commit `7d0a66e4bb9081d75c82ec4957c50034cb0ea449`, [model ID](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/arch/x86/include/asm/intel-family.h#L109-L110), [core mask](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/thermal/intel/intel_tcc.c#L33-L38) and [model selection](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/thermal/intel/intel_tcc.c#L83-L85) | **lead-only (copyleft)** for the conflicting eight-bit core readout explicitly selected for `06_8D`. No raw set-bit observation is supplied. |
| S7 | [Intel core DTS revision 8](https://github.com/shm11C3/HardwareVisualizer/blob/6a59d7214c042e8cc75fc43caf760c3fe9948932/docs/specs/sensors/cpu-intel-core-dts-msr.md), Detection, Read procedure and decode, Project collection constraints and Safety notes, and [PawnIO interface revision 7](https://github.com/shm11C3/HardwareVisualizer/blob/2495a1b0fbe4a8048feac9d122b98201e7970cbb/docs/specs/sensors/pawnio-interface.md#intelmsr), signed `IntelMSR` `0.2.11`, module source tag commit `52a7e536dff3e53c96917a28caac5e0fa6510696` | Existing Windows topology, affinity, demand, serialization, timeout and partial-result contract. The signed module permits both synchronous calling-processor reads. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, `GenuineIntel`, full `CPUID.01H:EAX = 0x000806D1`, and `CPUID.06H:EAX[0] = 1` at discovery and on each confirmed representative | mechanism | S1–S5, S7 |
| Exactly one active Windows processor group and one physical package with complete, consistent physical-core topology | mechanism | S7 |
| Existing signed `IntelMSR` transport, dedicated disposable sampling thread, confirmed processor affinity, affinity restoration and topology recheck | safety | S7 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Tiger Lake-H R0 `0x000806D1` core DTS | S1–S4 establish the mechanism across the documented H81 population. S5 shows exact i7-11800H paired raw observations. S6 is lead-only for the conflicting eight-bit mask. | Experimental under the common gates, pending maintainer approval of this proposal. |
| Other signatures or missing gates | Separate qualification is required. S2 identifies `0x000806C1` and `0x000806C2` as different signatures. | Outside this proposal. |

Use the existing non-hybrid physical-core identities and observed OS core
masks. Package thermal management has separate eligibility. Core sampling
has no package-thermal-management capability dependency (S7).

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x19C` | `IA32_THERM_STATUS` | 31 | Core DTS reading valid when set | Boolean | S4, S5 |
| `0x19C` | `IA32_THERM_STATUS` | 22:16 | Digital readout below calibrated Tjmax | Unsigned degrees Celsius | S1, S4, S5 |
| `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated Tjmax reference | Degrees Celsius | S1, S5 |

## Read procedure and decode

Use the common revision 8 non-hybrid sampling procedure (S7). On each
confirmed pinned representative, synchronously read `0x1A2` followed by
`0x19C`. Require successful reads, a target in 50–120 °C inclusive, and
status bit 31 set. Reject a status word with bit 23 set before applying
Intel's seven-bit field (S4–S6). Subtract the unsigned delta from the
calibrated reference. Accept a positive result no higher than the target.
A valid zero delta yields the target. The separate activation offset and
averaging window do not enter this conversion (S1).

Recheck the running processor and restore affinity. Preserve other
samples after an isolated fault or invalid word. Inconsistent topology
or failed affinity restoration invalidates the core set. Keep the
existing demand, timeout and cache rules. Concurrent requests share the
existing bounded acquisition with at most one pass in flight. The signed
IntelMSR read interface has no external hardware-mutex requirement. It
uses the calling processor and leaves affinity to the caller (S7).

## Quirks

- The i7-11800H capture assigns logical CPUs 0 and 2 to distinct physical
  cores in package 0 (S5, lines 1–15 and 147–160). Their status words are
  `0x88438808` and `0x883F8808` and both target words are `0x83640000`
  (lines 1214, 1217, 1422 and 1425). The reference is 100 °C, and the
  separate activation offset field is 3 °C (S1, S5). The core results
  decode to 33 °C and 37 °C. All status observations in the capture have
  bit 23 clear (S5).
- For H81, TGL018 leaves the current-limit status at `0x19C[12]`
  potentially inaccurate. The core decode uses the digital-readout and
  validity fields. TGL014 limits PECI signaling. The H81 column marks
  the package-C10 and BIOS PROCHOT# errata TGL019 and TGL020 inapplicable
  (S2).

## Safety notes

Use only the two signed reads (S7). Neither thermal status log clearing
nor any thermal, power or interrupt write forms part of core sampling.
Preserve independent package and fallback results on partial or
unavailable core sampling.

## Synthetic fixture acceptance

| Fixture | Expected behavior |
| --- | --- |
| Exact R0, core DTS set, PTM clear, complete one-package/one-group topology with SMT or without SMT | One representative per detected physical core, with the two allowed reads. |
| `0x000806C1`, `0x000806C2`, `0x000806D0`, `0x000806D2`, wrong vendor, absent leaf or DTS clear | No core topology query or MSR read under this scope. |
| Admitted discovery followed by wrong pinned processor, signature mismatch or missing core DTS at the representative | No read pair for that target. Preserve other valid samples under S7. |
| Valid target 100 °C, delta 30 °C and activation offset 0 or 3 °C. Repeat with valid zero delta. | 70 °C for either offset and 100 °C for zero delta. |
| Synthetic valid statuses `0x889E0000` and `0x881E0000` on separate cores with target 100 °C | Bit-23-set sample unavailable. The other core yields 70 °C. |
| Fault, target zero or outside the 50–120 °C policy range, clear bit 31, or delta equal to or greater than the target | That core unavailable. Preserve valid siblings and accept either target boundary when all other fields qualify. |
| Multiple groups or packages, topology changed, timeout or affinity restore failure | Apply S7's core-set validity and cache rules. Preserve independent package and fallback results. |

## Open questions

- Non-blocking for #2514 Experimental core DTS: the proposed path rejects an ambiguous sample before decoding.
  Intel reserves bit 23
  (S4), while Linux explicitly includes it in its `06_8D` core mask
  (S6, lead-only). Bit 23 is clear in the independent exact capture (S5).
- Non-blocking for #2514 Experimental core DTS: Intel documents the shared exact mechanism and the public i7-11800H capture corroborates the read pair.
  The independent raw
  capture covers the i7-11800H only. Xeon and embedded product coverage
  rests on Intel's exact shared mechanism and product evidence (S1–S4).
  No HardwareVisualizer Windows service run was verified.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-13 | Proposes Implementation-ready status for exact Tiger Lake-H R0 Experimental core DTS, pending maintainer approval. |
