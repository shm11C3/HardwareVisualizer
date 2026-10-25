# Nuvoton NCT67xx / NCT679x Super I/O Hardware Monitor Spec

| Field | Value |
| --- | --- |
| Revision | 7 |
| Status | Implementation-ready (rev 7) |
| Scope | Phase 3 Nuvoton-family motherboard temperature and fan RPM reads for the scoped `0xD802` / `NCT6799D` Verified path, the exact `0xD421` / `NCT6796D` / `NCT6796D-E` Experimental path, and the lead-only Experimental chip-id rows added in revision 7 (NCT6779D range `0xC560`-`0xC567`; exact `0xC803`, `0xC911`, `0xC913`, `0xD121`, `0xD352`, `0xD423`, `0xD429`, `0xD42A`, `0xD42B`, `0xD451`, `0xD801`, `0xD806`). Every enabled path uses read-oriented normal-HM banked access for bank 4 byte temperatures (`0x90`-`0x95`) and direct 16-bit RPM registers (`0xC0`-`0xCB`, plus `0xCE`/`0xCF` only for the seven-input lead-only rows) after chip-id, LDN B, HM-base, and (for NCT6791D-and-later lead-only rows) HM I/O-space-lock validation. It excludes exact package suffix claims such as `-R`, read-only HM access, voltage decode, count-based fan RPM decode, bank 4 `0x96`, fan-control/PWM writes, threshold/limit writes, lock clearing, activation writes, alarm clearing, GPIO, UI integration, and any Rust implementation. |
| Issue phase | Phase 3 (#1635) - Nuvoton NCT67xx / NCT679x temperature and fan RPM specification |

Revision 7 keeps this document **Implementation-ready** and widens the
Experimental coverage under [ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md).
The exact `0xD802` / `NCT6799D` path remains Verified by the independent
hardware evidence described below, and the exact `0xD421` / NCT6796D /
NCT6796D-E path remains Experimental on S1 alone; neither row changes in
substance. The new rows rest on **lead-only facts**: copyleft leads
describe the same LDN B / `CR60-61` discovery (S11, S14, S15) with the
`CR30` enable bit (S11), the same bank 4 byte temperatures (S12, S14), and
the same direct RPM pairs (S12) that S1 pins for NCT6796D, and differ only
in the raw chip ID, the number of populated fan inputs (S12, S14), and a
few read-only quirks. Those facts back Experimental rows only; no
Verified fact depends on them.

S1 does not identify `0xD802`. The package suffix/revision for `0xD802`
remains unknown and must not be surfaced as `NCT6799D-R`.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Nuvoton, *NCT6796D LPC/eSPI SI/O Datasheet*, version 0.6, publication release date July 6, 2017; official PDF: `https://www.nuvoton.com/resource-files/NCT6796D_Datasheet_V0_6.pdf` | Primary source for NCT6796D-only facts and family register semantics. Relevant pins: configuration protocol chapter 7, pp. 51-53; hardware-monitor LPC access chapter 8.3, pp. 54-55; temperature format chapter 8.6.3, pp. 60-62; fan speed reading chapter 8.8.1-8.8.3, pp. 66-67 (the 8.8.1 table on p. 66 lists seven fan inputs SYSFANIN, CPUFANIN, AUXFANIN0-AUXFANIN4 with bank 4 count pairs `0xB0`/`0xB1` ... `0xBA`/`0xBB`, `0xCC`/`0xCD` and 16-bit RPM pairs `0xC0`/`0xC1` ... `0xCA`/`0xCB`, `0xCE`/`0xCF`); fan count/RPM registers chapter 9.209-9.232, pp. 176-183; bank select and vendor ID chapter 9.30-9.31, pp. 110-111; HM read-only register chapter 9.481, pp. 277-280; global register default table and the note that the CR21 low digit encodes the IC version, p. 362; global chip ID CR20/CR21, p. 369; global CR28 bit 4 "HM IO space lock enable", p. 376; Logical Device B CR30/CR60-65, pp. 431-432. S1 does not identify `0xD802`. |
| S2 | [`superio-access.md`](superio-access.md) revision 3 | Primary clean-room source for the Phase 2 Nuvoton configuration-mode key sequence, global chip-id register reads (`0x20` / `0x21`), absent-id classification, standard Super I/O configuration port pairs, and ISA mutex policy. |
| S3 | HardwareVisualizer PR #1732 real-hardware diagnostic dump, captured 2026-06-27 on NZXT N7 B650E / AMD Ryzen 7 7800X3D / Windows 11 Pro | Independent hardware evidence for a reachable Nuvoton-class responder at slot 0 (`0x2E`/`0x2F`) with raw chip-id bytes `0xD8` / `0x02`; this is not the NCT6796D `0xD4` / `0x21` ID from S1. |
| S4 | HardwareVisualizer issue #1635 clean-room policy and [`README.md`](README.md) status-transition rules | Project policy source for the lead-only tagging rules and for allowing independently collected hardware dumps to verify facts when primary section/page references are unavailable. |
| S5 | Local standard-rights PawnIO probe on 2026-06-28 | Environment evidence only: `C:\Program Files\PawnIO` has `PawnIOLib.dll` and `LpcIO.bin`, PawnIO service is running, `pawnio_version` returned `0x00020000`, but `pawnio_open` returned `0x80070005` under medium integrity. |
| S6 | Local elevated PawnIO `LpcIO` probe on 2026-06-28, run from Administrator PowerShell on the S3-class machine | Independent hardware evidence for the pre-fix blocker: `pawnio_open`, `pawnio_load`, and `Global\Access_ISABUS.HTP.Method` mutex acquisition succeeded; slot 0 repeated `chipId=0xD802`; LDN B read `CR30=0x09`, `CR60/61=0x02/0x90` (`HM base=0x0290`), and `CR64/65=0x00/0x00` (no valid read-only HM base). `ioctl_find_bars` returned `0x80070490`, and a normal HM index-port write to `0x0295` returned `0x80070005`, so no temperature or fan RPM bytes were captured in that run. |
| S7 | [`docs/development/sensor-handoff/evidence/2026-06-28-superio-hm-dump-admin.md`](../../development/sensor-handoff/evidence/2026-06-28-superio-hm-dump-admin.md) and raw JSON captured from Administrator PowerShell on the S3-class machine | Independent local hardware evidence resolving the local HM byte-dump blocker: after slot selection, Nuvoton configuration-mode entry, LDN B selection, and `ioctl_find_bars`, bank select (`0x4E=0x04`) and all requested bank 4 reads (`0x90`-`0x95`, `0xB0`-`0xBB`, `0xC0`-`0xCF`) succeeded through normal HM ports `0x0295`/`0x0296`. Bytes `0xCC`-`0xCF` read `0xFF`, `0x1F`, `0x00`, `0x00`. |
| S8 | [`docs/development/sensor-handoff/evidence/2026-06-28-nuvoton-0xd802-source-hunt.md`](../../development/sensor-handoff/evidence/2026-06-28-nuvoton-0xd802-source-hunt.md) | Source-hunt evidence: public Nuvoton product-selection data and direct public URL probes did not expose an NCT6799D/NCT6799D-R datasheet or product page; third-party board photos and public user reports suggest NCT6799D-R, while the later independent AIDA64 dump supports `NCT6799D` for raw ID `0xD802`. |
| S9 | [`docs/development/sensor-handoff/evidence/2026-06-28-nuvoton-0xd802-aida64-dump.md`](../../development/sensor-handoff/evidence/2026-06-28-nuvoton-0xd802-aida64-dump.md), summarizing public AIDA64 text dumps attached to LibreHardwareMonitor issue #1720 | Independent dump evidence from an ASUS ROG Strix X670E-F Gaming WiFi board. The dump reports raw Super I/O device ID `D802h` and labels it `NCT6799D`; it records normal HM base `0x0290`, read-only HM base `0x0A00` on that board, bank 4 temperature bytes, and bank 4 direct RPM-style bytes. The public bank 4 dump row at `0xC0` also contains `0xCC`-`0xCF` = `0x07`, `0x17`, `0x15`, `0x59` (re-read from the public attachment on 2026-10-25). This is dump output only, not implementation source code. |
| S10 | [ADR 0011](../../adr/0011-experimental-sensor-enablement.md) | Accepted project policy source for Experimental enablement of recognized hardware when a plausible read-only path is fully defined but lacks hardware validation; it supplies no hardware-register facts. |
| S11 | Linux kernel `drivers/hwmon/nct6775-platform.c` and `drivers/hwmon/nct6775.h` (torvalds/linux `master`, retrieved 2026-10-25), GPL-2.0 | **lead-only (copyleft)**. Super I/O detection: the raw 16-bit chip ID is matched after masking its low three bits (`0xFFF8`) against the family values `0xC560`, `0xC800`, `0xC910`, `0xD120`, `0xD350`, `0xD420`, `0xD450`, `0xD428`, `0xD800`; the hardware monitor is logical device `0x0B`, its enable bit is `CR30` bit 0, and its base is `CR60/61`; the driver claims a two-port I/O region at base+5 (index) and base+6 (data) and selects a bank by writing index `0x4E` and then the bank number before each banked access; the driver's chip-name table covers both NCT6798D and NCT5585D under the kind matched by `0xD428`, and both NCT6796D-S and NCT6799D-R under the kind matched by `0xD800`; for the NCT6791D-and-later kinds the driver reads global `CR28` and, if bit 4 is set, clears it (a write) and logs that it enabled the hardware-monitor logical-device mapping; on listed ASUS boards the driver reaches the Super I/O and HM registers through an ACPI WMI method instead of port I/O. Restated in this document's own words; no code, structure, or identifier names were copied. |
| S12 | Linux kernel `drivers/hwmon/nct6775-core.c` (torvalds/linux `master`, retrieved 2026-10-25), GPL-2.0 | **lead-only (copyleft)**. Chip table comment: fan input counts NCT6779D 5; NCT6791D, NCT6792D, NCT6793D, NCT6795D 6; NCT6796D, NCT6797D, NCT6798D, NCT5585D, NCT6796D-S, NCT6799D-R 7; alternate IDs `0xD451` (NCT6797D) and `0xD429` (NCT6798D); NCT5585D under the `0xD428` kind; NCT6796D-S `0xD801`; NCT6799D-R `0xD802`. Register tables: for every kind from NCT6779D onward the 16-bit RPM registers are bank 4 `0xC0`, `0xC2`, `0xC4`, `0xC6`, `0xC8`, `0xCA`, `0xCE` and the byte temperature registers are bank 4 `0x90`-`0x95` (the `0xD428`/`0xD800` kinds add `0x96`); register addresses are 16-bit values whose high byte is the bank and whose low byte is the index. S12 does not itself describe Super I/O detection or port offsets. Restated; nothing copied. |
| S13 | Linux kernel `Documentation/hwmon/nct6775.rst` (torvalds/linux `master`, retrieved 2026-10-25), GPL-2.0 | **lead-only (copyleft)**. Documentation text: datasheets for NCT6779D-NCT6799D are "available from Nuvoton upon request", the NCT5585D datasheet is "not available" and the chip is "compatible to NCT6798D"; NCT6796D-S and NCT6799D-R "are very similar", their chip IDs indicate different versions, and the driver treats them the same way. |
| S14 | LibreHardwareMonitor `LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/` chip-ID enumeration (`Chip.cs`), Super I/O detection (`LpcIO.cs`), configuration-port helper (`LpcPort.cs`), and Nuvoton hardware-monitor driver (`Nct677X.cs`) (`master`, retrieved 2026-10-25), MPL-2.0 | **lead-only (copyleft)**. Exact raw IDs: `0xC5` with CR21 high nibble `6` = NCT6779D; `0xC803` NCT6791D; `0xC911` NCT6792D; `0xC913` NCT6792D-A; `0xD121` NCT6793D; `0xD352` NCT6795D; `0xD423` NCT6796D; `0xD42A` = NCT5585D on ASRock X870E Nova WiFi, otherwise NCT6796D-R; `0xD451` NCT6797D; `0xD42B` NCT6798D; `0xD802` = NCT6796D-S on ASRock X870E Nova WiFi and B650M HDV M.2, otherwise NCT6799D; `0xD806` NCT6701D. Fan input counts: NCT6779D 5; NCT6796D-R, NCT6796D-S, NCT6797D, NCT6798D, NCT6799D, NCT6701D, NCT5585D 7; NCT6791D, NCT6792D, NCT6792D-A, NCT6793D, NCT6795D, NCT6796D 6. Detection selects logical device `0x0B`, reads the base from `CR60/61` (twice, requiring equal values), and reads `CR30` only for NCT6701D; the hardware monitor is accessed through index port base+5 and data port base+6, selecting the bank by writing index `0x4E` and then the high byte of the 16-bit register address before each read. Bank 4 byte temperatures `0x90`-`0x95` for every listed chip except NCT5585D, whose table exposes only `0x93` and `0x95` of them. Fan speed is read from the 13-bit count registers (`0xB0`, `0xB2`, `0xB4`, `0xB6`, `0xB8`, `0xBA`, `0xCC`), not from the direct 16-bit RPM pairs. Global `CR28` bit 4 is the HM I/O-space lock for NCT6791D and later including NCT6701D and NCT5585D; the lead clears it (a write) and notes that the lock is already disabled when the Nuvoton vendor ID can be read. NCT6701D quirks: raw temperature bytes `0x00`, `0xA0`, `0x7E`-`0x80` are invalid; some firmware leaves LDN B inactive; a `CR60/61` value below `0x100` or with any bit of `0xF007` set is an invalid base, in which case the lead uses `CR64/65`; the vendor-ID check is skipped. Restated; nothing copied. |
| S15 | lm-sensors `prog/detect/sensors-detect` (`master`, retrieved 2026-10-25), GPL-2.0 | **lead-only (copyleft)**. Detection table: the same nine family values as S11 with mask `0xFFF8`, each with logical device `0x0B`; the `0xC560` entry covers both NCT5532D and NCT6779D. The generic Super I/O probe reads the logical-device base from `0x60/0x61`; it does not read `CR30` or any hardware-monitor register. |
| S16 | [ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md) | Accepted project policy source allowing lead-only facts to back Experimental scopes; it supplies no hardware-register facts. |

S11-S15 are copyleft monitoring implementations and documentation
consulted as leads under ADR 0023. Every fact taken from them is
restated in this document's own words and tables, carries its source ID,
and backs only scoped-enablement rows whose default enablement is
Experimental. No code excerpt, code structure, or implementation
identifier name was copied. The `0xD802` Verified row depends on none of
them; the `0xD421` Experimental row depends only on S1. S9 uses public
hardware dump text attached to an issue; no AIDA64 source, binary,
disassembly, or implementation structure was consulted.

## Validation outcome

Revision 7 passes the implementation-ready gate for the scoped
`0xD802` / `NCT6799D` Verified path, the exact `0xD421` / `NCT6796D` /
`NCT6796D-E` Experimental path, and the lead-only Experimental rows:

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
  at `0490`-`0495`. (S7, S9)
- Fan/RPM bytes: S7 captures local direct RPM register bytes at
  `0xC0`-`0xCB`; S9 captures non-zero direct high/low values at the same
  six register pairs. Zero direct RPM is treated as stopped-or-unconnected,
  not as a read failure. (S7, S9)
- Experimental D421 path: S1 pins the exact chip ID and the normal-HM,
  bank 4 temperature, and six direct RPM-pair layouts used by this
  revision. No D421 hardware dump is available, so this path remains
  Experimental under ADR 0011 rather than Verified. (S1, S10)
- Lead-only Experimental rows: for every chip in the new rows, S11, S14,
  and S15 describe the same LDN B / `CR60-61` discovery (S11 also the
  `CR30` enable bit), S12 and S14 describe the same bank 4 byte
  temperatures `0x90`-`0x95`, and S12 describes the same direct RPM pairs
  that S1 defines for NCT6796D. The chip IDs (S11, S12, S14, S15),
  per-chip fan counts (S12, S14), lock applicability (S11, S14), NCT6701D
  quirks (S14), and the NCT6796D-S similarity statement (S13) are
  lead-only facts and therefore cannot be Verified; the rows are
  Experimental under ADR 0011 and ADR 0023. (S1, S10, S16)
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

### Lead-only Experimental chip-id values

These raw IDs are enabled only as Experimental rows. "Range" means the
leads classify every value in the range as that chip; "exact" means a
lead names that specific value. Chip labels are surfaced without package
suffixes except where the only attested identity is a suffixed part.

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

ID-matching policy for this revision: match exact raw IDs, as the
existing `0xD802` and `0xD421` rows do. The single exception is NCT6779D,
for which no lead attests an exact value and all three leads state the
range semantics; it is matched as the range `0xC560`-`0xC567` that every
lead accepts. Any other raw ID, including other members of the lead
ranges above, remains Unsupported (see Open questions).

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| Nuvoton responder detection using Phase 2 raw chip-id bytes | Ready through `superio-access.md` rev 3; this document references the diagnostic result but does not change the Phase 2 scope. | Existing diagnostic may remain enabled. |
| NCT6796D/NCT6796D-E chip-id mapping (`0xD421`) | **Experimental**: exact identity and the compatible read-only profile are pinned by S1, but no matching hardware dump is available. No behavior is inferred from the `0xD802` identity. | Enabled as Experimental for the scoped normal-HM reads. |
| `0xD802` / `NCT6799D` chip-id mapping | **Verified** for the normal HM read scope. S7 proves the local raw ID; S9 independently maps `D802h` to `NCT6799D`. | Enabled as Verified for the scoped Phase 3 normal-HM reads. |
| Normal HM logical-device selection and base discovery | Ready for both S1-backed IDs where S1 defines the NCT6796D register semantics: select LDN B, read `CR30`, read `CR60/61`, validate the discovered normal HM base, then use base+`0x05`/base+`0x06`. S7/S9 additionally validate the sequence on `0xD802`. For the lead-only rows the same sequence is lead-only: LDN B and `CR60/61` (S11, S14, S15), `CR30` bit 0 (S11; S14 for NCT6701D), index/data offsets and bank select (S11, S14). | Enabled for `0xD421` as Experimental, `0xD802` as Verified, and the lead-only rows as Experimental. |
| Read-only HM base (`CR64/65`) | Board-variable in evidence: local S7 is `0x0000`, S9 is `0x0A00`. The ready scope does not need it. | Disabled. |
| Bank 4 byte temperature reads `0x90`-`0x95` | S1 directly defines this NCT6796D bank/register layout; S7 and S9 independently validate the same bank/register set on `0xD802` / NCT6799D. For the lead-only rows the layout is lead-only (S12, S14). | Enabled with generic source labels for `0xD421` as Experimental, `0xD802` as Verified, and the lead-only rows as Experimental. |
| Direct 16-bit fan RPM reads `0xC0`-`0xCB` | S1 directly defines these six NCT6796D direct high/low pairs; S7 and S9 independently validate the same six pairs on `0xD802` / NCT6799D. For the lead-only rows the register positions are lead-only (S12) and the number of populated pairs is lead-only (S12, S14). | Enabled with generic fan labels for `0xD421` as Experimental, `0xD802` as Verified, and the lead-only rows as Experimental for the populated pairs listed below. |
| Seventh direct RPM pair `0xCE`/`0xCF` (AUXFANIN4) | S1 p. 66 lists this pair as the AUXFANIN4 16-bit RPM reading for NCT6796D; S12 lists the same register as the seventh RPM register for every NCT6779D-and-later kind; S9 captured `0x15`/`0x59` (5465 RPM) and S7 captured `0x00`/`0x00` there on `0xD802`. Which chips populate the input is lead-only (S12, S14). Not adopted for `0xD802` or `0xD421` in this revision (see Open questions). | Enabled as Experimental only for the seven-input lead-only rows below. Disabled for `0xD802` and `0xD421`. |
| Count-based fan reads `0xB0`-`0xBB`, `0xCC`/`0xCD` | Captured by S7/S9 but unnecessary while direct RPM registers are available. | Disabled. |
| Bank 4 byte temperature `0x96` | Lead-only (S12, S14) for the `0xD428`/`0xD800` kinds only; not defined by S1 for NCT6796D. | Disabled. |
| HM I/O-space lock detection (global `CR28` bit 4) | S1 p. 376 defines the bit for NCT6796D. Applicability to NCT6791D and later and the read-only consequence are lead-only (S11, S14). Detection is a read; clearing the lock is a write and is out of scope. | Enabled as a read-only availability gate for the NCT6791D-and-later lead-only rows. Not applied to `0xD802`, `0xD421`, or NCT6779D. |

#### Lead-only Experimental chip rows

Each row is **Experimental** under ADR 0011 and ADR 0023. The Status
column names the lead-only facts and source IDs the row depends on. "Fan
inputs" is the number of populated direct RPM pairs starting at
`0xC0`/`0xC1`; pairs beyond that count are not read.

| Raw ID match | Chip label | Fan inputs (direct pairs read) | Seventh pair `0xCE`/`0xCF` | Lock gate | Status (lead-only facts and sources) | Default enablement |
| --- | --- | --- | --- | --- | --- | --- |
| `0xC560`-`0xC567` (range) | NCT6779D | 5 (`0xC0`-`0xC9`; do not read `0xCA`/`0xCB`) | Not populated | No | Lead-only: range identity (S11, S14, S15); five fan inputs (S12, S14); bank 4 `0x90`-`0x95` temperatures (S12, S14); `0xC0`-`0xC9` direct RPM pairs (S12). No exact ID or hardware dump exists. | Experimental |
| `0xC803` | NCT6791D | 6 (`0xC0`-`0xCB`) | Not populated | Yes | Lead-only: exact ID (S14) inside range (S11, S15); six fan inputs (S12, S14); bank 4 temperatures (S12, S14); direct RPM pairs (S12); lock applicability (S11, S14). | Experimental |
| `0xC911` | NCT6792D | 6 | Not populated | Yes | Lead-only: exact ID (S14) inside range (S11, S15); six fan inputs (S12, S14); bank 4 temperatures (S12, S14); direct RPM pairs (S12); lock applicability (S11, S14). | Experimental |
| `0xC913` | NCT6792D | 6 | Not populated | Yes | Lead-only: exact ID (S14, labelled NCT6792D-A there) inside range (S11, S15); six fan inputs (S12, S14); bank 4 temperatures (S12, S14); direct RPM pairs (S12); lock applicability (S11, S14). | Experimental |
| `0xD121` | NCT6793D | 6 | Not populated | Yes | Lead-only: exact ID (S14) inside range (S11, S15); six fan inputs (S12, S14); bank 4 temperatures (S12, S14); direct RPM pairs (S12); lock applicability (S11, S14). | Experimental |
| `0xD352` | NCT6795D | 6 | Not populated | Yes | Lead-only: exact ID (S14) inside range (S11, S15); six fan inputs (S12, S14); bank 4 temperatures (S12, S14); direct RPM pairs (S12); lock applicability (S11, S14). | Experimental |
| `0xD423` | NCT6796D | 6 (mirrors the `0xD421` row) | Disabled (S1 and S12 state seven inputs, S14 states six; see Open questions) | Yes | Lead-only: exact ID (S14) inside range (S11, S15). Layout is S1-backed for NCT6796D; lock register is S1 p. 376, applicability (S11, S14). | Experimental |
| `0xD429` | NCT6798D | 7 (`0xC0`-`0xCB`, `0xCE`/`0xCF`) | Enabled (Experimental) | Yes | Lead-only: exact ID (S12 only) inside range (S11, S15); seven fan inputs (S12); layout including the seventh pair (S12; S1 p. 66 for the register position); lock applicability (S11, S14). | Experimental |
| `0xD42A` | NCT6796D-R / NCT5585D | 7 | Enabled (Experimental) | Yes | Lead-only: exact ID (S14) inside range (S11, S15); label is board-dependent in S14 and S11 attributes the range to both NCT6798D and NCT5585D (see Open questions); seven fan inputs for both candidate labels (S12, S14); bank 4 temperatures `0x90`-`0x95` (S12 for the `0xD428` kind; S14 under the NCT6796D-R label, while S14's NCT5585D table exposes only `0x93` and `0x95`); direct RPM pairs including the seventh (S12; S1 p. 66 for the register position); lock applicability (S11, S14). | Experimental |
| `0xD42B` | NCT6798D | 7 | Enabled (Experimental) | Yes | Lead-only: exact ID (S14) inside range (S11, S15); seven fan inputs (S12, S14); bank 4 temperatures (S12, S14); direct RPM pairs including the seventh (S12; S1 p. 66 for the register position); lock applicability (S11, S14). | Experimental |
| `0xD451` | NCT6797D | 7 | Enabled (Experimental) | Yes | Lead-only: exact ID (S12, S14) inside range (S11, S15); seven fan inputs (S12, S14); bank 4 temperatures (S12, S14); direct RPM pairs including the seventh (S12; S1 p. 66 for the register position); lock applicability (S11, S14). | Experimental |
| `0xD801` | NCT6796D-S | 7 | Enabled (Experimental) | Yes | Lead-only: exact ID (S12; kind name S11) inside range (S11, S15); S13 states NCT6796D-S and NCT6799D-R are very similar; seven fan inputs (S12); layout (S12; S1 p. 66); lock applicability (S11, S14). S14 has no `0xD801` entry (see Open questions). | Experimental |
| `0xD806` | NCT6701D | 7 | Enabled (Experimental) | Yes | Lead-only: exact ID (S14 only); seven fan inputs (S14; S12 for the `0xD800` kind); bank 4 `0x90`-`0x95` layout (S14) and RPM pairs (S12 for the `0xD800` kind; S1 p. 66); lock applicability (S14); invalid temperature bytes `0x00`, `0xA0`, `0x7E`-`0x80` (S14); inactive-LDN and invalid-base quirks handled as unavailable (S14). | Experimental |

## Register map facts

### Configuration-space fields needed before hardware-monitor access

These facts are implementation-ready for the scoped `0xD421` /
NCT6796D / NCT6796D-E Experimental path, the `0xD802` / NCT6799D Verified
path, and the lead-only Experimental rows unless the row explicitly says
otherwise. For `0xD421` the S1-backed facts are a profile definition, not
a hardware-validation claim; for the lead-only rows the S11/S14/S15 facts
are lead-only profile definitions.

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x20` | Chip ID high byte | `7:0` | `0xD8` for the `0xD802` / NCT6799D scope; `0xD4` for the exact `0xD421` / NCT6796D / NCT6796D-E Experimental scope; the high bytes of the lead-only values listed under [Lead-only Experimental chip-id values](#lead-only-experimental-chip-id-values). | Raw byte | S7, S9; S1 p. 369; S11, S12, S14, S15 (lead-only) |
| `0x21` | Chip ID low byte | `7:0` | `0x02` for the `0xD802` / NCT6799D scope; `0x21` for the exact `0xD421` / NCT6796D / NCT6796D-E Experimental scope; the low bytes of the lead-only values. S1 states the low digit encodes the IC version. | Raw byte | S7, S9; S1 pp. 362, 369; S11, S12, S14, S15 (lead-only) |
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
mechanism for the lead-only rows (lead-only).

| Relative offset | Register | Access | Meaning | Source |
| --- | --- | --- | --- | --- |
| `+0x05` | HM index port | Write | Selects a Hardware Monitor register index. With base `0x0290`, this is port `0x0295`. | S1 pp. 54-55; S7, S9; S11, S14 (lead-only) |
| `+0x06` | HM data port | Read/write for read plumbing | Reads the selected Hardware Monitor register; also receives the bank value after selecting the bank register. With base `0x0290`, this is port `0x0296`. | S1 pp. 54-55; S7, S9; S11, S14 (lead-only) |
| HM index `0x4E` | Bank select register | Write for read plumbing | Bits `2:0` select the bank used by subsequent banked HM register reads; write data value `0x04` to access bank 4. Bit 7 selects the high or low byte of the vendor-ID register and is left at its default by this spec. | S1 pp. 110, 176; S7, S9; S11, S14 (lead-only) |

Required writes in this table are read-transaction plumbing only. They
must not be generalized to fan-control, threshold, alarm, GPIO, lock, or
activation writes.

### Temperature registers

Bank 4 byte temperature registers are enabled for every enabled ID. S1
confirms this layout for the `0xD421` / NCT6796D / NCT6796D-E Experimental
profile; S7/S9 validate it for `0xD802` / NCT6799D; S12/S14 describe the
same six registers for every lead-only row (lead-only).

| Bank | Register | Generic source label | Decode | Source |
| --- | --- | --- | --- | --- |
| `0x04` | `0x90` | SYSTIN | signed 8-bit degrees C | S1 p. 176; S7, S9; S12, S14 (lead-only) |
| `0x04` | `0x91` | CPUTIN | signed 8-bit degrees C | S1 p. 176; S7, S9; S12, S14 (lead-only) |
| `0x04` | `0x92` | AUXTIN0 | signed 8-bit degrees C | S1 p. 176; S7, S9; S12, S14 (lead-only) |
| `0x04` | `0x93` | AUXTIN1 | signed 8-bit degrees C | S1 p. 176; S7, S9; S12, S14 (lead-only) |
| `0x04` | `0x94` | AUXTIN2 | signed 8-bit degrees C | S1 p. 176; S7, S9; S12, S14 (lead-only) |
| `0x04` | `0x95` | AUXTIN3 | signed 8-bit degrees C | S1 p. 176; S7, S9; S12, S14 (lead-only) |

Lead-only decode exception for the `0xD806` / NCT6701D row: raw bytes
`0x00`, `0xA0`, and `0x7E`-`0x80` are not temperatures and must be
reported as unavailable for that input rather than decoded. (S14)

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

### Fan tachometer / RPM registers

Every enabled path reads direct 16-bit RPM registers only. S1 confirms
the six direct pairs for the `0xD421` Experimental profile and lists the
seventh pair in its fan speed reading table; S7/S9 validate the same six
pairs for `0xD802`; S12 describes the same seven register positions for
every lead-only row. Count registers remain disabled because direct RPM
registers avoid count-to-RPM conversion ambiguity.

| Generic fan | Bank | Direct RPM high | Direct RPM low | Decode | Enabled for | Source |
| --- | --- | --- | --- | --- | --- | --- |
| Fan 1 / SYSFANIN candidate | `0x04` | `0xC0` | `0xC1` | high byte then low byte, unsigned RPM | all enabled IDs | S1 p. 66; S7, S9; S12 (lead-only) |
| Fan 2 / CPUFANIN candidate | `0x04` | `0xC2` | `0xC3` | high byte then low byte, unsigned RPM | all enabled IDs | S1 p. 66; S7, S9; S12 (lead-only) |
| Fan 3 / AUXFANIN0 candidate | `0x04` | `0xC4` | `0xC5` | high byte then low byte, unsigned RPM | all enabled IDs | S1 p. 66; S7, S9; S12 (lead-only) |
| Fan 4 / AUXFANIN1 candidate | `0x04` | `0xC6` | `0xC7` | high byte then low byte, unsigned RPM | all enabled IDs | S1 p. 66; S7, S9; S12 (lead-only) |
| Fan 5 / AUXFANIN2 candidate | `0x04` | `0xC8` | `0xC9` | high byte then low byte, unsigned RPM | all enabled IDs | S1 p. 66; S7, S9; S12 (lead-only) |
| Fan 6 / AUXFANIN3 candidate | `0x04` | `0xCA` | `0xCB` | high byte then low byte, unsigned RPM | all enabled IDs except the NCT6779D range row | S1 p. 66; S7, S9; S12 (lead-only) |
| Fan 7 / AUXFANIN4 candidate | `0x04` | `0xCE` | `0xCF` | high byte then low byte, unsigned RPM | seven-input lead-only rows only (`0xD429`, `0xD42A`, `0xD42B`, `0xD451`, `0xD801`, `0xD806`); disabled for `0xD802`, `0xD421`, `0xD423` | S1 p. 66; S12 (lead-only); S7, S9 observations on `0xD802` |

S7 local direct RPM bytes:

| Registers | Raw high/low | Direct RPM |
| --- | --- | --- |
| `0xC0`/`0xC1` | `0x00`/`0x00` | `0` |
| `0xC2`/`0xC3` | `0x0B`/`0x08` | `2824` |
| `0xC4`/`0xC5` | `0x00`/`0x00` | `0` |
| `0xC6`/`0xC7` | `0x02`/`0xE2` | `738` |
| `0xC8`/`0xC9` | `0x03`/`0x2C` | `812` |
| `0xCA`/`0xCB` | `0x00`/`0x00` | `0` |
| `0xCE`/`0xCF` | `0x00`/`0x00` | `0` (not enabled for `0xD802`; recorded as evidence) |

S9 independent direct RPM bytes:

| Registers | Raw high/low | Direct RPM |
| --- | --- | --- |
| `0xC0`/`0xC1` | `0x03`/`0x59` | `857` |
| `0xC2`/`0xC3` | `0x03`/`0x2D` | `813` |
| `0xC4`/`0xC5` | `0x03`/`0x40` | `832` |
| `0xC6`/`0xC7` | `0x03`/`0x55` | `853` |
| `0xC8`/`0xC9` | `0x03`/`0x42` | `834` |
| `0xCA`/`0xCB` | `0x03`/`0x8A` | `906` |
| `0xCE`/`0xCF` | `0x15`/`0x59` | `5465` (not enabled for `0xD802`; recorded as evidence; the adjacent count bytes `0xCC`/`0xCD` = `0x07`/`0x17` are not saturated, consistent with a connected seventh input) |

Captured but disabled fan-related registers:

| Bank | Registers | Reason disabled |
| --- | --- | --- |
| `0x04` | `0xB0`-`0xBB`, `0xCC`/`0xCD` | Count-path bytes are not needed while direct RPM registers are available. |
| `0x04` | `0xCE`/`0xCF` for `0xD802`, `0xD421`, `0xD423` | The Verified `0xD802` row and the S1-backed `0xD421` row are unchanged in substance in rev 7; `0xD423` mirrors `0xD421`. See Open questions. |

### D421 layout boundary

The `0xD421` profile reuses only the layout facts that S1 explicitly
defines for NCT6796D: LDN B normal-HM base discovery, normal-HM index/data
offsets, bank 4 selection, temperature registers `0x90`-`0x95`, and the
six direct RPM pairs `0xC0`-`0xCB`. S1 does not identify `0xD802`, so the
`0xD421` profile does not assume that every register observed on a
`0xD802` board has the same meaning.

In particular, S7/S9 expose `0xCC`-`0xCF` as an AUXFANIN4/seventh-fan
candidate on `0xD802`; that extra layout is not adopted for `0xD421`.
Read-only HM, count-based RPM, and any other D802-only or board-specific
extension remain disabled. No confirmed difference exists in the three
S1-backed layouts used by the Experimental profile; the difference
recorded here is the deliberate boundary against extending the D802
observation beyond those facts. (S1, S7, S9)

### Lead-only row layout boundary

The lead-only rows reuse the S1 NCT6796D layout only where the leads
describe the same register positions for the matched chip: LDN B
discovery (S11, S14, S15), normal-HM index/data offsets and bank 4
selection (S11, S14), byte temperatures `0x90`-`0x95` (S12, S14), and the
populated direct RPM pairs (S12; S14 reads count registers instead).
They do not adopt bank 4 `0x96`, any half-degree word register,
voltages, count registers, or the read-only HM base. The `0xD806` row
additionally applies the lead-only invalid-byte filter (S14).

## Procedures

### Discovery procedure

1. Acquire `Global\Access_ISABUS.HTP.Method` with a bounded timeout
   before any Super I/O index/data or hardware-monitor I/O transaction.
   If the mutex cannot be acquired, abort the transaction rather than
   proceeding unlocked. (S2)
2. Use the Phase 2 Nuvoton configuration-mode procedure to read raw
   chip-id bytes from `0x20` / `0x21`. If the responder is absent under
   the Phase 2 absent-id rules, stop. (S2)
3. Enable the scoped normal-HM path only when the raw chip ID is
   `0xD802` (Verified, label `NCT6799D`), `0xD421` (Experimental, label
   `NCT6796D` / `NCT6796D-E`), or one of the lead-only Experimental rows
   (exact values, or the NCT6779D range `0xC560`-`0xC567`) with the chip
   label from that table. Do not add unproven package suffixes. Any
   other chip ID is outside this document's enabled runtime scope. Every
   Experimental attempt is limited to the layout boundary stated for its
   row, and its failures are reported as experimental. (S1, S7, S9; S11,
   S12, S14, S15 lead-only)
4. For lead-only rows whose Lock gate is "Yes", read global `CR28` while
   still in configuration mode. If bit 4 is set, stop and report the
   chip as unavailable with an experimental diagnostic; do not write
   `CR28`. (S1 p. 376; S11, S14 lead-only)
5. Select LDN `0x0B`, read `CR30`, and require bit 0 to be set. Do not
   write `CR30`. For `0xD806`, an inactive LDN B is a known firmware
   state and is reported as unavailable, not worked around. (S1, S7, S9;
   S14 lead-only)
6. Read `CR60/61` as the normal HM base and require it to be a valid I/O
   base in the Phase 2 / PawnIO-safe range. Record `CR64/65` if useful
   for diagnostics, but do not use it as an access base in rev 7, not
   even for `0xD806` (see Open questions). (S7, S9; S14 lead-only)
7. Call PawnIO `ioctl_find_bars` after slot selection and LDN B/base
   discovery, before normal HM index/data I/O. On the local board this
   order changed `ioctl_find_bars` from `0x80070490` to success and
   authorized normal HM port I/O. (S7)
8. Use normal HM index port `base + 0x05` and data port `base + 0x06`.
   Select bank 4 by writing index `0x4E`, then data `0x04`. For the
   lead-only rows the same offsets and bank mechanism are lead-only.
   (S1, S7, S9; S11, S14 lead-only)
9. Read the enabled register set: temperatures `0x90`-`0x95` and the
   direct RPM pairs populated for the matched row (`0xC0`-`0xC9` for the
   NCT6779D range; `0xC0`-`0xCB` for six-input rows, `0xD802`, `0xD421`,
   and `0xD423`; `0xC0`-`0xCB` plus `0xCE`/`0xCF` for seven-input
   lead-only rows). For `0xD421`, these addresses are used only because
   S1 defines them for NCT6796D; for lead-only rows, only because S12 and
   S14 describe the temperature bytes and S12 describes the direct RPM
   pairs for the matched chip. Abort the current sample on any failed
   index write or data read; do not synthesize values from partial reads.
   (S1, S7, S9; S12, S14 lead-only for temperatures; S12 lead-only for
   RPM pairs)
10. Exit Nuvoton configuration mode and release the ISA mutex in all
    paths. (S2)

### Temperature decode

- Decode each enabled bank 4 temperature byte as signed 8-bit degrees C.
  Preserve the raw byte alongside the decoded value in diagnostics.
- For the `0xD806` / NCT6701D row, report raw bytes `0x00`, `0xA0`, and
  `0x7E`-`0x80` as unavailable instead of decoding them. (S14 lead-only)
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
- Read only the pairs populated for the matched row; an unpopulated pair
  (for example `0xCA`/`0xCB` on the NCT6779D range row) is not read and
  not reported. (S12, S14 lead-only)
- Do not use count-path bytes at `0xB0`-`0xBB` or `0xCC`/`0xCD`, and do
  not use `0xCE`/`0xCF` for `0xD802`, `0xD421`, or `0xD423`.

## Quirks

- `0xD802` is implementation-ready only as `NCT6799D` without a package
  suffix claim. Public board-review and user-report leads make
  `NCT6799D-R` plausible, but rev 7 does not prove or expose `-R`. (S8,
  S9)
- `0xD421` is implementation-ready as an Experimental
  `NCT6796D` / `NCT6796D-E` profile only for the S1-backed normal-HM,
  bank 4 temperature, and six direct RPM-pair reads. It has no matching
  hardware dump and must not be treated as Verified. (S1, S10)
- For the observed local board, `ioctl_find_bars` must run after Nuvoton
  configuration/base discovery has made the HM base visible to PawnIO
  `LpcIO`; running it before config discovery returned `0x80070490` and
  left normal HM ports unauthorized. (S7)
- Read-only HM base availability is board-variable in current evidence:
  S7 observed `0x0000`, S9 observed `0x0A00`. Rev 7 does not use the
  read-only HM path. (S7, S9)
- HM I/O-space lock (lead-only applicability): global `CR28` bit 4 is
  defined by S1 for NCT6796D as "HM IO space lock enable". The leads
  apply it to NCT6791D and every later chip in the lead-only rows and
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
  chip. (S14 lead-only)
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
  datasheet and official chip-id table remain unavailable; rev 7 is
  scoped to independently verified raw chip ID `0xD802` / `NCT6799D` and
  does not claim package suffix, OEM variant, or package revision.
- Non-blocking for Phase 3: Board-specific physical header labels remain
  unresolved; rev 7 exposes generic fan labels and preserves zero RPM as
  stopped-or-unconnected without classifying physical connection state.
- Non-blocking for Phase 3: no matching hardware dump was captured for
  `0xD421` in this validation set. The exact S1-backed read profile is
  therefore Experimental and can graduate to Verified only after a
  maintainer-accepted dump confirms the chip ID, discovered normal-HM
  base, bank 4 reads, and direct RPM pairs.
- Non-blocking for Phase 3: Read-only HM access remains disabled for
  every ID; the enabled scopes use only normal HM bank 4 temperatures
  `0x90`-`0x95` and the direct RPM pairs listed per row.
- Non-blocking for Phase 3: the seventh direct RPM pair `0xCE`/`0xCF` is
  listed by S1 p. 66 as AUXFANIN4 for NCT6796D and read live on a
  `0xD802` board by S9 (5465 RPM), yet rev 7 leaves it disabled for the
  Verified `0xD802` row and the S1-backed `0xD421`/`0xD423` rows because
  those rows are kept unchanged in substance; extending them is a
  maintainer decision for a later revision. The seventh pair is enabled
  only on the seven-input lead-only rows, where it is lead-only.
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
  place it in a range that S11 attributes to both NCT6798D and NCT5585D; S12 lists NCT5585D under
  the same kind; S13 calls NCT5585D compatible to NCT6798D. S12 and S14
  give both candidates seven fan inputs, S12 describes the same bank 4
  temperature bytes and direct RPM pairs for the whole `0xD428` kind,
  and S14 describes the full `0x90`-`0x95` set under the NCT6796D-R label
  but only `0x93` and `0x95` under its NCT5585D label, so rev 7 surfaces
  the generic label `NCT6796D-R / NCT5585D`; the register path read by
  this spec is the same for both candidates.
- Non-blocking for Phase 3: NCT6796D fan count for `0xD423`. S1 (seven
  inputs including AUXFANIN4) and S12 (7) disagree with S14 (6). Rev 7
  reads six pairs for `0xD423`, mirroring `0xD421`; a dump or a
  maintainer decision can extend both rows together.
- Non-blocking for Phase 3: range matching. S11 and S15 match the family
  after masking the low three ID bits, S14 matches exact values except
  for the NCT6779D high-nibble match, and S1 p. 362 states that the CR21
  low digit encodes the IC version. Rev 7 matches exact values plus the
  NCT6779D range `0xC560`-`0xC567` that all three leads accept. Other
  range members (for example `0xC568`-`0xC56F`, which only S14 accepts,
  or `0xC800`-`0xC807` other than `0xC803`) stay Unsupported until a lead
  names them or a dump attests them; the exact-ID policy avoids guessing
  the identity of unattested versions.
- Non-blocking for Phase 3: `0xD806` / NCT6701D alternate base. S14 uses
  `CR64/65` as the hardware-monitor base when `CR60/61` is not a usable
  base, which conflicts with S1's definition of `CR64/65` as the read-only
  HM base for NCT6796D. Rev 7 requires a valid `CR60/61` and otherwise
  reports the chip unavailable; the lead-only fallback is not adopted.
- Non-blocking for Phase 3: a vendor-ID sanity read (S1 pp. 110-111
  define the 16-bit Nuvoton vendor ID `0x5CA3` at bank 0 index `0x4F`,
  byte-selected by bank-select bit 7) could detect an engaged lock or a
  non-Nuvoton responder without writes other than bank selection, but
  S14 skips it for NCT6701D. It is not adopted in rev 7; the `CR28` read
  is the only lock gate.
- Non-blocking for Phase 3: bank 4 `0x96` is described by S12/S14 as an
  additional byte temperature on the `0xD428`/`0xD800` kinds and is not
  defined by S1 for NCT6796D; it stays disabled.
- Non-blocking for Phase 3: no hardware dump exists for any lead-only
  row. Each lead-only row graduates to Verified only when a primary
  source or a maintainer-accepted independent hardware dump confirms
  every fact it depends on, including its chip ID, `CR28` bit 4 state,
  LDN B base, bank 4 temperature bytes, and the populated direct RPM
  pairs.

## Implementation-ready transition checklist

Revision 7 satisfies the directory-level checklist in [`README.md`](README.md):

- no unresolved provenance marker remains in this document;
- every enabled D421 fact is pinned to S1/S2, and every enabled D802 fact
  is pinned to S1/S2 or independently verified by S7 and S9 hardware
  dumps;
- every lead-only fact is tagged (`lead-only (copyleft)` in the Sources
  table for S11-S15, source IDs on each fact row and quirk entry) and is
  depended on only by scoped-enablement rows whose default enablement is
  Experimental; the `0xD802` Verified row depends on no lead-only fact;
- the scoped-enablement tables distinguish the D421 Experimental path,
  the D802 Verified path, the lead-only Experimental rows with their fan
  counts and lock gate, and disabled or unsupported scopes;
- every remaining open question is explicitly annotated as non-blocking
  for Phase 3; and
- the revision, status, and revision history record the change.

## Provenance text for a future clean-room implementation PR

Implementation PRs should pin this ready revision and commit, for example:

```text
Implemented from docs/specs/sensors/superio-access.md revision 3 (commit a8c167b1), limited to Super I/O configuration access and chip-id diagnostic facts.
Implemented from docs/specs/sensors/superio-nuvoton-nct67xx.md revision 7 (commit <ready-commit>), limited to the `0xD802` / NCT6799D Verified, `0xD421` / NCT6796D / NCT6796D-E Experimental, and lead-only Experimental normal HM scopes marked enabled in that revision.
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
