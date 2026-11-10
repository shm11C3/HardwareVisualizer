# Spec: AMD Matisse B0 CCD temperatures via SMN

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental read-only CCD temperatures on exact Matisse B0 signature `0x00870F10` and one physical AM4 package via the signed Windows PawnIO `RyzenSMU` interface. |
| Issue phase | [#2515](https://github.com/shm11C3/HardwareVisualizer/issues/2515) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD, [Processor Programming Reference for AMD Family 17h Model 71h, Revision B0 Processors](https://bugzilla.kernel.org/attachment.cgi?id=287401), 56176 rev 3.06, July 17, 2019, §§1.5.1, 1.6.1 and 1.8.1 pp. 26–27, extended CPUID p. 66 | Primary vendor document, archived by kernel.org. It identifies the B0 model, AM4 single-socket infrastructure and package field. PDF SHA-256 `547ce51e1335b07e26bb09b89d78d499b0256daf6a0c94bb3cee711384505e06`. |
| S2 | AMD, [Ryzen 9 3900X specifications](https://www.amd.com/en/support/downloads/drivers.html/processors/ryzen/ryzen-3000-series/amd-ryzen-9-3900x.html#amd_support_product_spec), General Specifications, and [AMD Ryzen Master 2.9 Reference Guide](https://www.amd.com/content/dam/amd/en/documents/products/software-tools/ryzen-master-quick-reference-guide.pdf#page=40), April 2022, Active CCD Mode, p. 40 | Primary product documentation. AMD identifies the 3900X as Matisse on AM4 and documents it as a multi-CCD processor. |
| S3 | InstLatx64, [Ryzen 9 3900X raw CPUID capture](https://github.com/InstLatx64/InstLatx64/blob/ddff8a9210c97b395d1463aa7e24c8bd56c9fcda/AuthenticAMD/AuthenticAMD0870F10_K17_Matisse_CPUID3.txt#L3-L31), Git blob `80a9918e91a031901dc2e1cd9bf1be5c7c139f0c`, lines 3–6 and 28–31 | Independent exact-CPU observation of `AuthenticAMD`, signature `0x00870F10`, and extended leaf EBX `0x20000000`. The capture contains no CCD thermal reads. |
| S4 | Guenter Roeck, [January 22, 2020 v4 hwmon submission and 3900X test](https://lists.openwall.net/linux-kernel/2020/01/22/1208) and [matching v4 patch 3/6](https://lists.openwall.net/linux-kernel/2020/01/22/1217) | First-hand Linux observation on a loaded Ryzen 9 3900X with the v4 series applied: `Tccd1` 61.8 °C and `Tccd2` 76.5 °C. Patch 3/6 is **lead-only (copyleft)** for that build's exact channel-to-address association and earlier encoding interpretation. |
| S5 | Guenter Roeck, [January 23, 2020 validity and conversion clarification](https://lists.openwall.net/linux-kernel/2020/01/23/987) and [Linux v6.18 AMD hardware monitor](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/k10temp.c), lines 68–74, 172–176, 244–249, 399–420 and 467–483 | **lead-only (copyleft)**. The v6.18 model `0x71` path identifies the CCD addresses. The channel discovery and read behavior establish validity, failure responses, temperature bits and scaling. The earlier clarification includes raw readings on a different CPU, which do not establish Matisse raw readback. |
| S6 | [PawnIO interface](pawnio-interface.md) rev 7, `RyzenSMU` and mutex sections, and [RyzenSMU API at tag 0.2.11](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/RyzenSMU.p), lines 126–127, 246–250, 266–282, 617–626, 793–814 and 865–908 | Upstream interface for the signed read transport. Family `0x17` model `0x71` selects the normal fixed AMD host-bridge path and both addresses are in its allow-list. The module's recognition is wider than this specification's identity gates. |
| S7 | Microsoft Learn, [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex) and [PROCESSOR_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_relationship) | Primary contract for Windows physical processor package enumeration. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, CPUID leaf 0 vendor `AuthenticAMD` and CPUID.01H:EAX exactly `0x00870F10` (family `0x17`, model `0x71`, stepping 0) | mechanism (the scoped die and revision of the CCD map) | S1–S6 |
| Extended CPUID leaf `0x80000001` is available and EBX bits 31:28 equal `0x2` | mechanism (AM4 package) | S1–S3 |
| `GetLogicalProcessorInformationEx` with `RelationProcessorPackage` succeeds and reports exactly one valid physical package record | mechanism (attribution through the fixed host bridge without a package selector) | S1, S6, S7 |
| The signed `RyzenSMU` module loads and the caller can hold `Global\Access_PCI` for the read | safety (authorized, serialized selector/data access) | S6 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact signature `0x00870F10`, package field `0x2`, one physical package, signed `RyzenSMU` read interface | Ryzen 9 3900X is the evidence identity (S1–S4). The separate raw CPUID capture and Linux output establish exact model identity and two measured channels on that model. The two channel addresses and the original report's encoding linkage are lead-only S4/S5. Validity, field width, scaling and inaccessible-word handling are lead-only S5. S6 pins the signed read interface. | Experimental with independent validity and plausibility checks on both fixed slots. |
| Other signature, stepping, unknown or different package field, or topology that fails the single-package gate | No matching mechanism in this revision. Other approved CCD scopes are specified separately. | Disabled under this specification. |

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x00059954` | CCD slot 1 temperature | 11 | Valid indication for slot 1 | 1 = valid | S4, S5 (lead-only) |
| `0x00059954` | CCD slot 1 temperature | 10:0 | Slot 1 temperature code | 0.125 °C per count, −49 °C offset | S4, S5 (lead-only) |
| `0x00059958` | CCD slot 2 temperature | 11 | Valid indication for slot 2 | 1 = valid | S4, S5 (lead-only) |
| `0x00059958` | CCD slot 2 temperature | 10:0 | Slot 2 temperature code | 0.125 °C per count, −49 °C offset | S4, S5 (lead-only) |

Each address is a 32-bit SMN word (S5 lead-only, S6).

## Read procedure and decode

1. Sample on demand through the existing Windows sensor acquisition path.
   Use only the signed `ioctl_read_smu_register` contract (S6). Make at
   most one read attempt for each scoped address per sample, with bounded
   acquisition of the caller-held mutex.
2. For each slot, treat a failed request, a zero word, an all-ones word
   or a clear bit 11 as unavailable. S5 uses bit 11 in channel discovery.
   The application checks it independently on every response.
3. Multiply bits 10:0 by 0.125 °C and subtract 49 °C (S5). Retain
   values from −40 °C through +120 °C inclusive, following the
   [Raphael CCD path](cpu-amd-raphael-ccd-smn.md) plausibility policy.
4. Keep both fixed slot identities for an admitted target. An
   unavailable value affects only its own slot. Preserve independent
   successful readings, including package temperature where available.

The original producer in S4 interpreted the low 12 bits as a count at
0.125 °C per count with a −305 °C offset. If bit 11 were clear, that
calculation could yield at most −49.125 °C. Both of the positive 3900X
readings therefore imply bit 11 set at the stated addresses. With that
bit set, its contribution is +256 °C, making the result equivalent to
the explicit 11-bit conversion above. This is an inference from the
reported converted values and their producer. S4 does not provide a
captured raw SMN word.

## Quirks

- AMD documents different CCD counts among Ryzen 3000 desktop products
  and supports disabling a CCD on multi-CCD processors (S2).
  Eligibility does not use a product name or core count. A slot whose
  word reports no valid value remains unavailable (S5).
- The CCD conversion is separate from the
  [package Tctl/Tdie conversion](cpu-amd-zen-smn.md). Apply the CCD
  offset only to the two listed CCD words (S4, S5).

## Safety notes

- Both words are in the signed module's read window `0x56000`–`0x5AFFF`
  (S6). Its read path selects the address and then reads the data
  inside the caller-held PCI mutex. The interface makes no selector
  restoration guarantee. Every read selects its own address, and the
  application performs no direct PCI or SMN operation.
- A failed identity or topology gate prevents CCD access. A failed mutex
  acquisition prevents the affected request. There is no retry, address
  scan, alternate transport, mailbox command, PM table request or SMN
  data write in this path.

## Synthetic fixture acceptance

| Fixture | Expected result |
| --- | --- |
| Exact Matisse B0 signature, AM4 field `0x2`, one valid package and successful authorized words `0x0B76` and `0x0BEC` | Report fixed CCD slots 1 and 2 at 61.75 °C and 76.5 °C. These are synthetic words consistent with the rounded S4 observations. Changing the brand string does not change the reads. |
| Neighbor signature or stepping, other vendor, other or unavailable package nibble, or empty, malformed or multiple package records | Return no target and make no requests under this scope. |
| Each slot in turn fails, returns zero or all ones, or has bit 11 clear despite a plausible low-11-bit field | Keep both identities on the admitted target. Mark only the affected value unavailable and preserve the other value. A later invalid sample cannot retain a stale successful temperature. |
| Values `0x0988`, `0x0848`, `0x0D48`, `0x0847` and `0x0D49` | Retain 0 °C, −40 °C and +120 °C and reject values one increment outside each boundary. |
| Sampling inactive, module unavailable or mutex acquisition fails | Skip the affected CCD requests. A skipped request on an admitted target leaves its slot unavailable. No direct fallback. |

## Open questions

- Non-blocking for the Matisse B0 Experimental scope: the exact model's public converted Linux measurements and their pinned producer establish slot attribution, and the signed module's permitted access is independently documented.
  No accepted raw Matisse SMN capture or Windows sample through this
  module is available for a Verified claim.
- Non-blocking for the Matisse B0 Experimental scope: additional signatures and packages remain disabled and can be evaluated in a separate scope.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-09 | Proposes Implementation-ready status for the exact Matisse B0 AM4 single-package Experimental scope, pending maintainer approval. Pins the vendor package field, 3900X observations, both CCD slots and the signed Windows read interface. |
