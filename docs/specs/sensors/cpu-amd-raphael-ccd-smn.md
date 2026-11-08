# Spec: AMD Ryzen Raphael CCD temperatures via SMN

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental CCD temperatures on Windows x86-64 for AMD family `0x19`, model `0x61`, stepping 2 (`0x00A60F12`) with the AM5 package field and one OS-enumerated physical package. The path uses the existing signed PawnIO `RyzenSMU` read interface. |
| Issue phase | [#2515](https://github.com/shm11C3/HardwareVisualizer/issues/2515) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD, [Ryzen 9 7950X General Specifications](https://www.amd.com/en/products/processors/desktops/ryzen/7000-series/amd-ryzen-9-7950x.html) and [Ryzen Processor Software Optimization](https://gpuopen.com/download/GDC2024_AMD_Ryzen_Processor_Software_Optimization.pdf#page=12), GDC 2024, slide 12 | Primary Raphael identity, AM5 package, and two CCDs plus one I/O die on the 7950X. |
| S2 | AMD, [Processor Programming Reference for AMD Family 19h Model 61h, Revision B1 Processors](https://www.amd.com/content/dam/amd/en/documents/processor-tech-docs/programmer-references/56713-B1_3_05.zip), 56713-B1 Rev 3.05, March 8, 2023, §1.7.1 Table 12 p. 33, CPUID register definitions pp. 68, 86–87, Definitions p. 486 | Primary platform reference for the CPUID fields and the AM5/FL1 package values in the platform-specific sections. The enumeration on p. 87 differs, as noted in Open questions. PDF SHA-256 `f79d1a937d86ec310e47599a25f1c1d21e5969fca6c69f0dfebfe2b9d68c05f1`. |
| S3 | InstLatx64, [Ryzen 9 7950X CPUID dump](https://raw.githubusercontent.com/InstLatx64/InstLatx64/17be9a9793c306cf96f8c286e34e705b780a0226/AuthenticAMD/AuthenticAMD0A60F12_K19_Raphael_04_CPUID.txt), Git blob `b88e7d4e58d2bfff5c44fb01d027bcbc602da76c`, lines 10–18, 47–48 and 76–79 | Independent AM5 observation of the `AuthenticAMD` vendor, `0x00A60F12` signature, stepping 2 and extended-leaf EBX equal to zero. Contains CPU identification only. |
| S4 | rebell21, [openSUSE Leap 15.5 report](https://www.opensuse-forum.de/thread/67043-mein-neuer-rechner-zeigt-die-cpu-temperatur-nicht-mehr-an/), posts 1, 4 and 8, January 17, 2024 | First-hand observation on one 7950X with decimal family/model/stepping 25/97/2 and kernel `5.14.21-150500.55.39-default`. Its Linux `k10temp-pci-00c3` output reports `Tccd1` at 88.2 °C and `Tccd2` at 91.0 °C. The report carries converted values only and is independent of S3. |
| S5 | AMD-authored [July 19, 2022 CCD-offset patch](https://lists.openwall.net/linux-kernel/2022/07/19/1998) and Linux [v6.18 AMD hardware monitor](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/k10temp.c), lines 68–74, 173–176, 244–249, 410–420 and 502–505 | **lead-only (copyleft)**. Model-range-specific CCD register offset, slot addresses and order, discovery validity bit and temperature encoding. The lead includes model `0x61`. This upstream pin is separate from the distribution kernel observed in S4. |
| S6 | [PawnIO interface](pawnio-interface.md) revision 7, `RyzenSMU` and mutex sections, and [RyzenSMU API at tag 0.2.11](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/RyzenSMU.p), lines 149–150, 614–626, 793–814 and 868–899 | Upstream interface for the signed module. The module selects Raphael for family/model `0x19`/`0x61` with package field 0 and Dragon Range with field 1. The allow-list, fixed host bridge, caller mutex and read-only IOCTL support the two thermal addresses. |
| S7 | Microsoft Learn, [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex) and [PROCESSOR_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_relationship) | Primary Windows physical processor package enumeration contract. |
| S8 | AMD, [Ryzen 9 7945HX General Specifications](https://www.amd.com/en/products/processors/laptop/ryzen/7000-series/amd-ryzen-9-7945hx.html) | Primary Dragon Range and FL1 identity for an excluded product. |
| S9 | [CPU-Z Validator record ygf2kxy7](https://valid.cpuid.com/ygf2kxy7), Processor and Motherboard sections | Independent 7950X identity observation. Reports RPL-B2, CPUID F.1.2 / 19.61 and AM5. |

## Detection

Check the admission gates before the first CCD SMN request. The
processor name is an evidence identity under
[ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md).

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, CPUID leaf 0 vendor `AuthenticAMD`, CPUID.01H:EAX equal to `0x00A60F12` (family `0x19`, model `0x61`, stepping 2) | mechanism (Raphael CCD map scoped to the observed signature) | S2, S3, S4, S5, S6 |
| CPUID extended leaf `0x80000001` is available and EBX bits 31:28 equal `0x0` | mechanism (AM5 package selection) | S2, S3, S6 |
| `GetLogicalProcessorInformationEx` with `RelationProcessorPackage` succeeds and yields exactly one valid physical package record | mechanism (the fixed host bridge interface has no package selector) | S6, S7 |
| The existing signed `RyzenSMU` module loads and the caller holds `Global\Access_PCI` for each request | safety (authorized read interface and serialized index/data transaction) | S6 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| `0x00A60F12`, package field 0, one physical package, signed `RyzenSMU` read interface, slots `0x00059B08` and `0x00059B0C` | Ryzen 9 7950X is the evidence identity (S1, S3, S4 and S9). S5 supplies lead-only CCD addresses, slot order, bit 11 and scaling. S2 and S3 support the AM5 field. S6 pins the signed transport. | Experimental under [ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md). |
| Dragon Range FL1, other package fields or signatures, or unknown or multiple physical packages | Outside this profile. FL1 is a separate package (S2, S6, S8). | Unsupported by this scope. |

## Register map (facts)

| Address | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- |
| SMN `0x00059B08` | 31:0 | CCD temperature slot 1 | 32-bit word | S5 |
| SMN `0x00059B0C` | 31:0 | CCD temperature slot 2 | 32-bit word | S5 |
| Each scoped word | 11 | CCD reading validity | Set means valid at channel discovery | S5 |
| Each scoped word | 10:0 | CCD temperature field | Unsigned, 0.125 °C per increment, reference offset −49 °C | S5 |

## Read procedure and decode

1. After the admission checks, sample only on demand through the
   existing Windows sensor acquisition path. Use the signed
   `ioctl_read_smu_register` interface with its caller-held mutex (S6).
   Make at most one attempt per authorized slot per sample and skip
   any request whose mutex cannot be acquired.
2. Reject a failed read, zero word, all-ones word or clear bit 11 for
   that slot. S5 checks validity at discovery. Requiring validity on
   every sample is the project's defensive reporting gate.
3. Decode bits 10:0 × 0.125 °C − 49 °C (S5). Accept only −40 °C
   through +120 °C, inclusive, using the plausibility policy of the
   [AMD package](cpu-amd-zen-smn.md) and [Vermeer CCD](cpu-amd-vermeer-ccd-smn.md)
   paths. The CCD formula is independent of package Tctl/Tdie decoding.
4. Identify the result by the fixed slot within the sole physical
   package. Keep either valid reading if the other is unavailable.
   Leave a missing slot in its own position and preserve independent
   package temperature results and fallbacks.

## Quirks

- The two slots describe register positions. They do not identify OS
  cores or establish the number of physical CCDs on every processor
  sharing the signature (S1, S4, S5).
- The Raphael addresses differ from the separately scoped [Vermeer
  addresses](cpu-amd-vermeer-ccd-smn.md). Select a map before any read
  and restrict this profile to its two slots (S5).

## Safety notes

- Both addresses are in the signed module's `0x56000`–`0x5AFFF` SMN
  read window (S6). The module performs its own index selection as
  part of the serialized read. The application issues only the two
  approved read requests and makes no direct PCI/SMN access, SMN data
  write, mailbox command or PM-table request for this scope.
- An unavailable identity, package topology, signed module or mutex
  prevents the corresponding CCD request. Do not scan other addresses,
  switch to another map after failure, or retry within the sample.

## Synthetic fixture acceptance

| Case | Required result |
| --- | --- |
| Admitted signature, AM5 package field, one physical package and words `0x0AC8` and `0x0B38` | Read only the two Raphael addresses, once each. Slot 1 reports 40 °C and slot 2 reports 54 °C. The result is the same with the observed name, a different name, or no name. |
| The same signature with package field 1 (FL1), another nonzero field, or a missing extended CPUID leaf | Make no Raphael CCD requests. |
| Other stepping of model `0x61`, adjacent model, another vendor, malformed topology or more than one physical package | Make no Raphael CCD requests. Separately authorized Vermeer and package temperature paths keep their own gates. |
| Slot 1 fails, returns zero or all-ones, or has a clear validity bit despite a plausible temperature field. Slot 2 succeeds | Slot 2 retains its own position and value. Preserve a valid package reading. Exercise the inverse cases and a later sample whose validity bit clears after an earlier successful sample. |
| Words `0x0988`, `0x0848`, `0x0D48`, `0x0847` and `0x0D49` | Retain 0 °C, −40 °C and +120 °C. Omit readings one increment below or above the limits. |
| No CCD request, module load failure or mutex failure before one or both calls | Omit affected reads. Keep independently successful readings and make no direct fallback request. |

## Open questions

- Non-blocking for the Experimental scope: the platform-specific AM5 definition is corroborated by the exact observed signature and package field.
  In S2, Table 12 and Definitions agree on AM5 value 0 and FL1 value 1.
  The generic CPUID field table on p. 87 instead reserves 0–3 and assigns
  4 to SP5. S3 independently shows field 0 on AM5, and S6 selects the
  corresponding module path. This scope makes no claim about other field
  values beyond their exclusion.
- Non-blocking for the Experimental scope: ADR 0023 permits the pinned CCD mapping and decode leads for the narrowly admitted configuration.
  The PPR title identifies silicon revision B1. The admitted signature
  has CPUID stepping 2 (S3), also reported as RPL-B2 (S9). S2 supports
  the CPUID and platform field definitions only. The
  CCD register map and encoding for the admitted scope rest on S5, with
  converted channel observations in S4. Verified status requires primary
  register documentation or a maintainer-accepted labeled SMN dump.
- Non-blocking for the Experimental scope: the Windows read interface, address allowance and arbitration are pinned independently of Linux output.
  The Linux report does not establish Windows hardware behavior. A Windows
  [Hardware Compatibility Report](https://github.com/shm11C3/HardwareVisualizer/issues/2500)
  would corroborate the path, while graduation still needs the register
  evidence identified above.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-09 | Propose Implementation-ready as Experimental for the scoped AM5 signature, two fixed Raphael CCD slots, and signed Windows SMN read path. |
