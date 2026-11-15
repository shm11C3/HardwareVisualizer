# Spec: AMD Genoa B1 CCD temperatures via SMN

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental Windows CCD temperatures for exact Genoa B1 signature `0x00A10F11` and SP5 package field 4 with one active physical package, via the signed PawnIO `RyzenSMU` read interface. |
| Issue phase | [#2702](https://github.com/shm11C3/HardwareVisualizer/issues/2702), follow-up to [#2515](https://github.com/shm11C3/HardwareVisualizer/issues/2515) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD [Family 19h Models 10h–1Fh Revision Guide](https://docs.amd.com/api/khub/documents/LZ~6p62H~zRDhkNiPAE9NQ/content#page=8), 57095 rev 1.05, November 2025, Table 2 p. 8 and Table 6 pp. 10–13 | Primary for the B1 and B2 signatures, SP5 product scope and public errata. |
| S2 | AMD Family 19h Model 11h B1 PPR, 55901 rev 0.53, May 4, 2023, [Vol. 1](https://www.scribd.com/document/952601815/55901-B1-pub-1), Tables 13–14 and the bus-number note pp. 34–38, §1.9.4 pp. 39–40, extended CPUID EBX p. 107, and [Vol. 4](https://www.scribd.com/document/1050625500/55901-B1-pub-4), §9.2.2 pp. 8–10 | AMD-authored public documents, mirrored. Primary for SP5 package encoding, single-socket topology, boot root-complex bus assignment and the NBIO SMN index/data aperture. |
| S3 | AMD [DPDK tuning guide for EPYC 9004](https://www.amd.com/content/dam/amd/en/documents/epyc-technical-docs/tuning-guides/58017-amd-epyc-9004-tg-data-plane-dpdk.pdf#page=9), 58017 rev 1.3, §2.11.1 Figure 2-7 and §2.12.1, and [5th Gen AMD EPYC Processor Architecture](https://docs.amd.com/api/khub/documents/UIqhAbjRhgnzgzzdVU4pUw/content#page=18), PID 242937605-C, March 2025, end note 9xx5-001 | Primary for Genoa's 12 physical CCD positions behind one I/O die and a 12-CCD EPYC 9654 configuration. The NUMA partition count can differ from the socket count. |
| S4 | InstLatx64 [EPYC 9124 CPUID record](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/AuthenticAMD/AuthenticAMD0A10F11_K19_Genoa_02_CPUID.txt#L3-L38), [EPYC 9654 Linux CPU inventory](https://github.com/cyring/CoreFreq/issues/540), and CIQ/Thomas-Krenn [RA1212-AIEPN+ certification](https://c3.ciq.com/submissions/3fa53d5b-31ab-45dc-b0de-fcc394a717ba) | Separate observations. The EPYC 9124 CPUID record contains exact signature and raw extended EBX `0x40000000`. The EPYC 9654 report identifies family 19h model 11h stepping 1 on a two-socket machine. The named one-socket EPYC 9124 server has AMD host bridge `1022:14A4` at bus 0, device 0, function 0, alongside other roots. |
| S5 | EPYC 9274F owner, March 2024 [CPU topology and PCI inventory](https://github.com/cyring/CoreFreq/issues/479#issuecomment-2002587206), [Linux CCD values](https://github.com/cyring/CoreFreq/issues/479#issuecomment-2002664547), and [raw SMN observation](https://github.com/cyring/CoreFreq/issues/479#issuecomment-2002681047). Historical reader [address and operation binding](https://github.com/cyring/Tips/blob/6a88926fff4377607c1087c30c533e96add8ffaa/C/zencli.c#L244-L251), lines 244–251, 373–376 and 792–802 | First-hand observations from the same reporter. The machine reports family 19h model 11h stepping 1, one active package, and the AMD host bridge at bus 0, device 0, function 0. It shows `Tccd1` through `Tccd8` on Linux and separately records `0x00000AC4` at `0x00059B08`. The historical reader source is **lead-only (copyleft)** for the raw capture route, which matches S2 and S8. The exact reader build was unpublished. |
| S6 | AMD's Babu Moger, [Linux commit 8bb050cd5cf4](https://github.com/torvalds/linux/commit/8bb050cd5cf494f3d0cb45a6b54a476af09edb8d); Linux v6.18, commit `7d0a66e4bb9081d75c82ec4957c50034cb0ea449`, [`k10temp` register fields](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/hwmon/k10temp.c#L69-L74), node-relative read at lines 160–176, decode at lines 244–250, discovery at lines 399–421, and [family 19h model dispatch](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/hwmon/k10temp.c#L489-L511) (model band at lines 507–510), and [`amd_node`](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/arch/x86/kernel/amd_node.c#L37-L51), lines 37–51, 86–126 and 257–300 | **Lead-only (copyleft)** for model 11h's twelve addresses, bit fields, units, discovery-time absence and distinction between the Linux node-owned and signed Windows apertures. The AMD-authored upstream change explicitly assigns the twelve-word map to the model band containing model 11h. |
| S7 | illumos commit `098e95820891f8a83fae679daece7fcf58b7b3a4`, [thermal register facts](https://github.com/illumos/illumos-gate/blob/098e95820891f8a83fae679daece7fcf58b7b3a4/usr/src/uts/intel/sys/amdzen/thm.h#L79-L179), lines 44–50, 79–88, 116–118 and 129–179, and [CCD discovery](https://github.com/illumos/illumos-gate/blob/098e95820891f8a83fae679daece7fcf58b7b3a4/usr/src/uts/intel/io/amdzen/smntemp.c#L334-L354) | **Lead-only (copyleft)**. A separate producer names Genoa explicitly, agrees on 12 per-die entries starting at `0x59B00` and the validity/payload fields, and leaves a validity-clear entry absent at discovery. |
| S8 | [PawnIO interface](pawnio-interface.md) rev 7, `RyzenSMU` and Mutex conventions, and [signed RyzenSMU 0.2.11 `ioctl_read_smu_register`](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/RyzenSMU.p#L793-L815) (caller mutex at line 800), allowed CCD window at lines 624–626, internal classification at lines 93–94 and 134–142, load admission at lines 864–914 (Genoa entry at line 229), and internal PCI route at lines 182–190 and 247–283 | Primary upstream contract for the signed read operation, the allowed SMN address window and caller-held mutex. The module's internal Genoa classification and fixed PCI host route are **lead-only (copyleft)**. |
| S9 | Microsoft Learn, [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex), [PROCESSOR_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_relationship) and [GetActiveProcessorCount](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getactiveprocessorcount) | Primary OS contract for physical package enumeration, per-group package masks and active processor count across groups. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, CPUID leaf 0 vendor `AuthenticAMD` and leaf 1 EAX exactly `0x00A10F11` (family `0x19`, display model `0x11`, stepping 1) | mechanism (selects the evidenced Genoa B1 map) | S1, S4–S8 |
| Extended leaf `0x80000001` is available and its EBX bits 31:28 equal 4 | mechanism (selects the documented SP5 package) | S2, S4 |
| Complete Windows physical-package enumeration reports one nonempty package with valid processor group masks covering all active logical processors | safety (the signed access has one fixed boot-package endpoint) | S2, S4, S5, S8 (lead-only for the module's internal route), S9 |
| The existing signed `RyzenSMU` read interface is available | safety (bounded, authorized access) | S8 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact Genoa B1 signature and SP5 package field with one complete active physical package, via signed `RyzenSMU` | The EPYC 9124 raw CPUID and one-socket PCI observations are independent (S4). The EPYC 9274F supplies single-package topology, eight converted Linux channels and a separate raw observation at the third fixed address (S5). AMD documents the exact revision, package, boot root and 12 physical CCD positions (S1–S3). The full thermal word map, validity and scaling rely on lead-only S6/S7, with the independent observation in S5. Attribution of the historical raw reader's aperture is lead-only S5. Internal signed-module classification and fixed route are lead-only S8. | Experimental, with validity checked separately on all twelve fixed positions. |
| Any other or unavailable identity, ambiguous, incomplete or multiple active physical packages, or an unavailable signed interface | Unsupported for this mechanism. | Disabled. |

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x00059B00`, `0x00059B04`, `0x00059B08`, `0x00059B0C`, `0x00059B10`, `0x00059B14`, `0x00059B18`, `0x00059B1C`, `0x00059B20`, `0x00059B24`, `0x00059B28`, `0x00059B2C` in order | CCD thermal positions 1–12, twelve 32-bit words | 11 | Valid for this position | 1 = valid | S3 (physical positions), S5 (position 3 raw word), S6/S7 (lead-only map and validity), S8 (access width) |
| Same twelve addresses | CCD thermal positions 1–12 | 10:0 | Temperature code for this position | Unsigned, 0.125 °C per count with a −49 °C offset | S6/S7 (lead-only fields), S6 (lead-only scaling) |

## Read procedure and decode

1. Use the existing Windows CCD detail acquisition on demand. Issue at
   most one signed read per listed word per sample. Acquire the existing
   `Global\Access_PCI` mutex independently for each transaction under the
   existing bounded timeout. Release it after the transaction. An
   acquisition timeout or read failure leaves that position unavailable.
   Continue with the other positions and use no retry or other address (S8).
2. Treat a transport failure, zero word, all-ones word or clear bit 11
   as unavailable for this position. Check bit 11 on each response.
   The OS observations in S5 and the discovery rules in S6/S7 do not
   establish ongoing validity for a later response.
3. Multiply the unsigned bits 10:0 by 0.125 °C and subtract 49 °C (S6).
   Use the existing [CCD plausibility window](cpu-amd-raphael-ccd-smn.md#read-procedure-and-decode)
   of −40 °C through +120 °C inclusive. The range is an application
   data-quality check.
4. Keep all twelve fixed position identities on package index 0. The
   one-based CCD slot is the word's position in the table. Keep an
   invalid or failed position unavailable and preserve valid siblings.
   The slot identity requires no OS core, NUMA node or processor group
   mapping (S2/S3; S6/S7 for the map). Existing [package temperature](cpu-amd-zen-smn.md)
   follows its independent rules.

## Quirks

- AMD's documented boot die assigns Bus A to bus 0. The segment-0
  root-complex function at device 0, function 0 is AMD `1022:14A4`.
  The local NBIO exposes the SMN index/data aperture on that function.
  With one active SP5 package, this is the boot package's local aperture.
  The single-socket SP5 design has no second socket link. The observed
  fixed route agrees with the EPYC 9274F capture and the separate EPYC
  9124 PCI inventory (S2, S4, S5; S8 lead-only for the module's route).
- The positive `0x00000AC4` observation decodes to 39.5 °C at slot 3
  under the documented Experimental decode (S5, S6). It is a separate
  read from the earlier Linux values. The number of valid slots can
  vary by product or available die. One physical package can cover
  multiple Windows processor groups and NUMA domains (S2–S4, S9).
- AMD revision guide 57095 rev 1.05 lists no applicable correction to
  these CCD reads. Its erratum 1394 concerns SMBus, which is outside
  the local PCI configuration-space transaction in this scope (S1,
  S2, S8).

## Safety notes

- The authorized operation is the existing signed `ioctl_read_smu_register`
  request. All twelve addresses are in the `0x56000`–`0x5AFFF` allow-list.
  The module accepts a 32-bit result per word and checks the AMD vendor at
  the fixed function on load (S8, lead-only for the module's vendor check).
- The signed module selects the SMN address at PCI offset `0xC4` and
  reads the data dword at `0xC8`. The transaction writes the selector and
  carries no write to the target data. The interface makes no selector
  restoration guarantee. The application holds the specified PCI mutex
  across each transaction (S2, S8; lead-only for internal module access).
- Limit this profile to the documented fixed boot-package aperture and
  the twelve CCD thermal words. The application must not write the SMN
  data register, change controls, use mailbox commands, scan for another
  BDF, or access configuration ports directly. Linux's separate node-owned
  `0x60`/`0x64` aperture supplies corroboration for the thermal word map
  (S6, lead-only). It provides no Windows socket-selection contract.

## Synthetic fixture acceptance

| Fixture | Expected result |
| --- | --- |
| Exact B1 signature and SP5 field, one complete package, demand present; slot 3 returns the observed `0x00000AC4`, slot 12 returns synthetic `0x00000B00`, and other positions return zero | Request exactly the twelve approved addresses once each. Report 39.5 °C at slot 3 and 47 °C at slot 12. Keep every other position unavailable, including holes before both valid values. |
| One complete active package spans multiple Windows groups; values at slots 1, 9 and 12 differ | Preserve the fixed address order and separate values at the three slot identities on package index 0. Group count, NUMA node count, core count and brand text do not change admission. |
| B2 (`0x00A10F12`) with SP5 field, different model or vendor, a missing extended leaf, non-SP5 field, invalid masks, missing or incomplete package enumeration, zero packages or two packages in one NUMA node | Reject the target before any CCD read. |
| A mutex timeout, read failure, zero, all ones or clear bit 11 with a plausible payload in one position | Keep only the affected position unavailable. Preserve all slot identities, successful siblings and any independent package result. A newly invalid response must replace an earlier valid result at that position. |
| Words `0x0988`, `0x0848`, `0x0D48`, `0x0847`, `0x0D49` | Retain 0 °C, −40 °C and +120 °C. Reject one increment outside either bound. |
| Missing signed interface or no sampling demand | Issue no CCD register requests and use no fallback transport. |

## Open questions

- Non-blocking for the Genoa B1 single-package Experimental scope: the two independently maintained maps and AMD's package diagram establish the twelve fixed positions.
  The raw EPYC 9274F capture confirms slot 3 only, and the separate Linux
  output shows slots 1–8. There is no public raw capture of slots 9–12
  on an admitted machine and no Windows sample through the signed
  HardwareVisualizer acquisition path.
- Non-blocking for the Genoa B1 single-package Experimental scope: the fixed boot-package access is covered for one active package.
  Multi-package attribution needs a separately established and
  authorized physical socket selection and ownership contract.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-16 | Proposes Implementation-ready status for exact single-package Genoa B1 SP5 Experimental readings, pending maintainer approval. Pins twelve fixed positions, per-read validity and the existing signed transport. |
