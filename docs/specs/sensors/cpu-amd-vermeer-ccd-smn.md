# Spec: AMD Ryzen 9 Vermeer B0 CCD temperatures via SMN

| Field | Value |
| --- | --- |
| Revision | 3 |
| Status | Implementation-ready (rev 3) |
| Scope | Experimental read-only CCD temperatures on the exact Ryzen 9 5900X and 5950X Vermeer B0 models through PawnIO `RyzenSMU` on Windows x86-64, with one OS-enumerated physical AM4 package. Package Tctl/Tdie has a separate specification. |
| Issue phase | [#2515](https://github.com/shm11C3/HardwareVisualizer/issues/2515) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD, [Ryzen 9 5900X General Specifications](https://www.amd.com/en/products/processors/desktops/ryzen/5000-series/amd-ryzen-9-5900x.html) | Primary product name, Vermeer codename and AM4 package |
| S2 | InstLatx64, [Ryzen 9 5900X Vermeer B0 CPUID dump](https://raw.githubusercontent.com/InstLatx64/InstLatx64/master/AuthenticAMD/AuthenticAMD0A20F10_K19_Vermeer2_CPUID.txt), Git blob `b90d151a978633573d7a26fd85199ab0d987919e`, CPU Info and CPUID Registers | Independent hardware identity observation. Contains no CCD temperature register dump. |
| S3 | Linux [v6.18 AMD hardware monitor](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/k10temp.c), GPL-2.0-or-later | **lead-only (copyleft)**. Family 19h model 21h CCD map, validity at channel discovery, unavailable responses and scaling. |
| S4 | LibreHardwareMonitor, [AMD Zen CPU monitor at commit `2808d80564b6`](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Cpu/Amd17Cpu.cs), MPL-2.0 | **lead-only (copyleft)**. Names 5900X as tested and uses the PawnIO SMN read interface for CCD temperatures at this pin. The test note omits the stepping, raw words and transport version. |
| S5 | [PawnIO interface](pawnio-interface.md) revision 7, `RyzenSMU` and mutex sections, and [module API at tag 0.2.11](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/RyzenSMU.p) | Signed transport and fixed host bridge access. Lines 134–144 select Vermeer for the supported family/model, lines 614–626 authorize both scoped addresses and lines 793–814 define the read-only IOCTL and caller-held mutex. No package selection argument. |
| S6 | Microsoft Learn, [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex) and [PROCESSOR_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_relationship) | Primary Windows physical processor package enumeration contract |
| S7 | AMD, *Processor Programming Reference for AMD Family 19h Model 21h, Revision B0 Processors*, 56214-B0 Rev 3.05, April 22, 2021, [public-use document mirror](https://www.scribd.com/document/1050633015/PPR-for-AMD-Family-19h-Model-21h), §2.2.1 pp. 74, 76–79, §4.2.1 p. 328, Definitions p. 357, Memory Map – SMN p. 361 | Primary for the AM4 package field and single-socket platform, processor name leaves, and THM aperture. The public register list omits the individual CCD temperature words. |
| S8 | Tweakers, [Ryzen 3000 and 5000 workshop](https://tweakers.net/reviews/8628/amd-ryzen-overklokken-workshop-voor-ryzen-3000-en-5000.html), Ryzen Master advanced view | First-hand observation of two CCDs on a Ryzen 9 5900X. The observation does not identify the register slots. |
| S9 | AMD, [Ryzen 9 5950X General Specifications](https://www.amd.com/en/products/processors/desktops/ryzen/5000-series/amd-ryzen-9-5950x.html) | Primary product name, Vermeer codename and AM4 package. |
| S10 | AMD, [Ryzen Processor Software Optimization](https://gpuopen.com/download/GDC_AMD_Ryzen_Processor_Software_Optimization.pdf#page=22), GDC 2022, slide 22 | Primary topology for the exact Ryzen 9 5950X SKU. The diagram shows two CCDs and an I/O die. |
| S11 | InstLatx64, [Ryzen 9 5950X CPUID dump at commit a7c87bf859a7](https://github.com/InstLatx64/InstLatx64/blob/a7c87bf859a71295dedab9629a9fbccae37b8301/AuthenticAMD/AuthenticAMD0A20F10_K19_Vermeer_CPUID1.txt), Git blob `901184c36c81fe8cfc0fd3da9cbe56c7ee8598ee`, lines 2–3 and 24–28 | Independent hardware identity observation. Pins the vendor, CPUID signature, AM4 package field and full processor name. Contains no CCD register readings. |
| S12 | [lm-sensors issue #389, original report](https://github.com/lm-sensors/lm-sensors/issues/389), March 2022 | First-hand observation on one Ryzen 9 5950X with decimal family/model/stepping 25/33/0. The initial Manjaro 5.16.7 report shows `Tccd1` at 30.5 °C and `Tccd2` at 31.5 °C under `k10temp-pci-00c3`. This is output from the Linux sensor interface. |
| S13 | Linux stable [v5.16.7 AMD hardware monitor](https://github.com/gregkh/linux/blob/v5.16.7/drivers/hwmon/k10temp.c), Git blob `3618a924e78e4a47dfb882c0c824e9a3e9c37b46`, lines 68–74, 183–194, 229–233, 379–389 and 449–454, GPL-2.0-or-later | **lead-only (copyleft)**. Pins the upstream address-to-label mapping, bit 11 check at channel discovery and scaling used to interpret S12. Stable v5.16.14 has the same blob. |

## Detection

- Require the `AuthenticAMD` vendor string and CPUID.01H:EAX
  `0x00A20F10` (family `0x19`, model `0x21`, stepping 0). Require
  CPUID.80000001H:EBX bits 31:28 equal to `0x2` (AM4) and one of
  the following full processor names from CPUID leaves `0x80000002`
  through `0x80000004`, removing trailing ASCII spaces and NUL
  padding. All gates are required.
  - `AMD Ryzen 9 5900X 12-Core Processor` (S1, S2, S7).
  - `AMD Ryzen 9 5950X 16-Core Processor` (S7, S9, S11).
- Require a successful `GetLogicalProcessorInformationEx` query for
  `RelationProcessorPackage` yielding exactly one valid physical
  package record. Reject unavailable, malformed or ambiguous
  topology before issuing any CCD request. The identified AM4
  platform has one socket, and the signed `RyzenSMU` module uses a
  fixed host bridge with no package selection input (S5–S7).

| Scope | Status | Default enablement |
| --- | --- | --- |
| Ryzen 9 5900X, CPUID `0x00A20F10`, AM4, single physical package, `RyzenSMU` 0.2.11 read interface | S1, S2, S6 and S7 pin model and package identification. S8 corroborates the two-CCD count. CCD addresses, bit 11, unavailable responses and scaling depend on lead-only S3. Lead-only S4 corroborates the first two addresses and the decode on words with bit 11 set, and names 5900X as tested. S5 pins the signed transport. | Experimental with the exact detection, access and plausibility gates. |
| Ryzen 9 5950X, CPUID `0x00A20F10`, AM4, single physical package, `RyzenSMU` 0.2.11 read interface | S6, S7, S9 and S11 pin model and package identification. S10 establishes two CCDs. S12 reports both channels on a 5950X with stepping 0. Lead-only S13 establishes the upstream mapping to the two SMN addresses, bit 11 at channel discovery and scaling. Lead-only S3 supplies the unavailable-response behavior. S5 pins the signed transport. | Experimental with the exact detection, access and plausibility gates. |
| Other Ryzen models or signatures, Threadripper, EPYC, multiple packages | Require separate scoped attribution and decode evidence. | Unsupported. Follow-up in #2515. |

## Register map (facts)

| Address | Meaning | Units / encoding | Source |
| --- | --- | --- | --- |
| SMN `0x00059954` | First CCD temperature slot | 32-bit read word | S3, S4 (5900X), S13 (5950X) |
| SMN `0x00059958` | Second CCD temperature slot | 32-bit read word | S3, S4 (5900X), S13 (5950X) |
| Both slots, bit 11 | Value is valid when set | Flag | S3, S13 |
| Both slots, bits 10:0 | Raw CCD temperature field | Unsigned, 0.125 °C per increment with a −49 °C reference offset | S3, S13, corroborated when bit 11 is set by S4 for 5900X |

## Read procedure and decode

1. Confirm the detection gates and successful signed module load.
   On demand for CCD temperatures, make at most one read of each
   enumerated word per sample through `ioctl_read_smu_register`
   with the PCI mutex (S5). The addresses lie inside its
   `0x56000`–`0x5AFFF` window. Do not scan other addresses or retry
   failed reads within the sample.
2. A failed read, all-zero or all-ones word, or clear validity bit
   means that slot is unavailable. A clear validity bit cannot
   establish whether a CCD is physically absent, disabled, or
   temporarily unreadable. S3 and S13 use bit 11 when detecting channels.
   Checking bit 11 on every sample is a defensive project gate.
3. For a valid word, temperature = bits 10:0 × 0.125 °C − 49 °C
   (S3, S4, S13). Apply the existing package path's −40 to +120 °C
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

## Safety notes

- Hold `Global\Access_PCI` across each SMN request. Skip the affected
  slot on mutex timeout. The signed module performs the SMN index
  selection. No application-side PCI access, SMN data write,
  SMU command or PM-table operation is part of the scope (S5).
- Exclude unsupported packages and models before accessing any CCD
  register. Package Tctl/Tdie remains governed by the independent
  [package temperature specification](cpu-amd-zen-smn.md).

## Synthetic fixture acceptance

| Case | Required result |
| --- | --- |
| Each exact model separately, with its own full processor name, B0 signature, AM4 and one package, raw words `0x0AC8` and `0x0B38` | Read only the two fixed SMN words. Slot 1 reports 40 °C, slot 2 reports 54 °C. Preserve the package reading. |
| For each supported model, slot 1 has a failed read, clear bit 11, zero or all-ones | Slot 1 is unavailable, slot 2 retains its identity, and valid package temperature remains available. Exercise the inverse failure as well. |
| Valid zero Celsius, both plausibility limits, and one raw increment beyond either limit | Keep `0x0988` (0 °C), `0x0848` (−40 °C) and `0x0D48` (120 °C). Reject `0x0847` and `0x0D49`. |
| Two packages, unreadable package topology, or a non-AM4 package field | Make no CCD access. Existing package temperature handling continues independently. |
| Exact 5950X name with B2 signature `0x00A20F12`, or another SKU or an altered processor name with the accepted B0 signature | Make no CCD access. The signature and exact name must both match. Existing package temperature handling continues independently. |
| Module unavailable, mutex unavailable or timed out, or CCD readout not requested | Make no unauthorized reads or retries. Preserve other successful readings. |

## Open questions

- Non-blocking for the Experimental scopes: ADR 0023 permits the pinned CCD address and decode leads for these exact models and the existing read-only transport.
  A labeled CCD register dump for each exact model and signature or
  primary documentation of the CCD registers is still needed for
  Verified enablement.
- Non-blocking for the 5950X Experimental scope: S12 corroborates both channels on the exact SKU and stepping, and S5 authorizes both reads on the supported Windows transport.
  The 5950X observation is Linux output. The connection to the two
  addresses uses the corresponding upstream driver in S13. No raw words or
  Windows PawnIO measurements were recorded for that observation.
- Non-blocking for the Experimental scopes: only the exact Vermeer B0 products in single-socket AM4 configurations are admitted.
  Other revisions and packages need independent address, count,
  validity and attribution evidence in #2515. Multiple packages
  would also require a package-selectable interface.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-07 | Initial fact-only draft for the Ryzen 9 5900X Vermeer B0 candidate. |
| 2 | 2026-11-08 | Pin the AM4 package and bounded sampling gates and add synthetic acceptance cases. Transition the exact single-package 5900X B0 scope to Implementation-ready as Experimental under ADR 0023. |
| 3 | 2026-11-08 | Add the exact single-package Ryzen 9 5950X B0 Experimental scope under ADR 0023. Record its own hardware identity, two-CCD topology and Linux channel observation. Reuse the two approved SMN addresses and signed Windows transport. |
