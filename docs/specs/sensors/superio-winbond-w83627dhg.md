# Spec: Winbond W83627DHG / W83627DHG-P / W83527HG hardware monitor (legacy W836xx temperatures and fans)

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Windows PawnIO `LpcIO` read-only motherboard temperatures and fan RPM on the legacy Winbond/Nuvoton W836xx hardware monitor whose readings sit at bank 0 `0x27` (8-bit SYSTIN), bank 1 and bank 2 `0x50`/`0x51` (9-bit CPUTIN and AUXTIN) and 8-bit fan counts with a 3-bit divisor. Enabled, all Experimental and all backed by vendor datasheets: raw chip IDs `0xA020`–`0xA02F` (W83627DHG: SYSTIN, CPUTIN, AUXTIN and five fan inputs) and raw chip IDs `0xB070`–`0xB07F` (shared by W83627DHG-P and W83527HG: SYSTIN, CPUTIN and the three fan inputs both parts have). Recorded but disabled: AUXTIN, AUXFANIN0 and AUXFANIN1 on `0xB07x`, every other W836xx ID including the #2523 targets W83627HF, W83627THF, W83627EHF, W83667HG, W83667HG-B and W83687THF (see the disposition table). Excludes every write other than the read-transaction plumbing listed in Safety notes, voltages, PECI and SST readings, the bank 6 speed registers, fan/PWM control, limits, alarms, beeps, and the Linux and ACPI paths. |
| Issue phase | #2523 (epic #2510) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Nuvoton, *W83627DHG-P / W83627DHG-PT Nuvoton LPC I/O* datasheet, Version 1.94, publication release date July 09, 2009. Text copy hosted by The Retro Web: `https://theretroweb.com/chip/documentation/w83627dhg-p-w83627dhg-pt-67c306df1a254061400772.pdf`, fetched 2026-11-29, SHA-256 `79db88090b858c9a3eeb15c00f2c1ec39a69802fc4ae94b6260e0c2faea33895`. The same version is published by Nuvoton as a scanned copy: `https://www.nuvoton.com/resource-files/DA00-WW83627DHG-P.pdf`, fetched 2026-11-29, SHA-256 `21bd78fd19d3f44cfc61dae668f870a982b4cec81c6e974fa3d0bb542917ea16`; its printed page numbers match the text copy. Pins (printed pages): HM pins (SYSTIN 104, CPUTIN 103, AUXTIN 102, AUXFANIN1 / SI 58) §5.7 p. 16, fan pins 111–113 and CPUFANIN1 / GP21 119 p. 17; configuration protocol §7.1.1–7.1.3 p. 29; HM LPC interface §8.2.1 p. 32; temperature format Table 8-1 and reading locations §8.3.3 p. 36; PECI and TBase pp. 40–41; fan count formula, divisor registers and Table 8-4 §8.6.1 p. 44; SMI# reset by reading the interrupt status registers §8.7.1 pp. 58–62; OVT# modes and Figure 8-23 §8.7.2 pp. 62–63; address and data port §9, §9.1–9.2 p. 66; OVT# configuration `0x18` §9.27 p. 77; value RAM §9.29 pp. 78–79; configuration `0x40` §9.30 p. 79; interrupt status `0x41`/`0x42` §9.31–9.32 p. 80; interrupt status 4 `0x45` §9.35 p. 82; fan divisor I `0x47` §9.37 p. 83; temperature source select `0x49` §9.39 pp. 84–85; fan divisor II `0x4B` §9.41 p. 86; SMI#/OVT# control `0x4C` §9.42 pp. 86–87; FAN IN/OUT control `0x4D` §9.43 pp. 87–88; bank select `0x4E` §9.44 p. 88; vendor ID `0x4F` §9.45 p. 89; HM chip ID `0x58` and fan divisor selection `0x59` §9.49–9.50 p. 92; `0x5D` §9.52 pp. 92–93; temperature selection `0x7C` and `0x7D` §9.83–9.84 p. 105; bank 1 `0x50`–`0x52` §9.85–9.87 pp. 106–107; bank 2 `0x50`–`0x52` §9.99–9.101 pp. 111–112; bank 4 interrupt status 3 `0x50` §9.106 p. 114; bank 4 offsets `0x54`–`0x56` §9.110–9.112 pp. 115–116; bank 5 value RAM 2 §9.118 p. 119; bank 6 speed registers §9.119–9.128 pp. 119–122; `CR20`–`CR23` p. 192; `CR24` p. 193; `CR26` p. 194; `CR29` p. 195; logical device B `CR30`, `CR60`/`CR61` §21.12 p. 227. | Primary (vendor datasheet; both copies pinned by hash). |
| S2 | Winbond, *W83627DHG Winbond LPC I/O* datasheet, Version 1.4 ("for UBC, UBE and UBF version"; cover date April 10, 2007, footer publication release date Aug 22, 2007). Copy hosted by The Retro Web: `https://theretroweb.com/chip/documentation/w83627dhg-63f0d2c5e4232931525907.pdf`, fetched 2026-11-29, SHA-256 `e9dd1473e87fbaa908f6fd546cdbb4e82a82e0bed197a92eacb85da9b0a698ca`. Pins (printed pages): HM pins (AUXTIN 102, CPUTIN 103, SYSTIN 104, AUXFANIN1 58, fans 111–113, CPUFANIN1 119) pp. 15–16; configuration protocol pp. 25–26; HM LPC interface p. 29; temperature format §7.3.3 pp. 32–33; PECI source of CPUTIN and AUXTIN p. 37; fan count formula and divisor registers p. 39; SMI# reset §7.7.1 pp. 50–52; OVT# modes §7.7.2 p. 53; address and data port §8.1–8.2 p. 56; `0x18` §8.27 p. 71; value RAM §8.29 pp. 71–72; `0x40` §8.30 p. 73; `0x41` §8.31 p. 73; `0x42` §8.32 p. 74; `0x45` (reserved) §8.35 p. 75; `0x47` §8.37 p. 76; `0x49` §8.39 p. 77; `0x4B` §8.41 p. 79; `0x4C` §8.42 p. 79; `0x4D` §8.43 p. 80; `0x4E` §8.44 p. 81; `0x4F` §8.45 pp. 81–82; `0x58` §8.49 p. 84; `0x59` §8.50 pp. 84–85; `0x5D` §8.52 p. 85; bank 1 `0x50`–`0x52` §8.70–8.72 pp. 94–96; bank 2 `0x50`–`0x52` §8.77–8.79 pp. 98–99; bank 4 `0x50` §8.84 p. 101; bank 4 offsets §8.88–8.90 pp. 103–104; bank 5 value RAM 2 §8.96 p. 107; `CR20`–`CR23` p. 180; `CR24` p. 181; `CR26` p. 182; `CR29` p. 183; logical device B §20.12 p. 210. | Primary (vendor datasheet; third-party-hosted copy pinned by hash). |
| S3 | Nuvoton, *W83527HG Nuvoton LPC I/O* datasheet, Version 1.5, December 25, 2009. Copy hosted by The Retro Web: `https://theretroweb.com/chip/documentation/da00-w83527hga1-644e98d187293470986888.pdf`, fetched 2026-11-29, SHA-256 `4ae4828f6d9145c4850f3666b9197720144df661564696094c4ed87080ff84f3`. Pins (printed pages): "Two thermal inputs" and "Three fan-speed monitoring inputs" p. 2; fan pins CPUFANIN0 44, SYSFANIN 45, CPUFANIN1 / GP21 1 p. 8; configuration protocol p. 18; HM LPC interface p. 21; temperature format pp. 23–24; fan count formula and divisor registers p. 27; SMI# reset §8.6.1 pp. 40–43; OVT# modes §8.6.2 p. 44; address and data port p. 46; `0x18` §9.20 p. 54; value RAM (with `0x2A` reserved) §9.22 pp. 54–55; `0x40` §9.23 p. 55; `0x41`/`0x42` pp. 56–57; `0x45` p. 58; `0x47` §9.30 p. 59; `0x49` §9.32 p. 60; `0x4C` §9.35 p. 62; `0x4D` §9.36 pp. 62–63; `0x4E` and `0x4F` §9.37–9.38 p. 63; `0x58`, `0x59`, `0x5D` §9.40–9.43 pp. 64–65; `0x7C` §9.71 p. 75; bank 1 `0x50`–`0x52` §9.73–9.75 pp. 76–77; bank 4 `0x50` §9.87 p. 81; bank 4 offsets §9.90–9.91 p. 82; `CR20`–`CR23` p. 108; `CR26` and `CR29` pp. 109–110; logical device B §16.6 p. 128. | Primary for the W83527HG, which reports the same raw chip ID as S1. |
| S4 | Nuvoton, *W83667HG-A Nuvoton LPC I/O* datasheet, Revision 1.2, January 8, 2010. `https://15h.org/images/3/34/W83667hg-a-datasheet-v1-2-67dd6c3d7aef5611225428.pdf` (the same file name is hosted by The Retro Web), fetched 2026-11-29, SHA-256 `65223bdca3d37fc504f99f0354ab6808e3aa9a8c872091ca33972554170b7fc1`. Pins: "SYSFAN temperature source reading" at `0x27` with its low bit at `0x7C` p. 85; report select `0x7D` (read/write) and report data `0x7E` p. 106; `CR20` = `0xB3`, `CR21` = `5Xh` p. 205; revision history entry 8 sets the attribute of bank 0 `0x41`, `0x42`, `0x45` to "Read Clear" p. 306. | Primary for the W83667HG-A / W83667HG-B disposition only. |
| S5 | Winbond, *W83627HF/F* data sheet, version 0.53 (07/26/1999). `https://theretroweb.com/chip/documentation/w83627hf-6b02f7396d4cf195112705.pdf`, fetched 2026-11-29, SHA-256 `5bf5c3c54d96c4f1656f6010e110701735247ac99d9a33de080a443d51bfd715`. `CR20` = `0x52`, `CR21` = `0x1y` ("y is version no") §13.1 p. 136; HM `0x58` returns `0x21` §11.7.23 p. 118. | Primary for the W83627HF disposition only. The Retro Web also lists a revision 2.27 (2007), not analysed. |
| S6 | Winbond, *W83627THF / W83627THG Winbond LPC I/O* datasheet, Revision 1.22, September 26, 2006. `https://theretroweb.com/chip/documentation/w83627thf-ver-1-22-67e7ee3e71477420549573.pdf`, fetched 2026-11-29, SHA-256 `7245c301b8450a0f273a4a1b65b43eb1f3c817b1dce792144df60c268cac20fe`. `CR20` = `0x82`, `CR21` = `0x83` §7.1 p. 76; revision history: "Chip ID is changed from 0x8283h to 0x8285h"; HM `0x58` returns `0x90` §5.8.21 p. 46. | Primary for the W83627THF disposition only. |
| S7 | Winbond, *W83627EHF/EF, W83627EHG/EG Winbond LPC I/O* datasheet, Revision 1.3, November 16, 2006. `https://theretroweb.com/chip/documentation/w83627e-h-f-e-h-g-6419fd57bda58954244818.pdf`, fetched 2026-11-29, SHA-256 `36cacd34a5011f1e739170ef9b4e0d915bcb873c2a66e5e43884c0b868116e1a`. `CR20` = `0x88`, `CR21` = `6Xh` ("X for IC version") §7.1 p. 92; HM `0x58` returns `0xA1` §6.8.51 p. 71. | Primary for the W83627EHF disposition only. |
| S8 | [`superio-access.md`](superio-access.md) revision 5 | Clean-room source for the Winbond/Nuvoton-lineage configuration keys (`0x87`, `0x87` / `0xAA`), the slot mapping, chip-ID reads at `0x20`/`0x21`, absent-ID classification, the slot-1 exit withhold, owed cleanup, the ISA mutex and the [sampling-path rediscovery cadence](superio-access.md#sampling-path-rediscovery-cadence). Reused, not respecified. |
| S9 | [`pawnio-interface.md`](pawnio-interface.md) revision 9, `LpcIO` section | Interface source: `ioctl_select_slot`, `ioctl_find_bars`, `ioctl_superio_inb`/`outb`, `ioctl_pio_inb`/`outb`; port access limited to the configuration pair and the 8-byte-aligned BAR windows; caller-held `Global\Access_ISABUS.HTP.Method`. |
| S10 | PawnIO.Modules `LpcIO.p` at tag `0.2.11` (commit `52a7e536dff3e53c96917a28caac5e0fa6510696`), LGPL-2.1-or-later | Upstream-published interface definition of the module this project calls. `ioctl_find_bars` reads global `0x20` and fails if it is `0x00` or `0xFF`; it records, for every logical device, the `0x60/61` and `0x62/63` values that read the same twice, are non-zero, not `0xFFFF` and at least `0x100`; a port is allowed only if the port rounded down to 8 bytes equals a recorded value (a value whose low three bits are `101b` is first rounded down). `ioctl_select_slot` clears the recorded list. Used for port-authorization behaviour only; no code copied. |
| S11 | [`superio-fintek-f71882-f71889.md`](superio-fintek-f71882-f71889.md) revision 2 and [`superio-nuvoton-nct6775-nct6776-nct610x.md`](superio-nuvoton-nct6775-nct6776-nct610x.md) revision 2 | Project sources for the shared procedure: stop at the first passing slot so no slot selection follows the cached `ioctl_find_bars`, cache the path before releasing the mutex, the unavailable-slot rule for position-keyed consumers, the -55 to +125 °C plausibility policy and the 23 437 RPM ceiling. |
| S12 | [ADR 0011](../../adr/0011-experimental-sensor-enablement.md), [ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md), [ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md) | Project policy. No hardware facts. |
| S13 | LibreHardwareMonitor `LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/` `Chip.cs`, `LpcIO.cs` and the Winbond hardware-monitor class file (commit `2808d80564b6761e1b01f42fabfd966ed695fcbc`, the #2523 comparison inventory, retrieved 2026-11-29), MPL-2.0 | **lead-only (copyleft)**. Chip table: `0x52` with `CR21` `0x17`, `0x3A` or `0x41` W83627HF; `0x82` with `CR21` high nibble 8 W83627THF; `0x85` with `CR21` `0x41` W83687THF; `0x88` with high nibble 5 or 6 W83627EHF; `0xA0` with high nibble 2 W83627DHG; `0xA5` with high nibble 1 W83667HG; `0xB0` with high nibble 7 W83627DHG-P; `0xB3` with high nibble 5 W83667HG-B. Hardware monitor at logical device B, base + 5 / + 6, vendor ID `0x5CA3` required; temperatures from bank 0 `0x27`, bank 1 and bank 2 `0x50`/`0x51` with a -55 to +125 °C filter; CPUTIN and AUXTIN suppressed when `0x49` selects a non-zero source on W83627DHG and W83627DHG-P; five fan counts at `0x28`, `0x29`, `0x2A`, `0x3F` and bank 5 `0x53` with 3-bit divisors; a count of `0xFF` shown as 0 RPM. It does not distinguish the W83527HG from the W83627DHG-P. Restated in this document's own words; no code, structure or identifiers copied. |
| S14 | coreboot `util/superiotool/winbond.c` (commit `95b01cee5e20d5916fcbf50f85c3baba011e4a4a`, retrieved 2026-11-28), GPL-2.0-or-later | **lead-only (copyleft)**. Register-dump tool ID table: `0x52` W83627HF/F/HG/G, `0x828` W83627THF/THG, `0x85x` W83687THF, `0x88` W83627EHF/EF/EHG/EG, `0xa02` W83627DHG, `0xa23` W83627UHG, `0xa51` W83667HG, `0xb07` listed twice, as "W83627DHG-P/-PT" and as "W83527HG", and `0xb35` "Nuvoton NCT5571D". Restated; nothing copied. |
| S15 | Linux kernel documentation `Documentation/hwmon/w83627ehf` (docs.kernel.org, retrieved 2026-11-29), GPL-2.0 | **lead-only (copyleft)**. Lists W83627EHF/EHG, W83627DHG, W83627DHG-P, W83667HG as "Datasheet: not available", W83667HG-B as "Available from Nuvoton upon request"; states that the driver chooses fan divisors itself (a write, not adopted). |
| S16 | The Retro Web motherboard catalog, listings filtered by chip, retrieved 2026-11-29: W83627H (HF/HG) chip 4474 836 boards; W83627DH (DHG, DHG-P, DHG-PT and variants in one entry) 4418 352; W83627TH 4502 192; W83627EH 4551 163; W83667H (HG-A, HG-AC) 4851 36; NCT5571D 5188 11; W83687TH 4521 10; W83527HG 5080 5 | Public catalog counts. Used only to rank targets; not a hardware fact and not a market-share measure. |

S13–S15 are copyleft implementations and documentation consulted as leads
under ADR 0023. No enabled row depends on them: every fact an enabled row
uses is pinned to S1, S2 or S3 and to the project sources S8–S12. The leads
corroborate chip identity and record the IDs of disabled targets. The
implementer works from this document only and does not open S13–S15.

## Target ranking and disposition (#2523)

Ranking uses the S16 catalog count as an installed-base proxy and then
primary-source availability. The rev 1 slice is the second-ranked W83627DH
catalog entry (W83627DHG and W83627DHG-P), because its datasheets, together
with the W83527HG datasheet for the part that shares the DHG-P chip ID (S1,
S2, S3), cover every register this document reads for every part that can
report the enabled IDs. The first-ranked W83627HF reports a different HM
chip ID and its register map is not analysed in this revision.

| Rank | Target (raw ID) | S16 boards | Primary source | Disposition in this revision |
| --- | --- | --- | --- | --- |
| 1 | W83627HF (`0x52` `0x1y`, S5; lead: `CR21` `0x17`, `0x3A`, `0x41`) | 836 | S5 v0.53 obtained; rev 2.27 listed, not analysed | **Deferred.** Its HM chip ID register returns `0x21` (S5 p. 118), not the `0xC1` of the enabled parts (S1 p. 92; S2 p. 84; S3 p. 64), so the enabled map is not assumed. Register map not analysed in rev 1. Remains tracked by #2523 until a follow-up issue is linked. |
| 2 | W83627DHG (`0xA020`–`0xA02F`) | 352 (shared entry) | S2, complete | **Enabled, Experimental**: SYSTIN, CPUTIN, AUXTIN, fans 1–5 |
| 2 | W83627DHG-P / W83627DHG-PT (`0xB070`–`0xB07F`) | (in the same entry) | S1, complete | **Enabled, Experimental**: SYSTIN, CPUTIN, SYSFANIN, CPUFANIN0, CPUFANIN1. AUXTIN, AUXFANIN0, AUXFANIN1 are **Unsupported** because the W83527HG reports the same ID (S3) and has no such inputs. |
| — | W83527HG (`0xB070`–`0xB07F`), not a #2523 target | 5 | S3, complete | Shares the DHG-P row and its subset; surfaced under the shared label. |
| 3 | W83627THF / THG (`0x8283`, `0x8285`, S6) | 192 | S6, complete | **Deferred.** Its HM chip ID register returns `0x90` (S6 p. 46), not `0xC1`, so the enabled map is not assumed. Register map not analysed in rev 1. Remains tracked by #2523 until a follow-up issue is linked. |
| 4 | W83627EHF / EHG (`0x886X`, S7; lead S13 also `0x885X`) | 163 | S7, complete | **Deferred.** S7 obtained; its HM chip ID register returns `0xA1` (S7 p. 71). Register analysis and the `0x885X` lead claim are pending. Remains tracked by #2523 until a follow-up issue is linked. |
| 5 | W83667HG-B (lead `0xB350`–`0xB35F`, S13) and W83667HG-A (`0xB35X`, S4) | 36 (W83667HG-A/AC entry) | S4 (W83667HG-A) | **Unsupported.** S4 assigns `0xB35X` to W83667HG-A, S13 labels it W83667HG-B and S14 labels it NCT5571D. S4's readings at `0x27` and banks 1–2 are firmware-selected fan-temperature sources, a different mechanism, and its report register needs a select write. Candidate facts below. |
| 6 | W83667HG (lead `0xA510`–`0xA51F`, S13, S14) | not separately listed | none obtained | **Unsupported.** No datasheet obtained (S15 lists it as not available). Remains tracked by #2523 until a follow-up issue is linked. |
| 7 | W83687THF (lead `0x8541`, S13; `0x85x`, S14) | 10 | none found | **Unsupported.** No datasheet; ID is lead-only. Remains tracked by #2523 until a follow-up issue is linked. |

No similar product number or software alias is treated as hardware
identity. A first enabled slice does not imply whole-family support.

## Detection

### Chip identity

| Fact | Source |
| --- | --- |
| Configuration mode is entered by writing `0x87` twice to the index port (`0x2E`, or `0x4E` when the `CR26` bit 6 HEFRAS strap is set) and left by writing `0xAA`. Registers are selected through the index port and accessed at index + 1. These are the Winbond/Nuvoton-lineage keys of S8. | S1 p. 29, p. 194; S2 pp. 25–26; S3 p. 18; S8 |
| W83627DHG: `CR20` = `0xA0`; `CR21` = `2Xh`, where X is the IC version (read only). | S2 p. 180 |
| W83627DHG-P / DHG-PT: `CR20` = `0xB0`; `CR21` = `7Xh`, X is the IC version. | S1 p. 192 |
| W83527HG: `CR20` = `0xB0`; `CR21` = `7Xh`, X is the IC version. The raw ID therefore does not distinguish W83527HG from W83627DHG-P. Lead S14 lists `0xb07` for both parts. | S3 p. 108; S14 corroborates |
| All three: HM vendor ID at `0x4F` reads `0x5C` when `0x4E` bit 7 (HBACS) is 1 and `0xA3` when it is 0. | S1 p. 89; S2 pp. 81–82; S3 p. 63 |

### ID-matching policy

| Raw ID (`CR20`:`CR21`) | Row | Chip label to surface |
| --- | --- | --- |
| `0xA020`–`0xA02F` | W83627DHG profile | `W83627DHG` |
| `0xB070`–`0xB07F` | Shared DHG-P / W83527HG profile | `W83627DHG-P / W83527HG` |
| Any other value | Outside this document | — |

The ranges are the full `X` ranges stated by S1, S2 and S3. They overlap no
raw ID handled by [`superio-nuvoton-nct67xx.md`](superio-nuvoton-nct67xx.md)
or [`superio-nuvoton-nct6775-nct6776-nct610x.md`](superio-nuvoton-nct6775-nct6776-nct610x.md).
The raw ID is always recorded in diagnostics.

### Runtime gates

Every condition is a mechanism or safety gate (ADR 0031). Board names, BIOS
versions, base address values and package suffixes are evidence identities
and never gate a read. Rows: **DHG** = `0xA02x`, **B07** = `0xB07x`.

| Condition | Gate kind | Rows | Source |
| --- | --- | --- | --- |
| Raw chip ID in an enabled range of the ID-matching table | mechanism | all | S1 p. 192; S2 p. 180; S3 p. 108 |
| `CR26` bit 5 (LOCKREG) is `0`. When `1`, "Disable R/W configuration registers"; stop and report the chip unavailable. Never write `CR26`. | safety | all | S1 p. 194; S2 p. 182; S3 p. 109 |
| `CR22` bit 6 (HM power down) is `1` ("not powered down") and `CR23` bit 0 (IPD) is `0`. Never write either. | mechanism | all | S1 p. 192; S2 p. 180; S3 p. 108 |
| Logical device `0x0B` `CR30` bit 0 is `1` (active). Never write `CR30`. | mechanism | all | S1 p. 227; S2 p. 210; S3 p. 128 |
| HM base B = `CR60`·256 + `CR61` lies in `0x0100`–`0x0FF8` and B & `0x0007` is `0`. The datasheets allow any two-byte boundary in `0x100`–`0xFFE`, but S10 authorizes only the 8-byte window containing B, so base + 6 (and for some values base + 5) of any other base falls outside it. Report such a base unavailable; do not work around it. | safety | all | S1 p. 227; S2 p. 210; S3 p. 128; S9; S10 |
| HM vendor ID reads `0x5C` / `0xA3` (procedure below) before the path is cached | mechanism | all | S1 p. 89; S2 pp. 81–82; S3 p. 63 |
| HM `0x40` bit 0 (Start) is `1` and bit 3 (INT_Clear) is `0`. Bit 0 = 0 is standby; bit 3 = 1 stops monitoring. Either makes the whole sample unavailable. Never write `0x40` (bit 7 restores power-on defaults). | mechanism | all | S1 p. 79; S2 p. 73; S3 p. 55 |
| SYSTIN: `0x18` bit 4 (OVT1_MODE) is `0`. In OVT# interrupt mode the vendor figure states the OVT# interrupt is reset when the temperature sensor registers are read, so `0x27` is not read when this bit is `1`. | safety | all | S1 pp. 63, 77; S2 pp. 53, 71; S3 pp. 44, 54 |
| SYSTIN: `0x7C` bit 3 (MNTEMP1_SEL) is `0` ("SYSTIN Temperature"; `1` = PECI1) | mechanism | B07 | S1 p. 105; S3 p. 75 |
| CPUTIN: `0x49` bits 2:0 are `000`. Any other value makes the bank 1 reading a PECI agent value (TBase + PECI), which "does not reflect the actual temperature". | mechanism | all | S1 pp. 40–41, 84–85, 106; S2 p. 37, p. 77; S3 p. 60 |
| CPUTIN: `0x7C` bit 4 (MNTEMP2_SEL) is `0` ("CPUTIN Temperature"; `1` = PECI1) | mechanism | B07 | S1 p. 105; S3 p. 75 |
| CPUTIN: bank 1 `0x52` bit 0 (STOP) is `0` | mechanism | all | S1 pp. 106–107; S2 p. 96; S3 p. 77 |
| CPUTIN: bank 1 `0x52` bit 1 (OVTMOD) is `0` (comparator). When `1`, do not read bank 1 `0x50`/`0x51`, for the same OVT# reason as SYSTIN. | safety | all | S1 pp. 63, 106–107; S2 pp. 53, 95–96; S3 pp. 44, 77 |
| AUXTIN: `0x49` bits 6:4 are `000` (otherwise PECI agent), bank 2 `0x52` bit 0 (STOP) is `0`, and bank 2 `0x52` bit 1 (OVTMOD) is `0` (safety, as above) | mechanism / safety | DHG | S2 pp. 37, 53, 77, 98–99 |
| Per fan input: its pin is routed to the tachometer (fan table below) | mechanism | all | S1 pp. 16–17, 83, 87–88, 193, 195; S2 pp. 15–16, 76, 80, 181, 183; S3 pp. 8, 59, 62–63, 109–110 |

No HM I/O-space lock is documented for these parts; the `CR28` bit 4 lock
of [`superio-nuvoton-nct67xx.md`](superio-nuvoton-nct67xx.md) is not applied
(S1 p. 194 and S2 p. 182 describe `CR28` as flash ROM and parallel-port
options).

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| DHG (`0xA020`–`0xA02F`) SYSTIN, CPUTIN, AUXTIN | **Experimental**. Every identity, gate, register and decode is pinned to S2. No lead-only fact. No hardware dump; best-evidenced member: none. | Enabled as Experimental, per input behind its gates |
| DHG fans SYSFANIN, CPUFANIN0, AUXFANIN0, CPUFANIN1, AUXFANIN1 | **Experimental**, S2 for counts, divisors, formula and routing. No lead-only fact. | Enabled as Experimental, per input behind its routing gate |
| B07 (`0xB070`–`0xB07F`) SYSTIN, CPUTIN | **Experimental**. S1 and S3 state the same registers, gates and decode for both parts that report this ID. No lead-only fact. No dump. | Enabled as Experimental, per input behind its gates |
| B07 fans SYSFANIN, CPUFANIN0, CPUFANIN1 | **Experimental**. S1 and S3 state the same count registers, divisor bits and routing bits for both parts. No lead-only fact. | Enabled as Experimental, per input behind its routing gate |
| B07 AUXTIN, AUXFANIN0, AUXFANIN1 | **Unsupported**. S1 documents them for the W83627DHG-P; S3 documents two thermal inputs and three fan inputs for the W83527HG and marks `0x2A` reserved. The raw ID cannot select between the two maps, so reading them would guess the part. | Disabled |
| Fan counts of `0xFF` (saturated at the firmware-selected divisor) | The count register is 8 bits; the datasheets give no stopped-fan value. | Reported unavailable, not 0 RPM |
| Bank 6 "speed (RPM)" registers `0x50`–`0x59` (S1 pp. 119–122) | S1 gives names and read/write attributes only, no unit, update or validity rule. | Disabled (not read) |
| Voltages, PECI/SST values, `0x7D` temperature register, alarms, limits, PWM | Out of scope | Disabled |
| Every ID outside the ID-matching table | See the disposition table | Disabled |

## Register map (facts)

### Configuration space (read in configuration mode)

| Register | Bits | Meaning | Rows | Source |
| --- | --- | --- | --- | --- |
| `0x07` | 7:0 | Logical device number; `0x0B` selects the hardware monitor | all | S1 p. 192; S2 p. 180; S3 p. 108 |
| `0x20` / `0x21` | 7:0 | Chip ID high / low byte (read only) | all | S1 p. 192; S2 p. 180; S3 p. 108 |
| `0x22` | bit 6 | HM power down: `0` = powered down, `1` = not powered down | all | S1 p. 192; S2 p. 180; S3 p. 108 |
| `0x23` | bit 0 | IPD: `1` puts the whole chip into power-down mode | all | S1 p. 192; S2 p. 180; S3 p. 108 |
| `0x24` | bit 1 | ENROM (strap): `0` = pin 58 is AUXFANIN1, `1` = pin 58 is SI | DHG | S2 p. 181 |
| `0x26` | bit 5 | LOCKREG: `0` = configuration registers read/write enabled, `1` = disabled. Bit 6 (HEFRAS) reports the configuration-port strap. | all | S1 p. 194; S2 p. 182; S3 p. 109 |
| `0x29` | bits 2:1 | Pin function select for CPUFANIN1 / CPUFANOUT1: `00` = CPUFANIN1, CPUFANOUT1 (default); `01` = GP21, GP20; other values are not listed | all | S1 p. 195; S2 p. 183; S3 pp. 109–110 |
| LDN B `0x30` | bit 0 | `1` = logical device active | all | S1 p. 227; S2 p. 210; S3 p. 128 |
| LDN B `0x60` / `0x61` | 15:0 | HM base address high / low byte, `0x100`–`0xFFE` on a two-byte boundary | all | S1 p. 227; S2 p. 210; S3 p. 128 |

### Hardware-monitor ports and bank select

| Item | Fact | Source |
| --- | --- | --- |
| Address (index) port | Base + `0x05`. Bits 6:0 are the register index; bit 7 is reserved. | S1 pp. 32, 66; S2 pp. 29, 56; S3 pp. 21, 46 |
| Data port | Base + `0x06` | S1 pp. 32, 66; S2 pp. 29, 56; S3 pp. 21, 46 |
| Bank select `0x4E` | Bits 2:0 (BANKSEL) select the bank for indexes `0x50`–`0x5F` only. Bit 7 (HBACS) selects the `0x4F` vendor-ID byte. Bit 5 (DHG and DHG-P: EN_AUXFANIN1_BP) and bit 4 (all: EN_CPUFANIN1_BP) are BEEP enables; bits 6 and 3 are reserved. The register is read/write. | S1 p. 88; S2 p. 81; S3 p. 63 |
| Unbanked indexes | All indexes read here outside `0x50`–`0x5F` (`0x18`, `0x27`–`0x2A`, `0x3F`, `0x40`, `0x47`, `0x49`, `0x4B`–`0x4F`, `0x7C`) are unbanked. `0x58`, `0x59` and `0x5D` are bank 0. | S1 p. 88; S2 p. 81; S3 p. 63 |

Because the bank-select register also holds BEEP enables, a bank-select
write that does not preserve bits 6:3 would change BEEP configuration. The
procedure below preserves them.

### HM registers read

| Index (bank) | Name | Bits | Meaning | Rows | Source |
| --- | --- | --- | --- | --- | --- |
| `0x18` (unbanked) | OVT# configuration | 4 | OVT1_MODE: `0` compare (default), `1` interrupt | all | S1 p. 77; S2 p. 71; S3 p. 54 |
| `0x27` (unbanked) | SYSTIN reading | 7:0 | 8-bit two's complement, 1 °C | all | S1 pp. 36, 78; S2 pp. 32–33, 71; S3 pp. 23–24, 54 |
| `0x28` / `0x29` (unbanked) | SYSFANIN / CPUFANIN0 reading | 7:0 | Count of the internal clock per revolution | all | S1 p. 78; S2 p. 71; S3 p. 54 |
| `0x2A` (unbanked) | AUXFANIN0 reading | 7:0 | Count (DHG). Reserved on W83527HG. | DHG | S2 p. 72; S3 p. 54 |
| `0x3F` (unbanked) | CPUFANIN1 reading | 7:0 | Count | all | S1 p. 79; S2 p. 72; S3 p. 55 |
| `0x40` (unbanked) | Configuration | 0, 3 | Start, INT_Clear (see gates) | all | S1 p. 79; S2 p. 73; S3 p. 55 |
| `0x47` (unbanked) | Fan divisor I | 7:6, 5:4, 2, 0 | CPUFANIN0 divisor bits 1:0; SYSFANIN divisor bits 1:0; FANINC4 (`1` = pin of CPUFANIN1 is a tachometer input); FANINC5 (`1` = pin 58 AUXFANIN1 is a tachometer input; DHG and DHG-P only) | all (bit 0: DHG) | S1 p. 83; S2 p. 76; S3 p. 59 |
| `0x49` (unbanked) | CPUFANOUT0 / AUXFANOUT monitor temperature source select | 2:0, 6:4 | `000` = CPUTIN / AUXTIN; `001` reserved; `010`–`101` = PECI agents 1–4, which also become the bank 1 / bank 2 reading | all (6:4: DHG) | S1 pp. 84–85; S2 pp. 37, 77; S3 p. 60 |
| `0x4B` (unbanked) | Fan divisor II | 7:6 | AUXFANIN0 divisor bits 1:0 | DHG | S2 p. 79 |
| `0x4C` (unbanked) | SMI#/OVT# control | 7 | CPUFANIN1 divisor bit 2 | all | S1 p. 86; S2 p. 79; S3 p. 62 |
| `0x4D` (unbanked) | FAN IN/OUT control | 0, 2, 4 | FANINC1 (SYSFANIN pin), FANINC2 (CPUFANIN0 pin), FANINC3 (AUXFANIN0 pin, DHG): `1` = tachometer input (default), `0` = the pin is driven as a control output | all (bit 4: DHG) | S1 pp. 87–88; S2 p. 80; S3 pp. 62–63 |
| `0x4E` (unbanked) | Bank select | 7:0 | See above | all | S1 p. 88; S2 p. 81; S3 p. 63 |
| `0x4F` (unbanked) | Vendor ID | 7:0 | `0x5C` with HBACS = 1, `0xA3` with HBACS = 0 | all | S1 p. 89; S2 pp. 81–82; S3 p. 63 |
| `0x59` (bank 0) | Fan divisor selection | 7, 3:2, 1:0 | AUXFANIN1 divisor bit 2; AUXFANIN1 divisor bits 1:0; CPUFANIN1 divisor bits 1:0 | all (7, 3:2: DHG) | S1 p. 92; S2 pp. 84–85; S3 p. 64 |
| `0x5D` (bank 0) | VBAT monitor control | 7, 6, 5 | AUXFANIN0 (DHG), CPUFANIN0, SYSFANIN divisor bit 2 | all (7: DHG) | S1 pp. 92–93; S2 p. 85; S3 pp. 64–65 |
| `0x7C` (unbanked) | Temperature selection | 4, 3 | MNTEMP2_SEL / MNTEMP1_SEL (see gates). Not documented on W83627DHG; not read there. | B07 | S1 p. 105; S3 p. 75 |
| `0x50` / `0x51` (bank 1) | CPUTIN high / low | 7:0 / 7 | TEMP<8:1> / TEMP<0>, 9-bit two's complement, 0.5 °C; `0x51` bits 6:0 reserved | all | S1 p. 106; S2 pp. 94–95; S3 p. 76 |
| `0x52` (bank 1) | CPUTIN configuration | 1, 0 | OVTMOD (`0` compare), STOP (`1` = stop monitoring CPUTIN) | all | S1 pp. 106–107; S2 pp. 95–96; S3 p. 77 |
| `0x50` / `0x51` (bank 2) | AUXTIN high / low | 7:0 / 7 | As CPUTIN | DHG | S2 p. 98 |
| `0x52` (bank 2) | AUXTIN configuration | 1, 0 | OVTMOD, STOP | DHG | S2 pp. 98–99 |
| `0x53` (bank 5) | AUXFANIN1 reading | 7:0 | Count | DHG | S2 p. 107 |

### Fan inputs

| Label | Input | Count | Divisor bit 2 / bits 1:0 | Routed when | Rows | Source |
| --- | --- | --- | --- | --- | --- | --- |
| Fan 1 | SYSFANIN | `0x28` | `0x5D` bit 5 / `0x47` bits 5:4 | `0x4D` bit 0 = 1 | all | S1 pp. 44, 83, 87–88, 92–93; S2 pp. 39, 76, 80, 85; S3 pp. 27, 59, 62–65 |
| Fan 2 | CPUFANIN0 | `0x29` | `0x5D` bit 6 / `0x47` bits 7:6 | `0x4D` bit 2 = 1 | all | same |
| Fan 3 | AUXFANIN0 | `0x2A` | `0x5D` bit 7 / `0x4B` bits 7:6 | `0x4D` bit 4 = 1 | DHG | S2 pp. 39, 79, 80, 85 |
| Fan 4 | CPUFANIN1 | `0x3F` | `0x4C` bit 7 / `0x59` bits 1:0 | `CR29` bits 2:1 = `00` and `0x47` bit 2 = 1 | all | S1 pp. 83, 86, 92, 195; S2 pp. 76, 79, 84–85, 183; S3 pp. 59, 62, 64, 109–110 |
| Fan 5 | AUXFANIN1 | bank 5 `0x53` | `0x59` bit 7 / `0x59` bits 3:2 | `CR24` bit 1 = 0 and `0x47` bit 0 = 1 | DHG | S2 pp. 76, 84–85, 107, 181 |

| Fact | Source |
| --- | --- |
| RPM = 1.35 × 10⁶ / (count × divisor). | S1 p. 44; S2 p. 39; S3 p. 27 |
| The divisor is 2^(bit2·4 + bit1·2 + bit0): `000` = 1, `001` = 2 (default), `010` = 4, `011` = 8, `100` = 16, `101` = 32, `110` = 64, `111` = 128. | S1 pp. 44, 93; S2 pp. 39, 85; S3 pp. 27, 65 |
| The count registers are 8 bits, so the largest count the chip can report is `0xFF`. With the default divisor 2 that bounds the lowest reportable speed at about 2647 RPM. | S1 pp. 44, 78–79; S2 pp. 39, 71–72 |

### Registers that clear on read or reset latches (never read)

| Register | Attribute | Source |
| --- | --- | --- |
| Bank 0 `0x41`, `0x42` (interrupt status 1, 2); bank 0 `0x45` (interrupt status 4 on DHG-P and W83527HG, reserved on DHG); bank 4 `0x50` (interrupt status 3) | Marked read only, but the SMI# interrupt "must be reset by reading all the interrupt status registers" and the figures mark the reset at that read; the later sibling datasheet S4 relabels `0x41`, `0x42`, `0x45` "Read Clear". | S1 pp. 58–62, 80, 82, 114; S2 pp. 50–52, 73–75, 101; S3 pp. 40–43, 56–58, 81; S4 p. 306 |
| `0x27`, bank 1 `0x50`/`0x51`, bank 2 `0x50`/`0x51` while the input's OVT# mode bit is `1` | Figure note: the OVT# interrupt is reset "when Temperature sensor registers are read". The runtime gates skip these reads in that state. | S1 p. 63; S2 p. 53; S3 p. 44 |

This document also never reads bank 4 `0x59`–`0x5B` (real-time status),
`0x7D`, bank 6, or any logical device other than `0x0B`.

## Read procedure and decode

### Detection (cached; configuration mode)

1. Acquire `Global\Access_ISABUS.HTP.Method` with a bounded timeout; on
   failure skip the attempt. Hold it for the whole transaction. (S8, S9)
2. Run the S8 owed-cleanup step first. Then visit slot 0, then slot 1, and
   stop at the first slot that passes steps 3–6. For the slot being
   visited, call `ioctl_select_slot`, enter configuration mode with `0x87`,
   `0x87` and read `CR20`/`CR21`. Absent-ID rules of S8 apply. Any ID
   outside the ID-matching table, or a later gate failure on that slot, ends
   the attempt on that slot with the `0xAA` exit, then the next slot is
   visited. On slot 1 the S8 exit withhold applies: when the combined ID is
   exactly `0x8733`, `0x8790`, `0x8695` or `0x8883`, no exit write of any kind
   is issued on that slot, the skip is reported as S8 describes, and no path
   from this document results. (S1, S2, S3, S8)
3. Read `CR26` and require bit 5 = 0; then read `CR22` (bit 6 = 1) and
   `CR23` (bit 0 = 0). Read `CR29` for the CPUFANIN1 routing; on the DHG row
   also read `CR24` for the AUXFANIN1 routing. (S1 pp. 192–195; S2
   pp. 180–183; S3 pp. 108–110)
4. Write `CR07` = `0x0B`; read `CR30` and require bit 0 = 1. Read `CR60`,
   `CR61`; apply the base gate. (S1 p. 227; S2 p. 210; S3 p. 128)
5. Call `ioctl_find_bars` on the slot being visited, after steps 2–4. No
   `ioctl_select_slot` follows it once the slot passes step 6. (S10, S11)
6. Through base + 5 / + 6: read `0x4E` (value O). Write `0x4E` =
   (O AND `0x78`) OR `0x80` and read `0x4F`; write `0x4E` = (O AND `0x78`)
   and read `0x4F`; require `0x5C` then `0xA3`. Write `0x4E` = O. (S1
   pp. 88–89; S2 pp. 81–82; S3 p. 63)
7. Exit configuration mode (`0xAA`) on the visited slot on every path,
   including failures, except where step 2 withholds it on slot 1. If an
   entry-key or exit write fails, detection visits no further slot, caches
   no path and records the owed cleanup with the sequence `0x87`, `0x87`,
   `0xAA`, under the S8 rules. (S8)
8. While the mutex is still held, cache the slot, row, base, and the
   configuration-space routing results (`CR29`, and `CR24` on DHG). A
   failed `CR29` or `CR24` read leaves the dependent fan unavailable while
   the path stays cached; it starts no detection. Then release the mutex.
   Sampling never enters configuration mode and never issues
   `ioctl_select_slot`. Any `ioctl_select_slot` on the shared `LpcIO` handle
   after step 5 invalidates the cached path; it is discarded and the next
   detection follows S8 rule R6. (S8, S10, S11)

A discovery that runs several Winbond/Nuvoton-lineage probes on one slot
may dispatch to this document's steps 3–8 from the chip-ID read of a shared
`0x87`, `0x87` entry, provided every step above runs in order for an ID in
this document's table and the S8 exit and cleanup rules hold.

### Rediscovery eligibility

Detection follows the S8 cadence. A rediscovery is a separate detection
transaction under its own mutex acquisition; no sample transaction enters
configuration mode, and a failed sample starts no detection. The S8 shared
table covers the mutex, the executor, `ioctl_find_bars` and
configuration-register read failures.

| Detection failure | Class | Reason |
| --- | --- | --- |
| The `0x87`, `0x87` entry-key write or the `0xAA` exit write fails on a visited slot (steps 2, 7) | Eligible | The chip may still be in configuration mode; the owed cleanup reissues the exit (S8). |
| An exit withheld on slot 1 under step 2 | Not a failure | S8 makes the withhold intentional. |
| Step 6: an HM port write or read fails | Eligible | `ioctl_find_bars` success does not prove a usable window; a later attempt runs step 5 again (S8). |
| Step 6: the vendor ID reads but is not `0x5C` / `0xA3` | Not eligible | A value read successfully. |
| `CR26` bit 5 set, `CR22` bit 6 clear, `CR23` bit 0 set, `CR30` bit 0 clear, or a refused base | Not eligible | A value read successfully. |
| Absent responder or a chip ID outside the ID-matching table | Not eligible | The probe completed. |

### Sampling (HM ports only, under the ISA mutex)

Each HM read is an index-port write followed by one data-port read. The only
data-port writes are the `0x4E` writes stated below.

1. Read `0x4E` (value O). If it fails, the whole sample is unavailable.
2. Read `0x40`. If it fails, or bit 0 = 0, or bit 3 = 1, every input is
   unavailable for this sample; go to step 6.
3. SYSTIN: read `0x18`; on B07 also read `0x7C`. Only if `0x18` bit 4 = 0
   (and on B07 `0x7C` bit 3 = 0) read `0x27`.
4. CPUTIN and AUXTIN: read `0x49` (and on B07 `0x7C` from step 3). For
   CPUTIN, when `0x49` bits 2:0 = `000` (and on B07 `0x7C` bit 4 = 0), write
   `0x4E` = (O AND `0xF8`) OR `0x01`, read bank 1 `0x52`, and only if its
   bits 1:0 are `00` read bank 1 `0x50` then `0x51`. On DHG, AUXTIN does the
   same with `0x49` bits 6:4 = `000`, bank value `0x02`, and bank 2
   `0x52`, `0x50`, `0x51`.
5. Fans: read the unbanked `0x4D`, `0x47`, `0x4C`, and on DHG `0x4B`.
   Write `0x4E` = (O AND `0xF8`) OR `0x00`, and only after that write
   succeeds read bank 0 `0x5D` and `0x59`. Both sit in the banked range
   `0x50`–`0x5F`, so neither is read while step 4 has bank 1 or bank 2
   selected. Then read the count of each routed fan input (`0x28`, `0x29`,
   `0x3F`; on DHG also `0x2A`, and bank 5 `0x53` after writing `0x4E` =
   (O AND `0xF8`) OR `0x05`). (S1 p. 88; S2 p. 81; S3 p. 63)
6. If any `0x4E` write was issued, write `0x4E` = O. Release the mutex.

Partial results: a failed read makes only the inputs that depend on it
unavailable (`0x18` → SYSTIN; `0x7C` → SYSTIN and CPUTIN on B07;
`0x49` → CPUTIN and AUXTIN; a divisor
or routing register → the fans using it). A failed `0x4E` write makes every
input of that bank unavailable for the sample and no read of that bank is
issued after it; a failed bank 0 write therefore makes every fan
unavailable, because each fan's divisor needs `0x5D` or `0x59`. Every
read of an index in `0x50`–`0x5F` follows a successful write of its own
bank in the same transaction; no other index read here is banked. A failed restore write in step 6 does not invalidate
values already read; it is recorded in the partial-sample diagnostic, and
the next sample reads `0x4E` afresh. Bits 6:3 of `0x4E` are never changed
by any write in this document.

### Temperature decode

| Input | Value | Basis |
| --- | --- | --- |
| SYSTIN | t = `0x27` as signed 8-bit, in °C | S1 p. 36; S2 pp. 32–33; S3 pp. 23–24 |
| CPUTIN, AUXTIN | v = high·2 + (low bit 7); if v ≥ 256 then v = v − 512; t = v × 0.5 °C | S1 pp. 36, 106; S2 pp. 32–33, 94–95, 98; S3 pp. 23–24, 76 |
| All | Publish only −55 ≤ t ≤ +125 °C inclusive; otherwise the input is unavailable for that sample (not clamped, not zero, not a device failure) | Table 8-1 range (S1 p. 36; S2 p. 32); S11 policy |

An input whose gate fails is not read and not reported as a measurement.
The high byte and the low bit are separate reads and no latch is
documented, so a change between the two reads can combine bytes from
different conversions for that sample.

### Fan decode

| Count | Reading | Basis |
| --- | --- | --- |
| `0x00` | Invalid, no RPM | Undefined by the datasheets |
| `0x01`–`0xFE` | RPM = floor(1 350 000 / (count × divisor)); a result above 23 437 RPM is invalid | S1 p. 44; S2 p. 39; S3 p. 27; S11 ceiling |
| `0xFF` | Unavailable: the counter is saturated at the firmware-selected divisor, so the speed is only known to be below 1 350 000 / (255 × divisor) RPM | S1 pp. 44, 78; S2 pp. 39, 71 |

A fan input whose routing gate fails is not read and not reported as a
measurement; a position-keyed consumer reports it as an explicit
unavailable slot at its position. (S11)

### Input identity and ordering

Labels are bound to physical inputs: `SYSTIN` (pin 104), `CPUTIN` (pin 103),
`AUXTIN` (pin 102, DHG only); `Fan 1`–`Fan 5` = SYSFANIN, CPUFANIN0,
AUXFANIN0, CPUFANIN1, AUXFANIN1 on DHG; on B07 the positions of Fan 3 and
Fan 5 are reported as explicit unavailable slots, so Fan 4 keeps its label.
Omitting an input never renumbers the others. The implementation PR
verifies the new chip labels and channel identities against the existing
identity contracts before use. (S1 pp. 16–17; S2 pp. 15–16; S3 p. 8; S11)

### Synthetic cases for implementation review

All fixtures are synthetic.

| Fixture | Expected observation |
| --- | --- |
| Slot 0, `0xA023`, `CR26=0x00`, `CR22=0xFF`, `CR23=0x00`, `CR24=0x40`, `CR29=0x00`, `CR30=0x01`, base `0x0290`; vendor bytes `0x5C`/`0xA3`; `0x4E=0x80`, `0x40=0x03`, `0x18=0x43`, `0x49=0x00`, bank 1 `0x52=0x00`, bank 2 `0x52=0x00`; `0x27=0x28`; bank 1 `0x50=0x2D`, `0x51=0x80`; bank 2 `0x50=0xC9`, `0x51=0x00`; `0x4D=0x95`, `0x47=0x55`, `0x4B=0x44`, `0x4C=0x10`, `0x5D=0x04`, `0x59=0x00`; counts `0x28=0x87`, `0x29=0xFF`, `0x2A=0x00`, `0x3F=0x87`, bank 5 `0x53=0x87` | Ports `0x295`/`0x296`. SYSTIN 40 °C, CPUTIN 45.5 °C, AUXTIN −55.0 °C; Fan 1 5000 RPM (divisor 2), Fan 2 unavailable (saturated), Fan 3 invalid, Fan 4 10000 RPM (divisor 1), Fan 5 10000 RPM (divisor 1). |
| `0xB071` with the same values (no `CR24` read, `0x7C=0x00`) | SYSTIN 40 °C, CPUTIN 45.5 °C; Fan 1, Fan 2 and Fan 4 as above; AUXTIN, Fan 3 and Fan 5 never read, Fan 3 and Fan 5 reported as unavailable slots. No read of bank 2, `0x2A`, `0x4B` or bank 5. |
| Divisor bits: `0x47` bits 5:4 = `11`, `0x5D` bit 5 = 0, count `0x96` | Divisor 8; 1125 RPM. |
| CPUFANIN1: `0x59` bits 1:0 = `10`, `0x4C` bit 7 = 1, count `0x10` | Divisor 64; 1318 RPM. |
| Divisor 1, counts `0x39`, `0x3A` | Invalid (23 684 RPM), 23 275 RPM. |
| SYSTIN bytes `0x7D`, `0x7E`, `0xC9`, `0xC8`, `0x80`, `0x00` | +125 °C, unavailable, −55 °C, unavailable, unavailable, 0 °C. |
| CPUTIN `0x7D`/`0x00`, `0x7D`/`0x80`, `0xFF`/`0x00`, `0xC8`/`0x80` | +125.0 °C, unavailable, −1.0 °C, unavailable. |
| `0x18` bit 4 = 1 | No read of `0x27`; SYSTIN unavailable; other inputs unaffected. |
| Bank 1 `0x52` = `0x02` (OVTMOD) or `0x01` (STOP) | No read of bank 1 `0x50`/`0x51`; CPUTIN unavailable. |
| `0x49` = `0x02` (CPUTIN source PECI agent 1) | No bank 1 select or read for CPUTIN; on DHG AUXTIN still read. `0x49` = `0x20` on DHG: AUXTIN not read. |
| B07 `0x7C` = `0x08`, then `0x10` | SYSTIN not read; then CPUTIN not read. |
| `0x40` = `0x02`, `0x0B` or `0xFF` | Every input unavailable; no temperature or count read. |
| `0x4E` read returns `0xB0` (HBACS and both BEEP enables set) | Bank writes are `0xB1`, `0xB2`, `0xB0`, `0xB5` as needed; detection vendor writes `0xB0` then `0x30`; the last `0x4E` write of every transaction is `0xB0`. |
| `0x4D` bit 2 = 0; `CR29` = `0x02`; DHG `CR24` = `0x42` | Fan 2, Fan 4, Fan 5 respectively never read and reported as unavailable slots. |
| Base `0x0295`, `0x0292`, `0x00F8`, `0x1000`, or `CR30=0x00`, `CR22` bit 6 = 0, `CR26` bit 5 = 1 | No `ioctl_find_bars` and no HM port access; `0xAA` exit; not eligible. |
| Vendor bytes `0xFF`/`0xFF` | Path not cached; not eligible. A failed vendor-port read: path not cached; eligible. |
| A failed bank 1 select write with valid SYSTIN and fans | CPUTIN unavailable, no bank 1 read issued; other values kept. |
| Raw IDs `0x5241`, `0x8285`, `0x8860`, `0x8541`, `0xA510`, `0xB350`, `0xA230`, `0xB470` | No logical-device selection by this document and no HM read; the raw-ID diagnostic of S8 is unchanged. |
| DHG sample with CPUTIN and AUXTIN read; bank 0 `0x5D` = `0x20` and bank 2 `0x5D` = `0x00` (a different register); `0x47` bits 5:4 = `01`; SYSFANIN count `0x87` | Recorded order: bank 2 select, bank 2 reads, then `0x4E` = bank 0, then the `0x5D` and `0x59` reads. SYSFANIN divisor 32 (`101`), 312 RPM. Reading `0x5D` under bank 2 would give divisor 2 and 5000 RPM; that order fails the fixture. |
| As above, but the bank 0 `0x4E` write fails | No `0x5D`, `0x59` or count-based RPM is published; every fan is unavailable; temperatures kept. |
| Recorded transaction | Writes are only `0x87`, `0x87`, `0xAA` and register indexes to the configuration index port, `CR07=0x0B`, the module's selections inside `ioctl_find_bars`, HM register indexes, and HM data writes to `0x4E` only. No read of `0x41`, `0x42`, `0x45`, bank 4, `0x7D`, bank 6. |
| Slot 0 passes; slot 1 holds another chip | No `ioctl_select_slot(1)` after slot 0's `ioctl_find_bars`. |
| Slot 1 reads `0x8733` (likewise `0x8790`, `0x8695`, `0x8883`) | No `0xAA` or other exit write on slot 1; skip reported as in S8. |
| Mutex timeout | Sample skipped; no port I/O. |

## Quirks

- The default fan divisor 2 saturates the 8-bit count below about 2647 RPM.
  Unless firmware programs a larger divisor, ordinary case and CPU fans read
  `0xFF` and are reported unavailable; this document never changes a
  divisor. (S1 p. 44; S2 p. 39)
- S1 Table 8-1 prints `1FFh` for both −0.5 °C and −1 °C; its binary column
  gives `1,1111,1110` for −1 °C. The decode above follows two's complement,
  which matches the binary column. (S1 p. 36; S2 p. 32)
- The chip adds the bank 4 offset registers `0x54`–`0x56` (SYSTIN, CPUTIN,
  AUXTIN) to the monitored value before it appears in the reading; they are
  not read. (S1 pp. 115–116; S2 pp. 103–104; S3 p. 82)
- Sensor type (thermistor or diode) is selected by `0x5D` bits 3:1 and does
  not change the reading format. Unconnected inputs can read an in-range
  value; labels name the chip input, not a board header. (S1 pp. 36–37, 93;
  S2 pp. 32–33, 85)
- The W83527HG datasheet repeats the five-register fan sentence of the
  larger parts while marking `0x2A` reserved and listing three fan inputs;
  the register table and feature list are used. (S3 pp. 2, 27, 54)

## Safety notes

- Writes allowed, all read-transaction plumbing: configuration enter
  (`0x87`, `0x87`) and exit (`0xAA`); register indexes to the configuration
  index port; `CR07` = `0x0B`; HM register indexes to base + 5; HM data
  writes to `0x4E` only, with the values stated in the procedure, which
  preserve bits 6:3 and end each transaction by restoring the value read.
  The `ioctl_find_bars` call selects logical devices inside the module.
  (S1, S2, S3, S8, S9, S10)
- Never write any other configuration register (in particular `CR02`
  software reset, `CR22`, `CR23`, `CR24`, `CR26`, `CR29`, `CR30`,
  `CR60`/`CR61`) or any other HM register (in particular `0x40`, `0x47`,
  `0x4B`, `0x4C`, `0x4D`, `0x49`, `0x59`, `0x5D`, `0x7C`, `0x7D`, `0x18`,
  the bank 1/2 `0x52` configuration registers, divisors, limits, PWM, BEEP
  or offset registers). (S1 pp. 77–93, 105–107, 192–195; S2 pp. 71–85,
  94–99, 180–183)
- Never read the interrupt status registers listed above, never read a
  temperature register whose OVT# mode bit is set, and never select a
  logical device other than `0x0B`.
- Hold the ISA mutex across every multi-step transaction; never proceed
  unlocked. (S8, S9)

## Candidate profiles (disabled)

Recorded so that a later revision can enable them. Nothing in this section
is enabled.

| Part | Facts already pinned | Missing for enablement |
| --- | --- | --- |
| W83627DHG-P AUXTIN, AUXFANIN0, AUXFANIN1 on `0xB07x` | Registers and routing as for DHG (S1 pp. 16, 78–79, 83, 86–88, 92–93, 111–112, 119, 193) | A runtime fact that distinguishes W83627DHG-P from W83527HG, or a dump showing which part a board carries. |
| W83667HG-A / W83667HG-B (`0xB35X`) | `CR20`/`CR21` = `0xB3`/`5Xh` (S4 p. 205); readings at `0x27` (+ `0x7C` bit 7) and banks 1–2 are fan temperature sources (S4 pp. 85, 105–106); `0x41`, `0x42`, `0x45` read clear (S4 p. 306) | Source-select register analysis for the three reading slots, and resolution of the S4 / S13 / S14 label conflict. The `0x7D`/`0x7E` report path is excluded because it needs a select write. |
| W83627EHF / EHG (`0x886X`) | Chip ID and HM chip ID (S7 pp. 71, 92) | Register analysis of S7; the S13 `0x885X` claim. |

## Open questions

- Non-blocking for #2523: chip-ID ambiguity at `0xB07x`. S1 and S3 assign
  the same `CR20`/`CR21` to W83627DHG-P and W83527HG. The B07 rows read only
  inputs both datasheets document at the same registers; AUXTIN, AUXFANIN0
  and AUXFANIN1 stay disabled until a distinguishing fact or a dump settles
  the part.
- Non-blocking for #2523: saturated counts. The datasheets do not state a
  stopped-fan value for the 8-bit count. `0xFF` is reported unavailable,
  which can hide a stopped fan but never shows a spinning fan as 0 RPM. A
  dump from a board whose firmware programs large divisors could justify a
  0 RPM mapping for those divisors.
- Non-blocking for #2523: `0x7C` MNTEMP1_SEL / MNTEMP2_SEL. S1 and S3 name
  the selection but not which reading registers it redirects. The rows treat
  a set bit as redirecting SYSTIN or CPUTIN to PECI1 and skip that input,
  which can only lose a value.
- Non-blocking for #2523: the OVT# figure says temperature reads reset the
  OVT# interrupt in interrupt mode, while the text says reading the
  interrupt status registers resets it. The gates skip temperature reads in
  interrupt mode, which satisfies both readings.
- Non-blocking for #2523: bank 6 speed registers (S1 pp. 119–122; S3
  pp. 85–87) are named "(RPM)" but carry no unit, update or validity rule,
  and S4 reserves part of the range on a sibling. They are not read.
- Non-blocking for #2523: the bank-select restore. Whether firmware that
  touches the hardware monitor without the ISA mutex assumes a particular
  bank is unknown; restoring the value read at the start of each
  transaction leaves the register as found.
- Non-blocking for #2523: W83627DHG IC versions. S2 covers the UBC, UBE and
  UBF versions and states `CR21` = `2Xh` without mapping X to them. The row
  accepts every X; the gates bound the effect, and a dump can record the
  observed value.
- Non-blocking for #2523: no hardware dump exists for any row. A row becomes
  Verified only when a maintainer-accepted dump confirms the chip ID,
  `CR22`/`CR26`/`CR30`, base, vendor ID, `0x40`, the source and OVT# gate
  registers, the readings, and the routed fan counts with their divisors.
- Non-blocking for #2523: deferred targets (W83627HF, W83627THF,
  W83627EHF, W83667HG, W83667HG-B, W83687THF) need a linked follow-up issue;
  until one exists they remain tracked by #2523.
