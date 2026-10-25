# Nuvoton NCT67xx / NCT679x Super I/O Hardware Monitor Spec

| Field | Value |
| --- | --- |
| Revision | 8 |
| Status | Implementation-ready (rev 8) |
| Scope | Phase 3 Nuvoton-family motherboard temperature and fan RPM reads for the scoped `0xD802` / `NCT6799D` Verified path (with Experimental sub-scopes for its seventh direct RPM pair and bank 4 `0x96`), the exact `0xD421` / `NCT6796D` / `NCT6796D-E` Experimental path, the lead-only Experimental chip-id rows (NCT6779D range `0xC560`-`0xC567`; exact `0xC803`, `0xC911`, `0xC913`, `0xD121`, `0xD352`, `0xD423`, `0xD429`, `0xD42A`, `0xD42B`, `0xD451`, `0xD801`, `0xD806`), and, new in revision 8, Experimental unrecognized-version members of the eight lead family ranges `0xC800`-`0xC807`, `0xC910`-`0xC917`, `0xD120`-`0xD127`, `0xD350`-`0xD357`, `0xD420`-`0xD427`, `0xD428`-`0xD42F`, `0xD450`-`0xD457`, `0xD800`-`0xD807` with a conservative per-range profile. Every enabled path uses read-oriented normal-HM banked access for bank 4 byte temperatures (`0x90`-`0x95`, plus `0x96` on the rows listed for it) and direct 16-bit RPM registers (`0xC0`-`0xCB`, plus `0xCE`/`0xCF` on seven-input rows) after chip-id, LDN B, HM-base, and (where the row says so) HM I/O-space-lock validation, and every byte temperature passes a plausibility filter. It excludes exact package suffix claims such as `-R`, read-only HM access, voltage decode, count-based fan RPM decode, fan-control/PWM writes, threshold/limit writes, lock clearing, activation writes, alarm clearing, GPIO, UI integration, and any Rust implementation. |
| Issue phase | Phase 3 (#1635) - Nuvoton NCT67xx / NCT679x temperature and fan RPM specification |

Revision 8 keeps this document **Implementation-ready** and applies
[ADR 0011](../../adr/0011-experimental-sensor-enablement.md) to four
reads that revision 7 kept disabled only because they were unverified.
The maintainer decided on 2026-10-25 (S18) that lack of complete
verification is not by itself a reason to disable a read when the device
is recognized and an existing read-only decode applies without inventing
hardware facts. Revision 8 therefore enables, as **Experimental**:

1. the seventh direct RPM pair `0xCE`/`0xCF` on `0xD802`, `0xD421`, and
   `0xD423` (S1 p. 66; S7, S9);
2. unrecognized versions inside each lead family range, with the most
   conservative profile of the range's named members (S1 p. 362; S11,
   S15);
3. all six bank 4 byte temperatures `0x90`-`0x95` on `0xD42A` (S12, S14);
4. the bank 4 byte temperature `0x96` on the rows whose lead kind
   describes it (S12, S14; S9 byte observation on `0xD802`).

Revision 8 also adds a plausibility filter to every Nuvoton byte
temperature, including the Verified `0xD802` row (S1 p. 60; S17, S18).

The exact `0xD802` / `NCT6799D` path remains Verified for everything it
covered in revision 7: chip identity, normal-HM discovery, bank 4
`0x90`-`0x95`, and direct RPM pairs `0xC0`-`0xCB`. Its seventh pair and
its `0x96` byte are Experimental sub-scopes; a failure of only those
reads is reported as experimental and does not invalidate the Verified
values of the same sample. The only change to Verified `0xD802` output
is that a byte temperature outside the plausibility bounds is now
reported unavailable; the maintainer accepted that change (S18).

