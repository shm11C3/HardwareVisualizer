# Spec: AMD Fire Range CCD temperatures via SMN

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental CCD temperatures on Windows x86-64 for AMD family `0x1A`, model `0x44`, stepping 0 (`0x00B40F40`) with the FL1 package field 1 and exactly one OS-enumerated physical package, through the existing signed PawnIO `RyzenSMU` read interface. Two fixed slots with independent validity. The AM5 package of the same signature is the separate [Granite Ridge scope](cpu-amd-granite-ridge-ccd-smn.md). Package Tctl/Tdie has its own [specification](cpu-amd-zen-smn.md). |
| Issue phase | [#2702](https://github.com/shm11C3/HardwareVisualizer/issues/2702), follow-up to [#2515](https://github.com/shm11C3/HardwareVisualizer/issues/2515) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD, [Processor Programming Reference for AMD Family 1Ah Model 44h, Revision B0 Processors](https://www.amd.com/content/dam/amd/en/documents/processor-tech-docs/programmer-references/57896.zip), 57896 Rev 3.00, August 28, 2024, §1.7.1 Table 12 p. 32, §1.8.1 pp. 32–33, CPUID definitions pp. 67 and 85–86, register instance notation (for example p. 294), and Definitions p. 485 | Primary. The same document covers both packages of this model and revision. Table 12 and Definitions map AM5 to package field 0 and FL1 to field 1. Section 1.8.1 describes up to two CCDs and one I/O die, and every per-CCD register instance range in the document is `ccd[1:0]`. The generic package enumeration on p. 86 differs, as recorded in Open questions. The document defines no CCD thermal register. PDF `57896-B0-PUB_3.00.pdf` SHA-256 `643cae09d0bdae788ab090c0c4185168482b424e79f9feeced3f14c6de1817e5`, re-hashed and text-checked November 28, 2026. |
| S2 | AMD General Specifications for [Ryzen 9 9955HX](https://www.amd.com/en/products/processors/laptop/ryzen/9000-series/amd-ryzen-9-9955hx.html), [Ryzen 9 9955HX3D](https://www.amd.com/en/products/processors/laptop/ryzen/9000-series/amd-ryzen-9-9955hx3d.html) and [Ryzen 9 9850HX](https://www.amd.com/en/products/processors/laptop/ryzen/9000-series/amd-ryzen-9-9850hx.html), retrieved November 28, 2026 | Primary product identity. All three pages list former codename Fire Range, Zen 5, CPU socket FL1 and package die count 3. Core counts are 16, 16 and 12. The pages give no CPUID stepping. |
| S3 | [CPU-Z Validator record 0xu4y0lh](https://valid.cpuid.com/0xu4y0lh), Processor and Motherboard sections, August 31, 2026, and [XMRig benchmark 5PREPT](https://xmrig.com/benchmark/5PREPT), Summary and CPU sections | Independent Ryzen 9 9955HX identity observations. CPU-Z reports one CPU, 16 cores, 32 threads, CPUID F.4.0 / 1A.44 and socket FL1 on an ASUS G614FM. XMRig 6.26.0 on Windows x64 reports CPUID `B40F40`, family 1A, model 44 and stepping 0. Neither shows raw extended-leaf EBX or a CCD register. |
| S4 | GabboRocchia, [LenovoLegionLinux issue #624](https://github.com/johnfanv2/LenovoLegionLinux/issues/624) body and [comment of October 7, 2026](https://github.com/johnfanv2/LenovoLegionLinux/issues/624#issuecomment-6034177417), `sensors` section | First-hand CachyOS report on a Lenovo Legion Pro 7 16AFR10H (DMI product 83RU, firmware SMCN19WW) with a Ryzen 9 9955HX. Linux `k10temp-pci-00c3` reports `Tctl` 55.5 °C, `Tccd1` 47.6 °C and `Tccd2` 47.2 °C. The report gives converted values without a kernel version, raw CPUID or raw CCD words. |
| S5 | Linux Hardware Database, [Ryzen 9 9955HX device page](https://linux-hardware.org/?id=cpu:amd-26-68-0-ryzen-9-9955hx-16-core-processor), [Ryzen 9 9955HX3D device page](https://linux-hardware.org/?id=cpu:amd-26-68-0-ryzen-9-9955hx3d-16-core-processor) and [probe ce763f1389](https://linux-hardware.org/?probe=ce763f1389) summary, retrieved November 28, 2026 | Independent hardware inventory. Both device identifiers encode decimal family 26, model 68 and stepping 0 (`0x1A`/`0x44`/0). The pages list 25 computers with the 9955HX and 17 with the 9955HX3D. The probe records a Lenovo Legion Pro 7 16AFR10H 83RU with a 9955HX, firmware SMCN11WW, Fedora 42 with kernel `6.16.11-200.fc42.x86_64`, and an AMD `1022:14D8` root complex. Its CCD sensor log was not retrieved, so it supplies identity and PCI inventory only. |
| S6 | David Hows, [March 28, 2025 Linux Zen 5 desktop hwmon submission](https://lists.openwall.net/linux-kernel/2025/03/28/11), [v6.16 hwmon pull](https://lists.openwall.net/linux-kernel/2025/06/02/1307), and [Linux v6.18 AMD hardware monitor](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/k10temp.c), lines 68–74, 108–110, 172–176, 192–207, 243–249, 403–420 and 513–519 | **lead-only (copyleft)**. The family 1Ah model 40h–4Fh branch supplies the CCD address offset, stride and eight-position discovery for model 44h. It does not consult the package field. The label table names channel positions by index, so `Tccd1` and `Tccd2` are the first two positions. The discovery and value paths supply the bit-11 validity flag, encoded temperature field and conversion. |
| S7 | [PawnIO interface](pawnio-interface.md) rev 9, `RyzenSMU` and mutex sections, and [RyzenSMU API at tag 0.2.11](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/RyzenSMU.p), lines 159–171, 247–283, 614–640, 793–814 and 870–912 | Upstream interface for the signed module: the allowed `0x56000`–`0x5AFFF` read window, the read IOCTL and the caller-held mutex. The module's internal classification and fixed host-bridge route are **lead-only (copyleft)**. Lines 168–169 classify family `0x1A` model `0x44` as one codename without reading the package field, so the codename does not distinguish FL1 from AM5. |
| S8 | Microsoft Learn, [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex) and [PROCESSOR_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_relationship) | Primary Windows physical processor package enumeration contract. |
| S9 | sklynic, [ryzen_smu issue #49](https://github.com/amkillam/ryzen_smu/issues/49), June 8, 2026, CPU line and CPUID table | Independent first-hand raw CPUID capture on a Ryzen 9 9955HX. Leaf 0 returns `AuthenticAMD`, leaf 1 EAX `0x00B40F40`, leaf `0x80000001` EBX `0x10000000` (bits 31:28 equal 1), and leaves `0x80000002`–`0x80000004` decode to `AMD Ryzen 9 9955HX 16-Core Processor`. The report concerns an SMU power-table tool and contains no CCD register or temperature. |

## Detection

Apply the admission gates before any CCD SMN request. Product names,
laptop models and firmware versions are evidence identities under
[ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md).

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, CPUID leaf 0 vendor `AuthenticAMD` and CPUID.01H:EAX equal to `0x00B40F40` (family `0x1A`, model `0x44`, stepping 0) | mechanism (the scoped model and revision of the CCD map) | S1, S3, S6, S7, S9 |
| CPUID extended leaf `0x80000001` is available and EBX bits 31:28 equal `0x1` | mechanism (FL1 package. Field 0 selects the separate AM5 scope) | S1, S3, S9 |
| `GetLogicalProcessorInformationEx` with `RelationProcessorPackage` succeeds and reports exactly one valid physical package record | safety (the signed module reads through a fixed host bridge with no package selector) | S3, S7 (lead-only route), S8 |
| The existing signed `RyzenSMU` module loads and the caller holds `Global\Access_PCI` for each request | safety (authorized, serialized read transaction) | S7 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact `0x00B40F40`, FL1 package field 1, one physical package and signed `RyzenSMU` access to the two scoped slots | Ryzen 9 9955HX is the best-evidenced identity: AMD product identity (S2), decoded signature and FL1 socket (S3), raw signature and package field 1 (S9), stepping-0 inventory (S5), and two converted Linux CCD channels on a Lenovo Legion Pro 7 16AFR10H (S4). The 9955HX3D and 9850HX are AMD-listed Fire Range FL1 products (S2), and S5 records the 9955HX3D at stepping 0. S6 supplies the lead-only slot addresses and order, bit-11 validity, temperature encoding and the label-to-position mapping used to read S4. S7 pins the signed transport. Its internal classification and fixed route are lead-only. Members that satisfy the same mechanism gates are Experimental. | Experimental under [ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md). |
| AM5 package field 0 | The separate [Granite Ridge scope](cpu-amd-granite-ridge-ccd-smn.md) applies. | Unsupported by this scope. |
| Other signatures, including `0x00B40F41`, other nonzero or unavailable package fields, unknown or multiple physical packages | Outside this map. | Unsupported by this scope. |

## Register map (facts)

| Address | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- |
| SMN `0x00059B08` | 31:0 | CCD temperature slot 1 | 32-bit word | S6, corroborated by S4 |
| SMN `0x00059B0C` | 31:0 | CCD temperature slot 2 | 32-bit word | S6, corroborated by S4 |
| Each scoped word | 11 | CCD reading validity | Set means valid at channel discovery | S6 |
| Each scoped word | 10:0 | CCD temperature field | Unsigned, 0.125 °C per increment, reference offset −49 °C | S6 |

## Read procedure and decode

1. Sample on demand in the existing Windows CCD acquisition path. For
   each slot, acquire `Global\Access_PCI` with the existing bounded
   timeout, hold it across one signed `ioctl_read_smu_register` request
   and release it when that request ends (S7). If acquisition is denied
   or times out, leave only that slot unavailable and continue with the
   other slot. Make at most one read attempt per slot per sample.
2. For each slot independently, reject a failed read, zero word,
   all-ones word or clear bit 11. S6 checks bit 11 at channel discovery.
   Checking it on every sample is the project's defensive reporting gate.
3. Decode bits 10:0 × 0.125 °C − 49 °C (S6). Retain values from −40 °C
   through +120 °C inclusive, following the plausibility policy of the
   [Raphael CCD path](cpu-amd-raphael-ccd-smn.md). Package Tctl/Tdie
   decoding and offsets are separate.
4. Attribute each result to its fixed one-based CCD slot within the sole
   package. Reuse the existing CCD temperature representation and slot
   identities. On an admitted target, keep both slot identities and mark
   a failed or invalid slot's temperature unavailable. Keep any valid
   sibling, the package temperature result and its fallbacks
   independently.

## Quirks

- The two register positions are the only exposed slots. AMD describes
  at most two CCDs for this model (S1) and three dies per package for
  each listed Fire Range product (S2). The valid bit decides whether a
  slot reports for the current sample. A clear bit does not establish
  whether a CCD is physically absent, disabled or temporarily
  unreadable (S6).
- The numeric addresses and field layout match the separate [Granite
  Ridge scope](cpu-amd-granite-ridge-ccd-smn.md) because the lead keys
  the map by family and model only (S6). The two scopes stay separate
  because the package field reports a different package, and each scope
  rests on its own package-specific observations.
- The signed module reports the same codename for both packages of this
  model (S7, lead-only). Admission uses the CPUID package field, not the
  module codename.

## Safety notes

- Both addresses are in the signed module's `0x56000`–`0x5AFFF` read
  window (S7). The module selects the SMN address through its fixed
  host bridge's index register as part of each read and offers no
  selector-restoration guarantee (S7, lead-only for the internal
  route). Use only the signed read interface for the two authorized
  reads. Direct PCI/SMN access, SMN data writes, mailbox requests and
  PM table requests are outside this scope.
- An unavailable admission fact prevents CCD reads. A failed mutex
  acquisition prevents only its corresponding call. Do not scan other
  addresses, select another map after a failure, or retry within a
  sample.

## Synthetic fixture acceptance

| Case | Required result |
| --- | --- |
| Exact signature, package field 1 and one package, with words `0x0AC8` and `0x0B38` | The only CCD requests are to `0x00059B08` and `0x00059B0C`, once each. Report slot 1 at 40 °C and slot 2 at 54 °C. A changed, absent or desktop-branded CPU name and any laptop or firmware identity leave admission unchanged. |
| Same signature with package field 0 | Make no request under this profile. The Granite Ridge profile applies its own gates. |
| Same signature with another nonzero package field or an unavailable extended CPUID leaf | Make no CCD request and return no target under this profile. |
| CPUID `0x00B40F41`, another family 1Ah model, another vendor, or unknown, malformed or multiple package records | Make no CCD request and return no target under this profile. Existing approved CCD and package paths keep their own gates. |
| One slot fails, returns zero or all ones, or clears bit 11 while the other slot reports a valid value | Keep both slot identities, mark only the affected temperature unavailable and preserve the other value. Exercise both positions and the transition from valid to invalid on a later sample. |
| Words `0x0988`, `0x0848`, `0x0D48`, `0x0847` and `0x0D49` | Retain 0 °C, −40 °C and +120 °C. Mark values one increment outside either limit unavailable in their slots. |
| Module unavailable, or mutex acquisition denied or timed out for one slot while the other succeeds | With no module, make no CCD request and return no target. For a mutex failure, do not issue or retry that slot's read, keep its identity with unavailable temperature and preserve the successful sibling. Exercise both slot orders. |

## Open questions

- Non-blocking for the Experimental scope: the platform table, glossary and a raw FL1 capture agree on the admitted field value.
  S1 pp. 32 and 485 give AM5 field 0 and FL1 field 1, while the
  generic register enumeration on p. 86 reserves values 0–3 and names
  SP5 at 4. S9 shows raw field 1 on a 9955HX, and S3 independently
  reports the FL1 socket for the same product. The gate admits only
  value 1.
- Non-blocking for the Experimental scope: the CCD map rests on a model-keyed lead with a converted same-package observation, as ADR 0023 permits.
  S6 selects the map by family and model and never reads the package
  field. Its author reports testing on a desktop 9950X3D. In S6, family
  1Ah exposes CCD channels only through the model 40h–4Fh branch, so
  the `Tccd1` and `Tccd2` channels in S4 indicate that, on an FL1
  9955HX, the first two positions passed the discovery validity check
  and decoded to plausible values below `Tctl`. S4 omits the kernel
  version, so this reading of S4 assumes the reporter's kernel used the
  upstream branch; S5 shows a kernel of the release that added it
  (S6) on the same laptop model, without a CCD log. No public raw SMN
  capture of either word on an FL1 part exists, and no
  HardwareVisualizer Windows run is asserted. Verified
  status requires primary register documentation or a
  maintainer-accepted labeled SMN dump, plus corroboration of the
  Windows read path.
- Non-blocking for the Experimental scope: stepping coverage is decided by CPUID, not by product name.
  No CPUID signature is pinned for the 9850HX. A 9850HX that reports
  `0x00B40F40` and package field 1 is admitted, and any other
  signature, including the stepping-1 signature `0x00B40F41` discussed
  in the [Granite Ridge specification](cpu-amd-granite-ridge-ccd-smn.md),
  stays outside revision 1.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-28 | Initial specification. Propose the transition to Implementation-ready (rev 1) for Experimental CCD temperatures on the exact FL1 stepping-0 signature via signed Windows SMN access. Maintainer sign-off is pending. |