- Non-blocking for #2523: The Retro Web counts (S16) are a catalog of
  documented boards, not shipment data, and the W83627DH entry merges DHG
  and DHG-P. They order the work and decide no gate.

## Implementation-ready transition checklist

- No unresolved provenance marker remains; every enabled fact is pinned to a
  printed page of S1, S2 or S3, and every access fact to S8–S11.
- Lead-only sources S13–S15 carry `lead-only (copyleft)`; no enabled row
  depends on them, and no row is Verified.
- Every runtime condition in Detection carries a gate kind; no evidence
  identity gates a read; rows are keyed by chip-ID range and input mechanism.
- Every open question is annotated non-blocking.
- Revision, status and history record the change. The flip to
  Implementation-ready takes effect with maintainer approval of the PR that
  introduces this revision.

Implementation PRs pin this revision, for example:

```text
Implemented from docs/specs/sensors/superio-winbond-w83627dhg.md revision 1 (commit <sha>), limited to the DHG and B07 rows marked enabled.
Implemented from docs/specs/sensors/superio-access.md revision 5 (commit <sha>), limited to the configuration access facts, the slot-1 exit withhold and the sampling-path rediscovery cadence.
No other external sensor documentation was used.
```

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-29 | Initial version for #2523. Ranks every #2523 target by The Retro Web catalog count and primary-source availability. Experimental W83627DHG (`0xA020`–`0xA02F`: SYSTIN, CPUTIN, AUXTIN, fans 1–5) and shared W83627DHG-P / W83527HG (`0xB070`–`0xB07F`: SYSTIN, CPUTIN, SYSFANIN, CPUFANIN0, CPUFANIN1) from the vendor datasheets, behind `CR26` LOCKREG, `CR22`/`CR23` power, LDN B active, aligned-base, vendor-ID, Start/INT_Clear, per-input PECI source, MNTEMP, STOP, OVT# interrupt-mode and per-fan routing gates. Bank-select writes preserve the BEEP bits and are restored. Saturated counts are unavailable. Interrupt status registers are never read. W83627HF, W83627THF and W83627EHF are deferred; W83667HG, W83667HG-B and W83687THF are Unsupported. |
