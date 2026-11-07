# Spec: AMD Ryzen 9 5900X CCD temperatures via SMN

| Field | Value |
| --- | --- |
| Revision | 2 |
| Status | Implementation-ready (rev 2) |
| Scope | Experimental read-only CCD temperatures on Ryzen 9 5900X Vermeer B0 through PawnIO `RyzenSMU` on Windows x86-64, with one OS-enumerated physical AM4 package. Package Tctl/Tdie has a separate specification. |
| Issue phase | [#2515](https://github.com/shm11C3/HardwareVisualizer/issues/2515) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD, [Ryzen 9 5900X General Specifications](https://www.amd.com/en/products/processors/desktops/ryzen/5000-series/amd-ryzen-9-5900x.html) | Primary product name, Vermeer codename and AM4 package |
| S2 | InstLatx64, [Ryzen 9 5900X Vermeer B0 CPUID dump](https://raw.githubusercontent.com/InstLatx64/InstLatx64/master/AuthenticAMD/AuthenticAMD0A20F10_K19_Vermeer2_CPUID.txt), Git blob `b90d151a978633573d7a26fd85199ab0d987919e`, CPU Info and CPUID Registers | Independent hardware identity observation. Contains no CCD temperature register dump. |
| S3 | Linux [v6.18 AMD hardware monitor](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/k10temp.c), GPL-2.0-or-later | **lead-only (copyleft)**. Family 19h model 21h CCD map, validity at channel discovery, unavailable responses and scaling. |
| S4 | LibreHardwareMonitor, [AMD Zen CPU monitor at commit `2808d80564b6`](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Cpu/Amd17Cpu.cs), MPL-2.0 | **lead-only (copyleft)**. Names 5900X as tested and uses the PawnIO SMN read interface for CCD temperatures at this pin. The test note omits the stepping, raw words and transport version. |
| S5 | [PawnIO interface](pawnio-interface.md) revision 7, `RyzenSMU` and mutex sections, and [module API at tag 0.2.11](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/RyzenSMU.p) | SMN window authorization, caller-held mutex, and fixed host bridge access. No package selection argument. |
| S6 | Microsoft Learn, [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex) and [PROCESSOR_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_relationship) | Primary Windows physical processor package enumeration contract |
| S7 | AMD, *Processor Programming Reference for AMD Family 19h Model 21h, Revision B0 Processors*, 56214-B0 Rev 3.05, April 22, 2021, [public-use document mirror](https://www.scribd.com/document/1050633015/PPR-for-AMD-Family-19h-Model-21h), §2.2.1 pp. 74, 76–79, §4.2.1 p. 328, Definitions p. 357, Memory Map – SMN p. 361 | Primary for the AM4 package field and single-socket platform, processor name leaves, and THM aperture. The public register list omits the individual CCD temperature words. |
| S8 | Tweakers, [Ryzen 3000 and 5000 workshop](https://tweakers.net/reviews/8628/amd-ryzen-overklokken-workshop-voor-ryzen-3000-en-5000.html), Ryzen Master advanced view | First-hand observation of two CCDs on a Ryzen 9 5900X. The observation does not identify the register slots. |

## Detection

- Require the `AuthenticAMD` vendor string and CPUID.01H:EAX
  `0x00A20F10` (family `0x19`, model `0x21`, stepping 0). Require
  CPUID.80000001H:EBX bits 31:28 equal to `0x2` (AM4) and the full
  processor name `AMD Ryzen 9 5900X 12-Core Processor` from CPUID
  leaves `0x80000002` through `0x80000004`, removing trailing ASCII
  spaces and NUL padding (S1, S2, S7). All gates are required.
- Require a successful `GetLogicalProcessorInformationEx` query for
  `RelationProcessorPackage` yielding exactly one valid physical
  package record. Reject unavailable, malformed or ambiguous
  topology before issuing any CCD request. The identified AM4
  platform has one socket, and the signed `RyzenSMU` module uses a
  fixed host bridge with no package selection input (S5–S7).

| Scope | Status | Default enablement |
| --- | --- | --- |
| Ryzen 9 5900X, CPUID `0x00A20F10`, AM4, single physical package, `RyzenSMU` 0.2.11 read interface | S1, S2, S6 and S7 pin model and package identification. S8 corroborates the two-CCD count. CCD addresses, bit 11, unavailable responses and scaling depend on lead-only S3. Lead-only S4 corroborates the first two addresses and the decode on words with bit 11 set, and names 5900X as tested. S5 pins the signed transport. | Experimental with the exact detection, access and plausibility gates. |
| Other Ryzen models or 5900X signatures, Threadripper, EPYC, multiple packages | No scoped attribution or approved decode in this draft. | Unsupported. Follow-up in #2515. |

## Register map (facts)

| Address | Meaning | Units / encoding | Source |
| --- | --- | --- | --- |
| SMN `0x00059954` | First CCD temperature slot | 32-bit read word | S3, S4 |
| SMN `0x00059958` | Second CCD temperature slot | 32-bit read word | S3, S4 |
| Both slots, bit 11 | Value is valid when set | Flag | S3 |
| Both slots, bits 10:0 | Raw CCD temperature field | Unsigned, 0.125 °C per increment with a −49 °C reference offset | S3, corroborated when bit 11 is set by S4 |

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
   temporarily unreadable. S3 uses bit 11 when detecting channels.
   Checking bit 11 on every sample is a defensive project gate.
3. For a valid word, temperature = bits 10:0 × 0.125 °C − 49 °C
   (S3, S4). Apply the existing package path's −40 to +120 °C
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
| Exact model and one package, raw words `0x0AC8` and `0x0B38` | Slot 1 reports 40 °C, slot 2 reports 54 °C. Preserve the package reading. |
| Slot 1 has a failed read, clear bit 11, zero or all-ones | Slot 1 is unavailable, slot 2 retains its identity, and valid package temperature remains available. Exercise the inverse failure as well. |
| Valid zero Celsius, both plausibility limits, and one raw increment beyond either limit | Keep `0x0988` (0 °C), `0x0848` (−40 °C) and `0x0D48` (120 °C). Reject `0x0847` and `0x0D49`. |
| Two packages, unreadable package topology, another stepping or SKU, or a non-AM4 package field | Make no CCD access. Existing package temperature handling continues independently. |
| Module unavailable, mutex unavailable or timed out, or CCD readout not requested | Make no unauthorized reads or retries. Preserve other successful readings. |

## Open questions

- Non-blocking for the first Experimental slice: ADR 0023 permits the pinned CCD address and decode leads for this exact model and read-only transport.
  A labeled CCD dump for the exact signature or primary documentation
  of the CCD registers is still needed for Verified enablement.
- Non-blocking for the first Experimental slice: only the single-socket AM4 configuration and exact Vermeer B0 product are admitted.
  Other revisions and packages need independent address, count,
  validity and attribution evidence in #2515. Multiple packages
  would also require a package-selectable interface.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-07 | Initial fact-only draft for the Ryzen 9 5900X Vermeer B0 candidate. |
| 2 | 2026-11-08 | Pin the AM4 package and bounded sampling gates and add synthetic acceptance cases. Transition the exact single-package 5900X B0 scope to Implementation-ready as Experimental under ADR 0023. |
