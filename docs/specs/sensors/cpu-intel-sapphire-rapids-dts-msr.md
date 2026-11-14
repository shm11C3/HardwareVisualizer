# Spec: Intel Sapphire Rapids `0x000806F8` core DTS

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Proposed Experimental Windows physical-core DTS for exact non-hybrid Sapphire Rapids `0x000806F8`, with one active processor group and one physical package. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

This revision proposes Experimental coverage. Maintainer approval of the
specification is required before implementation.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel [SDM Volume 4](https://cdrdv2.intel.com/v1/dl/getContent/671098), 335592-093US, September 2026, Table 2-2 (pp. 2-21–2-22), §2.17 and Table 2-39 (pp. 2-359–2-361), §2.17.8 and Table 2-52 (pp. 2-428–2-440) | Primary `06_8F` applicability and core status fields. The supplemental model table does not replace `0x19C`. |
| S2 | Intel [Sapphire Rapids Data Sheet Vol. 2 Registers](https://cdrdv2-public.intel.com/814094/814094__SPR_Data_Sheet_Vol_2_Registers_Rev_001.pdf), 814094 rev 001, July 2025, §6.1.5 (pp. 43–44) | Primary definition of the platform temperature target register and its MSR alias. REF_TEMP is the maximum junction reference. The activation offset affects throttling and leaves the temperature reading unchanged. The PCI register is evidence only. |
| S3 | Intel [INTEL-SA-01442 affected products](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-01442.html), exact `806F8` rows, [Affected Processors by CPU Model, February 2025](https://www.intel.com/content/dam/develop/public/us/en/include/software-security/affected-processors-2025tab-february2025.html), `06_8F` entries, and [Xeon W-3400/W-2400 media brief](https://download.intel.com/newsroom/2023/5g-communications/Intel-Xeon-W-3400-Xeon-W-2400-Media-Brief.pdf) | Primary exact overlap of workstation and server products, including Xeon CPU Max, and non-hybrid identification. The platform brief distinguishes the 64-lane and 112-lane workstation platforms. |
| S4 | Intel [4th Gen Xeon Scalable identification](https://edc.intel.com/content/www/de/de/design/products-and-solutions/processors-and-chipsets/eagle-stream/sapphire-rapids-specification-update/identification-information/), 772415-020US, March 2026, Component Identification via Capability Registers, [errata summary](https://edc.intel.com/content/www/us/en/design/products-and-solutions/processors-and-chipsets/eagle-stream/sapphire-rapids-specification-update/errata-summary-table/?language=en) and [errata details](https://edc.intel.com/content/www/us/en/design/products-and-solutions/processors-and-chipsets/eagle-stream/sapphire-rapids-specification-update/errata-details/), 772415-022US, August 2026, and [Edge Enhanced Specification Update](https://cdrdv2-public.intel.com/784461/784461_SapphireRapidsEE_SpecificationUpdate_002.pdf), 784461 rev 002, September 2023, Table 1 (p. 9) and errata (pp. 10–18) | Primary exact `806F8` on XCC E-5, MCC S-3, HBM B-3 and Edge Enhanced L-0/U-1, including supported single-socket configurations. Errata applicability for those configurations. |
| S5 | InstLatx64 commit `17be9a9793c306cf96f8c286e34e705b780a0226`, [index](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/index.htm#L720-L721), [Xeon w7-2475X](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00806F8_SapphireRapids_05_CPUID.txt), blob `bd8586f689af21a6bc402d5c5944d39cbb82f240`, and [Xeon w7-3475X](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00806F8_SapphireRapids_06_CPUID.txt), blob `ace2bafe70c4005fae7971634b337dc5daa0a046` | Independent exact signature, capability, package and core mapping, and paired per-logical-CPU raw thermal MSRs. See Quirks for line ranges and observed topology limits. |
| S6 | [Intel core DTS revision 8](https://github.com/shm11C3/HardwareVisualizer/blob/6a59d7214c042e8cc75fc43caf760c3fe9948932/docs/specs/sensors/cpu-intel-core-dts-msr.md), Detection, Read procedure and decode, Project collection constraints and Safety notes, and [PawnIO interface revision 7](https://github.com/shm11C3/HardwareVisualizer/blob/2495a1b0fbe4a8048feac9d122b98201e7970cbb/docs/specs/sensors/pawnio-interface.md#intelmsr), signed `IntelMSR` `0.2.11`, module source commit `52a7e536dff3e53c96917a28caac5e0fa6510696` | Existing Windows topology, affinity, collection bounds and transport. Both synchronous reads are authorized on the calling processor. |
| S7 | Linux v6.18 commit `7d0a66e4bb9081d75c82ec4957c50034cb0ea449`, [Sapphire Rapids model ID](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/arch/x86/include/asm/intel-family.h#L122), [mask](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/thermal/intel/intel_tcc.c#L43) and [selection](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/thermal/intel/intel_tcc.c#L93) | **lead-only (copyleft)** corroboration of the seven-bit core readout. The separate package readout in this source is outside this proposal. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, `GenuineIntel`, full `CPUID.01H:EAX = 0x000806F8` and `CPUID.06H:EAX[0] = 1` at discovery and on each confirmed representative | mechanism | S1, S3–S6 |
| Non-hybrid mechanism, available leaf `0x07` with `CPUID.(EAX=07H, ECX=0):EDX[15]` clear at discovery and on each confirmed representative | mechanism | S3, S5 |
| Exactly one active Windows processor group and one physical package with complete and consistent Windows physical-core masks | mechanism | S6 |
| Existing signed `IntelMSR` route, disposable sampling thread, confirmed target and topology recheck, with affinity restoration before publication | safety | S6 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact non-hybrid `0x000806F8` core DTS within the Windows topology limit | S1–S4 establish the shared Sapphire Rapids mechanism and exact product overlap. S5 is independent raw Xeon W evidence. S7 is lead-only corroboration of the field width. | Proposed Experimental pending maintainer approval and merged specification. |
| Other signatures, multiple packages or active groups, incomplete topology or unavailable transport | The common limits apply to both workstations and servers. Exact `0x000806F5`, `0x000806F6`, `0x000806F7` and Emerald Rapids `0x000C06F2` require separate qualification. | Outside this proposal. |

An eligible exact-signature server or CPU Max configuration uses the same
mechanism when all gates pass. The workstation and server names record
evidence; they do not select the path. Package thermal management has separate
eligibility, and its capability bit is independent of core DTS (S6).

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x19C` | `IA32_THERM_STATUS` | 31 | Core DTS reading valid when set | Boolean | S1, S5 |
| `0x19C` | `IA32_THERM_STATUS` | 22:16 | Digital readout below the junction reference | Unsigned degrees Celsius | S1, S2, S5 |
| `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Maximum junction reference used for the DTS conversion | Degrees Celsius | S2, S5 |

## Read procedure and decode

Use S6's existing non-hybrid sampling procedure and physical-core identities.
Read `0x1A2` followed by `0x19C` synchronously on one confirmed logical
representative per Windows physical core. Require successful reads, a target
in 50–120 °C inclusive, and status bit 31 set. Subtract the unsigned
seven-bit delta from the reference. Accept a positive result no higher than
the target. A valid zero delta yields the target. The separate throttle offset
does not enter the calculation (S2). The Intel core field excludes bit 23
and the model-specific corroboration agrees (S1, S7).

Recheck the running processor and restore affinity. Preserve other samples
after an isolated fault or invalid word. Inconsistent topology or failed
affinity restoration invalidates the core set. Apply S6's bounded demand,
timeout, serialization and cache rules.

## Quirks

- S5 assigns logical processors 0 and 2 to physical cores 0 and 1 in
  package 0 in both captures (lines 1–22 and 173–194). The 20-core w7-2475X
  has 40 logical processors and reads `0x88450800` and `0x88460800` for
  status with `0x045E0800` as both targets (lines 3510, 3513, 3716, 3719).
  The 94 °C reference and 69/70 °C deltas produce 25/24 °C. The target
  includes a separate nonzero activation offset.
- The 36-core w7-3475X has 72 logical processors in its capture. Statuses
  `0x883A0400` and `0x883D0400` have the same target `0x805F0800` (lines
  6294, 6297, 6500, 6503), giving 37/34 °C. This proves distinct per-core
  raw readings on that CPU. A configuration with all 72 logical processors
  active falls outside the single-group scope. Neither capture validates
  Windows processor-group admission. Bit 23 is clear in all 112 ordinary
  per-logical-processor status observations across both captures.
- S4's published errata do not identify a change to these thermal readout
  fields. SPR149 concerns CPUID leaf `0x0B` inside a Trust Domain. Core
  attribution in this proposal uses the validated Windows relationships
  (S6). SPR53 concerns a different MSR and SPR118 concerns writes to other
  MSRs. SPR130's potential instruction delay on SMT systems remains
  subject to the common sampling deadline and failure handling (S6).

## Safety notes

Use only the two signed reads (S6). Core sampling does not clear thermal
logs or change thermal, power or interrupt state. No PCI, PECI, platform
capability-register, BMC or additional MSR access is needed. Preserve
independent package and fallback results.

## Synthetic fixture acceptance

| Fixture | Expected behavior |
| --- | --- |
| Exact `0x000806F8`, core DTS set, PTM clear, complete one-package/one-group topology with SMT or without SMT | One pair of authorized reads per physical-core representative. CPU brand, chipset, nominal core count and PTM are independent of this eligibility. |
| w7-2475X observed pair on distinct admitted cores | Return 25 °C and 24 °C. |
| w7-3475X observed words as decode fixtures, then 72 active logical processors across multiple Windows groups | Decode the individual words to 37 °C and 34 °C. Reject the actual multi-group topology before reading any core. |
| Different signature including `0x000806F7` or `0x000C06F2`, wrong vendor, unavailable capability leaf, hybrid flag set or DTS clear | No reads under this scope. |
| Admitted discovery followed by wrong pinned processor, signature mismatch or loss of DTS at an individual representative | Leave that target unavailable before its reads and preserve other eligible cores. |
| Valid target 100 °C with delta 30 and activation offset 0 or 4 °C. Repeat with delta zero. | 70 °C for either offset and 100 °C for zero delta. |
| Fault, target zero or outside 50–120 °C, clear status validity or delta equal to or greater than target | Leave that core unavailable. Preserve valid siblings. Accept either target boundary when all other conditions pass. |
| Multiple packages, incomplete or changed topology, timeout or affinity restore failure | Apply S6's topology, cache and core-set failure behavior. Preserve independent package and fallback results. |

## Open questions

- Non-blocking for #2514 Experimental core DTS: Independent raw evidence
  covers two Xeon W workstation processors. Scalable, CPU Max and Edge
  Enhanced coverage within the exact scope rests on Intel's shared
  register and product evidence (S1–S4). No HardwareVisualizer Windows
  service validation was performed.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-13 | Proposes transition to Implementation-ready for exact Sapphire Rapids `0x000806F8` Experimental core DTS, pending maintainer sign-off. |
