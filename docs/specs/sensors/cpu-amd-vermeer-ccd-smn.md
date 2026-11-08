# Spec: AMD Ryzen Vermeer CCD temperatures via SMN

| Field | Value |
| --- | --- |
| Revision | 5 |
| Status | Implementation-ready (rev 5) |
| Scope | Experimental read-only CCD temperatures on AMD family `0x19` model `0x21` (Vermeer) processors through PawnIO `RyzenSMU` on Windows x86-64, with one OS-enumerated physical AM4 package and per-slot validity discovery. The Ryzen 7 5800X, Ryzen 9 5900X and Ryzen 9 5950X B0 observations are evidence identities. Package Tctl/Tdie has a separate specification. |
| Issue phase | [#2515](https://github.com/shm11C3/HardwareVisualizer/issues/2515) |

Revision 5 replaces the three exact product-name gates and the
5800X-only first-slot restriction of revision 4 with the mechanism gates
required by
[ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md):
the Vermeer die as the processor reports it, the AM4 package field, the
single-package topology, and each slot's own validity bit. The observed
products stay in the Status column.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD, [Ryzen 9 5900X General Specifications](https://www.amd.com/en/products/processors/desktops/ryzen/5000-series/amd-ryzen-9-5900x.html) | Primary product name, Vermeer codename and AM4 package |
| S2 | InstLatx64, [Ryzen 9 5900X Vermeer B0 CPUID dump](https://raw.githubusercontent.com/InstLatx64/InstLatx64/master/AuthenticAMD/AuthenticAMD0A20F10_K19_Vermeer2_CPUID.txt), Git blob `b90d151a978633573d7a26fd85199ab0d987919e`, CPU Info and CPUID Registers | Independent hardware identity observation. Contains no CCD temperature register dump. |
| S3 | Linux [v6.18 AMD hardware monitor](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/k10temp.c), GPL-2.0-or-later | **lead-only (copyleft)**. Family 19h model 21h CCD map keyed by family and model, validity at channel discovery, unavailable responses and scaling. |
| S4 | LibreHardwareMonitor, [AMD Zen CPU monitor at commit `2808d80564b6`](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Cpu/Amd17Cpu.cs), MPL-2.0 | **lead-only (copyleft)**. Names 5900X as tested and uses the PawnIO SMN read interface for CCD temperatures at this pin. The test note omits the stepping, raw words and transport version. |
| S5 | [PawnIO interface](pawnio-interface.md) revision 7, `RyzenSMU` and mutex sections, and [module API at tag 0.2.11](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/RyzenSMU.p) | Signed transport and fixed host bridge access. Lines 134–144 select Vermeer by family and model, lines 614–626 authorize both scoped addresses and lines 793–814 define the read-only IOCTL and caller-held mutex. No package selection argument. |
| S6 | Microsoft Learn, [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex) and [PROCESSOR_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_relationship) | Primary Windows physical processor package enumeration contract |
| S7 | AMD, *Processor Programming Reference for AMD Family 19h Model 21h, Revision B0 Processors*, 56214-B0 Rev 3.05, April 22, 2021, [public-use document mirror](https://www.scribd.com/document/1050633015/PPR-for-AMD-Family-19h-Model-21h), §2.2.1 pp. 74, 76–79, §4.2.1 p. 328, Definitions p. 357, Memory Map – SMN p. 361 | Primary for the family/model/stepping CPUID encoding, the AM4 package field and single-socket platform, processor name leaves, and THM aperture. The public register list omits the individual CCD temperature words. |
| S8 | Tweakers, [Ryzen 3000 and 5000 workshop](https://tweakers.net/reviews/8628/amd-ryzen-overklokken-workshop-voor-ryzen-3000-en-5000.html), Ryzen Master advanced view | First-hand observation of two CCDs on a Ryzen 9 5900X. The observation does not identify the register slots. |
| S9 | AMD, [Ryzen 9 5950X General Specifications](https://www.amd.com/en/products/processors/desktops/ryzen/5000-series/amd-ryzen-9-5950x.html) | Primary product name, Vermeer codename and AM4 package. |
| S10 | AMD, [Ryzen Processor Software Optimization](https://gpuopen.com/download/GDC_AMD_Ryzen_Processor_Software_Optimization.pdf#page=22), GDC 2022, slide 22 | Primary topology for the exact Ryzen 9 5950X SKU. The diagram shows two CCDs and an I/O die. |
| S11 | InstLatx64, [Ryzen 9 5950X CPUID dump at commit a7c87bf859a7](https://github.com/InstLatx64/InstLatx64/blob/a7c87bf859a71295dedab9629a9fbccae37b8301/AuthenticAMD/AuthenticAMD0A20F10_K19_Vermeer_CPUID1.txt), Git blob `901184c36c81fe8cfc0fd3da9cbe56c7ee8598ee`, lines 2–3 and 24–28 | Independent hardware identity observation. Pins the vendor, CPUID signature, AM4 package field and full processor name. Contains no CCD register readings. |
| S12 | [lm-sensors issue #389, original report](https://github.com/lm-sensors/lm-sensors/issues/389), March 2022 | First-hand observation on one Ryzen 9 5950X with decimal family/model/stepping 25/33/0. The initial Manjaro 5.16.7 report shows `Tccd1` at 30.5 °C and `Tccd2` at 31.5 °C under `k10temp-pci-00c3`. This is output from the Linux sensor interface. |
| S13 | Linux stable [v5.16.7 AMD hardware monitor](https://github.com/gregkh/linux/blob/v5.16.7/drivers/hwmon/k10temp.c), Git blob `3618a924e78e4a47dfb882c0c824e9a3e9c37b46`, lines 68–74, 183–194, 229–233, 379–389 and 449–454, GPL-2.0-or-later | **lead-only (copyleft)**. Pins the upstream address-to-label mapping, bit 11 check at channel discovery and scaling used to interpret S12. Stable v5.16.14 has the same blob. |
| S14 | AMD, [Ryzen 7 5800X General Specifications](https://www.amd.com/en/products/processors/desktops/ryzen/5000-series/amd-ryzen-7-5800x.html) | Primary product name, Vermeer codename and AM4 package. |
| S15 | [CPU-Z Validator record bxn060](https://valid.x86.fr/bxn060), January 14, 2026, Processor section | Independent 5800X identity observation. Records the full processor name, one CPU, VMR-B0, CPUID F.1.0 / 19.21, and AM4. |
| S16 | Mitsuya Shibata, [Ubuntu Weekly Recipe 697](https://gihyo.jp/admin/serial/01/ubuntu-recipe/0697), December 22, 2021, inxi and Psensor sections | First-hand Ubuntu 21.10 observation of a Ryzen 7 5800X at revision 0 in the inxi report, with ASRock B550 Steel Legend UEFI P1.80 and Linux 5.13.0-22-generic. The same system's k10temp PCI output shows Tccd1 at 21.0 °C and package Tctl/Tdie at 20.2 °C. |
| S17 | Canonical [Impish kernel mapping](https://people.canonical.com/~kernel/info/kernel-version-map.html) and [AMD hardware monitor at tag Ubuntu-5.13.0-22.22](https://git.launchpad.net/~ubuntu-kernel/ubuntu/+source/linux/+git/impish/plain/drivers/hwmon/k10temp.c?h=Ubuntu-5.13.0-22.22), Git blob `5ff3669c2b6084d63c247fb654da7dfee0624e3b`, lines 71–73, 184, 229–230, 390–391 and 452, GPL-2.0-or-later | **lead-only (copyleft)** for the mapping from Tccd1 to SMN `0x00059954` on family 19h model 21h, the bit 11 discovery gate and scaling. The source file matches Linux stable [v5.13.19](https://github.com/gregkh/linux/blob/v5.13.19/drivers/hwmon/k10temp.c) byte for byte. |
| S18 | Igor Wallossek, [5800X and 5600X physical CCD variation report](https://www.igorslab.de/en/ryzen-5-5600x-und-ryzen-7-5700x-mit-2-ccds-aufgetaucht-laesst-sich-die-cpu-zum-ryzen-9-unlocken-2/), January 11, 2021 | Reports physical two-CCD variants with a CCD unavailable for normal operation. It does not establish the register state or exact stepping of those 5800X variants. |
| S19 | InstLatx64, [Ryzen 7 5800X Vermeer B2 CPUID dump](https://raw.githubusercontent.com/InstLatx64/InstLatx64/master/AuthenticAMD/AuthenticAMD0A20F12_K19_Vermeer_03_CPUID.txt), Git blob `cb93d403b772007a311bea60c2e1c5e8edd0bf4b`, lines 11–19, 48–49 and 73–76 | Independent 5800X B2 identity observation. The processor name matches B0, while the raw signature is `0x00A20F12`: same family and model, stepping 2. Contains no CCD register readings. |

## Detection

Every runtime condition below is a mechanism gate. The processor name
is not read for admission (ADR 0031).

| Condition | Gate kind | Source |
| --- | --- | --- |
| CPUID leaf 0 reports `AuthenticAMD` | mechanism | S7 |
| CPUID.01H:EAX decodes to family `0x19` and model `0x21` (extended family and model fields per S7 §2.2.1); any stepping | mechanism | S3 (lead-only, keyed by family and model), S5 (module selects Vermeer by family and model), S7 |
| CPUID.80000001H:EBX bits 31:28 equal `0x2` (AM4) | mechanism | S7 |
| A successful `GetLogicalProcessorInformationEx` query for `RelationProcessorPackage` yields exactly one valid physical package record | mechanism (the signed module uses a fixed host bridge with no package selection input) | S5, S6 |
| Per slot, the read word has bit 11 set and is neither all-zero nor all-ones | mechanism (the register's own validity bit) | S3, S13, S17 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Family `0x19` model `0x21`, AM4, single physical package, `RyzenSMU` 0.2.11 read interface, both slots `0x00059954` and `0x00059958` | Observed at stepping 0 on three products: Ryzen 9 5900X (S1, S2, S8; lead-only S4 names it tested), Ryzen 9 5950X (S9–S12 with both channels in S12), Ryzen 7 5800X (S14–S16 with the first channel in S16). S19 observes the B2 stepping identity on a 5800X with no readback. CCD addresses, bit 11, unavailable responses and scaling depend on lead-only S3; lead-only S13 and S17 tie the Linux channel observations to the two SMN words; lead-only S4 corroborates both addresses and the decode on words with bit 11 set. S5 pins the signed transport. S7 pins the identity encoding and package field. | Experimental with the mechanism gates, per-slot validity and plausibility gates. |
| Other family or model (for example Cezanne `0x19`/`0x50`, Raphael `0x19`/`0x61`, any family `0x17` or `0x1A`), non-AM4 package field, or more than one physical package | Separate address, count, validity and attribution evidence is required. | Unsupported. Follow-up in #2515. |

The observed product names below are evidence identities. They record
where the S2, S11, S12, S15, S16 and S19 observations came from and are
the first candidates for graduation toward Verified once a
maintainer-accepted hardware report confirms the HardwareVisualizer
path. They do not gate the read.

| Observed processor name | Signature | Observations |
| --- | --- | --- |
| `AMD Ryzen 9 5900X 12-Core Processor` | `0x00A20F10` | S1, S2, S8, lead-only S4 |
| `AMD Ryzen 9 5950X 16-Core Processor` | `0x00A20F10` | S9–S12 |
| `AMD Ryzen 7 5800X 8-Core Processor` | `0x00A20F10`, `0x00A20F12` (S19, identity only) | S14–S16, S19 |

## Register map (facts)

| Address | Meaning | Units / encoding | Source |
| --- | --- | --- | --- |
| SMN `0x00059954` | First CCD temperature slot | 32-bit read word | S3, S4 (5900X), S13 (5950X), S17 (5800X) |
| SMN `0x00059958` | Second CCD temperature slot | 32-bit read word | S3, S4 (5900X), S13 (5950X) |
| Each authorized slot, bit 11 | Value is valid when set | Flag | S3, S13, S17 |
| Each authorized slot, bits 10:0 | Raw CCD temperature field | Unsigned, 0.125 °C per increment with a −49 °C reference offset | S3, S13, S17, corroborated when bit 11 is set by S4 for 5900X |

## Read procedure and decode

1. Confirm the detection gates and successful signed module load.
   On demand for CCD temperatures, make at most one read of each of
   the two authorized words per sample through `ioctl_read_smu_register`
   with the PCI mutex (S5). Both words are read on every admitted
   processor; the slot's own validity bit, not the product name,
   decides whether a slot reports (S3). Each address lies inside the
   module's `0x56000`–`0x5AFFF` window. Do not scan other addresses,
   remap slots or retry failed reads within the sample.
2. A failed read, all-zero or all-ones word, or clear validity bit
   means that slot is unavailable. A clear validity bit cannot
   establish whether a CCD is physically absent, disabled, or
   temporarily unreadable. S3, S13 and S17 use bit 11 at channel
   discovery. Checking it on every sample is a defensive project gate.
3. For a valid word, temperature = bits 10:0 × 0.125 °C − 49 °C
   (S3, S4, S13, S17). Apply the existing package path's −40 to +120 °C
   plausibility interval to the CCD result as a defensive gate.
   Do not apply the model-specific package Tctl offset to a CCD.
4. Identify each value by its physical package in the supported
   single-package topology and by the fixed register slot. Keep
   slot 2 in its own position when slot 1 is missing. Either slot
   can be unavailable independently of the sibling and the
   existing package Tctl/Tdie reading.

## Quirks

- For values with bit 11 set, S4's low-12-bit decode reduces to S3's
  low-11-bit decode because bit 11 contributes 256 °C to the
  intermediate value. S3 is the basis for refusing words without
  bit 11 and all-ones words. A failed sensor is never zero-filled.
- The slot number identifies the reading, without a claimed mapping
  to OS core IDs or a persistent package identifier.
- The scope makes no assertion about physically installed CCDs,
  active CCD count or which cores are active. On a product with one
  CCD, the second slot is expected to fail the validity gate and
  stays an unavailable slot 2 in its own position; it is never
  remapped onto slot 1. Physical two-CCD variants of single-CCD
  products have been reported (S18); a valid second-slot word on
  such a part retains the slot 2 identity.

## Safety notes

- Hold `Global\Access_PCI` across each SMN request. Skip the affected
  slot on mutex timeout. The signed module performs the SMN index
  selection. No application-side PCI access, SMN data write,
  SMU command or PM-table operation is part of the scope (S5).
- Exclude unsupported packages, families and models before accessing
  any CCD register. Package Tctl/Tdie remains governed by the
  independent [package temperature specification](cpu-amd-zen-smn.md).

## Synthetic fixture acceptance

| Case | Required result |
| --- | --- |
| Family `0x19` model `0x21` stepping 0, AM4, one package, raw words `0x0AC8` and `0x0B38`, with the processor name reporting an observed product, a synthetic unobserved name (for example a 5600X-style or 5800X3D-style string), or no name at all | Read only the two fixed SMN words. Slot 1 reports 40 °C, slot 2 reports 54 °C. The name does not change admission. Preserve the package reading. |
| Same identity with stepping 2 (`0x00A20F12`) and the same words | Identical admission and results. Stepping does not change admission within model `0x21`. |
| Slot 1 has a failed read, clear bit 11, zero or all-ones; slot 2 valid | Slot 1 is unavailable, slot 2 retains its identity and value, and valid package temperature remains available. Exercise the inverse failure as well. |
| Slot 1 word `0x0AC8`, slot 2 word with bit 11 clear (single-CCD pattern) | Slot 1 reports 40 °C. Slot 2 is unavailable in its own position. No remapping and no retry. |
| Valid zero Celsius, both plausibility limits, and one raw increment beyond either limit | Keep `0x0988` (0 °C), `0x0848` (−40 °C) and `0x0D48` (120 °C). Reject `0x0847` and `0x0D49`. |
| Two packages, unreadable package topology, or a non-AM4 package field | Make no CCD access. Existing package temperature handling continues independently. |
| Family `0x19` with another model (for example `0x00A50F00`, `0x00A60F12`), family `0x17` or `0x1A`, or a non-AMD vendor, regardless of the processor name | Make no CCD access. Package temperature and its existing fallback continue independently. |
| Module unavailable, mutex unavailable or timed out, or CCD readout not requested | Make no unauthorized reads or retries. Preserve other successful readings. |

## Open questions

- Non-blocking for the Experimental scope: ADR 0023 permits the pinned CCD address and decode leads for the Vermeer die and the existing read-only transport.
  A labeled CCD register dump for an identified model `0x21` processor
  or primary documentation of the CCD registers is still needed for
  Verified enablement.
- Non-blocking for the Experimental scope: the lead-only map (S3) and the signed module (S5) both key on family and model, so stepping 2 (S19) shares the mechanism.
  No readback observation exists at stepping 2. A Hardware Compatibility
  Report ([#2500](https://github.com/shm11C3/HardwareVisualizer/issues/2500))
  from a B2 part is the graduation evidence.
- Non-blocking for the Experimental scope: the second slot on single-CCD products is governed by the slot's own validity bit (S3), not by the product name.
  The 5800X observation (S16) shows only the first channel. The
  second slot's behavior on an identified 5800X has not been observed;
  the validity gate keeps it unavailable without a valid word.
- Non-blocking for the Experimental scope: only single-socket AM4 configurations are admitted.
  Other families and models need independent address, count, validity
  and attribution evidence in #2515. Multiple packages would also
  require a package-selectable interface.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-07 | Initial fact-only draft for the Ryzen 9 5900X Vermeer B0 candidate. |
| 2 | 2026-11-08 | Pin the AM4 package and bounded sampling gates and add synthetic acceptance cases. Transition the exact single-package 5900X B0 scope to Implementation-ready as Experimental under ADR 0023. |
| 3 | 2026-11-08 | Add the exact single-package Ryzen 9 5950X B0 Experimental scope under ADR 0023. Record its own hardware identity, two-CCD topology and Linux channel observation. Reuse the two approved SMN addresses and signed Windows transport. |
| 4 | 2026-11-09 | Propose Implementation-ready for the exact single-package Ryzen 7 5800X B0 first-slot Experimental scope under ADR 0023. Pin the independent hardware observation and its distribution's address-to-channel mapping. Keep the second slot outside the 5800X scope. |
| 5 | 2026-11-09 | Key the scope by the Vermeer die (family `0x19` model `0x21`, any stepping), AM4 package field, single package and per-slot validity bit under ADR 0031. Remove the three product-name gates and the 5800X first-slot restriction; keep the observed products as evidence identities. Add the gate-kind table and mechanism fixtures. |
