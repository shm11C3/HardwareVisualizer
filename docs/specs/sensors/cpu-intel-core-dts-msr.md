# Spec: Intel core temperature via digital thermal sensor MSRs

| Field | Value |
| --- | --- |
| Revision | 6 |
| Status | Implementation-ready (rev 6) |
| Scope | Read-only core DTS for the listed Intel x86-64 display models and exact hybrid signatures in one Windows processor group and physical package. Includes DTS without package thermal management. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

Revision 6 proposes Experimental coverage for additional documented
legacy MSR mechanisms. The maintainer approves the revision under the
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
| S13 | Intel, [12th Generation Intel Core Processor Specification Update, 682436-041, October 2026](https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/alder-lake-desktop/682436/041/identification-information/), Table 2-1 and [Summary Tables of Changes, H/P column](https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/alder-lake-desktop/682436/041/summary-tables-of-changes/), and [Affected Processors by CPU Model, February 2025](https://www.intel.com/content/dam/develop/public/us/en/include/software-security/affected-processors-2025tab-february2025.html), `906A3` entry | Primary exact Alder Lake H/P signature, errata applicability, and hybrid identification. S10's datasheet covers the H and P lines, including junction specifications in Table 18 (p. 92) and the DTS reference in §§4.1.3.1–4.1.3.2. |
| S14 | Intel, [12th Generation Intel Core for IoT Edge Specification Update, 743110-1.1, June 2023](https://cdrdv2-public.intel.com/743110/743110_Alder%20Lake%20-PS_Public_Specification%20Update_rev1.1.pdf), Tables 3–4 (pp. 8–9), §3.1 (p. 11), §4.1 Table 5 (pp. 12–15) and §5 (p. 16), and [IoT Edge Datasheet Vol. 1, 743329-001, September 2022](https://cdrdv2-public.intel.com/743329/743329_12thGen_Intel_Core_PS_Processor_Datasheet_Vol1_Rev001.pdf), §§4.1.3.1–4.1.3.2 (pp. 68–71) | Primary exact Alder Lake-PS L-0 signature, separate errata applicability and explicit PS thermal register semantics. |
| S15 | Intel, [SDM Volume 4, 335592-093US, September 2026](https://cdrdv2.intel.com/v1/dl/getContent/671098), SHA-256 `79f54b67b9bb3836856f04e0c45f50406c4d4b932c64800eb788b6b705a83dbd` | Primary exact model mappings and readable thermal MSR definitions. See the scoped rows for the relevant sections and tables. |
| S16 | Intel, [Ivy Bridge desktop thermal guide, 326767-005, January 2013](https://www.intel.com/content/dam/www/public/us/en/documents/design-guides/3rd-gen-core-lga1155-socket-guide.pdf), §§6.2.2–6.2.3 (pp. 53–56), [mobile datasheet Vol. 1, 326768-006, June 2013](https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/3rd-gen-core-family-mobile-vol-1-datasheet.pdf), Digital Thermal Sensor (p. 80), and [Xeon E5 v2 datasheet Vol. 1](https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/xeon-e5-v2-datasheet-vol-1.pdf), §5.1.2 (p. 105) | Primary. Client DTS uses the calibrated reference independently of the activation offset. The E5 v2 text describes the DTS relative to the TCC and identifies the target MSR. |
| S17 | Intel, Haswell [desktop 328897-010](https://cdrdv2-public.intel.com/328897/4th-gen-core-family-desktop-vol-1-datasheet.pdf), §§5.6–5.9, [mobile M/H 328901-007](https://cdrdv2-public.intel.com/328901/4th-gen-core-family-mobile-m-h-processor-lines-vol-1-datasheet.pdf), §§5.6.1–5.6.2, and [mobile U/Y 329001-007](https://cdrdv2-public.intel.com/329001/4th-gen-core-family-mobile-u-y-processor-lines-vol-1-datasheet.pdf), §§5.6.1–5.6.2 | Primary product thermal reference. The DTS readout is independent of the TCC activation offset. |
| S18 | Intel, Broadwell [desktop 332376-001v1](https://www.mouser.com/datasheet/2/612/desktop-5th-gen-core-family-datasheet-vol-1-765515.pdf) (Intel document mirror), §5.9 (p. 76), [mobile 332378-001v1](https://www.intel.com.br/content/dam/www/public/us/en/documents/datasheets/mobile-5th-gen-core-family-datasheet-vol-1.pdf), §5.6.2 (p. 76), and [Core M 330834-001v1](https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/core-m-processor-family-datasheet-vol-1.pdf), §§5.6.1–5.6.2 | Primary product thermal reference. The DTS readout is independent of the TCC activation offset. |
| S19 | InstLatx64, commit `17be9a9793c306cf96f8c286e34e705b780a0226`: [Clarkdale](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel0020652_Clarkdale_CPUID.txt#L180-L183), [Gulftown](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00206C2_Gulftown_CPUID.txt#L468-L471), [Ivy Bridge](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00306A9_IvyBridge_CPUID.txt#L350-L353), and [Ivy Bridge-E](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00306E4_IvyBridgeE_CPUID.txt#L515-L518) | Independent register observations. They corroborate the model mechanisms and do not establish Windows PawnIO operation or the accuracy of individual readings. |
| S20 | Intel, [6th Generation Intel Processor Family Specification Update, 332689-024](https://www.scribd.com/document/557109734/332689-024-6th-generation-intel-processor-family-spec-update) (Intel document mirror), erratum SKL135 | Primary erratum describing DDR4 systems with wider DTS error than specified. |
| S21 | PawnIO 2.2.0 [driver read native](https://github.com/namazso/PawnIO/blob/5cdf470831fdfff3f7f1d06363ca6b230f3bf35a/PawnIO/src/natives_impl_windows.cpp#L437-L448) with [PawnIOLib status conversion](https://github.com/namazso/PawnIO/blob/5cdf470831fdfff3f7f1d06363ca6b230f3bf35a/PawnIOLib/PawnIOLib.cpp#L109-L115), and the [IntelMSR 0.2.11 read contract](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/IntelMSR.p#L102-L118) from S3 | Upstream interface behavior only. The documented signed read path returns a failing status when the native MSR read faults. |

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
  checks are defined under Project collection constraints (S9–S14).
- Alder Lake H/P and Alder Lake-PS L-0 share full signature
  `0x000906A3` (S13, S14). Both have separate product evidence for the
  temperature reference (S10, S14), so the profile uses their shared
  signature without a product-name test. The signature selects the
  documented profile. A sample still requires the hybrid flag, per-target
  core type and DTS capability, valid MSR results and the topology gates.
  S14 lists PS L-0 products with both P- and E-cores. S13 lists the
  Alder Lake U15W and U9W lines at `0x000906A4`, and S14 gives that
  signature to PS R-0. The exact `0x000906A3` row excludes both groups
  even when they advertise hybrid and DTS capability.

The runtime gates follow [ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md).

| Condition | Gate kind | Source |
| --- | --- | --- |
| Intel x86-64, available CPUID leaves, and a listed display family/model or full hybrid signature | mechanism | S1, S9–S15. Selects the applicable core status and target MSRs. |
| Core DTS bit set on discovery and on the pinned processor | mechanism | S2. Package thermal management independently gates the package path. |
| For a listed hybrid signature, hybrid flag set and the pinned processor reports a supported core type | mechanism | S9–S14. Selects the applicable hybrid status and reference mechanism. |
| Validated single active Windows group/package, with non-overlapping physical-core masks covering all active processors | mechanism | S4, S5 and project attribution policy. |
| Pinned processor and expected CPUID signature match before reads, processor still matches after reads, and topology still matches before publication | safety | S4, S5 and project attribution policy. Rejects samples with unconfirmed attribution. |
| Existing signed IntelMSR transport is available, and the dedicated sampling thread successfully restores affinity | safety | S3, S5, S6. Restricts privileged access and prevents further work on a thread with uncertain affinity. |
| Reading Valid is set and the row's offset condition passes | mechanism | S1, S2, S9–S11, S14–S18. Qualifies the applicable DTS reference and status. |
| Both reads succeed and the reference and result pass the existing plausibility bounds | safety | S3, S21 and project sample policy in the read procedure. |

The legacy rows use display family/model to select the documented
mechanism and confirm the full discovered signature on each pinned
representative for attribution. Observed product names are evidence
identities. For a family/model with no applicable row in this scope,
skip both core MSRs before topology discovery even if CPUID advertises
core DTS. Separately approved core DTS scopes retain their own gates.

| Scope | Status | Default enablement |
| --- | --- | --- |
| Nehalem `06_1A`, `06_1E`, `06_1F`, `06_2E` | Verified by S1 §2.8 and Table 2-15 for signatures and target. S2 pins the core DTS and separate PTM bit. | Enabled with direct target decode and the capability, affinity and validity gates. |
| Sandy Bridge `06_2A`, `06_2D` | Verified by S1 §2.11 and Table 2-20 for signatures and target. S2 pins the core DTS and separate PTM bit. | Enabled with direct target decode and the capability, affinity and validity gates. |
| Silvermont Atom C2000 `06_4D` | Verified by S1 §2.4/§2.4.1 and Tables 2-6/2-10 for the signature, core DTS and package target. S2 pins the capability bits. | Enabled only with a zero target offset and the common gates. |
| Hybrid Alder Lake S/HX `06_97`, full signature `0x00090672` (stepping ID 2) | S9/S10 pin the signature, P- and E-core status and reference. S12 corroborates hybrid identification. | Experimental, subject to the common gates and per-target core type below. |
| Hybrid Alder Lake H/P and PS `06_9A`, full signature `0x000906A3` (stepping ID 3) | S13/S14 pin the H/P and PS identities and applicable errata. S9 documents core thermal status for `06_9A`, S10/S14 pin the reference for the respective products, and S12/S13 corroborate hybrid identification. | Experimental, subject to the common gates and per-target core type below. |
| Hybrid Raptor Lake S/HX and S/HX Refresh `06_B7`, full signature `0x000B0671` (B-0) | S9/S11 pin the signature, P- and E-core status and reference. S11/S12 identify the non-hybrid signature overlap. | Experimental, subject to the common gates and per-target core type below. |
| Westmere `06_25`, `06_2C`, `06_2F` | S15 §§2.9–2.10 (pp. 2-221–222) apply Table 2-15 (pp. 2-182–183). Core status, thread target, and no offset field in the target layout. S19 observes both MSRs on the Core i5-650 `0x00020652` and Core i7-990X `0x000206C2`. | Experimental. Direct target decode. |
| Ivy Bridge `06_3A` | S15 §2.12 (p. 2-264) applies Table 2-20 (pp. 2-230, 2-232). S16 documents the offset-independent reference for desktop and mobile. S19 observes both MSRs on the Core i7-3770K `0x000306A9`. | Experimental. Direct target decode. |
| Ivy Bridge-E `06_3E` | S15 §§2.12.1–2.12.2 (pp. 2-267, 2-279) apply Tables 2-20 and 2-26 (p. 2-269 for target). S16 corroborates the reference for Xeon E5 v2. S19 observes both MSRs with zero target offset on the Core i7-4930K `0x000306E4`. | Experimental. Zero offset in bits 27:24. |
| Haswell client `06_3C`, `06_45`, `06_46` | S15 §2.13 (p. 2-287) applies Table 2-20. S17 documents the offset-independent reference for desktop and mobile. | Experimental. Direct target decode. |
| Broadwell client `06_3D`, `06_47` | S15 §2.15 (p. 2-333) applies Table 2-20. S18 documents the offset-independent reference for desktop, mobile and Core M. | Experimental. Direct target decode. |
| Broadwell Xeon `06_4F`, `06_56` | S15 §§2.16–2.16.2 (pp. 2-337, 2-348, 2-351) document the E5/E7 v4 and Xeon D layout in Table 2-36 (pp. 2-340–341). Both the core status and package target are explicit. | Experimental. Zero offset in bits 27:24. |
| Silvermont `06_37`, `06_4A`, `06_5A`, `06_5D` and Airmont `06_4C` | S15 §§2.4 and 2.4.2 (pp. 2-123, 2-142) apply Table 2-6 (p. 2-127) with core status and package target. | Experimental. Zero offset in bits 29:24, as for the existing `06_4D` mechanism. |
| Goldmont `06_5C`, Goldmont Plus `06_7A`, and Tremont `06_86`, `06_96`, `06_9C` | S15 §§2.5–2.7 (pp. 2-144, 2-170, 2-175) retain the Table 2-6 thermal rows. Table 2-12 also identifies bits 27:24 as programmable when its platform-info bit is set. The full Table 2-6 field supplies the conservative zero gate. | Experimental. Zero in bits 29:24. |
| Pre-Nehalem Core 2 and earlier Atom | S15 §2.2 Table 2-3 and §2.3 Table 2-4 list core thermal status without establishing a readable calibrated `0x1A2` target. | Disabled. Follow-up in #2514. |
| Other signatures, unrecognized hybrid core types, or multiple processor groups/packages | Outside the evidence or attribution scope. Existing package collection remains covered by the [package DTS specification](cpu-intel-dts-msr.md). | Disabled for core reads. Follow-up in #2514. |

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| MSR `0x19C` | `IA32_THERM_STATUS` | 31 | Core digital readout valid when set | Flag | S1 and S15 Table 2-2, S2, S9 Table 2-39 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 22:16 | Core digital readout below the DTS reference | Unsigned degrees Celsius | S1 and S15 Table 2-2, S2, S9 Table 2-39 |
| MSR `0x1A2`, listed legacy models | `MSR_TEMPERATURE_TARGET` | 23:16 | Reference temperature for the row's applicable offset condition | Degrees Celsius | S15 Tables 2-6, 2-15, 2-20, 2-26 and 2-36. S16–S18 specify offset independence for the direct-decode client rows. |
| MSR `0x1A2`, listed Atom models | `MSR_TEMPERATURE_TARGET` | 29:24 | Target offset field in the inherited register definition | Degrees Celsius | S1 and S15 Table 2-6 |
| MSR `0x1A2`, models `06_3E`, `06_4F`, `06_56` | `MSR_TEMPERATURE_TARGET` | 27:24 | Programmable TCC activation offset | Degrees Celsius | S15 Tables 2-26 and 2-36 |
| MSR `0x1A2`, the listed hybrid signatures | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated Tjmax, the DTS reference | Degrees Celsius | S10/S11/S14 §§4.1.3.1–4.1.3.2 |
| MSR `0x1A2`, the listed hybrid signatures | `MSR_TEMPERATURE_TARGET` | 29:24 | Programmable TCC activation offset, which affects the thermal control threshold | Degrees Celsius | S10/S11/S14 §4.1.3.1 |

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
   result if either differs. Require successful reads, a target byte
   in 50–120 °C inclusive and core status bit 31 set. For a zero-offset
   row, reject the sample when the selected target mask is nonzero.
   Use mask `0x0F000000` for `06_3E`, `06_4F` and `06_56`, and
   `0x3F000000` for the listed Atom models, including `06_4D`.
   Direct-decode rows apply no offset mask (S1–S3, S5, S15–S18).
4. Subtract the unsigned digital readout from the target. Apply the
   existing plausibility gate `0 < temperature ≤ target`. A valid
   digital readout of zero returns the target temperature. The
   offset-independent legacy and hybrid references use the target byte
   at both zero and nonzero TCC offsets. For the hybrid rows this also
   holds when the TCC averaging window changes.
   A faulted read returns a failing transport status and leaves the
   core unavailable. A zero target, clear validity bit or failed
   plausibility gate likewise leaves it unavailable (S2, S10/S11/S14,
   S15–S18, S21, project gate).
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
| Unsupported signature, otherwise valid capabilities and topology, including Core 2, earlier Atom, or deferred Skylake | No core topology query or core MSR reads. Core DTS alone does not qualify a target read. |
| Each added legacy display model with a valid target of 100 °C and a valid 30 °C digital readout | 70 °C under its documented mechanism. Use an unobserved synthetic brand and repeat with PTM clear to verify that neither a product name nor PTM gates core collection. |
| Ivy Bridge, Haswell client and Broadwell client with a valid nonzero activation offset, 100 °C target and valid 30 °C digital readout | 70 °C. The activation offset does not change the reference. |
| Zero-offset legacy rows with bit 24 set, then only bit 28 set, with otherwise valid reads | Bit 24 rejects all zero-offset rows. Bit 28 rejects the Atom rows only. Ivy Bridge-E and Broadwell Xeon ignore bits outside their 27:24 mask. |
| Supported legacy discovery, pinned representative reports a different signature or either MSR read fails | For a signature mismatch, skip both MSRs on that target. For a transport failure, omit that core even if an output cell is zero or populated. Preserve valid core siblings and independent package/ACPI results. |
| Each exact hybrid signature with one SMT P-core and multiple E-cores in one group and package, all advertising core DTS | One temperature per Windows physical-core record. The P-core's second logical processor adds no channel. Each E-core keeps its own slot even when several E-cores share other hardware resources. |
| Hybrid full signature `0x000B0671` or `0x000906A3` with hybrid flag clear, or another full signature such as `0x00090675`, `0x000906A2`, `0x000906A4` or `0x000B06F2` | No new core topology query or core MSR reads, even if an unsupported stepping advertises DTS and the hybrid flag. |
| Exact hybrid discovery succeeds, but an individual pinned target has a different full signature, no core DTS, no leaf 1AH, or an unknown core type | Skip that target before MSR access. Preserve eligible core siblings and the independent package/fallback result. |
| Each hybrid signature with Tjmax 100 °C, TCC offset 10 °C, and valid digital readouts of 30 and 0 | 70 °C and 100 °C respectively. Changing the TCC offset or its averaging window does not change either decoded value. |
| `0x000906A3` with valid P- and E-core targets and different per-target Tjmax words, each sampled with zero and nonzero TCC offsets | Use each target's own reference with the same 7-bit delta and validity gate on both core types. An invalid E-core status leaves its slot unavailable and preserves a valid P-core and independent package data. |
| One package with SMT siblings, reordered enumeration and one failed target or core read | One representative per physical core. Reordering alone preserves identities. The isolated failure leaves its position unavailable and preserves valid siblings. |
| Topology changes during a fresh pass or between cache acquisition and reuse | No output from the mismatched core set. Invalidate the cached core set even if its age is acceptable. Verify that reused representative processor numbers do not inherit old values. |
| Restore fails after one successful core and one attempted core | Discard the entire core set and cache. No subsequent reads on the disposable thread. A later pass uses a fresh thread. Package and ACPI results remain independent. |
| No demand, concurrent requests, rate-limit hit or expired wait | No idle reads. Requests share at most one in-flight pass. Cache age stays tied to acquisition. No second pass starts while a timed-out pass is still in flight. |
| Synthetic target and status words at validity and temperature boundaries | Accept targets of 50 °C and 120 °C and reject 49 °C and 121 °C. Accept a valid zero digital readout at the target. Reject missing or zero targets, clear Reading Valid, and a decoded temperature of zero or less. |

## Quirks

- The same-target read follows the model-specific scope in S1 and S15.
  Table 2-15 labels `0x1A2` Thread for Nehalem and Westmere.
  Table 2-20 labels it Unique for Sandy Bridge and the legacy client
  rows that inherit that layout. Tables 2-6, 2-26 and 2-36 label it
  Package for the corresponding Atom, Ivy Bridge-E and Broadwell Xeon
  rows. S3 documents calling-processor execution for the signed module.
  Its package-scope wording applies where S1 or S15 documents a
  package-scope target.
- S9 specifies core scope for hybrid `IA32_THERM_STATUS` and lists
  P- and E-core MSR exceptions separately. Use the same pinned processor
  for both thermal reads, as on the existing path. On `06_9A`, the
  core status, digital readout and valid flag apply to both core types.
  The H/P and PS datasheets independently describe the DTS delta as
  relative to the factory-calibrated target with a separate TCC offset.
  The H/P and PS errata tables list no exception to the scoped DTS
  reads (S9, S10, S13, S14).
  Core numbering and labeling remain based on Windows core records (S4).
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
  corresponding capability and model gates. The transport fault path
  handles failure of an eligible read. Never use it to probe an
  undocumented target. Do not write an MSR, change thresholds, clear
  logs, or override firmware configuration (S1–S3, S15, S21).
- Affinity is per-thread. Keep the affinity interval synchronous and
  bounded to the target's reads. Do not repurpose a process-wide
  affinity mask as the physical core map (S3–S5).

## Open questions

- Non-blocking for #2514 single-group scope: only the listed mechanisms and one physical package and active group qualify.
  Other hybrid signatures need their own applicability evidence.
  Multi-package/group systems need a separately validated attribution
  and affinity-restore contract.
- Non-blocking for the legacy extension: each zero-offset row rejects a nonzero offset within its specified mask.
  Wider nonzero-offset coverage requires a model-specific DTS reference.
  The signed Windows path has no real-hardware validation for the new rows.
- Non-blocking for the legacy extension: Skylake `06_4E` and `06_5E` and Haswell-E `06_3F` are outside the enablement table.
  S20 records an asymmetric DTS error on DDR4. Establish the applicable
  Skylake steppings and readout consequences before extending coverage.
  For `06_3F`, reconcile the target and offset behavior for the product
  lines sharing that model before specifying a decode.
- Non-blocking for #2514 initial scope: topology attribution uses matching validated observations and confirmed read targets.
  The Windows topology interface cannot distinguish a topology change
  that occurs and reverses between observations.

- Non-blocking for the Alder Lake H/P and PS scope: Intel documents the exact signature and both product lines' DTS semantics.
  The supported Windows PawnIO path has not been verified on real
  hardware for this signature.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-07 | Initial fact-only draft for the listed single-group, single-package models and independent DTS/PTM detection. |
| 2 | 2026-11-07 | Documented service placement, bounded request-driven collection, topology and affinity failure handling, and synthetic acceptance fixtures. |
| 3 | 2026-11-08 | Proposed transition to Implementation-ready for the exact single-group, single-package scope. Pinned the model-specific target scopes and Windows group-mask definitions and recorded DTS-only CPUID observations and the scoped Atom erratum. |
| 4 | 2026-11-08 | Propose Implementation-ready scope for two exact Alder Lake and Raptor Lake hybrid signatures. Pin per-core qualification, Tjmax and its relationship to the TCC offset, and the existing single-group topology contract. |
| 5 | 2026-11-08 | Propose Implementation-ready Experimental coverage for exact `0x000906A3` Alder Lake H/P and PS. Pin separate product evidence and reuse the existing hybrid and single-group topology gates. |
| 6 | 2026-11-09 | Propose additional Experimental legacy DTS mechanisms with explicit target decoding and offset masks. Classify runtime gates under ADR 0031. |