S1 does not identify `0xD802`. The package suffix/revision for `0xD802`
remains unknown and must not be surfaced as `NCT6799D-R`.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Nuvoton, *NCT6796D LPC/eSPI SI/O Datasheet*, version 0.6, publication release date July 6, 2017; official PDF: `https://www.nuvoton.com/resource-files/NCT6796D_Datasheet_V0_6.pdf` | Primary source for NCT6796D-only facts and family register semantics. Relevant pins: configuration protocol chapter 7, pp. 51-53; hardware-monitor LPC access chapter 8.3, pp. 54-55; temperature format chapter 8.6.3, pp. 60-62 (Table 8-1 on p. 60 lists the temperature data format from +125 degrees C down to -55 degrees C; its 8-bit two's-complement examples are `0x7D` = +125, `0x19` = +25, `0x01` = +1, `0x00` = 0, `0xFF` = -1, `0xE7` = -25, `0xC9` = -55); fan speed reading chapter 8.8.1-8.8.3, pp. 66-67 (the 8.8.1 table on p. 66 lists seven fan inputs SYSFANIN, CPUFANIN, AUXFANIN0-AUXFANIN4 with bank 4 count pairs `0xB0`/`0xB1` ... `0xBA`/`0xBB`, `0xCC`/`0xCD` and 16-bit RPM pairs `0xC0`/`0xC1` ... `0xCA`/`0xCB`, `0xCE`/`0xCF`); fan count/RPM registers chapter 9.209-9.232, pp. 176-183; bank 4 value RAM chapter 9.208, pp. 175-176 (does not define bank 4 `0x96` as a temperature reading); bank select and vendor ID chapter 9.30-9.31, pp. 110-111; HM read-only register chapter 9.481, pp. 277-280; global register default table and the note that the CR21 low digit encodes the IC version, p. 362; global chip ID CR20/CR21, p. 369; global CR28 bit 4 "HM IO space lock enable", p. 376; Logical Device B CR30/CR60-65, pp. 431-432. S1 does not identify `0xD802`. |
| S2 | [`superio-access.md`](superio-access.md) revision 3 | Primary clean-room source for the Phase 2 Nuvoton configuration-mode key sequence, global chip-id register reads (`0x20` / `0x21`), absent-id classification, standard Super I/O configuration port pairs, and ISA mutex policy. |
| S3 | HardwareVisualizer PR #1732 real-hardware diagnostic dump, captured 2026-06-27 on NZXT N7 B650E / AMD Ryzen 7 7800X3D / Windows 11 Pro | Independent hardware evidence for a reachable Nuvoton-class responder at slot 0 (`0x2E`/`0x2F`) with raw chip-id bytes `0xD8` / `0x02`; this is not the NCT6796D `0xD4` / `0x21` ID from S1. |
| S4 | HardwareVisualizer issue #1635 clean-room policy and [`README.md`](README.md) status-transition rules | Project policy source for the lead-only tagging rules and for allowing independently collected hardware dumps to verify facts when primary section/page references are unavailable. |
| S5 | Local standard-rights PawnIO probe on 2026-06-28 | Environment evidence only: `C:\Program Files\PawnIO` has `PawnIOLib.dll` and `LpcIO.bin`, PawnIO service is running, `pawnio_version` returned `0x00020000`, but `pawnio_open` returned `0x80070005` under medium integrity. |
| S6 | Local elevated PawnIO `LpcIO` probe on 2026-06-28, run from Administrator PowerShell on the S3-class machine | Independent hardware evidence for the pre-fix blocker: `pawnio_open`, `pawnio_load`, and `Global\Access_ISABUS.HTP.Method` mutex acquisition succeeded; slot 0 repeated `chipId=0xD802`; LDN B read `CR30=0x09`, `CR60/61=0x02/0x90` (`HM base=0x0290`), and `CR64/65=0x00/0x00` (no valid read-only HM base). `ioctl_find_bars` returned `0x80070490`, and a normal HM index-port write to `0x0295` returned `0x80070005`, so no temperature or fan RPM bytes were captured in that run. |
| S7 | [`docs/development/sensor-handoff/evidence/2026-06-28-superio-hm-dump-admin.md`](../../development/sensor-handoff/evidence/2026-06-28-superio-hm-dump-admin.md) and raw JSON captured from Administrator PowerShell on the S3-class machine | Independent local hardware evidence resolving the local HM byte-dump blocker: after slot selection, Nuvoton configuration-mode entry, LDN B selection, and `ioctl_find_bars`, bank select (`0x4E=0x04`) and all requested bank 4 reads (`0x90`-`0x95`, `0xB0`-`0xBB`, `0xC0`-`0xCF`) succeeded through normal HM ports `0x0295`/`0x0296`. Bytes `0xCC`-`0xCF` read `0xFF`, `0x1F`, `0x00`, `0x00`. Bank 4 `0x96` was not in the requested set and was not captured. |
| S8 | [`docs/development/sensor-handoff/evidence/2026-06-28-nuvoton-0xd802-source-hunt.md`](../../development/sensor-handoff/evidence/2026-06-28-nuvoton-0xd802-source-hunt.md) | Source-hunt evidence: public Nuvoton product-selection data and direct public URL probes did not expose an NCT6799D/NCT6799D-R datasheet or product page; third-party board photos and public user reports suggest NCT6799D-R, while the later independent AIDA64 dump supports `NCT6799D` for raw ID `0xD802`. |
| S9 | [`docs/development/sensor-handoff/evidence/2026-06-28-nuvoton-0xd802-aida64-dump.md`](../../development/sensor-handoff/evidence/2026-06-28-nuvoton-0xd802-aida64-dump.md), summarizing public AIDA64 text dumps attached to LibreHardwareMonitor issue #1720 | Independent dump evidence from an ASUS ROG Strix X670E-F Gaming WiFi board. The dump reports raw Super I/O device ID `D802h` and labels it `NCT6799D`; it records normal HM base `0x0290`, read-only HM base `0x0A00` on that board, bank 4 temperature bytes, and bank 4 direct RPM-style bytes. The public bank 4 dump row at `0xC0` also contains `0xCC`-`0xCF` = `0x07`, `0x17`, `0x15`, `0x59`, and the row at `0x90` contains `0x96` = `0x19` (both re-read from the public attachment text on 2026-10-25). The dump's repeated sampling table covers `0490`-`0495` but not `0496`, so the dump does not label `0x96`. This is dump output only, not implementation source code. |
| S10 | [ADR 0011](../../adr/0011-experimental-sensor-enablement.md) | Accepted project policy source for Experimental enablement of recognized hardware when a plausible read-only path is fully defined but lacks hardware validation, and for keeping plausibility gates; it supplies no hardware-register facts. |
| S11 | Linux kernel `drivers/hwmon/nct6775-platform.c` and `drivers/hwmon/nct6775.h` (torvalds/linux `master`, retrieved 2026-10-25), GPL-2.0 | **lead-only (copyleft)**. Super I/O detection: the raw 16-bit chip ID is matched after masking its low three bits (`0xFFF8`) against the family values `0xC560`, `0xC800`, `0xC910`, `0xD120`, `0xD350`, `0xD420`, `0xD450`, `0xD428`, `0xD800`; the hardware monitor is logical device `0x0B`, its enable bit is `CR30` bit 0, and its base is `CR60/61`; the driver claims a two-port I/O region at base+5 (index) and base+6 (data) and selects a bank by writing index `0x4E` and then the bank number before each banked access; the driver's chip-name table covers both NCT6798D and NCT5585D under the kind matched by `0xD428`, and both NCT6796D-S and NCT6799D-R under the kind matched by `0xD800`; for the NCT6791D-and-later kinds the driver reads global `CR28` and, if bit 4 is set, clears it (a write) and logs that it enabled the hardware-monitor logical-device mapping; on listed ASUS boards the driver reaches the Super I/O and HM registers through an ACPI WMI method instead of port I/O. Restated in this document's own words; no code, structure, or identifier names were copied. |
| S12 | Linux kernel `drivers/hwmon/nct6775-core.c` (torvalds/linux `master`, retrieved 2026-10-25), GPL-2.0 | **lead-only (copyleft)**. Chip table comment: fan input counts NCT6779D 5; NCT6791D, NCT6792D, NCT6793D, NCT6795D 6; NCT6796D, NCT6797D, NCT6798D, NCT5585D, NCT6796D-S, NCT6799D-R 7; alternate IDs `0xD451` (NCT6797D) and `0xD429` (NCT6798D); NCT5585D under the `0xD428` kind; NCT6796D-S `0xD801`; NCT6799D-R `0xD802`. Register tables: for every kind from NCT6779D onward the 16-bit RPM registers are bank 4 `0xC0`, `0xC2`, `0xC4`, `0xC6`, `0xC8`, `0xCA`, `0xCE` and the byte temperature registers are bank 4 `0x90`-`0x95`; only the kinds matched by `0xD428` and `0xD800` add bank 4 `0x96` as a seventh byte temperature (the kinds matched by `0xD420` and `0xD450` do not); register addresses are 16-bit values whose high byte is the bank and whose low byte is the index. S12 does not itself describe Super I/O detection or port offsets. Restated; nothing copied. |
| S13 | Linux kernel `Documentation/hwmon/nct6775.rst` (torvalds/linux `master`, retrieved 2026-10-25), GPL-2.0 | **lead-only (copyleft)**. Documentation text: datasheets for NCT6779D-NCT6799D are "available from Nuvoton upon request", the NCT5585D datasheet is "not available" and the chip is "compatible to NCT6798D"; NCT6796D-S and NCT6799D-R "are very similar", their chip IDs indicate different versions, and the driver treats them the same way. |
| S14 | LibreHardwareMonitor `LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/` chip-ID enumeration (`Chip.cs`), Super I/O detection (`LpcIO.cs`), configuration-port helper (`LpcPort.cs`), and Nuvoton hardware-monitor driver (`Nct677X.cs`) (`master`, retrieved 2026-10-25), MPL-2.0 | **lead-only (copyleft)**. Exact raw IDs: `0xC5` with CR21 high nibble `6` = NCT6779D; `0xC803` NCT6791D; `0xC911` NCT6792D; `0xC913` NCT6792D-A; `0xD121` NCT6793D; `0xD352` NCT6795D; `0xD423` NCT6796D; `0xD42A` = NCT5585D on ASRock X870E Nova WiFi, otherwise NCT6796D-R; `0xD451` NCT6797D; `0xD42B` NCT6798D; `0xD802` = NCT6796D-S on ASRock X870E Nova WiFi and B650M HDV M.2, otherwise NCT6799D; `0xD806` NCT6701D. Fan input counts: NCT6779D 5; NCT6796D-R, NCT6796D-S, NCT6797D, NCT6798D, NCT6799D, NCT6701D, NCT5585D 7; NCT6791D, NCT6792D, NCT6792D-A, NCT6793D, NCT6795D, NCT6796D 6. Detection selects logical device `0x0B`, reads the base from `CR60/61` (twice, requiring equal values), and reads `CR30` only for NCT6701D; the hardware monitor is accessed through index port base+5 and data port base+6, selecting the bank by writing index `0x4E` and then the high byte of the 16-bit register address before each read. Bank 4 byte temperatures `0x90`-`0x95` for every listed chip except NCT5585D, whose table exposes only `0x93` and `0x95` of them. Bank 4 `0x96` is read as an additional byte temperature only in the NCT6798D/NCT6799D table and the NCT6796D-S table; the NCT6796D/NCT6796D-R/NCT6797D, NCT5585D, NCT6701D, and older tables do not read it, and the two tables that read it give it different source names. Fan speed is read from the 13-bit count registers (`0xB0`, `0xB2`, `0xB4`, `0xB6`, `0xB8`, `0xBA`, `0xCC`), not from the direct 16-bit RPM pairs. Global `CR28` bit 4 is the HM I/O-space lock for NCT6791D and later including NCT6701D and NCT5585D; the lead clears it (a write) and notes that the lock is already disabled when the Nuvoton vendor ID can be read. NCT6701D quirks: raw temperature bytes `0x00`, `0xA0`, `0x7E`-`0x80` are invalid; some firmware leaves LDN B inactive; a `CR60/61` value below `0x100` or with any bit of `0xF007` set is an invalid base, in which case the lead uses `CR64/65`; the vendor-ID check is skipped. Restated; nothing copied. |
| S15 | lm-sensors `prog/detect/sensors-detect` (`master`, retrieved 2026-10-25), GPL-2.0 | **lead-only (copyleft)**. Detection table: the same nine family values as S11 with mask `0xFFF8`, each with logical device `0x0B`; the `0xC560` entry covers both NCT5532D and NCT6779D. The generic Super I/O probe reads the logical-device base from `0x60/0x61`; it does not read `CR30` or any hardware-monitor register. |
| S16 | [ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md) | Accepted project policy source allowing lead-only facts to back Experimental scopes; it supplies no hardware-register facts. |
| S17 | [`superio-ite-it86xx-it87xx.md`](superio-ite-it86xx-it87xx.md) revision 2 (Procedures, sample step 5) and `core/src/utils/super_io.rs` (`is_plausible_ite_temperature`, at commit `e2733bc0`) | Project consistency source: the existing Super I/O byte-temperature plausibility policy accepts a decoded temperature only when -55 <= t <= 125 degrees C inclusive, treats 0 degrees C as a valid sample, and omits an implausible input without publishing zero, a clamped boundary, or a whole-device failure. It supplies no Nuvoton register fact. |
| S18 | Maintainer decision of 2026-10-25, conveyed to the spec-author role for this revision | Project policy source: the four rev 7 items that were disabled only for lack of verification are switched to Experimental under ADR 0011; unrecognized range versions use the most conservative named-member profile; the plausibility filter applies to every Nuvoton byte temperature including Verified `0xD802`, and hiding out-of-bounds `0xD802` values is accepted. It supplies no hardware-register facts. |

S11-S15 are copyleft monitoring implementations and documentation
consulted as leads under ADR 0023. Every fact taken from them is
restated in this document's own words and tables, carries its source ID,
and backs only scoped-enablement rows (or sub-scopes) whose default
enablement is Experimental. No code excerpt, code structure, string
literal, or implementation identifier name was copied. The `0xD802`
Verified scope depends on none of them; its Experimental sub-scopes
(seventh pair, `0x96`) may. The `0xD421` Experimental row depends only on
S1. S9 uses public hardware dump text attached to an issue; no AIDA64
source, binary, disassembly, or implementation structure was consulted.

## Validation outcome

Revision 8 passes the implementation-ready gate for the scoped
`0xD802` / `NCT6799D` Verified path, the exact `0xD421` / `NCT6796D` /
`NCT6796D-E` Experimental path, the lead-only Experimental rows, and the
Experimental unrecognized-version range members:

- Exact chip identity: S9 independently maps raw `D802h` to `NCT6799D`.
  S3/S7 prove the local target board exposes the same raw chip ID. The
  package suffix/revision is not proven and remains out of scope. (S3,
  S7, S9)
- HM base and bank selection: S7 proves the local target board exposes
  LDN B normal HM base `0x0290` and can select bank 4 through normal HM
  ports after `ioctl_find_bars` succeeds. S9 corroborates normal HM base
  `0x0290` on another `D802h` / NCT6799D board. (S7, S9)
- Temperature bytes: S7 captures local bank 4 `0x90`-`0x95`; S9 captures
  the same bank/register set and a repeated temperature sampling table
  at `0490`-`0495`. Every captured value lies inside the plausibility
  bounds, so the filter does not change the evidence-backed output.
  (S1 p. 60; S7, S9)
- Fan/RPM bytes: S7 captures local direct RPM register bytes at
  `0xC0`-`0xCB`; S9 captures non-zero direct high/low values at the same
  six register pairs. Zero direct RPM is treated as stopped-or-unconnected,
  not as a read failure. (S7, S9)
- Experimental sub-scopes on `0xD802`: the seventh pair `0xCE`/`0xCF` is
  the AUXFANIN4 RPM pair in S1 p. 66, S9 reads 5465 RPM there with
  non-saturated adjacent count bytes, and S7 reads `0x00`/`0x00`
  (stopped or unconnected); bank 4 `0x96` is described as a byte
  temperature for the `0xD800` kind by S12 and under both `0xD802` labels
  by S14, and S9 captured the byte as `0x19` (+25 degrees C). Neither
  sub-scope has a primary-source definition for `0xD802`, so both are
  Experimental. (S1, S7, S9, S10; S12, S14 lead-only)
- Experimental D421 path: S1 pins the exact chip ID and the normal-HM,
  bank 4 temperature, and all seven direct RPM-pair positions used by
  this revision. No D421 hardware dump is available, so this path remains
  Experimental under ADR 0011 rather than Verified. (S1, S10)
- Lead-only Experimental rows: for every chip in these rows, S11, S14,
  and S15 describe the same LDN B / `CR60-61` discovery (S11 also the
  `CR30` enable bit), S12 and S14 describe the same bank 4 byte
  temperatures `0x90`-`0x95`, and S12 describes the same direct RPM pairs
  that S1 defines for NCT6796D. The chip IDs (S11, S12, S14, S15),
  per-chip fan counts (S12, S14), lock applicability (S11, S14), the
  `0x96` byte (S12, S14), NCT6701D quirks (S14), and the NCT6796D-S
  similarity statement (S13) are lead-only facts and therefore cannot be
  Verified; the rows are Experimental under ADR 0011 and ADR 0023. (S1,
  S10, S16)
- Unrecognized-version range members: S11 and S15 classify every value
  of each masked range as one family, and S1 p. 362 states that the CR21
  low digit encodes the IC version. The profile applied to an unnamed
  version is the intersection of the named members' read sets with every
  safety gate any named member uses, so no register is read that no
  named member of the range reads. (S1 p. 362; S11, S15 lead-only; S18)
- Remaining uncertainties are explicitly non-blocking or disabled in
  [Open questions](#open-questions) and [Scoped enablement](#scoped-enablement).

## Detection

### Chip identity and base facts

| Fact | Source |
| --- | --- |
| Use the Phase 2 Nuvoton Super I/O config-entry sequence and read global `CR20`/`CR21` as raw chip-id high/low bytes. | S2 |
| NCT6796D/NCT6796D-E's global CR20 high byte is `0xD4` and CR21 low byte is `0x21`, so its configuration-space chip ID is `0xD421`. This is the distinct S1-backed Experimental scope and must not be conflated with the enabled `0xD802` scope. | S1 p. 369 |
| S1's global register default table states that CR21 is the low byte of the chip ID and that its low digit encodes the IC version (its example: `11` = A version, `12` = B version, `13` = C version). | S1 p. 362 |
| The observed local board in S3/S7 responded on slot 0 (`0x2E` index / `0x2F` data) with `idHigh=0xD8`, `idLow=0x02`, combined `chipId=0xD802`. | S3, S7 |
| The independent S9 AIDA64 dump reports raw device ID `D802h` and labels that responder `NCT6799D`; the same dump's per-LDN config tables show CR20/CR21 as `D8 02`. | S9 |
| The enabled runtime identity is therefore raw chip ID `0xD802` / `NCT6799D`. Do not claim package suffix `-R`, OEM variant, or package revision. | S7, S9 |
| The local S7 board reads LDN B `CR30=0x09`, normal HM base `0x0290`, and read-only HM base `0x0000`; the S9 board reads normal HM base `0x0290` and read-only HM base `0x0A00`. The enabled path uses only the normal HM base. | S7, S9 |
| On the local board, `ioctl_find_bars` failed before configuration/base discovery (`0x80070490`) but succeeded after Nuvoton configuration-mode entry and LDN B/base discovery. After that, normal HM bank select and reads through `0x0295`/`0x0296` succeeded. | S7 |
| Lead-only: three copyleft leads match the Nuvoton NCT6779D-and-later family by raw chip ID, select logical device `0x0B`, and read `CR60/61` as the hardware-monitor base (S11, S14, S15); the kernel lead also reads `CR30` bit 0 as the hardware-monitor enable for every kind, and S14 reads it only for NCT6701D. This is the same discovery S1 defines for NCT6796D and S7/S9 validate for `0xD802`. | S11, S14, S15 (lead-only); S1 pp. 431-432 |
| Lead-only: for every NCT6779D-and-later chip, S12 and S14 read bank 4 bytes `0x90`-`0x95` as the byte temperatures, and S12 reads bank 4 `0xC0`/`0xC1` ... `0xCA`/`0xCB` (plus `0xCE`/`0xCF` on seven-input chips) as 16-bit RPM pairs (S14 reads the 13-bit count registers instead); this is the same layout S1 defines for NCT6796D on pp. 66 and 176. | S12, S14 (lead-only, temperatures); S12 (lead-only, RPM pairs); S1 pp. 66, 176 |
| Lead-only: two leads (S11, S15) classify a raw chip ID by masking its low three bits (`0xFFF8`), so each family value covers eight consecutive IDs; combined with S1's statement that the CR21 low digit encodes the IC version, the masked bits distinguish versions of one family. | S11, S15 (lead-only); S1 p. 362 |

### Lead-only Experimental chip-id values

These raw IDs are enabled as Experimental rows with their exact
profiles. "Range" means the leads classify every value in the range as
that chip; "exact" means a lead names that specific value. Chip labels
are surfaced without package suffixes except where the only attested
identity is a suffixed part.

| Raw ID | Chip label to surface | Lead attestation | Lead range containing the value |
| --- | --- | --- | --- |
| `0xC560`-`0xC567` | NCT6779D | Range only: S11/S15 mask `0xFFF8` on `0xC560`; S14 matches `0xC5` with CR21 high nibble `6` (`0xC560`-`0xC56F`). No lead names an exact value. | `0xC560`-`0xC567` (S11, S15); `0xC560`-`0xC56F` (S14) |
| `0xC803` | NCT6791D | Exact (S14) | `0xC800`-`0xC807` (S11, S15) |
| `0xC911` | NCT6792D | Exact (S14) | `0xC910`-`0xC917` (S11, S15) |
| `0xC913` | NCT6792D (S14 labels this value NCT6792D-A) | Exact (S14) | `0xC910`-`0xC917` (S11, S15) |
| `0xD121` | NCT6793D | Exact (S14) | `0xD120`-`0xD127` (S11, S15) |
| `0xD352` | NCT6795D | Exact (S14) | `0xD350`-`0xD357` (S11, S15) |
| `0xD423` | NCT6796D | Exact (S14); consistent with the S1 p. 362 statement that the CR21 low digit is an IC version, so `0xD423` and the S1-stated `0xD421` are versions of the same part | `0xD420`-`0xD427` (S11, S15) |
| `0xD429` | NCT6798D | Exact (S12 alternate ID, single lead) | `0xD428`-`0xD42F` (S11, S15) |
| `0xD42A` | NCT6796D-R / NCT5585D (generic; see Open questions) | Exact (S14; label depends on the board in that lead) | `0xD428`-`0xD42F` (S11, S15), a kind S11 attributes to both NCT6798D and NCT5585D |
| `0xD42B` | NCT6798D | Exact (S14) | `0xD428`-`0xD42F` (S11, S15) |
| `0xD451` | NCT6797D | Exact (S14; S12 lists it as the NCT6797D alternate ID) | `0xD450`-`0xD457` (S11, S15) |
| `0xD801` | NCT6796D-S | Exact (S12 chip table; S11 attributes the kind to both NCT6796D-S and NCT6799D-R; single lead family, see Open questions) | `0xD800`-`0xD807` (S11, S15) |
| `0xD806` | NCT6701D | Exact (S14, single lead; S11/S15 would classify it under the `0xD800` kind without naming NCT6701D) | `0xD800`-`0xD807` (S11, S15) |

### ID-matching policy

Revision 8 matches in two steps:

1. **Named IDs.** An exact raw ID listed above, the exact `0xD802` and
   `0xD421` rows, and the NCT6779D range `0xC560`-`0xC567` keep their own
   profiles exactly as stated in their rows. The NCT6779D row stays the
   range every lead accepts (the intersection of S11/S15 and S14);
   `0xC568`-`0xC56F`, which only S14 accepts, remains Unsupported.
2. **Unrecognized versions.** Any other raw ID whose value masked with
   `0xFFF8` equals one of the eight family values `0xC800`, `0xC910`,
   `0xD120`, `0xD350`, `0xD420`, `0xD428`, `0xD450`, `0xD800` is enabled as
   **Experimental** with the conservative profile of that range in the
   table below. (S1 p. 362; S11, S15 lead-only; S18)

Every other raw ID remains Unsupported.

The conservative profile of a range is derived from the profiles of its
named members (the exact IDs listed in that range, including `0xD421`
and `0xD802`) by these rules (S18):

- fan pairs: the fewest direct RPM pairs read by any named member;
- temperatures: only the bank 4 byte temperatures read by every named
  member (the intersection);
- `CR28` bit 4 lock gate: applied if any named member applies it;
- NCT6701D invalid-byte filter: applied to every unrecognized
  `0xD800`-`0xD807` ID, because the range contains `0xD806`;
- the plausibility filter, the valid-`CR60/61` requirement, the
  no-`CR64/65`-fallback rule, and the disabled scopes apply as for every
  row.

#### Unrecognized-version range profiles

| Range (mask `0xFFF8`) | Named members (keep exact profiles) | Fan pairs read for unnamed versions | Temperatures read for unnamed versions | Lock gate | NCT6701D invalid-byte filter | Chip label to surface | Status (sources) | Default enablement |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `0xC800`-`0xC807` | `0xC803` (6 pairs, `0x90`-`0x95`, lock) | 6 (`0xC0`-`0xCB`) | `0x90`-`0x95` | Yes | No | NCT6791D family (unrecognized version) | Lead-only: range (S11, S15); member profile (S12, S14); version semantics S1 p. 362. | Experimental |
| `0xC910`-`0xC917` | `0xC911`, `0xC913` (each 6 pairs, `0x90`-`0x95`, lock) | 6 (`0xC0`-`0xCB`) | `0x90`-`0x95` | Yes | No | NCT6792D family (unrecognized version) | Lead-only: range (S11, S15); member profiles (S12, S14); S1 p. 362. | Experimental |
| `0xD120`-`0xD127` | `0xD121` (6 pairs, `0x90`-`0x95`, lock) | 6 (`0xC0`-`0xCB`) | `0x90`-`0x95` | Yes | No | NCT6793D family (unrecognized version) | Lead-only: range (S11, S15); member profile (S12, S14); S1 p. 362. | Experimental |
| `0xD350`-`0xD357` | `0xD352` (6 pairs, `0x90`-`0x95`, lock) | 6 (`0xC0`-`0xCB`) | `0x90`-`0x95` | Yes | No | NCT6795D family (unrecognized version) | Lead-only: range (S11, S15); member profile (S12, S14); S1 p. 362. | Experimental |
| `0xD420`-`0xD427` | `0xD421` (7 pairs, `0x90`-`0x95`, lock), `0xD423` (7 pairs, `0x90`-`0x95`, lock) | 7 (`0xC0`-`0xCB`, `0xCE`/`0xCF`) | `0x90`-`0x95` | Yes | No | NCT6796D family (unrecognized version) | Lead-only: range (S11, S15); lock applicability (S11, S14); layout S1 pp. 66, 176; S1 p. 362. | Experimental |
| `0xD428`-`0xD42F` | `0xD429`, `0xD42A`, `0xD42B` (each 7 pairs, `0x90`-`0x96`, lock) | 7 (`0xC0`-`0xCB`, `0xCE`/`0xCF`) | `0x90`-`0x96` | Yes | No | NCT6798D family (unrecognized version) | Lead-only: range and NCT6798D/NCT5585D kind (S11, S15); member profiles (S12, S14); `0x96` for the kind (S12); S1 p. 362. | Experimental |
| `0xD450`-`0xD457` | `0xD451` (7 pairs, `0x90`-`0x95`, lock) | 7 (`0xC0`-`0xCB`, `0xCE`/`0xCF`) | `0x90`-`0x95` | Yes | No | NCT6797D family (unrecognized version) | Lead-only: range (S11, S15); member profile (S12, S14); S1 p. 362. | Experimental |
| `0xD800`-`0xD807` | `0xD801` (7 pairs, `0x90`-`0x96`, lock), `0xD802` (Verified; 7 pairs with the seventh Experimental, `0x90`-`0x96` with `0x96` Experimental, no lock gate), `0xD806` (7 pairs, `0x90`-`0x95`, lock, invalid-byte filter) | 7 (`0xC0`-`0xCB`, `0xCE`/`0xCF`) | `0x90`-`0x95` (`0xD806` does not read `0x96`) | Yes (from `0xD801`, `0xD806`) | Yes | NCT6799D family (unrecognized version) | Lead-only: range and NCT6796D-S/NCT6799D-R kind (S11, S15); member profiles (S12, S14); NCT6701D filter (S14); S1 p. 362. The Verified `0xD802` facts are not extended to these IDs. | Experimental |

Label decision: an unnamed version is known only to belong to a family,
so its label names the family and states that the version is not
recognized; it does not claim an exact part, package suffix, or OEM
variant, and it never equals a named row's label, so diagnostics and
feedback can tell a named match from a range match. The anchor part
name for each range is the part the leads attribute the family value
to, without suffix: NCT6791D, NCT6792D, NCT6793D, NCT6795D, NCT6796D,
and NCT6797D for their ranges (S11, S12, S14); NCT6798D for
`0xD428`-`0xD42F`, because S11 names the kind after it and S13 calls the
other candidate, NCT5585D, compatible to NCT6798D; and NCT6799D for
`0xD800`-`0xD807`, because it is the project's only Verified member of
that range (S9) and a suffix-free name avoids the NCT6796D-S /
NCT6799D-R choice (S11, S13). The raw chip ID is still recorded in
diagnostics.

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| Nuvoton responder detection using Phase 2 raw chip-id bytes | Ready through `superio-access.md` rev 3; this document references the diagnostic result but does not change the Phase 2 scope. | Existing diagnostic may remain enabled. |
| NCT6796D/NCT6796D-E chip-id mapping (`0xD421`) | **Experimental**: exact identity, the read-only profile (normal HM, bank 4 `0x90`-`0x95`, seven direct RPM pairs), and the `CR28` bit 4 lock bit are pinned by S1 (p. 376 for the lock bit), but no matching hardware dump is available. No behavior is inferred from the `0xD802` identity. | Enabled as Experimental for the scoped normal-HM reads, behind the read-only `CR28` bit 4 lock gate. |
| `0xD802` / `NCT6799D` chip-id mapping | **Verified** for chip identity, normal-HM discovery, bank 4 `0x90`-`0x95`, and direct RPM pairs `0xC0`-`0xCB`. S7 proves the local raw ID; S9 independently maps `D802h` to `NCT6799D`. The seventh pair `0xCE`/`0xCF` and bank 4 `0x96` on this ID are **Experimental** sub-scopes (rows below); they do not change the Verified status of the rest of the row. | Enabled as Verified for the scoped Phase 3 normal-HM reads, with the two Experimental sub-scopes enabled as Experimental. A failure of only an Experimental sub-scope read is reported as experimental and leaves the Verified values of that sample intact. |
| Normal HM logical-device selection and base discovery | Ready for both S1-backed IDs where S1 defines the NCT6796D register semantics: select LDN B, read `CR30`, read `CR60/61`, validate the discovered normal HM base, then use base+`0x05`/base+`0x06`. S7/S9 additionally validate the sequence on `0xD802`. For the lead-only rows and unrecognized versions the same sequence is lead-only: LDN B and `CR60/61` (S11, S14, S15), `CR30` bit 0 (S11; S14 for NCT6701D), index/data offsets and bank select (S11, S14). | Enabled for `0xD421` as Experimental, `0xD802` as Verified, and the lead-only rows and unrecognized versions as Experimental. |
| Read-only HM base (`CR64/65`) | Board-variable in evidence: local S7 is `0x0000`, S9 is `0x0A00`. The ready scope does not need it. S14's NCT6701D fallback to it conflicts with S1's definition (see Open questions). | Disabled for every ID, including `0xD806` and unrecognized `0xD800`-`0xD807` versions. |
| Bank 4 byte temperature reads `0x90`-`0x95` | S1 directly defines this NCT6796D bank/register layout; S7 and S9 independently validate the same bank/register set on `0xD802` / NCT6799D. For the lead-only rows and unrecognized versions the layout is lead-only (S12, S14). For `0xD42A` all six are now read: S12 describes them for the whole `0xD428` kind and S14 reads all six under its NCT6796D-R label (only `0x93`/`0x95` under its NCT5585D label); the plausibility filter hides a byte that is not a plausible temperature. | Enabled with generic source labels for `0xD421` as Experimental, `0xD802` as Verified, and every lead-only row and unrecognized version as Experimental. |
| Bank 4 byte temperature `0x96` | Lead-only meaning: S12 adds it as a seventh byte temperature only for the kinds matched by `0xD428` and `0xD800`; S14 reads it under the NCT6798D/NCT6799D and NCT6796D-S tables. S1 does not define it for NCT6796D. S9 captured `0x19` there on a `0xD802` board; S7 did not request it. | Enabled as **Experimental** with generic label `Temperature 7` for `0xD802` (sub-scope of the Verified row), `0xD429`, `0xD42A`, `0xD42B`, `0xD801`, and unrecognized `0xD428`-`0xD42F` versions. Disabled for `0xD806` and unrecognized `0xD800`-`0xD807` versions (the only lead naming NCT6701D does not read it) and for every other row. |
| Direct 16-bit fan RPM reads `0xC0`-`0xCB` | S1 directly defines these six NCT6796D direct high/low pairs; S7 and S9 independently validate the same six pairs on `0xD802` / NCT6799D. For the lead-only rows and unrecognized versions the register positions are lead-only (S12) and the number of populated pairs is lead-only (S12, S14). | Enabled with generic fan labels for `0xD421` as Experimental, `0xD802` as Verified, and the lead-only rows and unrecognized versions as Experimental for the populated pairs listed per row. |
| Seventh direct RPM pair `0xCE`/`0xCF` (AUXFANIN4) | S1 p. 66 lists this pair as the AUXFANIN4 16-bit RPM reading for NCT6796D; S12 lists the same register as the seventh RPM register for every NCT6779D-and-later kind; S9 captured `0x15`/`0x59` (5465 RPM) and S7 captured `0x00`/`0x00` there on `0xD802`. For `0xD421` the position is S1-backed; for `0xD802` it rests on S1 family semantics plus the S9/S7 observations; which other chips populate the input is lead-only (S12, S14). | Enabled as **Experimental** (generic label `Fan 7`) for `0xD802` (sub-scope of the Verified row), `0xD421`, `0xD423`, the seven-input lead-only rows, and unrecognized versions of the `0xD420`, `0xD428`, `0xD450`, and `0xD800` ranges. Not read for six- or five-input rows. |
| Count-based fan reads `0xB0`-`0xBB`, `0xCC`/`0xCD` | Captured by S7/S9 but unnecessary while direct RPM registers are available. | Disabled. |
| HM I/O-space lock detection (global `CR28` bit 4) | S1 p. 376 defines the bit for NCT6796D, so for the exact `0xD421` profile the gate is primary-source-backed and adds only a read and a stop. Applicability to NCT6791D and later and the read-only consequence are lead-only (S11, S14). Detection is a read; clearing the lock is a write and is out of scope. | Enabled as a read-only availability gate for the exact `0xD421` Experimental profile, the NCT6791D-and-later lead-only rows, and every unrecognized-version range member. Not applied to `0xD802` (the S7/S9 dumps show its normal-HM reads working without the gate) or NCT6779D (no lead applies the lock to it). |
| Byte-temperature plausibility filter | Bounds from S1 Table 8-1 p. 60 (+125 to -55 degrees C), identical to the project's existing Super I/O plausibility bounds (S17); applies the ADR 0011 plausibility guardrail (S10, S18). Not lead-only. | Enabled for every enabled byte temperature on every row, including Verified `0xD802`. |

#### Lead-only Experimental chip rows

Each row is **Experimental** under ADR 0011 and ADR 0023. The Status
column names the lead-only facts and source IDs the row depends on. "Fan
inputs" is the number of populated direct RPM pairs starting at
`0xC0`/`0xC1`; pairs beyond that count are not read. Every row also
applies the plausibility filter.

| Raw ID match | Chip label | Fan inputs (direct pairs read) | Seventh pair `0xCE`/`0xCF` | Bank 4 temperatures read | Lock gate | Status (lead-only facts and sources) | Default enablement |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `0xC560`-`0xC567` (range) | NCT6779D | 5 (`0xC0`-`0xC9`; do not read `0xCA`/`0xCB`) | Not populated | `0x90`-`0x95` | No | Lead-only: range identity (S11, S14, S15); five fan inputs (S12, S14); bank 4 `0x90`-`0x95` temperatures (S12, S14); `0xC0`-`0xC9` direct RPM pairs (S12). No exact ID or hardware dump exists. | Experimental |
| `0xC803` | NCT6791D | 6 (`0xC0`-`0xCB`) | Not populated | `0x90`-`0x95` | Yes | Lead-only: exact ID (S14) inside range (S11, S15); six fan inputs (S12, S14); bank 4 temperatures (S12, S14); direct RPM pairs (S12); lock applicability (S11, S14). | Experimental |
| `0xC911` | NCT6792D | 6 | Not populated | `0x90`-`0x95` | Yes | Lead-only: exact ID (S14) inside range (S11, S15); six fan inputs (S12, S14); bank 4 temperatures (S12, S14); direct RPM pairs (S12); lock applicability (S11, S14). | Experimental |
| `0xC913` | NCT6792D | 6 | Not populated | `0x90`-`0x95` | Yes | Lead-only: exact ID (S14, labelled NCT6792D-A there) inside range (S11, S15); six fan inputs (S12, S14); bank 4 temperatures (S12, S14); direct RPM pairs (S12); lock applicability (S11, S14). | Experimental |
| `0xD121` | NCT6793D | 6 | Not populated | `0x90`-`0x95` | Yes | Lead-only: exact ID (S14) inside range (S11, S15); six fan inputs (S12, S14); bank 4 temperatures (S12, S14); direct RPM pairs (S12); lock applicability (S11, S14). | Experimental |
| `0xD352` | NCT6795D | 6 | Not populated | `0x90`-`0x95` | Yes | Lead-only: exact ID (S14) inside range (S11, S15); six fan inputs (S12, S14); bank 4 temperatures (S12, S14); direct RPM pairs (S12); lock applicability (S11, S14). | Experimental |
| `0xD423` | NCT6796D | 7 (`0xC0`-`0xCB`, `0xCE`/`0xCF`; mirrors the `0xD421` row) | Enabled (Experimental) | `0x90`-`0x95` | Yes | Lead-only: exact ID (S14) inside range (S11, S15). Layout including the seventh pair is S1-backed for NCT6796D (pp. 66, 176) and agrees with S12 (7 inputs); S14 states six inputs (see Open questions). Lock register is S1 p. 376, applicability (S11, S14). | Experimental |
| `0xD429` | NCT6798D | 7 (`0xC0`-`0xCB`, `0xCE`/`0xCF`) | Enabled (Experimental) | `0x90`-`0x96` | Yes | Lead-only: exact ID (S12 only) inside range (S11, S15); seven fan inputs (S12); layout including the seventh pair (S12; S1 p. 66 for the register position); `0x96` for the `0xD428` kind (S12; S14 NCT6798D table); lock applicability (S11, S14). | Experimental |
| `0xD42A` | NCT6796D-R / NCT5585D | 7 | Enabled (Experimental) | `0x90`-`0x96` | Yes | Lead-only: exact ID (S14) inside range (S11, S15); label is board-dependent in S14 and S11 attributes the range to both NCT6798D and NCT5585D (see Open questions); seven fan inputs for both candidate labels (S12, S14); bank 4 `0x90`-`0x95` for the whole `0xD428` kind (S12) and under S14's NCT6796D-R label (S14 reads only `0x93`/`0x95` under its NCT5585D label); `0x96` for the `0xD428` kind, which S11 attributes to NCT5585D as well (S12; S14 does not read it under either candidate label); direct RPM pairs including the seventh (S12; S1 p. 66 for the register position); lock applicability (S11, S14). Bytes that are not plausible temperatures are hidden by the plausibility filter. | Experimental |
| `0xD42B` | NCT6798D | 7 | Enabled (Experimental) | `0x90`-`0x96` | Yes | Lead-only: exact ID (S14) inside range (S11, S15); seven fan inputs (S12, S14); bank 4 temperatures (S12, S14); `0x96` (S12 for the kind; S14 NCT6798D table); direct RPM pairs including the seventh (S12; S1 p. 66 for the register position); lock applicability (S11, S14). | Experimental |
| `0xD451` | NCT6797D | 7 | Enabled (Experimental) | `0x90`-`0x95` | Yes | Lead-only: exact ID (S12, S14) inside range (S11, S15); seven fan inputs (S12, S14); bank 4 temperatures (S12, S14; neither lead reads `0x96` for this kind); direct RPM pairs including the seventh (S12; S1 p. 66 for the register position); lock applicability (S11, S14). | Experimental |
| `0xD801` | NCT6796D-S | 7 | Enabled (Experimental) | `0x90`-`0x96` | Yes | Lead-only: exact ID (S12; kind name S11) inside range (S11, S15); S13 states NCT6796D-S and NCT6799D-R are very similar; seven fan inputs (S12); layout (S12; S1 p. 66); `0x96` for the `0xD800` kind (S12; S14 NCT6796D-S table); lock applicability (S11, S14). S14 has no `0xD801` entry (see Open questions). | Experimental |
| `0xD806` | NCT6701D | 7 | Enabled (Experimental) | `0x90`-`0x95` | Yes | Lead-only: exact ID (S14 only); seven fan inputs (S14; S12 for the `0xD800` kind); bank 4 `0x90`-`0x95` layout (S14) and RPM pairs (S12 for the `0xD800` kind; S1 p. 66); lock applicability (S14); invalid temperature bytes `0x00`, `0xA0`, `0x7E`-`0x80` (S14); inactive-LDN and invalid-base quirks handled as unavailable (S14). `0x96` is not read: the only lead that names NCT6701D (S14) does not read it, and S12's `0x96` statement concerns a kind S11 names after NCT6796D-S/NCT6799D-R, not NCT6701D. | Experimental |

## Register map facts

### Configuration-space fields needed before hardware-monitor access

These facts are implementation-ready for the scoped `0xD421` /
NCT6796D / NCT6796D-E Experimental path, the `0xD802` / NCT6799D Verified
path, the lead-only Experimental rows, and unrecognized-version range
members unless the row explicitly says otherwise. For `0xD421` the
S1-backed facts are a profile definition, not a hardware-validation
claim; for the lead-only rows and unrecognized versions the
S11/S14/S15 facts are lead-only profile definitions.

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x20` | Chip ID high byte | `7:0` | `0xD8` for the `0xD802` / NCT6799D scope; `0xD4` for the exact `0xD421` / NCT6796D / NCT6796D-E Experimental scope; the high bytes of the lead-only values and ranges listed under [Detection](#detection). | Raw byte | S7, S9; S1 p. 369; S11, S12, S14, S15 (lead-only) |
| `0x21` | Chip ID low byte | `7:0` | `0x02` for the `0xD802` / NCT6799D scope; `0x21` for the exact `0xD421` / NCT6796D / NCT6796D-E Experimental scope; the low bytes of the lead-only values. S1 states the low digit encodes the IC version; the leads' `0xFFF8` mask ignores bits `2:0` of this byte when classifying a family. | Raw byte | S7, S9; S1 pp. 362, 369; S11, S12, S14, S15 (lead-only) |
| `0x28` (global) | HM IO space lock enable | bit `4` | `1` = the hardware-monitor I/O space is locked; `0` = not locked. S1 defines the bit for NCT6796D. The leads apply it to NCT6791D and later and report that HM port reads do not return register contents while it is set. This spec only reads the bit; clearing it is a write and is prohibited. Higher and lower bits are not interpreted. | Boolean lock bit | S1 p. 376; S11, S14 (lead-only for applicability and effect) |
| `0x07` | Logical Device Number select | `7:0` | Selects which logical device's registers are accessed at indexes `0x30` and above. | Raw logical-device number | S1 pp. 51-52, S2 |
| `0x0B` | Logical Device B / Hardware Monitor logical device | `7:0` | Selects Logical Device B, whose CR30 enables Hardware Monitor & SB-TSI and whose CR60/61 and CR64/65 define HM base addresses. | Raw logical-device number | S1 pp. 431-432; S7, S9; S11, S14, S15 (lead-only for the other chips) |
| `0x30` after LDN `0x0B` | Hardware Monitor & SB-TSI activation | bit `0` | `0` inactive, `1` active. Discovery may read this bit. This spec does not permit writing it. Higher bits are not interpreted by this revision. | Boolean active bit | S1 p. 431; S7 observed `0x09`; S9 observed `0x03`; S11, S14 (lead-only) |
| `0x60` / `0x61` after LDN `0x0B` | Normal HM base address | `15:0` | Selects the normal Hardware Monitor base address. The enabled scope requires a valid base; local and independent dumps observed `0x0290`. | I/O base address | S1 p. 431; S7, S9; S11, S14, S15 (lead-only) |
| `0x64` / `0x65` after LDN `0x0B` | Read-only HM base address | `15:0` | Optional read-only Hardware Monitor base. This revision records the value but does not use this path because evidence differs by board. | I/O base address | S1 p. 431; S7 observed `0x0000`; S9 observed `0x0A00` |

### Hardware-monitor access registers

For every enabled normal-HM path, use the base discovered from LDN B
`CR60/61` and these offsets. S1 confirms this layout for NCT6796D; S7/S9
validate it on `0xD802`; S11 and S14 describe the same offsets and bank
mechanism for the lead-only rows and unrecognized versions (lead-only).

| Relative offset | Register | Access | Meaning | Source |
| --- | --- | --- | --- | --- |
| `+0x05` | HM index port | Write | Selects a Hardware Monitor register index. With base `0x0290`, this is port `0x0295`. | S1 pp. 54-55; S7, S9; S11, S14 (lead-only) |
| `+0x06` | HM data port | Read/write for read plumbing | Reads the selected Hardware Monitor register; also receives the bank value after selecting the bank register. With base `0x0290`, this is port `0x0296`. | S1 pp. 54-55; S7, S9; S11, S14 (lead-only) |
| HM index `0x4E` | Bank select register | Write for read plumbing | Bits `2:0` select the bank used by subsequent banked HM register reads; write data value `0x04` to access bank 4. Bit 7 selects the high or low byte of the vendor-ID register and is left at its default by this spec. | S1 pp. 110, 176; S7, S9; S11, S14 (lead-only) |

Required writes in this table are read-transaction plumbing only. They
must not be generalized to fan-control, threshold, alarm, GPIO, lock, or
activation writes.

### Temperature registers

Bank 4 byte temperature registers are enabled per row as listed in
[Detection](#detection). S1 confirms `0x90`-`0x95` for the `0xD421` /
NCT6796D / NCT6796D-E Experimental profile; S7/S9 validate them for
`0xD802` / NCT6799D; S12/S14 describe the same six registers for every
lead-only row (lead-only). `0x96` is lead-only on every row that reads
it.

| Bank | Register | Generic source label | Decode | Rows | Source |
| --- | --- | --- | --- | --- | --- |
| `0x04` | `0x90` | SYSTIN | signed 8-bit degrees C, plausibility-filtered | every enabled row | S1 p. 176; S7, S9; S12, S14 (lead-only) |
| `0x04` | `0x91` | CPUTIN | signed 8-bit degrees C, plausibility-filtered | every enabled row | S1 p. 176; S7, S9; S12, S14 (lead-only) |
| `0x04` | `0x92` | AUXTIN0 | signed 8-bit degrees C, plausibility-filtered | every enabled row | S1 p. 176; S7, S9; S12, S14 (lead-only) |
| `0x04` | `0x93` | AUXTIN1 | signed 8-bit degrees C, plausibility-filtered | every enabled row | S1 p. 176; S7, S9; S12, S14 (lead-only) |
| `0x04` | `0x94` | AUXTIN2 | signed 8-bit degrees C, plausibility-filtered | every enabled row | S1 p. 176; S7, S9; S12, S14 (lead-only) |
| `0x04` | `0x95` | AUXTIN3 | signed 8-bit degrees C, plausibility-filtered | every enabled row | S1 p. 176; S7, S9; S12, S14 (lead-only) |
| `0x04` | `0x96` | Temperature 7 | signed 8-bit degrees C, plausibility-filtered (same decode as `0x90`-`0x95`) | `0xD802` (Experimental sub-scope), `0xD429`, `0xD42A`, `0xD42B`, `0xD801`, unrecognized `0xD428`-`0xD42F` | S12, S14 (lead-only); S9 byte observation on `0xD802` |

The `0x96` label is deliberately generic: the two leads that read it
name its source differently, and S1 does not define the register, so
this spec does not assign it a physical input name.

Lead-only decode exception for the `0xD806` / NCT6701D row and every
unrecognized `0xD800`-`0xD807` version: raw bytes `0x00`, `0xA0`, and
`0x7E`-`0x80` are not temperatures and must be reported as unavailable
for that input rather than decoded. (S14) Of these, only `0x00` lies
inside the plausibility bounds; the others are also rejected by the
plausibility filter.

### Temperature plausibility bounds

| Fact | Source |
| --- | --- |
| S1 Table 8-1 lists the temperature data format from +125 degrees C down to -55 degrees C; its 8-bit two's-complement examples run from `0x7D` (+125) through `0x00` (0) to `0xC9` (-55). | S1 §8.6.3 p. 60 |
| The project's existing Super I/O byte-temperature plausibility policy accepts -55 <= t <= 125 degrees C inclusive, keeps 0 degrees C as a valid sample, and omits an implausible input without publishing zero, a clamped boundary, or a whole-device failure. | S17 |
| Accepted bounds for every Nuvoton byte temperature in this spec: -55 <= t <= 125 degrees C inclusive after signed 8-bit decode. In raw-byte terms, `0x00`-`0x7D` and `0xC9`-`0xFF` are accepted; `0x7E`-`0xC8` (+126 through +127 and -128 through -56) are rejected. | S1 p. 60; S17; S18 |

S7 (`0x0F`-`0x29`) and S9 (`0x0E`-`0x24`, `0x96` = `0x19`) captured only
accepted bytes.

S7 captured these local raw bank 4 temperature bytes on the observed
NZXT N7 B650E `0xD802` board:

| Register | Raw value |
| --- | --- |
| `0x90` | `0x27` |
| `0x91` | `0x23` |
| `0x92` | `0x29` |
| `0x93` | `0x0F` |
| `0x94` | `0x13` |
| `0x95` | `0x10` |
| `0x96` | not captured (not in the requested set) |

S9 independently captured the same bank/register set on an ASUS ROG
Strix X670E-F `D802h` / NCT6799D board:

| Register | Raw value |
| --- | --- |
| `0x90` | `0x20` |
| `0x91` | `0x24` |
| `0x92` | `0x11` |
| `0x93` | `0x16` |
| `0x94` | `0x16` |
| `0x95` | `0x0E` |
| `0x96` | `0x19` (byte only; the dump does not label it) |

### Fan tachometer / RPM registers

Every enabled path reads direct 16-bit RPM registers only. S1 defines
all seven direct pairs for the `0xD421` Experimental profile; S7/S9
validate the first six pairs for `0xD802` and observe the seventh; S12
describes the same seven register positions for every lead-only row.
Count registers remain disabled because direct RPM registers avoid
count-to-RPM conversion ambiguity.

| Generic fan | Bank | Direct RPM high | Direct RPM low | Decode | Enabled for | Source |
| --- | --- | --- | --- | --- | --- | --- |
| Fan 1 / SYSFANIN candidate | `0x04` | `0xC0` | `0xC1` | high byte then low byte, unsigned RPM | all enabled IDs | S1 p. 66; S7, S9; S12 (lead-only) |
| Fan 2 / CPUFANIN candidate | `0x04` | `0xC2` | `0xC3` | high byte then low byte, unsigned RPM | all enabled IDs | S1 p. 66; S7, S9; S12 (lead-only) |
| Fan 3 / AUXFANIN0 candidate | `0x04` | `0xC4` | `0xC5` | high byte then low byte, unsigned RPM | all enabled IDs | S1 p. 66; S7, S9; S12 (lead-only) |
| Fan 4 / AUXFANIN1 candidate | `0x04` | `0xC6` | `0xC7` | high byte then low byte, unsigned RPM | all enabled IDs | S1 p. 66; S7, S9; S12 (lead-only) |
| Fan 5 / AUXFANIN2 candidate | `0x04` | `0xC8` | `0xC9` | high byte then low byte, unsigned RPM | all enabled IDs | S1 p. 66; S7, S9; S12 (lead-only) |
| Fan 6 / AUXFANIN3 candidate | `0x04` | `0xCA` | `0xCB` | high byte then low byte, unsigned RPM | all enabled IDs except the NCT6779D range row | S1 p. 66; S7, S9; S12 (lead-only) |
| Fan 7 / AUXFANIN4 candidate | `0x04` | `0xCE` | `0xCF` | high byte then low byte, unsigned RPM | Experimental on `0xD802` (sub-scope), `0xD421`, `0xD423`, `0xD429`, `0xD42A`, `0xD42B`, `0xD451`, `0xD801`, `0xD806`, and unrecognized versions of the `0xD420`, `0xD428`, `0xD450`, `0xD800` ranges | S1 p. 66; S7, S9 observations on `0xD802`; S12 (lead-only for the other chips) |

S7 local direct RPM bytes:

| Registers | Raw high/low | Direct RPM |
| --- | --- | --- |
| `0xC0`/`0xC1` | `0x00`/`0x00` | `0` |
| `0xC2`/`0xC3` | `0x0B`/`0x08` | `2824` |
| `0xC4`/`0xC5` | `0x00`/`0x00` | `0` |
| `0xC6`/`0xC7` | `0x02`/`0xE2` | `738` |
| `0xC8`/`0xC9` | `0x03`/`0x2C` | `812` |
| `0xCA`/`0xCB` | `0x00`/`0x00` | `0` |
| `0xCE`/`0xCF` | `0x00`/`0x00` | `0` (stopped or unconnected; Experimental `Fan 7` on `0xD802`) |

S9 independent direct RPM bytes:

| Registers | Raw high/low | Direct RPM |
| --- | --- | --- |
| `0xC0`/`0xC1` | `0x03`/`0x59` | `857` |
| `0xC2`/`0xC3` | `0x03`/`0x2D` | `813` |
| `0xC4`/`0xC5` | `0x03`/`0x40` | `832` |
| `0xC6`/`0xC7` | `0x03`/`0x55` | `853` |
| `0xC8`/`0xC9` | `0x03`/`0x42` | `834` |
| `0xCA`/`0xCB` | `0x03`/`0x8A` | `906` |
| `0xCE`/`0xCF` | `0x15`/`0x59` | `5465` (Experimental `Fan 7` on `0xD802`; the adjacent count bytes `0xCC`/`0xCD` = `0x07`/`0x17` are not saturated, consistent with a connected seventh input) |

Captured but disabled fan-related registers:

| Bank | Registers | Reason disabled |
| --- | --- | --- |
| `0x04` | `0xB0`-`0xBB`, `0xCC`/`0xCD` | Count-path bytes are not needed while direct RPM registers are available. |
| `0x04` | `0xCE`/`0xCF` for five- and six-input rows | Not populated on those rows per S12/S14 (lead-only). |

### Input identity and ordering stability

Each enabled input is identified by its generic label (`SYSTIN` ...
`AUXTIN3`, `Temperature 7`, `Fan 1` ... `Fan 7`), which is bound to one
register for all rows and revisions. Requirements (S18):

- No revision reassigns an existing label to a different register,
  renames an existing input, or renumbers an existing fan.
- Inputs added by revision 8 (`Temperature 7` at `0x96`, `Fan 7` at
  `0xCE`/`0xCF`, and on `0xD42A` the labels `SYSTIN`, `CPUTIN`, `AUXTIN0`,
  `AUXTIN2`) are appended after the inputs the row exposed in revision 7,
  in register order, wherever a consumer depends on list position.
- An input that is unavailable in a sample (plausibility rejection,
  invalid NCT6701D byte, or a failed Experimental sub-scope read) keeps
  its identity. A consumer that keys inputs by label may omit it for
  that sample. A consumer that depends on list position must report it
  as an explicit unavailable slot at its position, so later inputs keep
  their positions.

### D421 layout boundary

The `0xD421` profile reuses only the layout facts that S1 explicitly
defines for NCT6796D: LDN B normal-HM base discovery, normal-HM index/data
offsets, bank 4 selection, temperature registers `0x90`-`0x95`, and the
seven direct RPM pairs `0xC0`-`0xCB` and `0xCE`/`0xCF` of the S1 p. 66
table. S1 does not identify `0xD802`, so the `0xD421` profile does not
assume that every register observed on a `0xD802` board has the same
meaning: bank 4 `0x96` is not read for `0xD421`. The `0xD421` profile
also applies the read-only `CR28` bit 4 lock gate, because S1 p. 376
defines that bit for NCT6796D itself; the gate is a read and a stop, not
a layout extension. Read-only HM, count-based RPM, and any other
D802-only or board-specific extension remain disabled for `0xD421`.
(S1, S7, S9)

### Lead-only row layout boundary

The lead-only rows and unrecognized versions reuse the S1 NCT6796D
layout only where the leads describe the same register positions for the
matched chip or range: LDN B discovery (S11, S14, S15), normal-HM
index/data offsets and bank 4 selection (S11, S14), byte temperatures
`0x90`-`0x95` (S12, S14), `0x96` only on the rows listed for it (S12,
S14), and the populated direct RPM pairs (S12; S14 reads count registers
instead). They do not adopt any half-degree word register, voltages,
count registers, or the read-only HM base. The `0xD806` row and
unrecognized `0xD800`-`0xD807` versions additionally apply the lead-only
invalid-byte filter (S14).

## Procedures

### Discovery procedure

1. Acquire `Global\Access_ISABUS.HTP.Method` with a bounded timeout
   before any Super I/O index/data or hardware-monitor I/O transaction.
   If the mutex cannot be acquired, abort the transaction rather than
   proceeding unlocked. (S2)
2. Use the Phase 2 Nuvoton configuration-mode procedure to read raw
   chip-id bytes from `0x20` / `0x21`. If the responder is absent under
   the Phase 2 absent-id rules, stop. (S2)
3. Select the profile: first an exact named match (`0xD802` Verified,
   label `NCT6799D`; `0xD421` Experimental, label `NCT6796D` /
   `NCT6796D-E`; a lead-only exact row; or the NCT6779D range
   `0xC560`-`0xC567`), otherwise an unrecognized-version match of one of
   the eight masked ranges with that range's conservative profile and
   family label. Do not add unproven package suffixes. Any other chip ID
   is outside this document's enabled runtime scope. Every Experimental
   attempt is limited to the layout boundary stated for its row, and its
   failures are reported as experimental. (S1, S7, S9, S18; S11, S12,
   S14, S15 lead-only)
4. For profiles whose Lock gate is "Yes" (the exact `0xD421`
   Experimental profile, NCT6791D-and-later lead-only rows, and every
   unrecognized-version range member), read global `CR28` while still in
   configuration mode. If bit 4 is set, stop and report the chip as
   unavailable with an experimental diagnostic; do not write `CR28`. For
   `0xD421` the bit definition is S1 p. 376; for the other profiles its
   applicability is lead-only. `0xD802` skips this step because S7/S9
   show its normal-HM reads working without it. (S1 p. 376; S7, S9; S11,
   S14 lead-only)
5. Select LDN `0x0B`, read `CR30`, and require bit 0 to be set. Do not
   write `CR30`. For `0xD806` and unrecognized `0xD800`-`0xD807`
   versions, an inactive LDN B is a known firmware state and is reported
   as unavailable, not worked around. (S1, S7, S9; S14 lead-only)
6. Read `CR60/61` as the normal HM base and require it to be a valid I/O
   base in the Phase 2 / PawnIO-safe range. Record `CR64/65` if useful
   for diagnostics, but do not use it as an access base in rev 8, not
   even for `0xD806` or unrecognized `0xD800`-`0xD807` versions (see Open
   questions). (S7, S9; S14 lead-only)
7. Call PawnIO `ioctl_find_bars` after slot selection and LDN B/base
   discovery, before normal HM index/data I/O. On the local board this
   order changed `ioctl_find_bars` from `0x80070490` to success and
   authorized normal HM port I/O. (S7)
8. Use normal HM index port `base + 0x05` and data port `base + 0x06`.
   Select bank 4 by writing index `0x4E`, then data `0x04`. For the
   lead-only rows and unrecognized versions the same offsets and bank
   mechanism are lead-only. (S1, S7, S9; S11, S14 lead-only)
9. Read the enabled register set of the selected profile: the bank 4
   temperatures listed for the row (`0x90`-`0x95`, plus `0x96` where
   listed) and the direct RPM pairs populated for the row (`0xC0`-`0xC9`
   for the NCT6779D range; `0xC0`-`0xCB` for six-input rows; `0xC0`-`0xCB`
   plus `0xCE`/`0xCF` for seven-input rows, `0xD802`, `0xD421`, and
   `0xD423`). For `0xD421`, these addresses are used only because S1
   defines them for NCT6796D; for lead-only rows and unrecognized
   versions, only because S12 and S14 describe the temperature bytes and
   S12 describes the direct RPM pairs for the matched chip or kind. On
   every row other than `0xD802`, abort the current sample on any failed
   index write or data read; do not synthesize values from partial
   reads. (S1, S7, S9; S12, S14 lead-only)
10. On `0xD802`, read the Verified register set (`0x90`-`0x95`,
    `0xC0`-`0xCB`) first and apply the abort rule of step 9 to it. Then
    read the Experimental sub-scope registers (`0xCE`/`0xCF`, `0x96`). If
    an Experimental sub-scope index write or data read fails, stop
    reading further sub-scope registers in that sample, keep the
    Verified values already read, report the affected Experimental
    inputs as unavailable, and identify the failed read as experimental
    in the diagnostic detail. (S10, S18)
11. Exit Nuvoton configuration mode and release the ISA mutex in all
    paths. (S2)

### Temperature decode

- Decode each enabled bank 4 temperature byte as signed 8-bit degrees C.
  Preserve the raw byte alongside the decoded value in diagnostics.
- For the `0xD806` / NCT6701D row and unrecognized `0xD800`-`0xD807`
  versions, report raw bytes `0x00`, `0xA0`, and `0x7E`-`0x80` as
  unavailable instead of decoding them. (S14 lead-only)
- Apply the plausibility filter to every decoded value on every row,
  including Verified `0xD802`: accept only -55 <= t <= 125 degrees C.
  A value outside the bounds is reported unavailable for that input; it
  is not published as a number, not clamped, not replaced by zero, and
  does not fail the other inputs of the sample or the whole device. Zero
  degrees C remains a valid sample except where the NCT6701D filter
  applies. Diagnostics may keep the raw byte. (S1 p. 60; S10, S17, S18)
- Use generic source labels from the register table unless a later
  board-specific evidence file safely narrows physical labels.
- Do not merge motherboard/board temperature inputs with Phase 1 CPU
  package temperature sources.

### Fan RPM decode

- For enabled direct 16-bit RPM registers, combine high then low bytes
  and interpret the resulting unsigned 16-bit integer as RPM. (S1 p. 66;
  S7, S9)
- A direct RPM value of `0x0000` is a valid stopped-or-unconnected raw
  reading. Do not treat zero as a read failure and do not infer physical
  disconnection from zero alone. (S7, S9)
- Read only the pairs populated for the matched profile; an unpopulated
  pair (for example `0xCA`/`0xCB` on the NCT6779D range row, or
  `0xCE`/`0xCF` on six-input rows) is not read and not reported. (S12,
  S14 lead-only)
- Do not use count-path bytes at `0xB0`-`0xBB` or `0xCC`/`0xCD`.

## Quirks

- `0xD802` is implementation-ready only as `NCT6799D` without a package
  suffix claim. Public board-review and user-report leads make
  `NCT6799D-R` plausible, but rev 8 does not prove or expose `-R`. (S8,
  S9)
- `0xD421` is implementation-ready as an Experimental
  `NCT6796D` / `NCT6796D-E` profile only for the S1-backed normal-HM,
  bank 4 temperature, and seven direct RPM-pair reads. It has no matching
  hardware dump and must not be treated as Verified. (S1, S10)
- For the observed local board, `ioctl_find_bars` must run after Nuvoton
  configuration/base discovery has made the HM base visible to PawnIO
  `LpcIO`; running it before config discovery returned `0x80070490` and
  left normal HM ports unauthorized. (S7)
- Read-only HM base availability is board-variable in current evidence:
  S7 observed `0x0000`, S9 observed `0x0A00`. Rev 8 does not use the
  read-only HM path. (S7, S9)
- HM I/O-space lock: global `CR28` bit 4 is defined by S1 for NCT6796D
  as "HM IO space lock enable", which is why the exact `0xD421` profile
  applies the gate on primary-source grounds. The leads (lead-only
  applicability) apply it to NCT6791D and every later chip in the lead-only rows and
  describe its effect as the hardware-monitor logical-device mapping
  being disabled, so HM port reads return nothing meaningful (one lead
  tests this by whether the vendor-ID word can be read). Both leads clear
  the bit, which is a write; this spec only detects the state and reports
  the chip unavailable. (S1 p. 376; S11, S14 lead-only)
- NCT6779D range row: the chip has five fan inputs, so `0xCA`/`0xCB` is
  not populated and is not read. (S12, S14 lead-only)
- `0xD806` / NCT6701D (lead-only): some firmware leaves LDN B inactive
  (reported as unavailable here, never activated); `CR60/61` may hold a
  value that is not a usable base (below `0x100` or with any bit of
  `0xF007` set), in which case the lead falls back to `CR64/65` while
  this spec reports unavailable; raw temperature bytes `0x00`, `0xA0`,
  `0x7E`-`0x80` are invalid; the lead skips its vendor-ID check for this
  chip. Unrecognized `0xD800`-`0xD807` versions inherit the same
  unavailable-reporting and invalid-byte handling. (S14 lead-only)
- NCT6796D-S and NCT6799D-R are described as very similar parts whose
  chip IDs differ only by version; the kernel lead treats them the same
  way. (S13 lead-only)
- On listed ASUS boards the kernel lead reaches the Super I/O and
  hardware-monitor registers through an ACPI WMI method instead of direct
  port I/O, which indicates that firmware shares the HM region on those
  boards. The project's `Global\Access_ISABUS.HTP.Method` mutex remains
  the Windows-side mitigation; S9 shows direct port reads working on an
  ASUS `0xD802` board. (S11 lead-only; S9)

## Safety notes

- This document is read-oriented. Required writes are limited to
  read-transaction plumbing: Nuvoton configuration-mode enter/exit,
  logical-device selection, normal HM index selection, and normal HM bank
  selection.
- Do not write activation, lock (`CR28`), fan-control/PWM, Smart Fan,
  threshold, limit, alarm-clear, GPIO, or vendor/OEM registers from this
  spec.
- Hold the ISA mutex for the whole Super I/O / HM transaction and abort
  rather than racing another monitor.
- Require elevation/PawnIO access. If `pawnio_open` fails with
  `0x80070005`, surface an unavailable/permission diagnostic rather than
  retrying through unsafe access paths. (S5)
- Keep all disabled scopes disabled unless a later spec revision updates
  the scoped enablement table and records the necessary evidence.

## Open questions

- Non-blocking for Phase 3: A public Nuvoton NCT6799D/NCT6799D-R
  datasheet and official chip-id table remain unavailable; rev 8 is
  scoped to independently verified raw chip ID `0xD802` / `NCT6799D` and
  does not claim package suffix, OEM variant, or package revision.
- Non-blocking for Phase 3: Board-specific physical header labels remain
  unresolved; rev 8 exposes generic fan and temperature labels and
  preserves zero RPM as stopped-or-unconnected without classifying
  physical connection state.
- Non-blocking for Phase 3: no matching hardware dump was captured for
  `0xD421` in this validation set. The exact S1-backed read profile is
  therefore Experimental and can graduate to Verified only after a
  maintainer-accepted dump confirms the chip ID, discovered normal-HM
  base, bank 4 reads, and direct RPM pairs including `0xCE`/`0xCF`.
- Non-blocking for Phase 3: Read-only HM access remains disabled for
  every ID; the enabled scopes use only normal HM bank 4 temperatures
  and the direct RPM pairs listed per row.
- Non-blocking for Phase 3: the seventh direct RPM pair on `0xD802` is
  Experimental rather than Verified. S1 p. 66 defines it for NCT6796D
  only, and the two `0xD802` dumps show a plausible live value (S9, 5465
  RPM with non-saturated count bytes) and a stopped/unconnected zero
  (S7), but neither labels the pair. A maintainer-accepted `0xD802` dump
  that ties `0xCE`/`0xCF` to a known physical fan header (or a primary
  source for NCT6799D) can graduate the sub-scope to Verified.
- Non-blocking for Phase 3: `0xD802` labelling conflict. S12 and S11
  attribute `0xD801` to NCT6796D-S and `0xD802` to NCT6799D-R, while S14
  labels `0xD802` as NCT6796D-S on ASRock X870E Nova WiFi and B650M HDV
  M.2 and as NCT6799D elsewhere; the project's S9 dump labels `D802h`
  NCT6799D. The project reads the raw ID and uses generic source labels,
  the leads describe the same register path for both parts (S12 same
  kind; S13 "treated the same way"), and the Verified row's register
  facts rest on S7/S9, so the label conflict does not change the enabled
  path. The `0xD801` row is attested by a single lead family (S11/S12)
  and has no S14 entry.
- Non-blocking for Phase 3: `0xD42A` dual identity. S14 labels it
  NCT5585D on ASRock X870E Nova WiFi and NCT6796D-R otherwise; S11/S15
  place it in a range that S11 attributes to both NCT6798D and NCT5585D;
  S12 lists NCT5585D under the same kind; S13 calls NCT5585D compatible
  to NCT6798D. Rev 8 surfaces the generic label `NCT6796D-R / NCT5585D`
  and reads all six bytes `0x90`-`0x95` plus `0x96` as Experimental:
  S12 describes all seven for the whole `0xD428` kind and S14 reads
  `0x90`-`0x95` under its NCT6796D-R label, while S14's NCT5585D table
  reads only `0x93` and `0x95` and neither S14 table reads `0x96`. On an
  NCT5585D board the extra bytes may not be temperature readings; the
  plausibility filter hides out-of-bounds bytes, but an in-bounds byte
  without a connected sensor would still be shown. A maintainer-accepted
  dump from either board type can narrow or graduate the row.
- Non-blocking for Phase 3: NCT6796D fan count for `0xD423` (and
  `0xD421`). S1 (seven inputs including AUXFANIN4) and S12 (7) disagree
  with S14 (6). Rev 8 reads seven pairs on both IDs, with the seventh
  Experimental, because the S1 primary source defines the seventh pair;
  on a board that does not wire AUXFANIN4 the pair is expected to read
  zero, which is reported as stopped-or-unconnected.
- Non-blocking for Phase 3: unrecognized-version matching. S11 and S15
  match the family after masking the low three ID bits and S1 p. 362
  states that the CR21 low digit encodes the IC version, so rev 8
  enables unnamed members of the eight masked ranges with the
  conservative profile of their named members. No lead or dump attests
  any unnamed value, so a future version could populate fewer inputs or
  differ in ways the conservative profile does not anticipate; the
  plausibility filter and the zero-RPM rule bound the visible effect,
  and a dump for any unnamed value can promote it to a named row. The
  NCT6779D extension `0xC568`-`0xC56F` (S14 only) remains Unsupported
  because S11/S15 place those values outside the `0xC560` family.
- Non-blocking for Phase 3: `0xD806` / NCT6701D alternate base. S14 uses
  `CR64/65` as the hardware-monitor base when `CR60/61` is not a usable
  base, which conflicts with S1's definition of `CR64/65` as the read-only
  HM base for NCT6796D. Rev 8 requires a valid `CR60/61` and otherwise
  reports the chip (or unrecognized `0xD800`-`0xD807` version)
  unavailable; the lead-only fallback is not adopted.
- Non-blocking for Phase 3: a vendor-ID sanity read (S1 pp. 110-111
  define the 16-bit Nuvoton vendor ID `0x5CA3` at bank 0 index `0x4F`,
  byte-selected by bank-select bit 7) could detect an engaged lock or a
  non-Nuvoton responder without writes other than bank selection, but
  S14 skips it for NCT6701D. It is not adopted in rev 8; the `CR28` read
  is the only lock gate.
- Non-blocking for Phase 3: bank 4 `0x96` meaning. S1 does not define the
  register for NCT6796D, S12 adds it only for the `0xD428`/`0xD800`
  kinds, and S14 reads it under two tables that name its source
  differently. Rev 8 reads it as Experimental with the generic label
  `Temperature 7` on the rows listed for it. S9 captured `0x19` on a
  `0xD802` board but does not label it; S7 did not capture it. A
  maintainer-accepted dump that labels the input can graduate the
  `0xD802` sub-scope. `0xD806` is excluded because the only lead naming
  NCT6701D does not read the byte.
- Non-blocking for Phase 3: plausibility bounds. The -55 to +125 degrees
  C window is the datasheet's stated format range (S1 p. 60) and the
  project's existing Super I/O policy (S17). A connected sensor reporting
  outside the window, or a disconnected input that reads an in-window
  byte, is not distinguished by this filter; the filter only prevents
  publishing bytes the datasheet range excludes.
- Non-blocking for Phase 3: no hardware dump exists for any lead-only
  row or unrecognized version. Each lead-only row graduates to Verified
  only when a primary source or a maintainer-accepted independent
  hardware dump confirms every fact it depends on, including its chip
  ID, LDN B base, bank 4 temperature bytes, the populated direct RPM
  pairs, and, for rows whose lock gate is enabled, the `CR28` bit 4
  state.

## Implementation-ready transition checklist

Revision 8 satisfies the directory-level checklist in [`README.md`](README.md):

- no unresolved provenance marker remains in this document;
- every enabled D421 fact is pinned to S1/S2, and every Verified D802
  fact is pinned to S1/S2 or independently verified by S7 and S9 hardware
  dumps; the new plausibility filter rests on S1 p. 60 and project policy
  (S17, S18), not on a lead;
- every lead-only fact is tagged (`lead-only (copyleft)` in the Sources
  table for S11-S15, source IDs on each fact row and quirk entry) and is
  depended on only by scoped-enablement rows or sub-scopes whose default
  enablement is Experimental; the `0xD802` Verified scope depends on no
  lead-only fact, and its seventh-pair and `0x96` reads are Experimental
  sub-scopes whose failure does not affect the Verified values;
- the scoped-enablement tables distinguish the D421 Experimental path,
  the D802 Verified path with its Experimental sub-scopes, the lead-only
  Experimental rows with their fan counts, temperature sets, and lock
  gate, the unrecognized-version range profiles, and disabled or
  unsupported scopes;
- every remaining open question is explicitly annotated as non-blocking
  for Phase 3; and
- the revision, status, and revision history record the change.

## Provenance text for a future clean-room implementation PR

Implementation PRs should pin this ready revision and commit, for example:

```text
Implemented from docs/specs/sensors/superio-access.md revision 3 (commit a8c167b1), limited to Super I/O configuration access and chip-id diagnostic facts.
Implemented from docs/specs/sensors/superio-nuvoton-nct67xx.md revision 8 (commit <ready-commit>), limited to the `0xD802` / NCT6799D Verified scope with its Experimental sub-scopes, the `0xD421` / NCT6796D / NCT6796D-E Experimental scope, the lead-only Experimental rows, and the unrecognized-version range profiles marked enabled in that revision.
No other external sensor documentation was used.
```

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-06-28 | Initial Phase 3 Nuvoton-family draft; records Phase 2 diagnostic linkage, expected hardware-monitor spec shape, safety constraints, and unresolved provenance/dump requirements. |
| 2 | 2026-06-28 | Fetched and verified the official NCT6796D V0.6 datasheet; pinned NCT6796D configuration, HM base, temperature, and fan RPM facts; recorded that S1 maps NCT6796D to `0xD421` while the observed S3 board is `0xD802`; recorded standard-rights PawnIO dump failure (`0x80070005`). Status remains Draft. |
| 3 | 2026-06-28 | Added elevated PawnIO `LpcIO` evidence for the observed `0xD802` board: LDN B active, normal HM base `0x0290`, read-only HM base `0x0000`, and blocked HM index/data access (`ioctl_find_bars=0x80070490`, `pio_outb(0x0295)=0x80070005`). Status remains Draft. |
| 4 | 2026-06-28 | Added successful elevated normal-HM bank 4 byte-dump evidence, recorded the `ioctl_find_bars` ordering requirement, narrowed stopped/zero-RPM handling, and documented the `0xD802` source-hunt result. Exact-chip mapping remains unresolved, so status remains Draft. |
| 5 | 2026-06-28 | Added independent AIDA64 dump evidence mapping `0xD802` to `NCT6799D`, scoped support to normal HM bank 4 temperatures and direct RPM pairs, disabled unresolved suffix/read-only/AUXFANIN4 scopes, and flipped status to Implementation-ready. |
| 6 | 2026-09-13 | Added exact raw `0xD421` / NCT6796D / NCT6796D-E as an Experimental read-only scope using only the S1-confirmed normal-HM, bank 4 temperature, and six direct RPM-pair layouts; explicitly kept D802-only seventh-fan observations and other unverified extensions out of scope. Status remains Implementation-ready. |
| 7 | 2026-10-25 | Added lead-only (ADR 0023) Experimental rows for NCT6779D (`0xC560`-`0xC567`), NCT6791D `0xC803`, NCT6792D `0xC911`/`0xC913`, NCT6793D `0xD121`, NCT6795D `0xD352`, NCT6796D `0xD423`, NCT6798D `0xD429`/`0xD42B`, `0xD42A` (NCT6796D-R / NCT5585D), NCT6797D `0xD451`, NCT6796D-S `0xD801`, and NCT6701D `0xD806` with per-row fan counts, a seventh direct RPM pair `0xCE`/`0xCF` for seven-input rows, a read-only `CR28` bit 4 HM I/O-space-lock gate, and NCT6701D read-only quirks; added S1 pins for the 8.8.1 seven-input table (p. 66), the CR21 version note (p. 362), and CR28 bit 4 (p. 376); recorded S7/S9 `0xCC`-`0xCF` bytes as evidence; added copyleft lead sources S11-S15 tagged lead-only and ADR 0023 as S16; recorded the `0xD802` / NCT6796D-S labelling conflict, `0xD42A` dual identity, range-matching policy, and NCT6701D alternate-base conflict as non-blocking open questions. `0xD802` Verified and `0xD421` Experimental rows unchanged in substance. Status remains Implementation-ready. |
| 8 | 2026-10-25 | Applied ADR 0011 per the maintainer decision of 2026-10-25 (S18) to four reads rev 7 kept disabled only for lack of verification, all as Experimental: the seventh direct RPM pair `0xCE`/`0xCF` on `0xD802` (sub-scope of the Verified row), `0xD421`, and `0xD423`; unrecognized versions of the eight masked lead family ranges with a per-range conservative profile and family label (NCT6779D row unchanged at `0xC560`-`0xC567`); all six bank 4 temperatures on `0xD42A`; and bank 4 `0x96` (generic `Temperature 7`) on `0xD802`, `0xD429`, `0xD42A`, `0xD42B`, `0xD801`, and unrecognized `0xD428`-`0xD42F` versions. Added a byte-temperature plausibility filter (-55 to +125 degrees C, S1 Table 8-1 p. 60; consistent with S17) for every row including Verified `0xD802`; added the D802 sub-scope failure rule and an input identity/ordering stability requirement; recorded the S9 `0x96` byte (`0x19`), the S14 per-table `0x96` coverage, and S17/S18 as sources; updated the seventh-pair, `0xD42A`, `0xD423` fan-count, range-matching, and `0x96` open questions and added a plausibility-bounds open question. Lock-gate stop, no `CR64/65` fallback, disabled count registers, and disabled read-only HM unchanged. Review amendments before merge (same revision, PR #2464): the read-only `CR28` bit 4 lock gate now also applies to the exact `0xD421` profile on S1 p. 376 alone (`0xD802` stays ungated because S7/S9 show its HM reads working), and the ordering-stability rule now requires position-keyed consumers to emit an explicit unavailable slot for an unavailable input. Status remains Implementation-ready. |
