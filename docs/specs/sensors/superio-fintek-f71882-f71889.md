# Spec: Fintek F71882 / F71889 Super I/O hardware monitor (temperatures and fan RPM)

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Windows PawnIO `LpcIO` read-only motherboard temperatures and fan RPM on Fintek Super I/O parts whose hardware monitor (logical device 4) has three 1 °C temperature bytes at `0x72`/`0x74`/`0x76` and MSB-latched fan counts at `0xA0`+`0x10`·n. Enabled, both Experimental and both backed by a complete vendor datasheet: exact raw chip ID `0x0541` (F71882, temperatures TEMP1–3 and fans FAN1–4) and exact raw chip ID `0x0723` (F71889, temperatures TEMP1–3 and fans FAN1–3). Recorded but disabled: every other Fintek ID, including the #2522 targets F71808E, F71811, F71858, F71862, F71869, F71869A, F71878AD, F71889ED and F71889A (see the disposition table). Excludes every write other than the read-transaction plumbing listed in Safety notes, voltages, PECI/TSI/SMBus digital temperatures, fan/PWM control, limits, alarms, beeps, and the Linux and ACPI paths. |
| Issue phase | #2522 (epic #2510) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Fintek (Feature Integration Technology Inc.), *F71882 Super Hardware Monitor + LPC I/O* datasheet, Version V0.29P, release date July 2009. Copy hosted by The Retro Web: `https://theretroweb.com/chip/documentation/fintek-f71882fg-644d579cc597f534919963.pdf`, fetched 2026-11-28, SHA-256 `d70fc91e4284496a0834b0dc49653b0197965b4a1293a2060b4efe5680ffa4a1`. Printed page = PDF page − 6. Pins (printed pages): pin descriptions §6.6 p. 10; FDC read-clear status bits p. 16; temperature ranges §7.5 p. 37; fan count formula p. 39; saturated count `0xFFF` p. 45; configuration protocol §8 p. 51; LDN, chip ID and vendor ID §8.1.2–8.1.6 p. 55; software power down `CR25` §8.1.7 p. 56; ROM address select `CR27` §8.1.9 pp. 56–57; `CR28` §8.1.10 p. 57; `CR2B` §8.1.13 p. 58; HM logical device `CR30`/`CR60`/`CR61` §8.6.1 p. 68; HM `0x01` §8.6.2.1 and `0x03` §8.6.2.3 p. 69; HM `0x0A` §8.6.2.4 p. 70; HM `0x11` §8.6.2.13 p. 72; HM `0x61` §8.6.2.18 pp. 73–74; HM `0x62` §8.6.2.19 p. 74; HM `0x6B` §8.6.2.22 pp. 75–76; HM `0x6F` §8.6.2.25 p. 76; temperature table `0x70`–`0x8F` pp. 76–77; HM `0x91` §8.6.2.28 pp. 78–79; HM `0x92` §8.6.2.29 p. 79; fan 1 `0xA0`/`0xA1` p. 84; fan 2 `0xB0`/`0xB1` p. 87; fan 3 `0xC0`/`0xC1` p. 91; fan 4 `0xD0`/`0xD1` p. 94; ordering information §10 p. 119. | Primary (vendor datasheet; third-party-hosted copy pinned by hash). |
| S2 | Fintek, *F71889 Super Hardware Monitor + LPC I/O* datasheet, Version V0.28P, release date April 2010. Copy hosted by The Retro Web: `https://theretroweb.com/chip/documentation/f71889-v0-28p-644e44017ae64530567520.pdf`, fetched 2026-11-28, SHA-256 `4628125a331da06a29e9750eb2f0508e32fb39923d6b3a9315c90b563ea1df66`. Printed page = PDF page − 7. Pins (printed pages): pin descriptions §6.6 p. 12; temperature range p. 17; fan count formula p. 19; saturated count `0xFFF` pp. 24–25; T1 architecture and HM `0x01` §7.5.1 pp. 25–26; HM `0x03` §7.5.3 p. 26; HM `0x0A` §7.5.5 p. 27; HM `0x11` §7.5.12 p. 29; HM `0x61` §7.5.17 p. 30; HM `0x62` §7.5.18 p. 31; HM `0x6F` §7.5.24 p. 32; temperature table `0x70`–`0x8F` pp. 32–33; HM `0x7E` §7.5.25 pp. 33–34; HM `0x7F` §7.5.26 p. 34; HM `0x91` §7.5.29 p. 35; HM `0x9F` §7.5.40 p. 38; fan 1 `0xA0`/`0xA1` and the 2's complement boundary bytes p. 39; fan 2 `0xB0`/`0xB1` p. 41; fan 3 `0xC0`/`0xC1` p. 44; HM `0xEC` bank select and `0xEE` SMBus status auto-clear p. 50; HM chip ID and vendor ID `0x5A`/`0x5B`/`0x5D`/`0x5E` §7.5.87–7.5.90 p. 51; configuration protocol §8 p. 61; LDN, chip ID, vendor ID, `CR25` §8.1.2–8.1.7 p. 65; `CR27` §8.1.9 p. 66; `CR2B` §8.1.12 pp. 67–68; HM logical device §8.6.1 p. 74; key-protected registers (VID device) p. 88; ordering information §10 p. 99. | Primary (vendor datasheet; third-party-hosted copy pinned by hash). |
| S3 | Fintek, *F71858 Hardware KBC with ACPI, Temp. and Fan Control* datasheet, Version V0.26P, July 2007. `https://theretroweb.com/chip/documentation/f71858-642af30e35c4f351221860.pdf`, fetched 2026-11-28, SHA-256 `537077326dd5eefc61baac6157543c582819ab6972322ac431a703921fa80c2c`. PDF page = printed page. Pins: temperature formats pp. 18–19; configuration protocol and exit keys p. 30; chip ID p. 30; §7.4 "Hardware Monitor Registers (Index port: 0x295; Data port: 0x296)", LDN 2 and HM base default `0x0295` p. 36. | Primary for a sibling part. Used for the F71858 disposition and as corroboration of the HM port layout at the default base; it is not an F71882 or F71889 source. |
| S4 | Fintek, *F71869 Super I/O + Hardware Monitor* datasheet, Version V1.1, July 2009. `https://theretroweb.com/chip/documentation/f71869-v1-1-644e438569de1692527854.pdf`, fetched 2026-11-28, SHA-256 `476135cba154cc188228181423083711845227841e41bddcae854a453a94493f`. Printed page = PDF page − 1. Pins: temperature Table 4 pp. 19–20; register summary with `CR20`/`CR21` p. 34; `CR20`–`CR24` described as Reserved §6.1.3 p. 39; `CR2B` p. 43; HM logical device p. 51; HM `0x01` p. 52; temperatures and `0x6F` p. 61; fans pp. 70, 73, 77; ordering information p. 108. | Primary for the deferred F71869 candidate profile only. |
| S5 | Fintek, *F71889A Super Hardware Monitor + LPC I/O* datasheet, Version V0.21P, Sep 2011. `https://theretroweb.com/chip/documentation/f71889a-684eefb815d6e688158353.pdf`, fetched 2026-11-28, SHA-256 `378ba40ffc332942788d6a0ad2c44240bf17240d59e09bb55daa5a0844eae637`. States global `CR20`/`CR21` = `0x10`/`0x05` and HM chip/vendor ID registers `0x5A`/`0x5B`/`0x5D`/`0x5E`. | Primary for the deferred F71889A candidate only; not analysed beyond identity in this revision. |
| S6 | Fintek, *F71868A Super I/O + Hardware Monitor* datasheet, Revision V0.17P, May 2015. `https://theretroweb.com/chip/documentation/sio-fintek-f71868a-datasheet-67a8e2d464c2c121022287.pdf`, fetched 2026-11-28, SHA-256 `defc2eb22422d3d3415d8e7610f169388c459cc9bfd4b5383acaa3c64a48aebf`. States global `CR20`/`CR21` = `0x11`/`0x06` and vendor ID `0x19`/`0x34`, PDF p. 62. | Primary for the deferred F71868A candidate only. |
| S7 | Fintek, *F81866A 6 UARTs Super IO* datasheet, Version V0.12P, Jan 2012. `https://theretroweb.com/chip/documentation/f81866-65be9af75043b934046746.pdf`, fetched 2026-11-28, SHA-256 `f28ae6428ffeb7fefc13adf9a2bccbf37935b955c5c4c7b81f71b81b92083334`. States `CR20`/`CR21` = `0x10`/`0x10` (PDF p. 117) and temperature readings at HM `0x70`, `0x72`, `0x74` (PDF p. 60). | Primary for the F81866A disposition only (not a #2522 target). |
| S8 | Fintek, *F71808E* datasheet V0.19P, Dec 2009, three-page excerpt (general description and feature list only) hosted by datasheet.su: `https://pdf5.datasheet.su/feature-integration-technology-inc-f71808e_c7e1809a3f.pdf`, fetched 2026-11-28, SHA-256 `78ad3d3a5489aaa89772e5f808f2e441ea6c1d94e1731a0eb75965a3caa47c33`. States two thermal inputs and three fan speed inputs. | Primary excerpt; no register map. |
| S9 | [`superio-access.md`](superio-access.md) revision 4 | Clean-room source for the Winbond/Nuvoton-lineage configuration keys (`0x87`, `0x87` / `0xAA`), the slot mapping, chip-ID reads at `0x20`/`0x21`, absent-ID classification, the slot-1 ITE exit-withhold rule and the ISA mutex. |
| S10 | [`pawnio-interface.md`](pawnio-interface.md) revision 9, `LpcIO` section | Interface source: `ioctl_select_slot`, `ioctl_find_bars`, `ioctl_superio_inb`/`outb`, `ioctl_pio_inb`/`outb`; port access limited to the configuration pair and the 8-byte-aligned BAR windows; caller-held `Global\Access_ISABUS.HTP.Method`. |
| S11 | PawnIO.Modules `LpcIO.p` at tag `0.2.11` (commit `52a7e536dff3e53c96917a28caac5e0fa6510696`), LGPL-2.1-or-later | Upstream-published interface definition of the module this project calls. `ioctl_find_bars` reads global `0x20` and fails if it is `0x00` or `0xFF`; for every logical device it records the `0x60/61` and `0x62/63` values that read the same twice, are non-zero, not `0xFFFF` and at least `0x100`; a value whose low three bits are `101b` is first rounded down to its 8-byte boundary; a port is allowed only if the port rounded down to 8 bytes equals a recorded value. `ioctl_select_slot` first resets the module state, clearing the configuration port and the recorded BAR list, so any slot selection after `ioctl_find_bars` withdraws that authorization. Used for port-authorization behaviour only; no code copied. |
| S12 | [`superio-nuvoton-nct67xx.md`](superio-nuvoton-nct67xx.md) revision 9 and [`superio-nuvoton-nct6775-nct6776-nct610x.md`](superio-nuvoton-nct6775-nct6776-nct610x.md) revision 1 | Project sources for the shared Super I/O procedure: `ioctl_find_bars` after configuration-mode entry and base discovery, the unavailable-slot rule for position-keyed consumers, and the structure of the read-to-clear register list. |
| S13 | [ADR 0011](../../adr/0011-experimental-sensor-enablement.md), [ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md), [ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md) | Project policy. No hardware facts. |
| S14 | Linux kernel `drivers/hwmon/f71882fg.c` (torvalds/linux commit `7cd8495c0f16275c4f9abe1cef09a7b5fcd6d046`, retrieved 2026-11-28), GPL-2.0 | **lead-only (copyleft)**. Detection requires manufacturer ID `0x1934` at `CR23`/`CR24` before matching the chip ID; IDs `0x0901` F71808E, `0x1001` F71808A, `0x0507` and `0x0903` F71858, `0x0601` F71862, `0x1106` F71868A, `0x0814` F71869, `0x1007` F71869A, `0x0541` F71882, `0x0723` F71889 (FG), `0x0909` F71889ED, `0x1005` F71889A, `0x1010` F81866A; hardware monitor is logical device 4 (logical device 2 on F71858), requires `CR30` bit 0, refuses base `0`, clears the low three base bits and uses base + 5 / + 6 as index/data; refuses the device when HM `0x01` bit 2 is set or bits 1:0 are both clear; decodes the non-F71858 temperature bytes as signed; fan speed is 1.5 × 10⁶ / count; F71808E has two temperatures and three fans, F71882 four fans. Restated in this document's own words; no code, structure or identifiers copied. |
| S15 | Linux kernel `Documentation/hwmon/f71882fg.rst` (same commit), GPL-2.0 | **lead-only (copyleft)**. Groups F71882FG with F71883FG, F71869F with F71869E, F71862FG with F71863FG; states that the F81801U is the 64-pin variant of the F71889FG with the same device ID; lists the F71808E, F71808A and F71869A datasheets as not public. |
| S16 | LibreHardwareMonitor `LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/` `Chip.cs`, `LpcIO.cs`, `F718XX.cs` (commit `2808d80564b6761e1b01f42fabfd966ed695fcbc`, the #2522 comparison inventory, retrieved 2026-11-28), MPL-2.0 | **lead-only (copyleft)**. Chip-ID table agrees with S14 for `0x0541`, `0x0723`, `0x0814`, `0x0901`, `0x0909`, `0x1005`, `0x0507`, `0x0601`; labels `0x1007` "F71869A/F71811", labels `0x1106` F71878AD and describes it as the F71868A, and maps `0x1118` to F71811; hardware monitor at logical device 4; rounds a base whose low three bits are `101b` down to 8 bytes and uses base + 5 / + 6; reads fan counts MSB then LSB and treats counts at or above `0x0FFF` as stopped; reads two temperatures on F71808E. Restated; nothing copied. |
| S17 | The Retro Web motherboard catalog, listings filtered by chip, retrieved 2026-11-28: F71882 (chip 5066) 73 boards, F71889F (5073) 17, F71868AD (5883) 9, F71889A (6154) 8, F71858 (4872) 5, F71869 (5071) 2 | Public catalog counts. Used only to rank targets; not a hardware fact and not a market-share measure. |

S14–S16 are copyleft implementations and documentation consulted as leads
under ADR 0023. Every fact taken from them is restated, carries its source
ID, and backs only rows whose default enablement is Experimental, or is
recorded in Open questions. The implementer works from this document only
and does not open S14–S16.

## Target ranking and disposition (#2522)

Ranking uses the S17 catalog count as an installed-base proxy and then
primary-source availability. The rev 1 slice is the two highest-ranked
targets whose complete datasheet covers every register this document reads.

| Rank | Target (raw ID) | S17 boards | Primary source | Disposition in this revision |
| --- | --- | --- | --- | --- |
| 1 | F71882 / F71882FG (`0x0541`) | 73 | S1, complete | **Enabled, Experimental**: TEMP1–3, FAN1–4 |
| 2 | F71889 / F71889F (`0x0723`) | 17 | S2, complete | **Enabled, Experimental**: TEMP1–3, FAN1–3 |
| 3 | F71868A / F71868AD (`0x1106`), not a #2522 target by name | 9 | S6, complete | Unsupported in rev 1; candidate below. It is the follow-up (#2815) that would cover the F71878AD label. |
| 4 | F71889A / F71889AD (`0x1005`) | 8 | S5, complete | Unsupported in rev 1; follow-up candidate in #2815 (S5 obtained, register analysis pending). |
| 5 | F71858 (`0x0507`) | 5 | S3, complete | Unsupported: different mechanism (logical device 2, exit keys `0x78`×2 or `0xAA`, two-byte temperature formats with a table select). Follow-up profile in #2815. |
| 6 | F71869 (`0x0814`) | 2 | S4, complete | Unsupported in rev 1; candidate profile recorded below. S4 describes `CR23`/`CR24` as reserved, so the vendor-ID gate would rest on S14 alone. |
| — | F71878AD | no separate page found | none | Unsupported. No datasheet. S16 labels raw `0x1106` F71878AD and calls it the F71868A; S6 assigns `0x1106` to F71868A. The alias is lead-only, so the F71868A follow-up in #2815 covers the ID under its primary label. |
| — | F71889ED / F71889E (`0x0909`) | none found | none obtained | Unsupported. A V0.19P datasheet is indexed by third-party sites but could not be retrieved; IDs are lead-only (S14, S16). Follow-up: #2815. |
| — | F71869A (`0x1007`) | none found | none obtained | Unsupported. S15 lists the datasheet as not public. Follow-up: #2815. |
| — | F71811 | none found | none | Unsupported. Its ID is unresolved: S16 gives both `0x1007` (shared label with F71869A) and `0x1118`. Follow-up: #2815. |
| — | F71862 / F71862FG (`0x0601`) | none found | none obtained | Unsupported. A V0.27P datasheet is indexed by a third-party site but could not be retrieved. Follow-up: #2815. |
| — | F71808E (`0x0901`) | none found | S8, three-page excerpt | Unsupported: no register map. S15 lists the datasheet as not public. Follow-up: #2815. |
| — | F81866A (`0x1010`), not a #2522 target | not counted | S7, complete | Unsupported: its temperatures sit at `0x70`/`0x72`/`0x74`, a different layout (S7). |

"Follow-up" means #2815, which is linked from #2522 and tracks the deferred
targets until a later revision adds their rows. No similar product number or software
alias is treated as hardware identity.

## Detection

### Chip identity

| Fact | Source |
| --- | --- |
| Configuration mode is entered by writing `0x87` twice to the index port and left by writing `0xAA` to the index port. Registers are selected by writing the index to the index port and accessed at the data port. The default pair is `0x4E`/`0x4F`; a power-on strap on SOUT1 selects `0x2E`/`0x2F`, reported by `CR27` bit 4 (PORT_4E_EN). These are the same keys as the Winbond/Nuvoton lineage in S9. | S1 pp. 51, 56; S2 pp. 61, 66; S9 |
| F71882: `CR20` = `0x05`, `CR21` = `0x41` (read only). | S1 p. 55 |
| F71889: `CR20` = `0x07`, `CR21` = `0x23` (read only). | S2 p. 65 |
| Both: `CR23` = `0x19`, `CR24` = `0x34` (read only), "Vendor ID of Fintek devices". | S1 p. 55; S2 p. 65 |
| F71889: HM index `0x5A` = `0x03` and `0x5B` = `0x04` (HM chip ID); HM index `0x5D` = `0x19` and `0x5E` = `0x34` (HM vendor ID), all read only. S1 documents no HM identity register for the F71882. | S2 p. 51 |
| Ordering part numbers: F71882FG (S1), F71889F (S2). Lead-only: S15 groups F71883FG with F71882FG and states that F81801U shares the F71889FG device ID. Such parts reporting an enabled ID are read with that row's profile and label. | S1 p. 119; S2 p. 99; S15 |

### ID-matching policy

| Raw ID (`CR20`:`CR21`) | Row | Chip label to surface |
| --- | --- | --- |
| `0x0541` | F71882 profile | `Fintek F71882` |
| `0x0723` | F71889 profile | `Fintek F71889` |
| Any other value | Outside this document | — |

Matching is exact. The raw ID is always recorded in diagnostics. A
listed ID whose vendor-ID check fails is unavailable, not reclassified.

### Runtime gates

Every condition is a mechanism or safety gate (ADR 0031). Board names, BIOS
versions, base address values and part-number suffixes are evidence
identities and never gate a read.

| Condition | Gate kind | Rows | Source |
| --- | --- | --- | --- |
| Raw chip ID matches the ID-matching table exactly | mechanism | all | S1 p. 55; S2 p. 65 |
| `CR23`/`CR24` read `0x19`/`0x34` | mechanism | all | S1 p. 55; S2 p. 65; S14 corroborates |
| `CR25` bit 4 (SOFTPD_HM) is `0`. When set, the HM clock is stopped and every reading is stale; report the chip unavailable. Never write `CR25`. | mechanism | all | S1 p. 56; S2 p. 65 |
| Logical device 4 `CR30` bit 0 (HM_EN) is `1`. Never write `CR30`. | mechanism | all | S1 p. 68; S2 p. 74 |
| HM base B = `CR60`·256 + `CR61` satisfies: B & `0xFFF8` lies in `0x0100`–`0x0FF8`, and B & `0x0007` is `0` or `5`. Otherwise report unavailable; do not work around. S11 authorizes exactly these two forms (any other low-bit pattern is recorded but never matches a port) and records no window that overlaps `0xCF8`–`0xCFF`, so such a base also fails authorization. The `0x0FF8` upper bound is the conservative project bound of the Nuvoton specifications (S12). | safety | all | S1 p. 68; S2 p. 74; S10; S11; S14, S16 (lead-only, port derivation) |
| F71889: HM `0x5D`/`0x5E` read `0x19`/`0x34` through the HM ports before the path is cached | mechanism | F71889 | S2 p. 51 |
| HM `0x01` bit 2 (POWER_DOWN) is `0`; bit 0 (V_T_START) is `1` for temperatures; bit 1 (FAN_START) is `1` for fans. A zero start bit "puts the part in standby mode". A read of `0xFF` (no responder) fails the bit 2 test. Never write `0x01`. | mechanism | all | S1 p. 69; S2 pp. 25–26; S14 corroborates |
| Per temperature n: HM `0x6F` bit n (Tn_DIODE_OPEN) is `0` | mechanism | all | S1 p. 76; S2 p. 32 |
| F71882 TEMP1: HM `0x0A` bits 1:0 (MEAS_TYPE) are `00` (external diode). `01` (PECI) and `10` (AMDSI) change what TEMP1 reports. | mechanism | F71882 | S1 p. 70 |
| F71889 TEMP1: HM `0x0A` bit 0 (DIODE_T1_EN) is `1`, and either bit 1 (DIG_T1_EN) is `0` or HM `0x7F` bits 1:0 (T1_SRC_SEL_REG) are `00`. Any other combination sources TEMP1 from PECI/TSI, an average or a maximum. | mechanism | F71889 | S2 pp. 25, 27, 34 |
| Per fan input: its pin is routed to the tachometer (fan table below) | mechanism | all | S1 pp. 10, 56–58; S2 pp. 12, 67–68 |

No configuration-register lock and no HM I/O-space lock is documented for
either part: the Nuvoton `CR26` LOCKREG and `CR28` lock steps have no
counterpart and are not applied. The only key-protected registers are in the
VID device (S2 p. 88), which this document never selects. The HM stop and
power-down bits above are the gating bits that matter. (S1 pp. 55–58, 68–69;
S2 pp. 25–26, 65–68, 88)

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| F71882 (`0x0541`) TEMP1–3 | **Experimental**. Every identity, gate, register and decode is pinned to S1. Lead-only: the HM index/data port derivation (S14, S16; S3 corroborates at the default base). No hardware dump. | Enabled as Experimental |
| F71882 (`0x0541`) FAN1–4 | **Experimental**, S1 for registers, routing, latch and formula. Lead-only: the same HM index/data port derivation (S14, S16). | Enabled as Experimental, per input behind its routing gate |
| F71889 (`0x0723`) TEMP1–3 | **Experimental**, S2 for every identity, gate, register and decode, plus the S2 HM vendor ID. Lead-only: the HM index/data port derivation (S14, S16). Best-evidenced member: none (no dump). | Enabled as Experimental |
| F71889 (`0x0723`) FAN1–3 | **Experimental**, S2 for registers, routing, latch and formula. Lead-only: the HM index/data port derivation (S14, S16). | Enabled as Experimental, per input behind its routing gate |
| F71882 temperature bytes `0x80`–`0xFF` | S1 gives −20 to +127 °C for one input and 0 to +145 °C for the other two without stating the encoding; S14 decodes all three as signed (lead-only, conflicts with the 0 to +145 °C statement). | Disabled (reported unavailable) |
| Digital temperatures (PECI, AMDSI/TSI, SST, PCH SMBus) at HM `0x7A`–`0x7D` and `0xE0`–`0xEF`, voltages, alarms, limits, PWM | Out of scope | Disabled |
| Every ID outside the ID-matching table | See the disposition table | Disabled |

## Register map (facts)

### Configuration space (read in configuration mode)

| Register | Bits | Meaning | Rows | Source |
| --- | --- | --- | --- | --- |
| `0x07` | 7:0 | Logical device number; `0x04` selects the hardware monitor | all | S1 p. 55; S2 p. 65 |
| `0x20` / `0x21` | 7:0 | Chip ID 1 / 2 (read only) | all | S1 p. 55; S2 p. 65 |
| `0x23` / `0x24` | 7:0 | Vendor ID 1 / 2, `0x19` / `0x34` (read only) | all | S1 p. 55; S2 p. 65 |
| `0x25` | bit 4 | SOFTPD_HM: `1` = HM powered down, HM clock stopped | all | S1 p. 56; S2 p. 65 |
| `0x27` | bit 4 | PORT_4E_EN: `0` = configuration port `0x2E`/`0x2F`, `1` = `0x4E`/`0x4F` (strap) | all | S1 p. 56; S2 p. 66 |
| `0x27` | bit 6 | F71882 SPI_EN: `0` = SPI disabled, `1` = enabled (strap) | F71882 | S1 p. 56 |
| `0x28` | bit 0 | F71882 GPIO40_SEL: `0` = pin 25 is FANIN3, `1` = GPIO40 | F71882 | S1 p. 57 |
| `0x2B` | bit 0 | F71882 GPIO10_SEL: with SPI disabled, `0` = pin 59 is GPIO10, `1` = FANIN4 | F71882 | S1 p. 58 |
| `0x2B` | bits 1:0 | F71889 GPIO10_SEL: `00` = pin 25 is FANIN3, `01` = IRRX1, `10` reserved, `11` = GPIO10 | F71889 | S2 pp. 67–68 |
| LDN 4 `0x30` | bit 0 | HM_EN: `1` = hardware monitor enabled (default `1`) | all | S1 p. 68; S2 p. 74 |
| LDN 4 `0x60` / `0x61` | 7:0 | HM base address MSB / LSB, default `0x02` / `0x95` | all | S1 p. 68; S2 p. 74 |

FANIN1 (pin 21) and FANIN2 (pin 23) are dedicated tachometer inputs on both
parts. (S1 p. 10; S2 p. 12)

### Hardware-monitor ports

| Item | Fact | Source |
| --- | --- | --- |
| Index port | (B & `0xFFF8`) + 5 | S14, S16 (lead-only); S3 p. 36 shows index `0x295` at base `0x0295` on the F71858 |
| Data port | (B & `0xFFF8`) + 6 | S14, S16 (lead-only); S3 p. 36 shows data `0x296` |
| Banking | None for the registers read here. S1 documents no HM bank selector. On F71889, `0x9F` bit 7 (FAN_RATE_PROG_SEL) changes only the meaning of `0x9B`, and `0xEC` bit 7 (MCH_BANK_SEL) selects between two banks of `0xE0`–`0xE9`; neither range is read and neither selector is written. | S1 pp. 69–94; S2 pp. 38, 50 |

At the default base `0x0295` both readings of the S3 statement (index = B,
or index = (B & `0xFFF8`) + 5) give ports `0x295`/`0x296`.

### HM registers read

| Index | Name (vendor mnemonic) | Bits | Meaning | Rows | Source |
| --- | --- | --- | --- | --- | --- |
| `0x01` | Configuration | 2 / 1 / 0 | POWER_DOWN / FAN_START / V_T_START (defaults 0 / 1 / 1). F71889 bits 7:5 are BETA_EN, INTEL_MODEL, TSI_EN and are not gates. | all | S1 p. 69; S2 pp. 25–26 |
| `0x0A` | PECI AMDSI Select | 1:0 | MEAS_TYPE: `00` external diode, `01` PECI, `10` AMDSI, `11` reserved | F71882 | S1 p. 70 |
| `0x0A` | SST and VTT_SEL | 1 / 0 | DIG_T1_EN (default 0) / DIODE_T1_EN (default 1) | F71889 | S2 p. 27 |
| `0x5D` / `0x5E` | HM Vendor ID 1 / 2 | 7:0 | `0x19` / `0x34` | F71889 | S2 p. 51 |
| `0x6F` | DIODE OPEN Status (read only) | 3 / 2 / 1 | T3 / T2 / T1 open. Bit 1 reports PECI/TSI errors instead when those interfaces feed TEMP1; with external diode it reports an open or shorted BJT. | all | S1 p. 76; S2 p. 32 |
| `0x72` / `0x74` / `0x76` | Temperature 1 / 2 / 3 reading (read only) | 7:0 | 1 °C per LSB. Inputs D1+ (CPU, pin 91), D2+ (pin 90), D3+ (System, pin 89). | all | S1 pp. 10, 77; S2 pp. 12, 32–33 |
| `0x7F` | T1_SRC_SEL_REG (the datasheet heading reads "Temperature Filter Select Register") | 1:0 | `00` D1+ only, `01` digital (PECI/TSI), `10` average, `11` maximum; used only when DIODE_T1_EN and DIG_T1_EN are both 1 | F71889 | S2 p. 34 |
| `0xA0` / `0xA1` | FAN1 count MSB / LSB (read only) | 7:0 | Defaults `0x0F` / `0xFF`. Reading the MSB latches the LSB; read MSB first, then LSB. | all | S1 p. 84; S2 p. 39 |
| `0xB0` / `0xB1` | FAN2 count MSB / LSB | 7:0 | Same | all | S1 p. 87; S2 p. 41 |
| `0xC0` / `0xC1` | FAN3 count MSB / LSB | 7:0 | Same | all | S1 p. 91; S2 p. 44 |
| `0xD0` / `0xD1` | FAN4 count MSB / LSB | 7:0 | Same | F71882 | S1 p. 94 |

| Fact | Source |
| --- | --- |
| Fan speed: RPM = 1.5 × 10⁶ / count. The datasheets recommend fans with two tachometer pulses per revolution. | S1 p. 39; S2 p. 19 |
| A count of `0xFFF` is the power-on default and the value the fan-fault description gives for a fan that does not turn while driven. 1.5 × 10⁶ / `0xFFF` = 366.3 RPM. | S1 pp. 45, 84; S2 pp. 24–25, 39 |
| Temperature ranges: F71882 "one of these sensors can be measured from −20 °C to 127 °C, the others from 0 °C to 145 °C (2 sets)". F71889: −40 °C to 127 °C; its temperature boundary bytes are described as 2's complement values from −128 to 127 °C. | S1 p. 37; S2 pp. 17, 39 |
| HM `0x6B` bits 3:1 select thermistor (`0`) or BJT (`1`, default) per input. S1 describes one reading register per input and no mode-specific encoding; `0x6B` is not read. | S1 pp. 75–77 |

### Fan pin routing

| Fan | F71882 routed when | F71889 routed when | Source |
| --- | --- | --- | --- |
| FAN1 | always (pin 21) | always (pin 21) | S1 p. 10; S2 p. 12 |
| FAN2 | always (pin 23) | always (pin 23) | S1 p. 10; S2 p. 12 |
| FAN3 | `CR28` bit 0 = `0` | `CR2B` bits 1:0 = `00` | S1 p. 57; S2 pp. 67–68 |
| FAN4 | `CR27` bit 6 = `0` and `CR2B` bit 0 = `1` | (no FAN4) | S1 pp. 10, 56, 58 |

### Registers that clear (never read, never written)

| Register | Attribute | Source |
| --- | --- | --- |
| HM `0x03` bit 0 (case open), `0x11` (voltage interrupt status), `0x61` (temperature interrupt status), `0x91` (fan interrupt status) | Write 1 to clear; the datasheets document no clearing on read | S1 pp. 69, 72–74, 78–79; S2 pp. 26, 29, 30, 35 |
| F71889 HM `0xEE` SMBus status bits 4:1 | Cleared automatically by the next SMBus transfer | S2 p. 50 |
| F71882 FDC (logical device 0) I/O-space status bits | Cleared by a read | S1 p. 16 |

No HM register on either part is documented as clearing when read. The
real-time status registers `0x62` and `0x92` follow the current condition
and are not needed. This document reads none of the registers in the table
and never selects logical device 0 or touches its I/O space.

## Read procedure and decode

### Detection (cached; configuration mode)

1. Acquire `Global\Access_ISABUS.HTP.Method` with a bounded timeout; on
   failure skip the attempt. Hold it for the whole transaction. (S9, S10)
2. Visit the slots in order 0, then 1, and stop at the first slot that
   passes steps 3–6. For the slot being visited, call `ioctl_select_slot`,
   enter configuration mode with `0x87`, `0x87` and read `CR20`/`CR21`.
   Absent-ID rules of S9 apply. Match the ID-matching table; any other ID,
   or a later gate failure on that slot, ends the attempt on that slot with
   the `0xAA` exit and moves to the next slot. Exception: on slot 1, when
   the combined ID read is exactly one of S9's exit-withhold values
   `0x8733`, `0x8790`, `0x8695` or `0x8883`, no exit write of any kind is
   issued on that slot (S9 allows no substitute exit), the skip is
   reported as S9 describes, and detection ends with no Fintek path. The
   chip is left as the firmware left it. (S1, S2, S9)
3. Read `CR23`/`CR24`; require `0x19`/`0x34`. Read `CR25`; require bit 4 = 0.
   Read the routing registers of the row (`CR27`, `CR28`, `CR2B` on F71882;
   `CR2B` on F71889). (S1 pp. 55–58; S2 pp. 65–68)
4. Write `CR07` = `0x04`; read `CR30` and require bit 0 = 1. Read `CR60`,
   `CR61`; apply the base gate. (S1 p. 68; S2 p. 74)
5. Call `ioctl_find_bars` after steps 2–4, on the slot being visited.
   This is the last call in the detection sequence that changes the
   module state: no `ioctl_select_slot` follows it once the slot passes
   step 6, so the recorded BAR list is the one sampling uses. (S11, S12)
6. F71889 only: through the HM ports read `0x5D` and `0x5E`; require
   `0x19`/`0x34`. (S2 p. 51)
7. Exit configuration mode (`0xAA`) on the visited slot on every path,
   including failures, except where step 2 withholds the exit on slot 1.
   (S9)
8. While the mutex is still held, cache the slot, profile, port base and
   the set of routed fan inputs, so that any later `ioctl_select_slot` on
   the shared handle finds the cached path and invalidates it. Only then
   release the mutex, after the last slot attempt. Configuration mode is
   not entered again for sampling, and sampling issues no
   `ioctl_select_slot`. A failed routing read leaves that fan unavailable
   until rediscovery. (S12)

Both slots share the one `LpcIO` executor handle the project keeps per
module (S10). Because `ioctl_select_slot` clears the recorded BARs (S11),
any slot selection on that handle after step 5, whether by this document or
by another vendor's discovery, invalidates the cached path. The path is then
unavailable until this detection sequence runs again in full, ending with
`ioctl_find_bars` for the matching slot. A slot-1 match is used only when
slot 0 did not pass, so no slot is selected after its own BAR discovery.

### Sampling (HM ports only, under the ISA mutex)

1. Read `0x01`. If bit 2 = 1, report every input unavailable for this
   sample and stop. Temperatures need bit 0 = 1, fans need bit 1 = 1.
   (S1 p. 69; S2 p. 26)
2. Temperatures: read `0x6F`, then `0x0A` (and on F71889 `0x7F` when
   `0x0A` bits 1:0 = `11`), then `0x72`, `0x74`, `0x76`. A failed `0x6F`
   read makes every temperature unavailable; a failed `0x0A`/`0x7F` read
   makes TEMP1 unavailable. (S1, S2)
3. Fans: for each routed input read the MSB register, then the LSB
   register. A failed read makes that fan unavailable. (S1 p. 84; S2 p. 39)
4. Each HM access is an index-port write followed by one data-port read.
   No data-port write occurs. Successful unrelated inputs remain usable
   when one read fails. Release the mutex.

### Temperature decode

| Row | Byte | Reading | Basis |
| --- | --- | --- | --- |
| F71882 | `0x00`–`0x7D` | 0 to +125 °C | S1 p. 77 (1 °C units); the 0x00–0x7F values mean the same under either reading of the S1 ranges |
| F71882 | `0x7E`, `0x7F` | Unavailable | Above the +125 °C acceptance bound |
| F71882 | `0x80`–`0xFF` | Unavailable | Encoding not stated by S1 (see Open questions) |
| F71889 | signed 8-bit −40 to +125 | That value in °C | S2 pp. 17, 32–33, 39 |
| F71889 | any other byte, including `0x80` (−128) | Unavailable | Outside the S2 range or above +125 °C |

The acceptance bounds are the datasheet ranges intersected with the
project's −55 to +125 °C plausibility policy (S12). An out-of-bounds byte
is unavailable for that sample: not clamped, not zero, not a device failure.
A temperature whose gate fails is not reported as a measurement.

### Fan decode

| Count (MSB·256 + LSB) | Reading | Basis |
| --- | --- | --- |
| `0x0000` | Invalid, no RPM | Undefined by S1/S2 |
| `0x0001`–`0x003F` | Invalid, no RPM (would exceed 23 437 RPM) | Project plausibility guard, the same cutoff as [`superio-ite-primary-lpc-expansion.md`](superio-ite-primary-lpc-expansion.md) |
| `0x0040`–`0x0FFE` | RPM = floor(1 500 000 / count): 23 437 to 366 RPM | S1 p. 39; S2 p. 19 |
| `0x0FFF` | 0 RPM (no measurable rotation) | S1 pp. 45, 84; S2 pp. 24–25, 39 |
| `0x1000`–`0xFFFF` | Invalid, no RPM | Counts above `0xFFF` are not described |

A fan input whose routing gate fails is not read and not reported as a
measurement; a position-keyed consumer reports it as an explicit
unavailable slot at its position. (S12)

### Input identity and ordering

Labels are bound to physical inputs: `TEMP1` (D1+), `TEMP2` (D2+), `TEMP3`
(D3+); `FAN1`–`FAN4` = FANIN1–FANIN4 (FAN4 on F71882 only). Omitting an
input never renumbers the others. Source label: a Fintek Super I/O source
label alongside the existing Nuvoton and ITE ones; the implementation PR
verifies the new channel identities against the existing identity
contracts before use. (S1 p. 10; S2 p. 12; S12)

### Synthetic cases for implementation review

All fixtures are synthetic.

| Fixture | Expected observation |
| --- | --- |
| Slot 1, `0x0541`, vendor `0x19`/`0x34`, `CR25=0x00`, `CR27=0x50` (SPI enabled), `CR28=0x00`, `CR2B=0x00`, `CR30=0x01`, base `0x0295`; HM `0x01=0x03`, `0x0A=0x00`, `0x6F=0x00`, temperatures `0x28`, `0x1E`, `0x7D`; FAN1 `0x02`/`0xEE`, FAN2 `0x0F`/`0xFF`, FAN3 `0x00`/`0x00` | Ports `0x295`/`0x296`. TEMP1 40 °C, TEMP2 30 °C, TEMP3 125 °C; FAN1 2000 RPM, FAN2 0 RPM, FAN3 invalid; FAN4 omitted before any count read (SPI enabled) and reported as an unavailable slot. |
| Same, base `0x0290`, then `0x0A40`, then `0x0A45` | Ports `0x295`/`0x296`, `0x0A45`/`0x0A46`, `0x0A45`/`0x0A46`. |
| Same, base `0x0000`, `0x0293`, `0x00F8`, `0x1000` or `CR30=0x00` | No HM index write. |
| `0x0541` with vendor `0xFF`/`0xFF`, or `CR25` bit 4 set | No logical-device selection after the failed gate; `0xAA` exit; no HM read. |
| F71882 temperature bytes `0x7E`, `0x80`, `0xD8`, `0x00` | Unavailable, unavailable, unavailable, 0 °C. |
| F71889 temperature bytes `0xD8`, `0xD7`, `0x80`, `0x00`, `0x7D`, `0x7E` | −40 °C, unavailable, unavailable, 0 °C, +125 °C, unavailable. |
| Routed fan counts `0x0000`, `0x003F`, `0x0040`, `0x0FFE`, `0x0FFF`, `0x1000`, `0xFFFF` | Invalid, invalid, 23 437 RPM, 366 RPM, 0 RPM, invalid, invalid. |
| HM `0x01` = `0xFF`; `0x02`; `0x01` | All unavailable; temperatures unavailable and fans read; fans unavailable and temperatures read. |
| F71882 `0x0A` bits 1:0 = `01` or `10` | TEMP1 unavailable; TEMP2 and TEMP3 unaffected. |
| F71889 `0x0A` = `0x01`; `0x03` with `0x7F=0x00`; `0x03` with `0x7F=0x01`, `0x02` or `0x03`; `0x02` | TEMP1 read; read; unavailable; unavailable. |
| `0x6F` = `0x04` | TEMP2 unavailable; TEMP1 and TEMP3 keep their labels and values. |
| F71889 `0x0723` with HM `0x5D`/`0x5E` = `0xFF`/`0xFF` | Path not cached; no sampling. |
| F71889 `CR2B` bits 1:0 = `01` or `11` | FAN3 omitted before its count reads. |
| A failed FAN2 LSB read with valid FAN1 | FAN2 unavailable; FAN1 kept under its label. |
| Raw IDs `0x0814`, `0x1005`, `0x0909`, `0x1106`, `0x1007`, `0x0601`, `0x0901`, `0x0507`, `0x1010`, `0x1118`, or `0xFFFF` | No logical-device selection by this document and no HM read; the raw ID diagnostic of S9 is unchanged. |
| Recorded transaction | Writes are only `0x87`, `0x87`, `0xAA` and register indexes to the configuration index port, `CR07=0x04`, the module's own selections inside `ioctl_find_bars`, and register indexes to the HM index port. No HM data-port write; no read of `0x03`, `0x11`, `0x61`, `0x91`, `0x62`, `0x92` or `0xE0`–`0xEF`. |
| Slot 0 reports `0x0541` and passes every gate; slot 1 holds another chip | Detection ends on slot 0: no `ioctl_select_slot(1)` after slot 0's `ioctl_find_bars`, and sampling reads ports `0x295`/`0x296` without any slot selection. |
| Slot 0 reports `0x0541` but fails the base gate; slot 1 reports `0x0723` and passes | Slot 0 ends with `0xAA` without BAR discovery; `ioctl_select_slot(1)` precedes the only `ioctl_find_bars` whose BARs are cached. |
| Any `ioctl_select_slot` on the shared handle after the path is cached | Cached path invalidated; no HM read until the full detection sequence has run again. |
| Slot 0 does not pass; slot 1 reads `0x8733` (likewise `0x8790`, `0x8695`, `0x8883`) | No `0xAA` or other exit write on slot 1; the skip is reported as in S9; no Fintek path. |
| Mutex timeout | Sample skipped; no port I/O. |

## Quirks

- F71882 TEMP2 and TEMP3 are specified up to +145 °C, so a hot input can
  read above the accepted byte range and is reported unavailable. (S1 p. 37)
- On F71889 the chip can apply a firmware-set slope adjustment (HM `0x7E`
  bits 3:0) and beta compensation (`0x01` bit 7) to the D1+ reading before
  it appears in `0x72`. They are not read; the published value is the
  chip's reading. (S2 pp. 25, 33–34)
- Unconnected thermistor inputs can read in range; the diode-open bit and
  the bounds only remove what the chip reports as open or out of range.
  Labels name the chip input, not a board header. (S1 pp. 37, 75–76)
- The RPM formula assumes two tachometer pulses per revolution. (S1 p. 39;
  S2 p. 19)

## Safety notes

- Writes allowed, all read-transaction plumbing: configuration enter
  (`0x87`, `0x87`) and exit (`0xAA`) on the selected index port; register
  indexes written to the configuration index port; `CR07` = `0x04`; HM
  register indexes written to the HM index port. The `ioctl_find_bars`
  call selects logical devices inside the module. (S1, S2, S9, S10, S11)
- Never write any other configuration register (in particular `CR02`
  software reset, `CR25`, `CR27`, `CR28`, `CR2B`, `CR30`, `CR60`/`CR61`),
  any HM register (including the F71889 `0x9F` and `0xEC` selectors), any
  status register, limit, PWM, fan mode, PECI or SMBus register. (S1 pp.
  55–58, 68–70; S2 pp. 25–27, 38, 50, 65–68)
- Never read the clearing registers listed above. Never select logical
  devices other than 4.
- Hold the ISA mutex across every multi-step transaction; never proceed
  unlocked. (S9, S10)

## Candidate profiles (disabled)

Recorded so that a later revision can enable them. Nothing in this section
is enabled.

| Part | Facts already pinned | Missing for enablement |
| --- | --- | --- |
| F71869 (`0x0814`) | `CR20`/`CR21` = `0x08`/`0x14` in the register summary (S4 p. 34); LDN 4 `CR30` bit 0, base default `0x0295` (p. 51); HM `0x01` bits 2:0 as above (p. 52); `0x6F`, `0x72`/`0x74`/`0x76` with Table 4 giving −40 to +127 °C in 2's complement and `0x80` = open (pp. 19–20, 61); fans `0xA0`/`0xB0`/`0xC0` MSB-latched (pp. 70, 73, 77); FAN3 routed when `CR2B` bit 0 = `0` (default `1`, p. 43) | S4 §6.1.3 (p. 39) lists `CR20`–`CR24` as Reserved, so the vendor-ID gate is lead-only (S14); no `CR25` HM power-down bit is described. |
| F71889A (`0x1005`) | Identity and HM vendor ID registers (S5) | Register analysis of S5. |
| F71868A (`0x1106`) | Identity and vendor ID (S6 PDF p. 62); temperatures and fans at the same indexes (S6 PDF pp. 88, 98) | Register analysis of S6; label policy for the F71878AD alias (S16). |
| F71858 (`0x0507`) | S3 | A separate profile: logical device 2, different exit keys and two-byte temperatures. |

## Open questions

- Non-blocking for #2522: HM port derivation. S1 and S2 give only the
  default base `0x0295`. The index = (B & `0xFFF8`) + 5 rule for other
  bases rests on S14 and S16 (lead-only); S3 (F71858, p. 36) and the S11
  rounding of `101b` bases corroborate it. An Experimental row may rest on
  it (ADR 0023); a maintainer-accepted dump from a board whose base is not
  `0x0295` would settle it.
- Non-blocking for #2522: F71882 temperature encoding above `0x7F`. S1
  states −20 to +127 °C for one input and 0 to +145 °C for the other two
  without naming the inputs or the encoding; S14 decodes all three as
  signed, which conflicts with the 0 to +145 °C statement. The row
  publishes only `0x00`–`0x7D`, which both readings agree on. A dump with
  a known hot or negative input would settle it.
- Non-blocking for #2522: counts above `0x0FFF`. The defaults (`0x0F`/`0xFF`)
  and the `0xFFF` fan-fault statement imply a 12-bit counter, but S1/S2 do
  not state the width. Such counts are reported invalid.
- Non-blocking for #2522: diode-open bits with a thermistor input. S1/S2
  describe `0x6F` for diodes. If a thermistor input reports open, the
  input is unavailable, which loses a value but never publishes a wrong one.
- Non-blocking for #2522: F71889 TEMP1 diode-open bit with the digital
  interface enabled but not selected (`DIG_T1_EN` = 1, `T1_SRC_SEL_REG` =
  `00`). S2 lets bit 1 report PECI/TSI errors; such an error makes TEMP1
  unavailable, which is the safe direction.
- Non-blocking for #2522: identity conflicts in the leads for disabled IDs.
  `0x1106` is F71868A in S6 and S14 and F71878AD in S16; `0x1007` is
  F71869A in S14 and "F71869A/F71811" in S16, which also maps `0x1118` to
  F71811. S6 wins for `0x1106`; the others stay Unsupported until a
  datasheet or dump settles them.
- Non-blocking for #2522: S4 lists `CR20`/`CR21` values in its register
  summary (p. 34) but describes `CR20`–`CR24` as Reserved in the register
  detail (p. 39). This affects only the disabled F71869 candidate.
- Non-blocking for #2522: parts sharing an enabled ID (F71883FG with
  `0x0541`, F81801U with `0x0723`) are lead-only (S15). They are read with
  the row's profile; the vendor-ID, start, diode-open, source, routing and
  bounds gates limit the effect, and a dump can add them as evidenced
  members.
- Non-blocking for #2522: no hardware dump exists for either row. A row
  becomes Verified only when a maintainer-accepted dump confirms the chip
  and vendor IDs, `CR25`, LDN 4 `CR30`, base, port behaviour, HM `0x01`,
  `0x0A`, `0x6F`, the temperature bytes and the routed fan counts, and the
  port derivation no longer rests on the leads.
- Non-blocking for #2522: The Retro Web counts (S17) are a catalog of
  documented boards, not shipment data. They order the work and decide no
  gate.

## Implementation-ready transition checklist

- No unresolved provenance marker remains; every F71882 fact is pinned to a
  printed page of S1, every F71889 fact to S2, and every access fact to
  S9–S12.
- Every lead-only fact is tagged (S14–S16 carry `lead-only (copyleft)`):
  the HM port derivation backs the two Experimental rows and is named in
  their Status; the remaining lead facts only corroborate or concern
  disabled IDs. No row is Verified.
- Every runtime condition in Detection carries a gate kind; no evidence
  identity gates a read; rows are keyed by chip ID.
- Every open question is annotated non-blocking.
- Revision, status and history record the change. The flip to
  Implementation-ready takes effect with maintainer approval of the PR that
  introduces this revision.

Implementation PRs pin this revision, for example:

```text
Implemented from docs/specs/sensors/superio-fintek-f71882-f71889.md revision 1 (commit <sha>), limited to the F71882 and F71889 rows marked enabled.
No other external sensor documentation was used.
```

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-28 | Initial version for #2522. Experimental F71882 (`0x0541`, TEMP1–3, FAN1–4) and F71889 (`0x0723`, TEMP1–3, FAN1–3) from the vendor datasheets, behind chip-ID, Super I/O vendor-ID, `CR25` HM power-down, LDN 4 active, aligned-base, F71889 HM vendor-ID, HM start/power-down, per-input diode-open, TEMP1 source and per-fan routing gates; lead-only HM port derivation; F71882 temperature bytes above `0x7D` withheld; write-1-to-clear and read-clear registers listed; ranked disposition for every #2522 target with F71869, F71889A, F71868A and F71858 recorded as candidates and every deferred target tracked in #2815; detection stops at the first passing slot so that no `ioctl_select_slot` clears the cached BAR authorization, and the path is cached before the mutex is released so a later selection can invalidate it; on slot 1 the S9 exit withhold also suppresses the `0xAA` exit for its four IDs. |
