# Spec: AMD Granite Ridge CCD temperatures via SMN

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental CCD temperatures on Windows x86-64 for AMD family `0x1A`, model `0x44`, stepping 0 (`0x00B40F40`), with the AM5 package field and exactly one OS-enumerated physical package. The path uses the existing signed PawnIO `RyzenSMU` read interface. |
| Issue phase | [#2515](https://github.com/shm11C3/HardwareVisualizer/issues/2515) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD, [Processor Programming Reference for AMD Family 1Ah Model 44h, Revision B0 Processors](https://www.amd.com/content/dam/amd/en/documents/processor-tech-docs/programmer-references/57896.zip), 57896 Rev 3.00, August 28, 2024, §1.7.1 Table 12 p. 32, §1.8.1 pp. 32–33, CPUID definitions pp. 67 and 85–86, and Definitions p. 485 | Primary platform and signature fields. Table 12 and Definitions map AM5 to field 0 and FL1 to field 1. Section 1.8.1 describes up to two CCDs and one I/O die. The generic package enumeration on p. 86 differs, as recorded in Open questions. PDF SHA-256 `643cae09d0bdae788ab090c0c4185168482b424e79f9feeced3f14c6de1817e5`. |
| S2 | AMD [Ryzen 9 9950X General Specifications](https://www.amd.com/en/products/processors/desktops/ryzen/9000-series/amd-ryzen-9-9950x.html) and [CPU-Z Validator record cz9kh9wc](https://valid.cpuid.com/cz9kh9wc), Processor and Motherboard sections, October 14, 2026 | AMD lists Granite Ridge, AM5 and three dies. The independent validator identifies one 9950X package on an ASRock X870E Taichi Lite as GNR-B0, CPUID F.4.0 / 1A.44 and AM5. |
| S3 | InstLatx64, [Ryzen 9 9950X CPUID dump](https://raw.githubusercontent.com/InstLatx64/InstLatx64/17be9a9793c306cf96f8c286e34e705b780a0226/AuthenticAMD/AuthenticAMD0B40F40_K20_GraniteRidge_02_CPUID.txt), Git blob `0c68cc7f92c6ad733d281fd0078af84b179a486a`, lines 48–49 and 77–80, and [Ryzen 9 9950X3D CPUID dump](https://raw.githubusercontent.com/InstLatx64/InstLatx64/17be9a9793c306cf96f8c286e34e705b780a0226/AuthenticAMD/AuthenticAMD0B40F40_K20_GraniteRidge_06_CPUID.txt), Git blob `3a845776134829e2c0339d0f270212c3fde3a6d8`, lines 48–49 and 77–80 | Independent raw observations. Both captures show `AuthenticAMD`, exact CPUID signature `0x00B40F40` and extended-leaf EBX 0. The cited leaves identify the CPU and contain no CCD thermal reads. |
| S4 | Moongoat, [CachyOS report](https://discuss.cachyos.org/t/cpu-temperature-will-ramp-up-and-overheat-after-boot-or-wake-from-suspend/30114/1) and [sensor output](https://discuss.cachyos.org/t/cpu-temperature-will-ramp-up-and-overheat-after-boot-or-wake-from-suspend/30114/7), May 22, 2026 | First-hand 9950X report on an ASRock X870E Taichi Lite with kernel `7.0.9-1-cachyos`. Linux `k10temp-pci-00c3` reports `Tccd1` at 33.0 °C and `Tccd2` at 32.2 °C. The output contains converted readings and comes from a different report than S2 and S3. |
| S5 | David Hows, [March 28, 2025 Linux Zen 5 desktop hwmon submission](https://lists.openwall.net/linux-kernel/2025/03/28/11), [v6.16 hwmon pull](https://lists.openwall.net/linux-kernel/2025/06/02/1307), and [Linux v6.18 AMD hardware monitor](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/k10temp.c), lines 68–74, 172–176, 244–249, 403–420 and 513–519 | **lead-only (copyleft)**. The author reports testing on a 9950X3D. The family 1Ah model 40h–4Fh branch supplies the CCD address offset and stride. The CCD discovery and value interfaces supply the validity flag, encoded temperature field and conversion. S4 is a separate distribution-kernel observation. |
| S6 | [PawnIO interface](pawnio-interface.md) rev 7, `RyzenSMU` and mutex sections, and [RyzenSMU API at tag 0.2.11](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/RyzenSMU.p), lines 159–171, 266–282, 617–626, 793–814 and 873–908 | Upstream interface for the signed module. Family 1Ah model 44h uses the normal fixed host-bridge read interface. Both addresses are allowed by the signed read window. The module's recognition is wider than this specification's package and stepping gates. |
| S7 | Microsoft Learn, [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex) and [PROCESSOR_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_relationship) | Primary Windows physical processor package enumeration contract. |
| S8 | AMD [Ryzen 9 9955HX General Specifications](https://www.amd.com/en/products/processors/laptop/ryzen/9000-series/amd-ryzen-9-9955hx.html) and [CPU-Z Validator record 0xu4y0lh](https://valid.cpuid.com/0xu4y0lh), Processor and Motherboard sections, August 31, 2026 | Fire Range FL1 example. The observed 9955HX shares CPUID F.4.0 / 1A.44 with the AM5 capture in S2. |
| S9 | GIGABYTE, [MC13-BL0 BIOS R19 release notes](https://device.report/m/abd1fa294112aa3ef6409194cc7fe34a2441fef614576e54aedb27df0c6e1ae0.pdf), p. 1, October 16, 2025, and [Linux v6.18 AMD microcode documentation in source](https://github.com/torvalds/linux/blob/v6.18/arch/x86/kernel/cpu/microcode/amd.c), lines 93–114, 133–139, 211–216 and 278–292 | OEM document via public archive. It names Granite Ridge B0 patch `0x0B404032` and B1 patch `0x0B404102`, with the same CPUID column value. The Linux microcode source is **lead-only (copyleft)** and identifies different model 44h stepping fields in those patch IDs. This is scope-boundary evidence only. |
| S10 | AMD [EPYC 4565P General Specifications](https://www.amd.com/en/products/processors/server/epyc/4005-series/amd-epyc-4565p.html), ASRock Industrial [IMB-A1003 CPU Support List](https://download.asrock.com/IPC/List/CPU/IMB-A1003.pdf), and [XMRig hardware observation sFyWP](https://xmrig.com/benchmark/sFyWP) | AMD identifies AM5 and 1P. ASRock labels the part Grado. The observation on an ASUS ProArt X870E-CREATOR WIFI reports the same full CPUID signature. It provides neither the package field nor any temperature or transport result. |

## Detection

Apply the gates before any CCD SMN request. A product name or board
name records evidence under
[ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md).

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, CPUID leaf 0 vendor `AuthenticAMD` and CPUID.01H:EAX equal to `0x00B40F40` (family `0x1A`, model `0x44`, stepping 0) | Mechanism (the scoped model and revision of the CCD map) | S1–S6 |
| CPUID extended leaf `0x80000001` is available and EBX bits 31:28 equal `0x0` | Mechanism (AM5 package rather than the overlapping FL1 package) | S1–S3, S8 |
| `GetLogicalProcessorInformationEx` with `RelationProcessorPackage` succeeds and reports exactly one valid physical package record | Mechanism (attribution through a fixed host-bridge interface with no package selector) | S6, S7 |
| The existing signed `RyzenSMU` module loads and the caller holds `Global\Access_PCI` for each request | Safety (authorized, serialized read transaction) | S6 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact `0x00B40F40`, AM5 package field 0, one physical package and signed `RyzenSMU` access to the two scoped slots | 9950X is the best-evidenced identity (S1–S4). The 9950X3D has independent signature and package-field evidence (S3) and the upstream author's test (S5). S5 supplies the lead-only slot addresses and order, bit-11 validity and temperature encoding. S6 pins signed transport. Members that satisfy the same mechanism gates are Experimental. | Experimental under [ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md). |
| Other signatures, nonzero or unavailable package field, unknown or multiple physical packages | Outside this map. A matching Fire Range signature is insufficient because FL1 has a separate package field (S1, S8). | Unsupported by this scope. |

## Register map (facts)

| Address | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- |
| SMN `0x00059B08` | 31:0 | CCD temperature slot 1 | 32-bit word | S5 |
| SMN `0x00059B0C` | 31:0 | CCD temperature slot 2 | 32-bit word | S5 |
| Each scoped word | 11 | CCD reading validity | Set means valid at channel discovery | S5 |
| Each scoped word | 10:0 | CCD temperature field | Unsigned, 0.125 °C per increment, reference offset −49 °C | S5 |

## Read procedure and decode

1. Sample on demand in the existing Windows sensor acquisition path.
   Use the signed `ioctl_read_smu_register` interface (S6) with bounded
   acquisition of its caller-held mutex. Make at most one read attempt
   per slot per sample.
2. For each slot independently, reject a failed read, zero word,
   all-ones word or clear bit 11. S5 uses bit 11 at channel discovery.
   Checking it on every sample is the project's defensive reporting gate.
3. Decode bits 10:0 × 0.125 °C − 49 °C (S5). Retain values from −40 °C
   through +120 °C inclusive, following the plausibility policy of the
   [Raphael CCD path](cpu-amd-raphael-ccd-smn.md). Package Tctl/Tdie
   decoding and offsets are separate.
4. Attribute each result to its fixed CCD slot within the sole package.
   Reuse the existing CCD temperature representation and slot identities.
   On an admitted target, preserve both slot identities and mark a
   failed or invalid slot's temperature unavailable. Keep any valid
   slot, package temperature result and fallback independently.

## Quirks

- The first two register positions are the only exposed slots. AMD
  describes a maximum of two physical CCDs for this processor design
  (S1). The valid bit determines whether either slot can report a
  temperature for the current sample (S5).
- The numeric addresses match the separate [Raphael
  scope](cpu-amd-raphael-ccd-smn.md). The Granite Ridge map is derived
  independently from the Zen 5 desktop branch in S5. Select the map by
  the admitted mechanism before reading.

## Safety notes

- Both addresses are in the signed module's `0x56000`–`0x5AFFF` read
  window (S6). The signed interface performs its own index selection
  as part of the serialized read. Use only that interface for the two
  authorized reads. Direct PCI/SMN access, SMN data writes, mailbox
  requests and PM table requests are outside this scope.
- An unavailable admission fact prevents CCD reads. A failed mutex
  acquisition prevents its corresponding call. Do not scan other
  addresses, select another map after a failure, or retry within a sample.

## Synthetic fixture acceptance

| Case | Required result |
| --- | --- |
| Exact admitted identity and one package, with words `0x0AC8` and `0x0B38` | The only CCD requests are to `0x00059B08` and `0x00059B0C`, once each. Report slot 1 at 40 °C and slot 2 at 54 °C. A changed, absent or server-branded CPU name does not change admission for the same mechanism. |
| Same signature with package field 1, another nonzero field or unavailable extended CPUID leaf | Make no CCD request and return no target under this profile. |
| CPUID `0x00B40F41`, another family 1Ah model, another vendor, or unknown, malformed or multiple package records | Make no CCD request and return no target under this profile. Existing approved CCD and package paths keep their own gates. |
| One slot fails, returns zero or all ones, or clears bit 11 while the other slot reports a valid value | Keep both slot identities, mark only the affected temperature unavailable and preserve the other value. Exercise both positions and the transition from valid to invalid on a later sample. A one-CCD result needs no name or core-count rule. |
| Words `0x0988`, `0x0848`, `0x0D48`, `0x0847` and `0x0D49` | Retain 0 °C, −40 °C and +120 °C. Mark values one increment outside either limit unavailable in their slots. |
| Sampling is inactive, module unavailable, or mutex acquisition fails for one or both calls | Make no affected request. If a slot is skipped on an admitted target, keep its identity with unavailable temperature. Preserve independent successful results and make no direct fallback request. |

## Open questions

- Non-blocking for the Experimental scope: the exact platform table, glossary and observed AM5 CPUID agree.
  S1 pp. 32 and 485 give AM5 field 0 and FL1 field 1, while the
  generic register enumeration on p. 86 reserves values 0–3 and
  names SP5 at 4. S2 corroborates the AM5 product identity. The raw
  EBX values in S3 corroborate field 0, the only admitted value.
- Non-blocking for the Experimental scope: revision 1 admits only the evidenced B0 signature.
  The S9 OEM notes repeat `0x00B40F40` for two microcode entries,
  but the named B1 patch `0x0B404102` encodes family 1Ah, model 44h,
  stepping 1 (`0x00B40F41`) under the Zen patch-ID contract pinned in S9.
  The [MAXSUN BIOS guide](https://manual.holod.ru/manual/1559/1268209_7248225.pdf),
  §3 Advanced, shows that signature and patch on a Ryzen 5 9500F.
  [AMD's 9500F specifications](https://www.amd.com/en/products/processors/desktops/ryzen/9000-series/amd-ryzen-5-9500F.html)
  identify AM5, and an [independent hardware capture](https://browser.geekbench.com/v6/cpu/18655362)
  reports one AM5 package with model 44h, stepping 1. These sources
  establish identity. S5 selects the CCD map by model range. Raw B1
  package-field evidence and an exact B1 CCD observation tied to the
  SMN slots and validity flag remain unresolved. `0x00B40F41` is
  outside revision 1. The gate uses CPUID and does not read the
  active microcode revision.
- Non-blocking for the Experimental scope: the register facts are independently scoped from the signed transport contract.
  S5 supplies the mapping and decode under ADR 0023. S4 corroborates
  both converted channels on a 9950X. S10 establishes a same-signature
  AM5 server-branded example, with no separate field-level or thermal
  observation. No Windows device validation is asserted. Verified
  status requires primary register documentation or a
  maintainer-accepted labeled SMN dump and corroboration of the
  Windows read path.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-09 | Propose the transition to Implementation-ready (rev 1) for Experimental CCD temperatures on the exact AM5 stepping-0 signature via signed Windows SMN access. Maintainer sign-off is pending. |
