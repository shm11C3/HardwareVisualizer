# Spec: AMD Storm Peak CCD temperatures via SMN

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental CCD temperatures on Windows x86-64 for AMD family `0x19`, model `0x18`, stepping 1 (`0x00A10F81`, Storm Peak: Ryzen Threadripper 7000 and Threadripper PRO 7000 WX-Series) with package field 8 and exactly one OS-enumerated physical package. The path reads twelve fixed positions, each with its own validity bit, through the existing signed PawnIO `RyzenSMU` read interface. The section [Disposition of #2702 targets](#disposition-of-2702-targets) ranks and dispositions every other deferred Zen CCD target. Package Tctl/Tdie has its own [specification](cpu-amd-zen-smn.md). |
| Issue phase | [#2702](https://github.com/shm11C3/HardwareVisualizer/issues/2702), follow-up to [#2515](https://github.com/shm11C3/HardwareVisualizer/issues/2515) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD General Specifications for [Ryzen Threadripper PRO 7975WX](https://www.amd.com/en/support/downloads/drivers.html/processors/ryzen-threadripper-pro/amd-ryzen-threadripper-pro-7000-wx-series/amd-ryzen-threadripper-pro-7975wx.html) and [Ryzen Threadripper PRO 7995WX](https://www.amd.com/en/support/downloads/drivers.html/processors/ryzen-threadripper-pro/amd-ryzen-threadripper-pro-7000-wx-series/amd-ryzen-threadripper-pro-7995wx.html), retrieved November 29, 2026 | Primary product identity. Both pages list former codename Storm Peak, Zen 4 and CPU socket sTR5. The 7975WX page lists 32 cores, package die count 5, one CCD size and one I/O die size. The 7995WX page lists 96 cores and package die count 13. The pages give no CPUID value. |
| S2 | InstLatx64 CPUID records: [Threadripper PRO 7995WX](https://github.com/InstLatx64/InstLatx64/blob/d993c2d5e7d3cbfa9f9aa659954c3f438d7258d7/AuthenticAMD/AuthenticAMD0A10F81_K19_StormPeak_01_CPUID.txt), Git blob `1d1c1d525e7d31a347b7d962d01a4bfad01cb768`, lines 6, 11, 17, 30, 48–49 and 77; [Threadripper PRO 7975WX](https://github.com/InstLatx64/InstLatx64/blob/7938af8a61da1274eba38390a348ff5267be4261/AuthenticAMD/AuthenticAMD0A10F81_K19_StormPeak_02_CPUID.txt), Git blob `be912c1ce33815d3d619f8f5485e9f6fa5c4192c`, lines 5, 10, 16, 29, 47–48 and 76–78; and [Threadripper 7960X](https://github.com/InstLatx64/InstLatx64/blob/7938af8a61da1274eba38390a348ff5267be4261/AuthenticAMD/AuthenticAMD0A10F81_K19_StormPeak_03_CPUID.txt), Git blob `25b67e4330fa707a0d76765fcfc8a05dd3907b0a`, lines 5–6 and 34–37 | Independent raw observations. All three records show vendor `AuthenticAMD`, leaf 1 EAX `0x00A10F81` and leaf `0x80000001` EBX `0x80000000` (bits 31:28 equal 8). The 7995WX record comes from Windows 11 on an AMD reference board for Storm Peak, and every logical-processor record names package 0. The 7975WX record comes from Windows 10 on an ASUS Pro WS WRX90E-SAGE SE. The records identify the CPU and contain no CCD thermal reads. |
| S3 | Linux Hardware Database [probe cfba81369c](https://linux-hardware.org/?probe=cfba81369c), January 5, 2024: Host section, [cpuid log](https://linux-hardware.org/?probe=cfba81369c&log=cpuid) and [lspci log](https://linux-hardware.org/?probe=cfba81369c&log=lspci) | Independent inventory of a Threadripper 7980X on an ASUS Pro WS TRX50-SAGE WIFI under NixOS 24.05 with kernel `6.7.0-rc7`. The cpuid log decodes family 19h, model 18h, stepping 1, package type 8, and the brand string `AMD Ryzen Threadripper 7980X 64-Cores`. The lspci log lists an AMD host bridge `1022:14a4` at `00:00.0`. The probe has no sensor log. |
| S4 | Apaxhren, AMD Community thread [Threadripper PRO temps normal or concerning?](https://web.archive.org/web/20241102222343/https://community.amd.com/t5/pc-processors/threadripper-pro-temps-normal-or-concerning/m-p/705389), original post of August 26, 2024 and the author's reply of August 27, 2024, Internet Archive snapshot of November 2, 2024 | First-hand report from an HP workstation with a Threadripper PRO 7975WX under Ubuntu 22.04 LTS. Linux `k10temp-pci-00c3` reports `Tctl` 95.2 °C, `Tccd1` 94.7 °C, `Tccd2` 95.3 °C, `Tccd3` 94.4 °C and `Tccd4` 92.8 °C under partial load. The report gives converted values without a kernel version, raw CPUID or raw CCD words. The live AMD Community page now redirects. |
| S5 | AMD's Babu Moger, [Linux commit 8bb050cd5cf4](https://github.com/torvalds/linux/commit/8bb050cd5cf494f3d0cb45a6b54a476af09edb8d), November 24, 2021, first released in v5.17, and Linux v6.18, commit `7d0a66e4bb9081d75c82ec4957c50034cb0ea449`, [AMD hardware monitor](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/hwmon/k10temp.c), lines 69–74, 160–176, 244–250, 399–421, 467–487, 489–511 (model band at lines 507–510) and 513–519, and the [v6.1 AMD hardware monitor](https://github.com/torvalds/linux/blob/v6.1/drivers/hwmon/k10temp.c), lines 444 and 452–453 | **lead-only (copyleft)**. The AMD-authored commit message states that family 19h models 10h–1Fh and A0h–AFh support up to 12 CCDs. In v6.18 the family 19h model 10h–1Fh band supplies the CCD address offset, the four-byte stride and twelve-position discovery. Model 18h is inside that band. The discovery and value paths supply the bit-11 validity flag, the encoded temperature field and the conversion. The label table names channels by position, so `Tccd1` through `Tccd4` are the first four positions. |
| S6 | [PawnIO interface](pawnio-interface.md) rev 9, `RyzenSMU` and mutex sections, and [RyzenSMU API at tag 0.2.11](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/RyzenSMU.p), lines 141–142, 229–230, 247–283, 614–628, 793–814 and 864–914 | Upstream interface for the signed module: the allowed `0x56000`–`0x5AFFF` read window, the read IOCTL and the caller-held mutex. The module's internal classification and fixed host-bridge route are **lead-only (copyleft)**. Lines 141–142 classify family 19h model 18h as Storm Peak without reading the package field, and the load path at lines 864–914 admits that classification. Reads go through configuration offsets `0xC4`/`0xC8` of bus 0, device 0, function 0. |
| S7 | Microsoft Learn, [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex), [PROCESSOR_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_relationship) and [GetActiveProcessorCount](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getactiveprocessorcount) | Primary Windows contract for physical package enumeration, per-group package masks and the active processor count across groups. |
| S8 | AMD, [Revision Guide for AMD Family 19h Models 10h–1Fh Processors](https://docs.amd.com/api/khub/documents/LZ~6p62H~zRDhkNiPAE9NQ/content#page=8), 57095 rev 1.05, November 2025, Table 2 p. 8 and Table 7 p. 14 | Primary. The guide covers the SP5 EPYC 9004 revisions `0x00A10F11` and `0x00A10F12` only. It lists no Storm Peak revision and no sTR5 package column. PDF SHA-256 `c279f4ee838329ea607a171988ca699f97be4dbc9f8ac11918ec37207c18326c`, checked November 29, 2026. Scope-boundary evidence only. |
| S9 | c.s, Framework Community thread [Missing per-core CPU temperatures from k10temp?](https://community.frame.work/t/missing-per-core-cpu-temperatures-from-k10temp/55833), first post of August 8, 2024 and replies through August 12, 2024 | First-hand report on a Framework Laptop 16 with a Ryzen 7840HS (decimal family 25, model 116, that is model `0x74`, Phoenix) under Debian with kernel 6.9.10. The AMD hardware monitor exposes only `Tctl`. A second owner reports the same. Disposition evidence only. |
| S10 | kalle123, debianforum.de thread [AMD Ryzen und lm-sensors](https://debianforum.de/forum/viewtopic.php?t=187793), first post of September 9, 2023, and CreatingNull, [cockpit issue #18098 comment of July 1, 2023](https://github.com/cockpit-project/cockpit/issues/18098) | First-hand reports. A Ryzen 7 5700G (Cezanne) under Debian kernel `6.1.0-11-amd64` and a Ryzen 3 1200 (kernel not stated) each show only `Tctl` from the AMD hardware monitor. A separate owner reports only `Tctl` on a Ryzen 5 5600G. Disposition evidence only. |
| S11 | Rong Zhang, [Linux commit 451b1c19dc7c](https://github.com/torvalds/linux/commit/451b1c19dc7cbbad194a6e717b6a732c443d7dd5), September 10, 2026, reviewed by Mario Limonciello (AMD), and its [review thread mirror](https://ratatoskr.run/lkml/2026/09/17548150/t), September 10–11, 2026 | The commit message identifies family 1Ah models 20h–2Fh as single-CCD mobile processors, with model 24h Strix Point. It records nine channels between `Tccd4` and `Tccd16` reading 22.0 °C or 148.4 °C to 149.2 °C beside a `Tctl` of 54.1 °C on such a part when the server-band map of S12 was applied. In the thread, the author states that mobile processors do not support CCD temperatures. The hwmon maintainer asks whether this is documented, and no document is cited. The range change itself is **lead-only (copyleft)**. Disposition evidence only. |
| S12 | Gabriel Ford, [Linux commit 8440d5aca227](https://github.com/torvalds/linux/commit/8440d5aca227d623801b5f8a2b9e3d86b7daa8bf), August 13, 2026 | **lead-only (copyleft)**. Adds a family 1Ah server-band CCD map with a different offset and up to sixteen positions. The commit message reports testing only on an EPYC 9555P. S11 narrows the band to models 00h–1Fh. Disposition evidence only. |
| S13 | Level1Techs Forums thread [Threadripper 9960x pegged at 95 celsius](https://forum.level1techs.com/t/threadripper-9960x-pegged-at-95-celsius/245923), posts 9 and 17, February 13–15, 2026 | First-hand report that a Threadripper 9960X (Shimada Peak, family 1Ah model 08h) under kernel 6.18.9 shows only `Tctl`. That kernel predates S12. Disposition evidence only. |

## Detection

Apply the admission gates before any CCD SMN request. Product names,
workstation and board identities are evidence identities under
[ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md).

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, CPUID leaf 0 vendor `AuthenticAMD` and CPUID.01H:EAX exactly `0x00A10F81` (family `0x19`, model `0x18`, stepping 1) | mechanism (selects the observed Storm Peak die and revision of the twelve-position map) | S2, S3, S5, S6 |
| CPUID extended leaf `0x80000001` is available and EBX bits 31:28 equal `0x8` | mechanism (admits only the package field observed on Storm Peak) | S2, S3 |
| Complete Windows physical-package enumeration with `RelationProcessorPackage` reports exactly one nonempty package whose valid processor group masks cover all active logical processors | safety (the signed module reads through one fixed host-bridge endpoint with no package selector) | S2, S6 (lead-only route), S7 |
| The existing signed `RyzenSMU` module loads and the caller holds `Global\Access_PCI` for each request | safety (authorized, serialized read transaction) | S6 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact `0x00A10F81`, package field 8, one complete physical package and signed `RyzenSMU` access to the twelve scoped positions | Threadripper PRO 7975WX is the best-evidenced identity: AMD product identity and die count (S1), raw signature and package field (S2), and four converted Linux CCD channels on an HP workstation (S4). The 7995WX has AMD identity (S1) and a raw single-package record across 192 logical processors (S2). The 7960X has a raw record (S2). The 7980X has a decoded identity, package type and host-bridge inventory (S3). S5 supplies the lead-only twelve addresses and their order, bit-11 validity, temperature encoding and the label-to-position mapping used to read S4. S6 pins the signed transport. Its internal classification and fixed route are lead-only. Members that satisfy the same mechanism gates are Experimental. | Experimental under [ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md). |
| Genoa B1 `0x00A10F11` with SP5 package field 4 | The separate [Genoa B1 scope](cpu-amd-genoa-b1-ccd-smn.md) applies. | Unsupported by this scope. |
| Other signatures, including other model `0x18` steppings, other package fields, an unavailable extended leaf, and unknown, incomplete or multiple physical packages | Outside this map. | Unsupported by this scope. |

## Register map (facts)

| Address | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- |
| SMN `0x00059B00`, `0x00059B04`, `0x00059B08`, `0x00059B0C`, `0x00059B10`, `0x00059B14`, `0x00059B18`, `0x00059B1C`, `0x00059B20`, `0x00059B24`, `0x00059B28`, `0x00059B2C` in order | 31:0 | CCD temperature positions 1–12, one 32-bit word each | 32-bit word | S5 (lead-only map), S4 (positions 1–4 observed as converted channels), S6 (read width) |
| Each scoped word | 11 | CCD reading validity | Set means valid at channel discovery | S5 |
| Each scoped word | 10:0 | CCD temperature field | Unsigned, 0.125 °C per increment, reference offset −49 °C | S5 |

## Read procedure and decode

1. Sample on demand in the existing Windows CCD acquisition path. For
   each position, acquire `Global\Access_PCI` with the existing bounded
   timeout, hold it across one signed `ioctl_read_smu_register` request
   and release it when that request ends (S6). If acquisition is denied
   or times out, leave only that position unavailable and continue with
   the others. Make at most one read attempt per position per sample.
2. For each position independently, reject a failed read, zero word,
   all-ones word or clear bit 11. S5 checks bit 11 at channel discovery.
   Checking it on every response is the project's defensive reporting
   gate.
3. Decode bits 10:0 × 0.125 °C − 49 °C (S5). Retain values from −40 °C
   through +120 °C inclusive, following the plausibility policy of the
   [Raphael CCD path](cpu-amd-raphael-ccd-smn.md). Package Tctl/Tdie
   decoding and offsets are separate.
4. Attribute each result to its fixed one-based CCD position within the
   sole package, which is package index 0. Reuse the existing CCD
   temperature representation and slot identities. On an admitted
   target, keep all twelve position identities and mark a failed or
   invalid position's temperature unavailable. Keep every valid sibling,
   the package temperature result and its fallbacks independently. The
   position identity needs no OS core, NUMA node or processor group
   mapping.

## Quirks

- The twelve register positions are the only exposed slots. AMD lists
  package die counts of 5 for the 7975WX and 13 for the 7995WX, each
  with one I/O die size listed (S1). The lead discovers twelve
  positions for this model band (S5). The 7975WX report shows the first
  four positions (S4). The valid bit decides whether a position reports
  for the current sample. A clear bit does not establish whether a CCD
  is physically absent, disabled or temporarily unreadable, and the
  valid positions on a part with fewer CCDs can leave holes.
- The numeric addresses and field layout match the separate [Genoa B1
  scope](cpu-amd-genoa-b1-ccd-smn.md) because the lead keys the map by
  the family 19h model 10h–1Fh band (S5). The scopes stay separate
  because the signature and package field differ, and each scope rests
  on its own observations.
- A set validity bit alone does not prove that a word belongs to the
  admitted die. On a Strix Point part, a map from another model band
  exposed channels whose words had the validity bit set at discovery and
  decoded to 148.4 °C to 149.2 °C or 22.0 °C beside a 54.1 °C `Tctl`
  (S11). The exact signature gate and the plausibility bounds apply to
  every position.
- One physical package can span several Windows processor groups. The
  7995WX record has 192 logical processors in package 0 (S2, S7). The
  package gate counts physical packages, not groups.
- The host bridge at bus 0, device 0, function 0 is AMD `1022:14a4` on
  the observed 7980X system (S3). The existing package temperature path
  reads SMN `0x00059800` through the same signed module on family `0x19`
  ([package temperature specification](cpu-amd-zen-smn.md)).

## Safety notes

- All twelve addresses are in the signed module's `0x56000`–`0x5AFFF`
  read window (S6). The module selects the SMN address through its
  fixed host bridge's index register as part of each read and offers no
  selector-restoration guarantee (S6, lead-only for the internal route).
  Use only the signed read interface for the twelve authorized reads.
  Direct PCI/SMN access, SMN data writes, mailbox requests and PM table
  requests are outside this scope.
- An unavailable admission fact prevents CCD reads. A failed mutex
  acquisition prevents only its corresponding call. Do not scan other
  addresses, select another map after a failure, or retry within a
  sample.

## Synthetic fixture acceptance

| Case | Required result |
| --- | --- |
| Exact signature, package field 8 and one package in one processor group, with words `0x0AC8`, `0x0B38`, `0x0B00` and `0x0AC4` at positions 1–4 and zero at positions 5–12 | Request exactly the twelve approved addresses, once each and in order. Report 40 °C, 54 °C, 47 °C and 39.5 °C at positions 1–4 and keep positions 5–12 unavailable. A changed, absent, PRO or non-PRO CPU name and any workstation or board identity leave admission unchanged. |
| One complete package spanning three Windows processor groups with 192 active logical processors, with valid values at positions 1, 9 and 12 only | Preserve the fixed address order and the three values at their own positions on package index 0. Keep every other position unavailable, including holes before valid positions. Group count, NUMA node count and core count do not change admission. |
| Genoa B1 `0x00A10F11` with package field 4 | Make no request under this profile. The Genoa B1 profile applies its own gates. |
| `0x00A10F81` with package field 4 or another value other than 8, or an unavailable extended CPUID leaf | Make no CCD request and return no target under this profile. |
| Another model `0x18` stepping such as `0x00A10F80` or `0x00A10F82`, family `0x19` model `0x08`, family `0x1A` model `0x08`, another vendor, or unknown, malformed, incomplete or multiple package records | Make no CCD request and return no target under this profile. Existing approved CCD and package paths keep their own gates. |
| One position fails, returns zero or all ones, or clears bit 11 while other positions report valid values | Keep all position identities, mark only the affected temperature unavailable and preserve the others. Exercise the first, a middle and the last position, and the transition from valid to invalid on a later sample. |
| Words `0x0988`, `0x0848`, `0x0D48`, `0x0847`, `0x0D49` and `0x0E2D` | Retain 0 °C, −40 °C and +120 °C. Mark values one increment outside either limit and the bit-11-set word decoding to 148.625 °C unavailable in their positions. |
| Module unavailable, or mutex acquisition denied or timed out for one position while others succeed | With no module, make no CCD request and return no target. For a mutex failure, do not issue or retry that position's read, keep its identity with unavailable temperature and preserve the successful siblings. |

## Disposition of #2702 targets

The ranking orders targets by the project's estimate of installed base
among HardwareVisualizer's Windows desktop and laptop users, following
the priority rubric of the sensor coverage loop. It is a planning order,
not a hardware fact. The disposition and its evidence are facts.

| Rank | Target (CPUID family/model) | Disposition | Evidence | Evidence that would unblock it |
| --- | --- | --- | --- | --- |
| 1 | Zen 3, Zen 3+ and Zen 4 APUs, mobile and AM4/AM5 desktop: Cezanne and Barcelo (`0x19`/`0x50`), Rembrandt (`0x19`/`0x44`), Phoenix, Phoenix 2 and Hawk Point (`0x19`/`0x74`, `0x75`, `0x78`, `0x7C`) | Unsupported | The signed module recognizes these models (S6, lines 145–156, lead-only classification). The lead assigns CCD maps to their model bands (S5, lines 489–506). On a 5700G with kernel `6.1.0-11` and a 7840HS with kernel 6.9.10, the lead's discovery exposed no CCD channel, so no position had bit 11 set at probe (S9, S10). Upstream v6.1 already assigns maps to both model bands (S5). No positive Rembrandt or Hawk Point observation was found. S11 states that mobile processors do not support CCD temperatures, and cites no document. | A primary register document, or a maintainer-accepted labeled SMN dump from an identified part showing a valid word at a stated address. |
| 2 | Zen 5 mobile: Strix Point (`0x1A`/`0x20`, `0x24`), Krackan Point (`0x1A`/`0x60`, `0x68`), Strix Halo (`0x1A`/`0x70`) | Unsupported | No lead assigns a CCD map to these models (S5, lines 513–519; S11). Under ADR 0031 rule 3 the map is not guessed. On Strix Point, another band's map exposed channels whose words had the validity bit set at discovery and decoded to implausible values (S11). | Same as rank 1. |
| 3 | Zen 2 APUs: Renoir (`0x17`/`0x60`), Lucienne (`0x17`/`0x68`), Mendocino (`0x17`/`0xA0`), Van Gogh (`0x17`/`0x90`, `0x91`) | Unsupported | The lead assigns maps to models `0x60`, `0x68` and `0xA0` (S5, lines 476–487) and none to Van Gogh. No positive observation was found. | A same-model observation of valid CCD channels tied to stated addresses, or the rank 1 evidence. |
| 4 | Desktop steppings outside the merged scopes: Granite Ridge and Fire Range other than `0x00B40F40`, including Granite Ridge B1 `0x00B40F41` | Deferred | Identity evidence for B1 exists but no B1 package-field or CCD observation does ([Granite Ridge specification](cpu-amd-granite-ridge-ccd-smn.md), Open questions). No other Raphael, Dragon Range, Matisse, Castle Peak or Fire Range stepping identity was found. Vermeer already admits every model `0x21` stepping ([Vermeer specification](cpu-amd-vermeer-ccd-smn.md)). | A raw package-field capture and a same-stepping CCD channel observation, or a maintainer decision to key these scopes by family and model as Vermeer revision 5 did. |
| 5 | Zen and Zen+ desktop and APUs: Summit Ridge, Pinnacle Ridge, Raven Ridge and Picasso (`0x17`/`0x01`, `0x08`, `0x11`, `0x18`) | Deferred | The lead discovers four positions (S5, lines 467–475). A Ryzen 3 1200 report shows only `Tctl` without a kernel version (S10). No positive desktop observation was found. | A same-model observation of a valid CCD channel. |
| 6 | Threadripper 7000 and PRO 7000 WX-Series, Storm Peak (`0x19`/`0x18`) | Experimental, this document | See Detection. | Graduation needs primary register documentation or a maintainer-accepted labeled SMN dump. |
| 7 | Threadripper 9000 and PRO 9000 WX-Series, Shimada Peak (`0x1A`/`0x08`) | Deferred | The signed module recognizes the model (S6, lines 160–175, lead-only). The only map whose band contains it is the server-band lead tested on an EPYC 9555P (S12). A 9960X showed only `Tctl` on a kernel that predates that lead (S13). | A Shimada Peak observation of valid channels tied to the S12 addresses and the observed package field. |
| 8 | Threadripper PRO 5000 WX-Series, Chagall (`0x19`/`0x08`), and Threadripper 1000/2000 (`0x17`/`0x01`, `0x08` with package field 7) | Deferred | Chagall has a lead map (S5, lines 491–497) but no collected observation. The first two Threadripper generations have a four-position lead map (S5, lines 467–475), the module classifies them by package field 7 (S6, lines 101–111, lead-only), and no die-to-position attribution evidence exists. | A same-model observation of valid channels and the package field. |
| 9 | EPYC with one physical package other than Genoa B1: Genoa B2 and Genoa-X, Bergamo and Siena (`0x19`/`0xA0`), Milan (`0x19`/`0x00`, `0x01`), Rome (`0x17`/`0x31`), Naples (`0x17`/`0x01`), Turin and Turin Dense (`0x1A`/`0x02`, `0x11`) | Deferred | Each needs a model-specific map and its own observation. The Turin map is a 2026 lead with one tested part (S12). The [Genoa B1 scope](cpu-amd-genoa-b1-ccd-smn.md) is merged. | Per model: the map, the package field and a single-package observation. |
| 10 | Multiple physical packages on any EPYC model | Unsupported | The signed module reads through one fixed host bridge with no package selector (S6, lead-only route). | A signed interface that selects the physical package, with documented package-to-endpoint attribution. |
| 11 | Linux Vermeer and other Linux CCD endpoints | Unchanged by this revision | Tracked by [#2668](https://github.com/shm11C3/HardwareVisualizer/issues/2668) and the draft [Vermeer Linux specification](cpu-amd-vermeer-linux-k10temp.md). S11 shows that a channel can be exposed for words whose validity bit was set at discovery while their values are implausible. | The per-read validity, endpoint ownership and package attribution listed in #2668. |
| 12 | Fire Range candidate and Genoa B1 candidate named in #2702 | Done | [Fire Range specification](cpu-amd-fire-range-ccd-smn.md) rev 1 and [Genoa B1 specification](cpu-amd-genoa-b1-ccd-smn.md) rev 1 are Implementation-ready. | Not applicable. |

## Open questions

- Non-blocking for the Experimental scope: the package-field gate admits only the value observed on four Storm Peak products.
  S2 shows raw field 8 on the 7995WX, 7975WX and 7960X, and S3
  independently decodes package type 8 on a 7980X. AMD's public
  revision guide for this model range covers SP5 only (S8), and no
  public AMD document defining the sTR5 field value was found. The
  signed module does not read the field for this model (S6). Any other
  value stays outside revision 1.
- Non-blocking for the Experimental scope: the CCD map rests on a model-band lead with a converted same-die observation, as ADR 0023 permits.
  S5 selects the map by the family 19h model 10h–1Fh band, and its
  AMD-authored commit states the twelve-CCD capacity for that band. In
  S5, the band's only CCD channels come from that map, so the `Tccd1`
  to `Tccd4` channels in S4 indicate that the first four positions
  passed discovery and decoded to values plausible against `Tctl` on a
  7975WX. S4 omits the kernel version, so this reading assumes the
  reporter's kernel used the upstream band. Positions 5–12 are not
  observed on any admitted part. No public raw SMN capture of any
  position on Storm Peak exists, and no HardwareVisualizer Windows run
  is asserted. Verified status requires primary register documentation
  or a maintainer-accepted labeled SMN dump, plus corroboration of the
  Windows read path.
- Non-blocking for the Experimental scope: stepping coverage is decided by CPUID, not by product name.
  Every observed Storm Peak record reports stepping 1 (S2, S3). Another
  stepping of model `0x18` stays outside revision 1.
- Non-blocking for the Experimental scope: the twelve positions are fixed register positions, not a CCD-to-core or CCD-to-group mapping.
  The application keeps position identities only. Holes are expected on
  parts with fewer than twelve CCDs. Attribution to cores, NUMA nodes or
  processor groups is outside this scope.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-29 | Initial specification. Propose the transition to Implementation-ready (rev 1) for Experimental Storm Peak CCD temperatures on the exact `0x00A10F81` signature with package field 8 and one physical package, via signed Windows SMN access to twelve fixed positions. Rank and disposition every #2702 target. Maintainer sign-off is pending. |
