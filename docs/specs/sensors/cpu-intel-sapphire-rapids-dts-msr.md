# Spec: Intel Sapphire Rapids `0x000806F8` core DTS

| Field | Value |
| --- | --- |
| Revision | 2 |
| Status | Implementation-ready (rev 2) |
| Scope | Proposed Experimental Windows physical-core DTS for exact non-hybrid Sapphire Rapids `0x000806F8`, with one physical package and one active processor group, or exactly two active groups on Windows 11 x64. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

Revision 2 proposes an Experimental two-group extension for [#2700](https://github.com/shm11C3/HardwareVisualizer/issues/2700).
It requires maintainer approval and a merged spec pin before implementation.
Implementation also requires the merged revision 1 sampling path from [#2692](https://github.com/shm11C3/HardwareVisualizer/pull/2692).
The existing single-group scope continues to use the revision 1 procedure.

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
| S8 | Microsoft [Processor Groups](https://github.com/MicrosoftDocs/win32/blob/bc82a65f74ef2c4f2f44a3d7db5f7cfebfdc3583/desktop-src/ProcThread/processor-groups.md#behavior-starting-with-windows-11-and-windows-server-2022) and [Windows 11 release information](https://learn.microsoft.com/en-us/windows/release-health/windows11-release-information#version-21h2-os-build-22000---end-of-updates) | Primary Windows scheduling model, group-relative numbering and Windows 11 client baseline. The two-group scope uses the Windows 11 x64 scheduling model. |
| S9 | Microsoft Windows SDK API documentation at `656e9656bfaef34dd29cd7ee9e1146aec17e924c`, [GetLogicalProcessorInformationEx](https://github.com/MicrosoftDocs/sdk-api/blob/656e9656bfaef34dd29cd7ee9e1146aec17e924c/sdk-api-src/content/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex.md), [PROCESSOR_RELATIONSHIP](https://github.com/MicrosoftDocs/sdk-api/blob/656e9656bfaef34dd29cd7ee9e1146aec17e924c/sdk-api-src/content/winnt/ns-winnt-processor_relationship.md), and [GROUP_RELATIONSHIP](https://github.com/MicrosoftDocs/sdk-api/blob/656e9656bfaef34dd29cd7ee9e1146aec17e924c/sdk-api-src/content/winnt/ns-winnt-group_relationship.md) | Primary variable record sizes, core and package group masks, active groups and per-group masks. See Parameters, Members and Remarks. |
| S10 | Microsoft Windows SDK API documentation at the same revision, [SetThreadGroupAffinity](https://github.com/MicrosoftDocs/sdk-api/blob/656e9656bfaef34dd29cd7ee9e1146aec17e924c/sdk-api-src/content/processtopologyapi/nf-processtopologyapi-setthreadgroupaffinity.md), [GetThreadGroupAffinity](https://github.com/MicrosoftDocs/sdk-api/blob/656e9656bfaef34dd29cd7ee9e1146aec17e924c/sdk-api-src/content/processtopologyapi/nf-processtopologyapi-getthreadgroupaffinity.md), and [GetCurrentProcessorNumberEx](https://github.com/MicrosoftDocs/sdk-api/blob/656e9656bfaef34dd29cd7ee9e1146aec17e924c/sdk-api-src/content/processthreadsapi/nf-processthreadsapi-getcurrentprocessornumberex.md) | Primary hard single-group thread affinity, previous and current group affinity views, and the actual executing processor's group and number. See Parameters and Remarks. |
| S11 | [App temperature DTO](https://github.com/shm11C3/HardwareVisualizer/blob/c42b3e8bb89eec629c9f3860aeaea5570b48634a/src-tauri/src/models/hardware.rs#L43-L49), [App mapping](https://github.com/shm11C3/HardwareVisualizer/blob/c42b3e8bb89eec629c9f3860aeaea5570b48634a/src-tauri/src/services/hardware_service.rs#L140-L145), and [CPU temperature view](https://github.com/shm11C3/HardwareVisualizer/blob/c42b3e8bb89eec629c9f3860aeaea5570b48634a/src/features/hardware/components/CpuDetailTemperatures.tsx#L23-L34) | Existing group-relative identity across the App and view. The current row key uses both fields and the label uses just the group-local processor number. |
| S12 | Microsoft [Win32_OperatingSystem](https://github.com/MicrosoftDocs/win32/blob/bc82a65f74ef2c4f2f44a3d7db5f7cfebfdc3583/desktop-src/CIMWin32Prov/win32-operatingsystem.md), Syntax, Properties (BuildNumber, ProductType), Remarks and Requirements | Primary local `root/cimv2` singleton contract. BuildNumber is a read-only string containing the numeric OS build. ProductType is a read-only integer and 1 identifies a workstation. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, `GenuineIntel`, full `CPUID.01H:EAX = 0x000806F8` and `CPUID.06H:EAX[0] = 1` at discovery and on each confirmed representative | mechanism | S1, S3–S6 |
| Non-hybrid mechanism, available leaf `0x07` with `CPUID.(EAX=07H, ECX=0):EDX[15]` clear at discovery and on each confirmed representative | mechanism | S3, S5 |
| Exactly one physical package, one active group under revision 1 or two active groups on positively identified Windows 11 x64 under revision 2, with complete and consistent per-group physical-core masks | mechanism | S6, S8, S9, S12 and project scope. |
| Existing signed `IntelMSR` route, disposable sampling thread, confirmed target and topology recheck, with affinity restoration before publication. Two-group sampling first establishes a known explicit affinity baseline on its own thread. | safety | S6, S8–S10 and project policy. |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact non-hybrid `0x000806F8` core DTS in one group and package | S1–S4 establish the shared Sapphire Rapids mechanism and exact product overlap. S5 is independent raw Xeon W evidence. S7 is lead-only corroboration of the field width. | Proposed Experimental pending maintainer approval and merged specification. |
| Exact non-hybrid `0x000806F8` core DTS in one package across two active Windows 11 x64 groups | S1–S5 establish the same per-core DTS mechanism; S8–S10 establish the topology and affinity contract. S5 does not record Windows groups. S7 is lead-only corroboration of the field width. | Proposed Experimental for #2700, subject to the merged revision 1 implementation and maintainer approval. |
| Other signatures, multiple packages, unsupported group topology, incomplete topology or unavailable transport | Other topology extensions require separate qualification. Exact `0x000806F5`, `0x000806F6`, `0x000806F7` and Emerald Rapids `0x000C06F2` require separate qualification. | Outside this proposal. |

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

### One package across two active groups

Microsoft exposes variable-size relationship records. The declared record size
bounds its group mask array. A physical core has exactly one group mask. A
physical package can have more than one group mask (S9).

- Apply the common bounds to every request that needs core temperatures,
  including cache hits. Require successful enumeration with one group
  relationship reporting exactly two active groups, one package relationship
  containing one nonempty mask for each distinct active group, and one
  nonempty single-group mask per physical core. Reject truncated records,
  inconsistent sizes or counts (including any active count that differs
  from its mask's population), unknown or repeated group numbers within the
  package, and inactive bits. The package masks must equal the active masks
  for both groups. Core masks must be disjoint within each group and their
  union must equal that group's package mask. Compare snapshots by group,
  active and package masks, and the unordered set of group-scoped core masks.
  A topology mismatch or enumeration failure invalidates the core cache and
  prevents publication of a new core set (S6, S8, S9 and project policy).
- For an exact-signature demand with validated two-group topology, establish
  the scheduling baseline from the local read-only `Win32_OperatingSystem`
  singleton in the service. Require exactly one result, workstation
  `ProductType = 1` and an all-decimal `BuildNumber` that fits an unsigned
  32-bit integer and is at least 22000, the first Windows 11 client build.
  The service reapplies this boot-local result to later requests without
  requerying it on every poll. The initial lookup is subject to the
  existing core-pass deadline and in-flight limit. A missing, duplicate,
  invalid, unavailable or late result denies two-group reads for that
  pass. Read neither display name nor version branding. A single-group
  request does not need the lookup (S6, S8, S12 and project policy).
- On Windows 11, a default thread can execute in multiple groups, while
  `GetThreadGroupAffinity` reports only its primary group. Setting explicit
  group affinity confines that thread to one group. On older Windows,
  cross-group thread assignment can extend the process's group membership
  (S8, S10).
- Establish and verify an explicit singleton baseline on the disposable
  service sampling thread before any MSR read. Its group and processor must
  be active in the validated topology. The initial implicit state is
  allowed to end with this private thread. Leave other threads and process
  affinity untouched (S6, S8, S10 and project policy).
- Select one representative per core using the existing lowest-set-bit
  rule within that core's group. A failed singleton pin yields an unavailable
  core and causes no MSR access on it. Confirm group and group-relative
  processor number before each authorized read and after the pair. Recheck
  the exact CPU identity and capabilities on the pinned target. Restore
  and verify the explicit baseline after every successful pin, including
  fault and skip paths, before accepting that result or processing another
  target. A failed or unverifiable baseline or restoration invalidates the
  entire core set and cache. End this disposable thread without further
  sensor reads (S6, S10 and project policy).
- Retain a single in-flight core pass, one representative and up to two
  signed reads per core, and the existing deadline. An isolated pin, read
  or decode failure can leave that core unavailable while useful siblings
  remain. On pass expiry, stop scheduling representatives, discard the
  expired core set and wait for its private worker to exit before another
  pass. Keep the independently obtained package and fallback results
  (S6 and project policy).
- Keep the existing representative identity pair, Windows group and
  group-relative logical processor number, through delivery and rendering.
  Two-group results identify both numbers in visible labels, including
  unavailable rows. Single-group labels retain their existing format.
  Core identities are session-local to the validated topology (S6, S8,
  S11 and project policy).

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
  raw readings on that CPU. A configuration with all 72 logical processors exceeds one
  Windows group's capacity. Neither capture records a Windows group mask
  or validates two-group admission. Bit 23 is clear in all 112 ordinary
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
| w7-3475X observed words as decode fixtures, then a synthetic 36-core/72-logical-processor Windows 11 topology with one package, two valid groups, and complete SMT masks | Decode the observed words to 37 °C and 34 °C. In the synthetic topology, read one pair per confirmed physical-core representative and never read an SMT sibling as a second core. |
| Group-local logical processor 0 in both admitted groups, with one unavailable reading | Preserve both distinct identities and display both group and processor numbers. An unavailable row keeps its own identity. Single-group labels retain their existing format. |
| Two-group discovery with workstation build 21999, server build 26100, a missing/duplicate/invalid or late OS row, a third active group, a second package, truncated or inconsistent record size, repeated or outside group ID, overlapping cores, or a gap in either group's mask | Skip both core MSRs and invalidate any applicable core cache. Preserve independent package and fallback results. |
| A local singleton workstation build of 22000 on a valid exact two-group target, then a later request on the same service instance | Admit the Windows scheduling baseline and reuse its cached OS facts. Verify fresh topology before serving a sample. Repeat with one active group and verify that no OS lookup is necessary. |
| Default Windows 11 thread reports a primary-group-only affinity view. Repeat with failure to establish the explicit baseline, successful cross-group pin and restore, a restricted cross-group pin, and failed baseline restoration after a fault | Never use the initial affinity view as the full implicit state. On setup failure perform no core reads. Restore to the verified explicit baseline after a successful pin. On restricted pin leave the core unavailable without its MSRs. On restoration failure discard the full set and cache and end the disposable worker. |
| Verified pin followed by a group/processor mismatch, or by a read/validity failure in one group | Do not publish that core temperature. Preserve useful siblings if the baseline and topology remain valid. |
| Equivalent topology in reordered records, then a changed core mask in either group on a cache hit or before publication, or a pass deadline while a signed read is in flight | Record order does not affect identity or cache. Changed topology invalidates the set and cache. Deadline stops new reads, discards the expired set and blocks starting another pass until the old worker exits. Preserve independent package and fallback results. |
| Different signature including `0x000806F7` or `0x000C06F2`, wrong vendor, unavailable capability leaf, hybrid flag set or DTS clear | No reads under this scope. |
| Admitted discovery followed by wrong pinned processor, signature mismatch or loss of DTS at an individual representative | Leave that target unavailable before its reads and preserve other eligible cores. |
| Valid target 100 °C with delta 30 and activation offset 0 or 4 °C. Repeat with delta zero. | 70 °C for either offset and 100 °C for zero delta. |
| Fault, target zero or outside 50–120 °C, clear status validity or delta equal to or greater than target | Leave that core unavailable. Preserve valid siblings. Accept either target boundary when all other conditions pass. |
| Single-group multiple-package, incomplete or changed topology, pass timeout or affinity restore failure | Apply S6's existing topology, cache and core-set failure behavior. Preserve independent package and fallback results. |

## Open questions

- Non-blocking for #2514 Experimental core DTS: Independent raw evidence
  covers two Xeon W workstation processors. Scalable, CPU Max and Edge
  Enhanced coverage within the exact scope rests on Intel's shared
  register and product evidence (S1–S4). No HardwareVisualizer Windows
  service validation was performed.

- Non-blocking for #2700 Experimental two-group core DTS: Microsoft's documented topology and hard affinity govern runtime admission. The Xeon w7-3475X evidence establishes the thermal mechanism with 72 logical processors. It does not independently capture Windows group relationships or cross-group operation through HardwareVisualizer. An incomplete or unsupported runtime topology fails the admission checks (S5, S8–S10).

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-13 | Proposes transition to Implementation-ready for exact Sapphire Rapids `0x000806F8` Experimental core DTS, pending maintainer sign-off. |
| 2 | 2026-11-16 | Proposes Implementation-ready for the exact one-package, two-group Windows 11 Experimental extension after the revision 1 implementation merges, pending maintainer sign-off. |
