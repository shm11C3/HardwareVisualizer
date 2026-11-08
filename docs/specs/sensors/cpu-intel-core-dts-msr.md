# Spec: Intel core temperature via digital thermal sensor MSRs

| Field | Value |
| --- | --- |
| Revision | 4 |
| Status | Implementation-ready (rev 4) |
| Scope | Read-only core DTS on the listed Intel x86-64 signatures in a single Windows processor group and physical package. Includes DTS without package thermal management and two exact hybrid signatures. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

Revision 4 proposes a readiness update for the exact hybrid rows. The
maintainer approves the revision under the
[specification gate](README.md#status-transition-draft--implementation-ready).

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | [Intel SDM Volume 4, 335592-078US, December 2022](https://cdrdv2-public.intel.com/671098/335592-sdm-vol-4.pdf), Tables 2-1 and 2-2 (`19CH`, pp. 2-18–2-19), §2.4 and Table 2-6 (`19CH`, `1A2H`, p. 2-95), §2.4.1 and Table 2-10 (p. 2-107), §2.8 and Table 2-15 (`19CH`, `1A2H`, pp. 2-146–2-147), §2.11 and Table 2-20 (`19CH`, `1A2H`, pp. 2-183–2-185) | Primary model signatures and MSR scopes and fields |
| S2 | [Package DTS specification](cpu-intel-dts-msr.md) revision 3, sources S1–S3 and Detection/Register map | Intel SDM 325462-075US, June 2021, Volume 2A CPUID leaf 06H and Volume 3B §14.8.5.2, core DTS availability, semantics and field layout |
| S3 | [PawnIO interface](pawnio-interface.md) revision 7, `IntelMSR` section | `0x19C` and `0x1A2` read authorization and synchronous calling-processor execution |
| S4 | Microsoft Learn, [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex), [SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-system_logical_processor_information_ex), [PROCESSOR_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_relationship), [GROUP_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-group_relationship) and [PROCESSOR_GROUP_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_group_info) | Primary Windows active-core, package and group mask relationships |
| S5 | Microsoft Learn, [Processor Groups](https://learn.microsoft.com/en-us/windows/win32/procthread/processor-groups), [GROUP_AFFINITY](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-group_affinity), [SetThreadGroupAffinity](https://learn.microsoft.com/en-us/windows/win32/api/processtopologyapi/nf-processtopologyapi-setthreadgroupaffinity) and [GetCurrentProcessorNumberEx](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getcurrentprocessornumberex) | Primary Windows processor identity and thread affinity contract |
| S6 | [ADR 0029](../../adr/0029-windows-sensor-service.md), [service request handling](../../../core/src/sensor_service/windows/server.rs), [sampling cache](../../../core/src/sensor_service/windows/providers.rs), [service client](../../../core/src/sensor_service/windows/client.rs), [core affinity sampler](../../../core/src/infrastructure/providers/windows/pawn_io/cpu/core_temperature.rs) and [PawnIO client](../../../core/src/infrastructure/providers/windows/pawn_io/client.rs) | Project boundary and current call chain. The core pass uses a disposable thread and an independent synchronous PawnIO client. The service client has a two-second exchange deadline. |
| S7 | InstLatx64 [Bloomfield CPUID dump](https://instlatx64.github.io/InstLatx64/GenuineIntel/GenuineIntel00106A4_Bloomfield_CPUID.txt) and [Atom C2750 CPUID dump](https://instlatx64.github.io/InstLatx64/GenuineIntel/GenuineIntel00406D8_Avoton_CPUID2.txt), leaves 1 and 6 | Independent observations of the DTS-only capability combination. Neither dump establishes thermal readback or application behavior. |
| S8 | Intel, [Atom C2000 Product Family Specification Update, 329460-020US, June 2018](https://www.intel.com/content/dam/www/public/us/en/documents/specification-updates/atom-c2000-family-spec-update.pdf), Table 9 (p. 16) and erratum AVR32 (p. 26) | Primary retail CPUID corroboration and a low-temperature reporting limit on C2308 and C2508 |
| S9 | Intel, [SDM Documentation Changes, 252046-081, October 2025](https://cdrdv2-public.intel.com/868136/252046-081-sdm-change-document.pdf), replacement Volume 4 §2.17 and Table 2-39 (pp. 2-353–2-355), §2.17.5 (p. 2-393), and Volume 1 Tables 21-17 and 21-63 (pp. 21-29 and 21-66) | Primary hybrid display-model coverage, core scope for `0x19C`, hybrid MSR exceptions, hybrid flag and per-logical-processor core type. |
| S10 | Intel, [12th Generation Intel Core Processor Datasheet Vol. 1, 655258-011, May 2025](https://cdrdv2-public.intel.com/655258/655258-011.pdf), §§4.1.3.1–4.1.3.2 (pp. 76–79), and [Specification Update, 682436-037, January 2026](https://cdrdv2-public.intel.com/682436/682436_037.pdf), §2.1 Table 2-1 and §3 Summary Tables of Changes | Primary exact CPUID for Alder Lake S and HX, core temperature reference and TCC offset, and errata applicability. |
| S11 | Intel, [13th and 14th Generation Intel Core Processor Datasheet Vol. 1, 743844-015, May 2025](https://cdrdv2-public.intel.com/743844/743844-015.pdf), §§4.1.3.1–4.1.3.2 (pp. 80–83), §15.1 Table 101 (p. 219), and [Specification Update, 740518-028, February 2026](https://cdrdv2-public.intel.com/740518/740518_028.pdf), §2.1 and §3 Summary Tables of Changes | Primary exact Raptor Lake S/HX and S/HX Refresh CPUID, core temperature reference and TCC offset, and errata applicability. Table 101 also records the signature shared by the non-hybrid E line. |
| S12 | Intel, [Best Practices for Real-Time Optimizations With the 12th Generation Intel Core Processors, 737973-1.1, December 2022](https://cdrdv2-public.intel.com/737973/737973_best_practices_for_real-time_optimizations_rev1.1.pdf), §§1.0, 2.1 and 2.4.1 and Table 2 (pp. 5–6 and 8–9), and [Affected Processors by CPU Model, April 2025 table](https://www.intel.com/content/dam/develop/public/us/en/include/software-security/affected-processors-2025tab-april2025.html), entries for `90672` and `B0671` | Primary corroboration of the hybrid core microarchitectures and the CPUID hybrid flag that distinguishes Raptor Lake S/HX from the same-signature non-hybrid Xeon E. |

## Detection

- CPUID leaf 0 reports `GenuineIntel` and a maximum basic leaf at least
  06H. Use the CPUID.01H display family and display model from Table 2-1
  (S1, S2).
- CPUID.06H:EAX[0] advertises core DTS. CPUID.06H:EAX[6] independently
  advertises the package MSR. The latter may be clear on a usable core
  DTS path. A clear bit 6 permits no package-status MSR read (S2).
  The Bloomfield and Atom C2750 observations in S7 report leaf-6 EAX
  values `0x00000003` and `0x00000005`, respectively. Both have bit 0
  set and bit 6 clear.
- Windows returns one `RelationProcessorCore` record per active
  physical core. Its group mask contains the logical siblings of that
  core. A `RelationProcessorPackage` record identifies the logical
  processors of a physical package. Use the group number and
  group-relative processor number together (S4, S5).
- The hybrid rows require the full CPUID.01H:EAX signature below and
  CPUID.07H.00H:EDX[15] set. Apply these gates before topology discovery
  and check them again on each pinned representative. A shared signature
  without the hybrid bit does not qualify. The required Windows topology
  checks are defined under Project collection constraints (S9–S12).

| Scope | Status | Default enablement |
| --- | --- | --- |
| Nehalem `06_1A`, `06_1E`, `06_1F`, `06_2E` | Verified by S1 §2.8 and Table 2-15 for signatures and target. S2 pins the core DTS and separate PTM bit. | Enabled subject to the capability, affinity and validity gates. |
| Sandy Bridge `06_2A`, `06_2D` | Verified by S1 §2.11 and Table 2-20 for signatures and target. S2 pins the core DTS and separate PTM bit. | Enabled subject to the capability, affinity and validity gates. |
| Silvermont Atom C2000 `06_4D` | Verified by S1 §2.4/§2.4.1 and Tables 2-6/2-10 for the signature, core DTS and package target. S2 pins the capability bits. | Enabled only with a zero target offset and the common gates. |
| Hybrid Alder Lake S/HX `06_97`, full signature `0x00090672` (stepping ID 2) | S9/S10 pin the signature, P- and E-core status and reference. S12 corroborates hybrid identification. | Experimental, subject to the common gates and per-target core type below. |
| Hybrid Raptor Lake S/HX and S/HX Refresh `06_B7`, full signature `0x000B0671` (B-0) | S9/S11 pin the signature, P- and E-core status and reference. S11/S12 identify the non-hybrid signature overlap. | Experimental, subject to the common gates and per-target core type below. |
| Pre-Nehalem Core 2 and earlier Atom | Temperature reference unresolved for a model-scoped absolute reading. | Disabled. Follow-up in #2514. |
| Other signatures, unrecognized hybrid core types, or multiple processor groups/packages | Outside the evidence or attribution scope. Existing package collection remains covered by the [package DTS specification](cpu-intel-dts-msr.md). | Disabled for core reads. Follow-up in #2514. |

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| MSR `0x19C` | `IA32_THERM_STATUS` | 31 | Core digital readout valid when set | Flag | S1 Table 2-2, S2, S9 Table 2-39 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 22:16 | Core digital readout below the DTS reference | Unsigned degrees Celsius | S1 Table 2-2, S2, S9 Table 2-39 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Temperature target | Degrees Celsius | S1 Tables 2-6, 2-15, 2-20 |
| MSR `0x1A2`, model `06_4D` | `MSR_TEMPERATURE_TARGET` | 29:24 | Programmable target offset | Degrees Celsius | S1 Table 2-6 |
| MSR `0x1A2`, the two hybrid signatures | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated Tjmax, the DTS reference | Degrees Celsius | S10/S11 §§4.1.3.1–4.1.3.2 |
| MSR `0x1A2`, the two hybrid signatures | `MSR_TEMPERATURE_TARGET` | 29:24 | Programmable TCC activation offset, which affects the thermal control threshold | Degrees Celsius | S10/S11 §4.1.3.1 |

## Read procedure and decode

1. Select one representative logical processor per Windows core record.
   Use the least set bit in its group mask as its session-local identity.
   Keep core identity independent of sampling order or sample validity.
   The mask's other bits are SMT siblings, so they do not produce
   additional temperature channels (S4, S5).
2. On the synchronous sampling thread, set the representative's group
   affinity with just its bit and capture the previous affinity. Confirm
   the running processor with `GetCurrentProcessorNumberEx`. Skip the
   core if affinity could not be set or the group/number differs.
   Confirm the expected CPUID vendor, signature and DTS support on the
   target processor before reading. For a hybrid signature also require
   a supported CPUID.1AH.00H leaf on that processor and EAX[31:24] of
   `0x40` (Intel Core, P-core) or `0x20` (Intel Atom, E-core). Skip the
   target before MSR access if this leaf is absent or reports another type.
   The type is a qualification input. The existing core identity
   remains the Windows group and representative processor number
   (S1, S2, S4, S5, S9, S12).
3. Read `0x1A2` and `0x19C` through the same calling processor using the
   signed `IntelMSR` module. Check the running group and processor
   again after the reads while still pinned, and discard the core
   result if either differs. Accept a successful target in 50–120 °C
   inclusive and a core read with bit 31 set. For `06_4D`, reject the
   sample if bits 29:24 of `0x1A2` are nonzero (S1–S3, S5, project
   gate).
4. Subtract the unsigned digital readout from the target. Apply the
   existing plausibility gate `0 < temperature ≤ target`. A valid
   digital readout of zero returns the target temperature. For both
   hybrid signatures the readout remains relative to Tjmax at zero
   and nonzero TCC offsets. Use the target byte directly for the
   subtraction, regardless of the offset or its averaging window.
   A faulted read, zero target, clear validity bit or failed plausibility
   gate leaves the core unavailable (S2, S10–S11, project gate).
5. Restore the previous affinity after each successful pin, including
   failed read paths. Admit each core result only after affinity
   restoration succeeds (S5, project gate).

## Project collection constraints

- Use the existing typed sensor-service boundary and CPU acquisition
  cadence. Core MSR reads require current monitoring demand and a due
  sample. Share a rate-limited acquisition among concurrent requests.
  A cache hit retains its original acquisition time. Without a request
  that needs core temperatures, perform no core topology queries or
  affinity changes (S6, project policy).
- For every request that needs core temperatures on a matching CPU,
  validate the active-group, package and core relationships, including
  on a cache hit. Accept exactly one active group and one physical
  package in that group. The package mask must equal that group's
  active mask. Each core mask must be nonempty, contained in the
  package mask and disjoint from the other core masks. Their union
  must equal the package mask.
  Use the group number, active/package masks and unordered set of core
  masks to compare topology snapshots. A malformed or failed enumeration
  yields no core data (S4, project gates).
- Collect on a dedicated disposable service thread, never on the shared
  blocking pool. Allow one core pass in flight per service, with at
  most one target and two MSR reads per validated core and no retry
  sweep. The wait budget must fit within the existing service exchange
  deadline and allow the batch to return without core results. On
  expiry, stop scheduling targets, discard the expired result and wait
  for that pass to exit before starting another one. An already-running
  synchronous PawnIO call can continue past the deadline. A timed-out
  core call must not block unrelated providers through a shared
  executor or sampler lock (S3, S6, project policy).
- An affinity restore failure invalidates the entire core pass and its
  cached core results. Terminate the dedicated thread without performing
  any more sensor operations on it. A later pass requires a fresh
  thread and fresh topology validation. A pinned thread must never
  return to a shared worker pool (S5, S6, project policy).
- Recheck the topology before publishing newly acquired core readings.
  If enumeration fails or the snapshots differ, discard the core set
  and invalidate the core cache. If the current topology differs from
  the topology of a cached set, discard that cache even when it is still
  within its rate limit. Core identifiers are session-local and scoped
  to the validated topology. Never transfer a previous temperature to a
  new core because its representative processor number was reused
  (S4, project policy).
- Preserve usable siblings for isolated read failures within a
  successfully validated pass. Core-pass failure leaves existing
  package readings and ACPI fallback independently available. Core
  temperatures never populate package temperature or package
  thermal-status fields.

### Synthetic acceptance fixtures

| Fixture | Expected behavior |
| --- | --- |
| Exact supported signature, core DTS set, PTM clear | Eligible for core target/status reads. No `0x1B1` read. A valid core temperature never replaces the package/ACPI value. |
| Exact supported signature, core DTS set, PTM set | Eligible for core reads. The existing package path samples independently. |
| Exact signature, core DTS clear, PTM clear or set | No core MSR reads in either case. Package behavior remains governed by its existing gates. |
| Unsupported signature, otherwise valid capabilities and topology | No core topology query or core MSR reads. |
| Each exact hybrid signature with one SMT P-core and multiple E-cores in one group and package, all advertising core DTS | One temperature per Windows physical-core record. The P-core's second logical processor adds no channel. Each E-core keeps its own slot even when several E-cores share other hardware resources. |
| Hybrid full signature `0x000B0671` with hybrid flag clear, or another full signature such as `0x00090675` or `0x000B06F2` | No new core topology query or core MSR reads. |
| Exact hybrid discovery succeeds, but an individual pinned target has a different full signature, no core DTS, no leaf 1AH, or an unknown core type | Skip that target before MSR access. Preserve eligible core siblings and the independent package/fallback result. |
| Each hybrid signature with Tjmax 100 °C, TCC offset 10 °C, and valid digital readouts of 30 and 0 | 70 °C and 100 °C respectively. Changing the TCC offset or its averaging window does not change either decoded value. |
| One package with SMT siblings, reordered enumeration and one failed target or core read | One representative per physical core. Reordering alone preserves identities. The isolated failure leaves its position unavailable and preserves valid siblings. |
| Topology changes during a fresh pass or between cache acquisition and reuse | No output from the mismatched core set. Invalidate the cached core set even if its age is acceptable. Verify that reused representative processor numbers do not inherit old values. |
| Restore fails after one successful core and one attempted core | Discard the entire core set and cache. No subsequent reads on the disposable thread. A later pass uses a fresh thread. Package and ACPI results remain independent. |
| No demand, concurrent requests, rate-limit hit or expired wait | No idle reads. Requests share at most one in-flight pass. Cache age stays tied to acquisition. No second pass starts while a timed-out pass is still in flight. |
| Synthetic target and status words at validity and temperature boundaries | Reject missing/zero or out-of-range targets, clear Reading Valid and nonzero Silvermont offsets. Accept a valid zero digital readout at the target and reject a decoded temperature of zero or less. |

## Quirks

- The same-target read follows the model-specific scope in S1.
  Table 2-15 labels `0x1A2` Thread for Nehalem, Table 2-20 labels it
  Unique for Sandy Bridge, and Table 2-6 labels it Package for the
  Silvermont models. S3 documents calling-processor execution for
  the signed module. Its package-scope wording applies where S1
  documents a package-scope target.
- S9 specifies core scope for hybrid `IA32_THERM_STATUS` and lists
  P- and E-core MSR exceptions separately. Use the same pinned processor
  for both thermal reads, as on the existing path. The exact new
  signatures cover the listed S and HX processor lines and the
  documented S/HX Refresh variants. Core numbering and labeling remain
  based on Windows core records (S4, S9–S12).
- No universal TjMax is assumed. Pre-Nehalem Core and earlier Atom
  signatures that lack an evidenced target MSR need a separate
  processor-specific temperature reference before they can be enabled.
- Intel erratum AVR32 documents a core DTS lower limit of −27 °C on
  Atom C2308 and C2508, with no workaround (S8). The core path's
  positive-temperature gate excludes that range. The separate uncore
  sensor covered by that erratum is outside this specification.
- The core value represents the Windows topology record it came from.
  A missing core cannot shift another reading into its position. No
  relationship to a persistent system-wide physical core ID is claimed.

## Safety notes

- Read only the documented MSRs, on a confirmed processor and with the
  corresponding capability and model gates. Do not write an MSR, change
  thresholds, clear logs, or override firmware configuration (S1–S3).
- Affinity is per-thread. Keep the affinity interval synchronous and
  bounded to the target's reads. Do not repurpose a process-wide
  affinity mask as the physical core map (S3–S5).

## Open questions

- Non-blocking for #2514 single-group scope: only the listed exact signatures and one physical package and active group qualify.
  Other hybrid signatures need their own applicability evidence.
  Multi-package/group systems need a separately validated attribution
  and affinity-restore contract.
- Non-blocking for #2514 initial scope: Silvermont samples with nonzero target offsets are rejected.
  The relationship of a nonzero offset to the digital readout remains
  outside this slice.
- Non-blocking for #2514 initial scope: topology attribution uses matching validated observations and confirmed read targets.
  The Windows topology interface cannot distinguish a topology change
  that occurs and reverses between observations.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-07 | Initial fact-only draft for the listed single-group, single-package models and independent DTS/PTM detection. |
| 2 | 2026-11-07 | Documented service placement, bounded request-driven collection, topology and affinity failure handling, and synthetic acceptance fixtures. |
| 3 | 2026-11-08 | Proposed transition to Implementation-ready for the exact single-group, single-package scope. Pinned the model-specific target scopes and Windows group-mask definitions and recorded DTS-only CPUID observations and the scoped Atom erratum. |
| 4 | 2026-11-08 | Propose Implementation-ready scope for two exact Alder Lake and Raptor Lake hybrid signatures. Pin per-core qualification, Tjmax and its relationship to the TCC offset, and the existing single-group topology contract. |
