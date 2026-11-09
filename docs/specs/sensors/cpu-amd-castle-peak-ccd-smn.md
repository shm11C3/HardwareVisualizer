# Spec: AMD Castle Peak B0 CCD temperatures via SMN

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental CCD temperatures on exact Castle Peak B0 signature `0x00830F10` and sTRX4 package field with one physical package, through the signed Windows PawnIO `RyzenSMU` read interface. |
| Issue phase | [#2515](https://github.com/shm11C3/HardwareVisualizer/issues/2515) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD, [Family 17h Model 31h B0 PPR](https://www.amd.com/content/dam/amd/en/documents/processor-tech-docs/programmer-references/55803-ppr-family-17h-model-31h-b0-processors.pdf), 55803 rev 0.91, September 1, 2020, `1.6.1, 1.7.1 and 2.1.13.1 (extended CPUID EBX) and [Family 17h Models 30h–3Fh Revision Guide](https://www.amd.com/content/dam/amd/en/documents/processor-tech-docs/revision-guides/56323-PUB_1_01.pdf#page=8), 56323 rev 1.01, April 2023, Table 2 | Primary for signature structure and the read-only package field. The documents describe SP3 and do not assign the Castle Peak nibble or specify the eight CCD thermal words. |
| S2 | AMD, specifications for [3960X](https://www.amd.com/en/support/downloads/drivers.html/processors/ryzen-threadripper/ryzen-threadripper-3000-series/amd-ryzen-threadripper-3960x.html#amd_support_product_spec), [3970X](https://www.amd.com/en/support/downloads/drivers.html/processors/ryzen-threadripper/ryzen-threadripper-3000-series/amd-ryzen-threadripper-3970x.html#amd_support_product_spec), and [3990X](https://www.amd.com/en/support/downloads/drivers.html/processors/ryzen-threadripper/ryzen-threadripper-3000-series/amd-ryzen-threadripper-3990x.html#amd_support_product_spec), and [Ryzen Master 2.9 Reference Guide](https://www.amd.com/content/dam/amd/en/documents/products/software-tools/ryzen-master-quick-reference-guide.pdf#page=35), April 2022, pp. 34–35 and 40 | Primary for the Castle Peak product and sTRX4 scope. The guide shows eight CCD positions on the 3990X and describes disabling whole CCDs on multi-CCD products. |
| S3 | InstLatx64 raw CPUID archive at `17be9a9793c306cf96f8c286e34e705b780a0226`, [3960X](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/AuthenticAMD/AuthenticAMD0830F10_K17_CastlePeak_CPUID2.txt#L5-L31), [3970X](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/AuthenticAMD/AuthenticAMD0830F10_K17_CastlePeak_CPUID.txt#L46-L72), [PRO 3955WX](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/AuthenticAMD/AuthenticAMD0830F10_K17_Rome_CPUID6.txt#L5-L31), [PRO 3975WX](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/AuthenticAMD/AuthenticAMD0830F10_K17_Rome_CPUID7.txt#L5-L31), and [EPYC 7742](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/AuthenticAMD/AuthenticAMD0830F10_K17_Rome_CPUID.txt#L5-L31) | Independent raw observations. All five report signature `0x00830F10`. The two sTRX4 units report extended EBX `0x70000000`. Both PRO units and EPYC report `0x40000000`. |
| S4 | Guenter Roeck, [January 23, 2020 v4 hwmon submission and raw 3970X observations](https://lists.openwall.net/linux-kernel/2020/01/23/987) | First-hand 3970X idle and load words and a separate idle Linux temperature display on the same exact CPU model. The patch portion is **lead-only (copyleft)** for channel mapping and conversion. |
| S5 | Independent first-hand reports for [3960X Linux temperatures](https://forums.unraid.net/topic/129653-power-consumption-threadripper-3960x/), [3990X Linux temperatures and CPU identification](https://lists.debian.org/debian-user/2021/10/msg00895.html), [3990X Windows CPUID and socket topology](https://forum.level1techs.com/t/windows-11-got-something-right-fixed-3990x-numa-issue/173932/10), and [3970X PCI inventory](https://forum.level1techs.com/t/threadripper-3970x-gpu-passthrough-issues/153567) | The 3960X displays slots 1, 3, 5 and 7. The 3990X displays slots 1–8 on Linux and independently reports exact signature `0x00830F10` and one Windows socket spanning two processor groups. The PCI observation locates AMD `1022:1480` at bus 0, device 0, function 0 on a 3970X with several root buses. |
| S6 | Linux v6.18, commit `7d0a66e4bb9081d75c82ec4957c50034cb0ea449`, [AMD thermal monitor](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/hwmon/k10temp.c#L68-L74), lines 68–74, 172–176, 239–251, 399–420 and 467–486, and [AMD SMN transport](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/arch/x86/kernel/amd_node.c#L37-L51), lines 37–51, 86–126 and 257–300 | **Lead-only (copyleft)** for the model 31h eight-word map, label-to-address relation, channel-discovery validity, decode, and distinction between the Linux and signed Windows access paths. |
| S7 | [PawnIO interface](pawnio-interface.md) rev 7, `RyzenSMU and Mutex conventions, and [signed RyzenSMU 0.2.11 interface source](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/RyzenSMU.p#L118-L119), lines 118–119, 246–282, 617–626, 793–814 and 865–908 | Upstream module contract. Family 17h model 31h with package field 7 selects Castle Peak. The signed read entry point admits all eight words and takes the normal fixed AMD host-bridge route. |
| S8 | Microsoft Learn, [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex) and [PROCESSOR_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_relationship) | Primary OS contract for enumerating physical packages and their processor group masks. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, CPUID leaf 0 vendor `AuthenticAMD` and CPUID.01H:EAX exactly `0x00830F10` (family `0x17`, model `0x31`, stepping 0) | mechanism (the evidenced die and revision) | S1, S3–S7 |
| Extended CPUID leaf `0x80000001` available and EBX bits 31:28 equal `0x7` | mechanism (selects the sTRX4 Castle Peak module route and excludes the observed Rome and PRO package encoding) | S1–S3, S7 |
| Complete Windows physical processor package enumeration reports one nonempty package with valid group masks | safety (the fixed PCI target cannot select a socket) | S5, S7, S8 |
| The existing signed `RyzenSMU` read transport is available and its caller-held PCI mutex can be acquired before a request | safety (signed bounded transaction and serialization) | S7 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact signature `0x00830F10`, package field `0x7`, one physical package, and signed `RyzenSMU` read interface | 3960X and 3970X supply exact raw identity (S3). The 3970X supplies raw thermal evidence (S4). The 3990X independently corroborates the same signature, eight channels and one physical package across groups (S2, S5). The eight-word map, absent-slot handling, validity and conversion rely on lead-only S4/S6. S7 pins the signed read transport. | Experimental with independent checks on eight fixed slots. |
| Other signature or stepping, any other or unavailable package field, or incomplete, ambiguous or multiple physical packages | No matching mechanism under this specification. | Disabled. |

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x00059954`, `0x00059958`, `0x0005995C`, `0x00059960`, `0x00059964`, `0x00059968`, `0x0005996C`, `0x00059970`, in that order | CCD thermal slots 1–8, eight 32-bit words | 11 | Valid indication for the same slot | 1 = valid | S4, S6 (lead-only map and validity), S5 (observed channels), S7 (access width) |
| Same eight addresses | CCD thermal slots 1–8 | 10:0 | Temperature code for the same slot | 0.125 °C per count with a −49 °C offset | S4, S6 (lead-only decode) |

## Read procedure and decode

1. Use the existing Windows sensor acquisition path. Sample on demand.
   Issue at most one signed read request per listed word per sample
   with bounded acquisition of the caller-held mutex (S7).
2. Handle each response independently. A failed read, zero word,
   all-ones word or clear bit 11 leaves that slot unavailable. The
   Linux producer uses bit 11 to discover channels. The application
   checks the validity bit on every response (S4, S6).
3. Multiply the unsigned value in bits 10:0 by 0.125 °C and subtract
   49 °C (S4, S6). Use the existing [Raphael CCD plausibility
   window](cpu-amd-raphael-ccd-smn.md#read-procedure-and-decode) of
   −40 °C through +120 °C inclusive. This is a data-quality bound.
4. Keep all eight fixed slot identities on the admitted package. The
   one-based slot is the address position in the table. Keep a failed
   or invalid reading unavailable without renumbering later slots.
   Preserve other valid slots and any independent package temperature.

## Quirks

- The 3970X report records positive odd-slot words during idle and load.
  The same capture records zero words for slots 2, 4 and 6 and ends at
  slot 7. The 3990X Linux report shows valid values at every position,
  including slot 8. The fixed map covers populated and unpopulated
  positions without a model-name test or a fixed count of populated
  CCDs (S2, S4–S6).
- Package count is independent of processor group and NUMA node count.
  A single package can span more than one Windows processor group
  (S5, S8). The read path targets the package SMN endpoint and requires
  no thread affinity or assignment of a CCD to OS cores (S7).

## Safety notes

- The signed module accepts the eight words within its `0x56000`–`0x5AFFF`
  read window. Its normal route selects the SMN address through
  offset `0xC4` of PCI bus 0, device 0, function 0 and reads the
  result from offset `0xC8`. It checks the AMD vendor at the fixed
  function on load. The module makes no selector restoration guarantee.
  The caller holds the existing PCI mutex for every transaction and
  every transaction selects its own address (S7).
- The Linux observations support the SMN word identities and decode.
  Linux uses its node-owned `0x60`/`0x64` aperture (S6). The Windows
  route is supported by the signed module's own exact Castle Peak
  recognition and read contract (S7).
- A failed identity or topology gate prevents CCD access. This profile
  uses the eight enumerated words only. It does not scan PCI or SMN,
  perform direct port access, issue mailbox commands, request a PM
  table or write SMN data. The independent [package temperature
  path](cpu-amd-zen-smn.md) remains available through its existing rules.

## Synthetic fixture acceptance

| Fixture | Expected result |
| --- | --- |
| Exact signature, package nibble 7 and one physical package, with 3970X capture-shaped words `0x0ABC`, zero, `0x0AD8`, zero, `0x0ADE`, zero, `0x0AE4`, and a synthetic invalid slot 8 | Request the eight listed addresses once each. Report 38.5, 42, 42.75 and 43.5 °C at slots 1, 3, 5 and 7. Preserve unavailable identities 2, 4, 6 and 8. |
| One complete package spanning two Windows processor groups and eight valid words including an independent valid value at slot 8 | Preserve eight distinct values at slots 1–8 on package index 0. Keep eligibility and request order independent of brand, core count, group count and NUMA count. |
| Same signature with package nibble 4, unknown or unavailable extended package field, different signature or stepping, wrong vendor, malformed group masks, missing enumeration, empty package or multiple packages | Return no target and issue no CCD requests. |
| An individual read fails, returns zero or all ones, or has a clear validity bit with a plausible lower field | Mark only that slot unavailable. Preserve the other identities, their valid results and package temperature. A subsequent invalid sample must not retain the prior successful value. |
| Words `0x0988`, `0x0848`, `0x0D48`, `0x0847` and `0x0D49` | Retain 0 °C, −40 °C and +120 °C. Reject values one increment outside either bound. |
| Sampling is inactive, the module is unavailable, or the mutex cannot be acquired | Skip the corresponding requests. Preserve unavailable identities on an admitted target and use no direct fallback. |

## Open questions

- Non-blocking for the Castle Peak B0 Experimental scope: the raw 3960X and 3970X identities and the signed module independently pin the positive package gate, and the 3990X establishes all eight thermal positions.
  The public 3990X evidence omits its raw package nibble and provides no
  Windows CCD sample through this module. A target still needs the
  positive nibble 7 gate. No SKU name substitutes for that check.
- Non-blocking for the Castle Peak B0 Experimental scope: the package field and physical package gate select the supported sTRX4 path.
  Other package values, including the observed sWRX8 and SP3 examples,
  remain outside the scope. Neither the signature nor a temperature
  report alone establishes their signed Windows access behavior.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-10 | Proposes Implementation-ready status for the exact single-package Castle Peak B0 sTRX4 Experimental scope, pending maintainer approval. Pins the eight slot positions, per-read validity, and signed RyzenSMU route. |
