# Spec: Intel core temperature via digital thermal sensor MSRs

| Field | Value |
| --- | --- |
| Revision | 3 |
| Status | Implementation-ready (rev 3) |
| Scope | Read-only core DTS on the listed Intel x86-64 display-family/model signatures in a single Windows processor group and physical package. Includes DTS without package thermal management. Existing package DTS is specified separately. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | [Intel SDM Volume 4, 335592-078US, December 2022](https://cdrdv2-public.intel.com/671098/335592-sdm-vol-4.pdf), Tables 2-1 and 2-2 (`19CH`, pp. 2-18–2-19), §2.4 and Table 2-6 (`19CH`, `1A2H`, p. 2-95), §2.4.1 and Table 2-10 (p. 2-107), §2.8 and Table 2-15 (`19CH`, `1A2H`, pp. 2-146–2-147), §2.11 and Table 2-20 (`19CH`, `1A2H`, pp. 2-183–2-185) | Primary model signatures and MSR scopes and fields |
| S2 | [Package DTS specification](cpu-intel-dts-msr.md) revision 3, sources S1–S3 and Detection/Register map | Intel SDM 325462-075US, June 2021, Volume 2A CPUID leaf 06H and Volume 3B §14.8.5.2, core DTS availability, semantics and field layout |
| S3 | [PawnIO interface](pawnio-interface.md) revision 7, `IntelMSR` section | `0x19C` and `0x1A2` read authorization and synchronous calling-processor execution |
| S4 | Microsoft Learn, [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformation), [SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-system_logical_processor_information_ex), [PROCESSOR_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_relationship), [GROUP_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-group_relationship) and [PROCESSOR_GROUP_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_group_info) | Primary Windows active-core, package and group mask relationships |
| S5 | Microsoft Learn, [Processor Groups](https://learn.microsoft.com/en-us/windows/win32/procthread/processor-groups), [GROUP_AFFINITY](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-group_affinity), [SetThreadGroupAffinity](https://learn.microsoft.com/en-us/windows/win32/api/processtopologyapi/nf-processtopologyapi-setthreadgroupaffinity) and [GetCurrentProcessorNumberEx](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getcurrentprocessornumberex) | Primary Windows processor identity and thread affinity contract |
| S6 | [ADR 0029](../../adr/0029-windows-sensor-service.md), [service request handling](../../../core/src/sensor_service/windows/server.rs), [sampling cache](../../../core/src/sensor_service/windows/providers.rs), [service client](../../../core/src/sensor_service/windows/client.rs) and [PawnIO client](../../../core/src/infrastructure/providers/windows/pawn_io.rs) | Project boundary and current call chain. The service handles sample requests on a shared blocking pool and invokes PawnIO synchronously. The client has a two-second exchange deadline. The current service has no per-core affinity sampler. |
| S7 | InstLatx64 [Bloomfield CPUID dump](https://instlatx64.github.io/InstLatx64/GenuineIntel/GenuineIntel00106A4_Bloomfield_CPUID.txt) and [Atom C2750 CPUID dump](https://instlatx64.github.io/InstLatx64/GenuineIntel/GenuineIntel00406D8_Avoton_CPUID2.txt), leaves 1 and 6 | Independent observations of the DTS-only capability combination. Neither dump establishes thermal readback or application behavior. |
| S8 | Intel, [Atom C2000 Product Family Specification Update, 329460-020US, June 2018](https://www.intel.com/content/dam/www/public/us/en/documents/specification-updates/atom-c2000-family-spec-update.pdf), Table 9 (p. 16) and erratum AVR32 (p. 26) | Primary retail CPUID corroboration and a low-temperature reporting limit on C2308 and C2508 |

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
- The initial scope is limited to the exact non-hybrid signatures below.
  The required Windows topology checks are defined under Project
  collection constraints.

| Scope | Status | Default enablement |
| --- | --- | --- |
| Nehalem `06_1A`, `06_1E`, `06_1F`, `06_2E` | Verified by S1 §2.8 and Table 2-15 for signatures and target. S2 pins the core DTS and separate PTM bit. | Enabled subject to the capability, affinity and validity gates. |
| Sandy Bridge `06_2A`, `06_2D` | Verified by S1 §2.11 and Table 2-20 for signatures and target. S2 pins the core DTS and separate PTM bit. | Enabled subject to the capability, affinity and validity gates. |
| Silvermont Atom C2000 `06_4D` | Verified by S1 §2.4/§2.4.1 and Tables 2-6/2-10 for the signature, core DTS and package target. S2 pins the capability bits. | Enabled only with a zero target offset and the common gates. |
| Pre-Nehalem Core 2 and earlier Atom | Temperature reference unresolved for a model-scoped absolute reading. | Disabled. Follow-up in #2514. |
| Other signatures, hybrid processor types, or multiple processor groups/packages | Outside the initial evidence or attribution scope. Existing package collection remains covered by the [package DTS specification](cpu-intel-dts-msr.md). | Disabled for core reads. Follow-up in #2514. |

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| MSR `0x19C` | `IA32_THERM_STATUS` | 31 | Core digital readout valid when set | Flag | S1 Table 2-2, S2 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 22:16 | Core digital readout below the DTS reference | Unsigned degrees Celsius | S1 Table 2-2, S2 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Temperature target | Degrees Celsius | S1 Tables 2-6, 2-15, 2-20 |
| MSR `0x1A2`, model `06_4D` | `MSR_TEMPERATURE_TARGET` | 29:24 | Programmable target offset | Degrees Celsius | S1 Table 2-6 |

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
   target processor before reading (S1, S2, S5).
3. Read `0x1A2` and `0x19C` through the same calling processor using the
   signed `IntelMSR` module. Check the running group and processor
   again after the reads while still pinned, and discard the core
   result if either differs. Accept a successful target in 50–120 °C
   inclusive and a core read with bit 31 set. For `06_4D`, reject the
   sample if bits 29:24 of `0x1A2` are nonzero (S1–S3, S5, project
   gate).
4. Subtract the unsigned digital readout from the target. Apply the
   existing plausibility gate `0 < temperature ≤ target`. A valid
   digital readout of zero returns the target temperature. A faulted
   read, zero target, clear validity bit or failed plausibility gate
   leaves the core unavailable (S2, project gate).
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

- Non-blocking for #2514 initial scope: hybrid and multiple-package or multiple-group systems are disabled.
  Hybrid systems need per-type scope and reference evidence.
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
