# Spec: Super I/O Nuvoton NCT67xx/NCT679x hardware-monitor reads

| Field | Value |
| --- | --- |
| Revision | 3 |
| Status | Draft — not implementation-ready |
| Scope | Phase 3 Nuvoton-family register-map draft for motherboard temperature and fan RPM reads. Revision 3 pins NCT6796D-E/NCT6796D datasheet facts, records the mismatch between that datasheet and the observed `0xD802` hardware, records elevated `0xD802` configuration-space evidence, and keeps decode disabled until an exact-chip primary source plus hardware-monitor register bytes are available. Excludes voltage decode, fan-control/PWM writes, threshold/limit writes, alarm clearing, UI integration, and any Rust implementation. |
| Issue phase | Phase 3 (#1635) - Nuvoton NCT67xx / NCT679x temperature and fan RPM specification |

This revision is intentionally a **Draft**. It verifies the prior draft
against the official NCT6796D datasheet and finds that the datasheet's
configuration-space chip ID is `0xD421`, while the Phase 2 real board
dump observed `0xD802`. Therefore the NCT6796D facts below are useful
primary-source register-map evidence, but they do **not** prove that the
observed board can safely use this map. A later elevated local probe
confirmed that the `0xD802` responder exposes Logical Device B with
normal HM base `0x0290`, but it did not capture temperature or fan RPM
bytes. No implementation may use this document as a ready clean-room
input until the blocking items in
[Open questions](#open-questions) are resolved and the status is flipped
using the checklist in [`README.md`](README.md).

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Nuvoton, *NCT6796D LPC/eSPI SI/O Datasheet*, version 0.6, publication release date July 6, 2017; official PDF: `https://www.nuvoton.com/resource-files/NCT6796D_Datasheet_V0_6.pdf` | Primary source for NCT6796D register facts. Relevant pins: configuration protocol chapter 7, pp. 51-53; hardware-monitor LPC access chapter 8.3, pp. 54-55; temperature format chapter 8.6.3, pp. 60-62; fan speed reading chapter 8.8.1-8.8.3, pp. 66-67; fan count/RPM registers chapter 9.209-9.232, pp. 176-183; HM read-only register chapter 9.481, pp. 277-280; global chip ID CR20/CR21, p. 369; Logical Device B CR30/CR60-65, pp. 431-432. |
| S2 | [`superio-access.md`](superio-access.md) revision 3 | Primary clean-room source for the Phase 2 Nuvoton configuration-mode key sequence, global chip-id register reads (`0x20` / `0x21`), absent-id classification, standard Super I/O configuration port pairs, and ISA mutex policy. |
| S3 | HardwareVisualizer PR #1732 real-hardware diagnostic dump, captured 2026-06-27 on NZXT N7 B650E / AMD Ryzen 7 7800X3D / Windows 11 Pro | Independent hardware evidence for a reachable Nuvoton-class responder at slot 0 (`0x2E`/`0x2F`) with raw chip-id bytes `0xD8` / `0x02`; this is not the NCT6796D `0xD4` / `0x21` ID from S1. |
| S4 | HardwareVisualizer issue #1635 clean-room policy and [`README.md`](README.md) status-transition rules | Project policy source for keeping copyleft implementations non-normative and for leaving unresolved datasheet/dump gaps as Draft-only open questions. |
| S5 | Local standard-rights PawnIO probe on 2026-06-28 | Environment evidence only: `C:\Program Files\PawnIO` has `PawnIOLib.dll` and `LpcIO.bin`, PawnIO service is running, `pawnio_version` returned `0x00020000`, but `pawnio_open` returned `0x80070005` under medium integrity. No Phase 3 register dump was captured in this session. |
| S6 | Local elevated PawnIO `LpcIO` probe on 2026-06-28, run from Administrator PowerShell on the S3-class machine | Independent hardware evidence: `pawnio_open`, `pawnio_load`, and `Global\Access_ISABUS.HTP.Method` mutex acquisition succeeded; slot 0 repeated `chipId=0xD802`; LDN B read `CR30=0x09`, `CR60/61=0x02/0x90` (`HM base=0x0290`), and `CR64/65=0x00/0x00` (no valid read-only HM base). `ioctl_find_bars` returned `0x80070490`, and a normal HM index-port write to `0x0295` returned `0x80070005`, so no temperature or fan RPM bytes were captured. |

No LibreHardwareMonitor, OpenHardwareMonitor, Linux kernel, lm-sensors,
or decompiled monitoring-tool source is a normative source for this
revision. No fact below rests on those implementations.

## Validation outcome

Revision 3 does **not** pass the implementation-ready gate:

- The only official datasheet fetched and verified in this session is
  NCT6796D/NCT6796D-E. It identifies global CR20/CR21 as `0xD4`/`0x21`
  (`chipId=0xD421`), not the observed board's `0xD8`/`0x02`
  (`chipId=0xD802`). (S1, S3)
- The observed `0xD802` chip still needs an exact primary-source model
  mapping or an exact-chip datasheet. (S3)
- Elevated PawnIO access captured the observed chip's LDN B activation
  and normal HM base, but PawnIO `LpcIO` did not permit the normal HM
  index/data port transaction, so no temperature or fan RPM register
  bytes were captured. (S6)

## Detection

### Applicability

| Fact | Source |
| --- | --- |
| This document applies only after a Nuvoton-compatible Super I/O responder is detected through the Phase 2 configuration-mode diagnostic path. The diagnostic reads global chip-id bytes from configuration registers `0x20` and `0x21` while holding `Global\Access_ISABUS.HTP.Method`. | S2 |
| A raw chip-id reading of `0x00`/`0x00` or `0xFF`/`0xFF` is treated as absent/no usable responder for the Phase 2 diagnostic. Mixed values must stay visible as raw bytes for triage. | S2 |
| NCT6796D's global CR20 high byte is `0xD4` and CR21 low byte is `0x21`, so the configuration-space chip ID is `0xD421`. | S1 p. 369 |
| The observed Nuvoton-class board in S3 responded on slot 0 (`0x2E` index / `0x2F` data) with `idHigh=0xD8`, `idLow=0x02`, combined `chipId=0xD802`. | S3 |
| The observed `0xD802` responder is not proven to be NCT6796D/NCT6796D-E by S1 and must remain disabled for NCT6796D-specific decode. | S1, S3 |
| The elevated S6 probe on the observed `0xD802` board read LDN B `CR30=0x09`, normal HM base `0x0290`, and read-only HM base `0x0000`. The normal HM base is reachable in configuration space, but no sensor-byte read was completed through PawnIO. | S6 |

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| Nuvoton responder detection using Phase 2 raw chip-id bytes | Ready only through `superio-access.md` rev 3; this document references the diagnostic result but does not change the Phase 2 ready scope. | Existing diagnostic may remain enabled. |
| NCT6796D/NCT6796D-E chip-id mapping (`0xD421`) | Primary-source pinned from S1, but not hardware-validated on the S3 board. | Disabled until an NCT6796D/NCT6796D-E hardware dump is captured, or until scope is explicitly limited to datasheet-only support. |
| Observed chip-id `0xD802` | Draft. Observed in S3, but exact Nuvoton model/revision is unresolved. S1 disproves treating it as NCT6796D. | Disabled. |
| Hardware Monitor logical-device selection and base discovery | Draft. NCT6796D facts are pinned from S1; the observed `0xD802` board partially matches the LDN B/base shape with `CR30=0x09` and HM base `0x0290`, but the exact chip remains unresolved and the read-only HM base is `0x0000`. | Disabled. |
| Temperature register reads | Draft. NCT6796D register facts are pinned from S1, but S6 did not capture hardware-monitor temperature bytes for the observed board. | Disabled. |
| Fan tachometer / RPM reads | Draft. NCT6796D register facts are pinned from S1, but S6 did not capture hardware-monitor fan count/RPM bytes for the observed board. | Disabled. |

## Register map facts

### Configuration-space fields needed before hardware-monitor access

These fields are NCT6796D facts from S1. They are not implementation-ready
for the observed S3 hardware while the exact `0xD802` model is unresolved.

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x20` | Chip ID high byte | `7:0` | NCT6796D high byte `0xD4`. | Raw byte | S1 p. 369 |
| `0x21` | Chip ID low byte | `7:0` | NCT6796D low byte `0x21`. | Raw byte | S1 p. 369 |
| `0x07` | Logical Device Number select | `7:0` | Selects which logical device's registers are accessed at indexes `0x30` and above. | Raw logical-device number | S1 pp. 51-52 |
| `0x0B` | Logical Device B / Hardware Monitor logical device | `7:0` | Selects Logical Device B, whose CR30 enables Hardware Monitor & SB-TSI and whose CR60/61 and CR64/65 define HM base addresses. | Raw logical-device number | S1 pp. 431-432 |
| `0x30` after LDN `0x0B` | Hardware Monitor & SB-TSI activation | bit `0` | `0` inactive, `1` active. Discovery may read this bit. This spec does not permit writing it. | Boolean active bit | S1 p. 431 |
| `0x60` / `0x61` after LDN `0x0B` | HM base address | `15:0` | Selects the normal Hardware Monitor base address in `<0x100:0xFFE>` on a 2-byte boundary. | I/O base address | S1 p. 431 |
| `0x64` / `0x65` after LDN `0x0B` | Read-only HM base address | `15:0` | Selects the read-only Hardware Monitor base address in `<0x100:0xFFE>` on a 2-byte boundary. | I/O base address | S1 p. 431 |

### Hardware-monitor I/O access fields

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `base + 0x05` | Hardware Monitor address/index port | `7:0` | I/O port used to select a hardware-monitor internal register; standard index/data locations are usually `0x295`/`0x296`. | Port offset from discovered HM base | S1 pp. 54-55 |
| `base + 0x06` | Hardware Monitor data port | `7:0` | I/O port used to read or write the selected hardware-monitor internal register. | Port offset from discovered HM base | S1 pp. 54-55 |
| internal `0x4E` | Hardware Monitor bank-select register | `7:0` | Selects the bank for banked hardware-monitor internal registers. Bank writes are allowed only as read-transaction plumbing. | Bank selector | S1 p. 55 |

### Temperature sensor registers

NCT6796D exposes SYSTIN, CPUTIN, AUXTIN0, AUXTIN1, AUXTIN2, AUXTIN3,
and AUXTIN4 temperature values. The data format for those sensors is
9-bit two's-complement with 0.5 C resolution when using the high/low
temperature-source registers. The HM read-only table also exposes byte
temperature readings for all seven sources. (S1 pp. 60-62, 277-280)

| Access path | Address | Name | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| Read-only HM base | offset `0x10` | SYSTIN temperature reading | System temperature input. | Byte temperature reading; exact signedness/invalid handling still needs dump verification before enablement. | S1 pp. 277-278 |
| Read-only HM base | offset `0x11` | CPUTIN temperature reading | CPU socket/board temperature input. Distinct from Phase 1 CPU package temperature. | Byte temperature reading; exact signedness/invalid handling still needs dump verification before enablement. | S1 pp. 277-278 |
| Read-only HM base | offset `0x12` | AUXTIN0 temperature reading | Auxiliary temperature input 0. | Byte temperature reading; exact signedness/invalid handling still needs dump verification before enablement. | S1 pp. 277-278 |
| Read-only HM base | offset `0x13` | AUXTIN1 temperature reading | Auxiliary temperature input 1. | Byte temperature reading; exact signedness/invalid handling still needs dump verification before enablement. | S1 pp. 277-278 |
| Read-only HM base | offset `0x14` | AUXTIN2 temperature reading | Auxiliary temperature input 2. | Byte temperature reading; exact signedness/invalid handling still needs dump verification before enablement. | S1 pp. 277-278 |
| Read-only HM base | offset `0x15` | AUXTIN3 temperature reading | Auxiliary temperature input 3. | Byte temperature reading; exact signedness/invalid handling still needs dump verification before enablement. | S1 pp. 277-278 |
| Read-only HM base | offset `0x16` | AUXTIN4 temperature reading | Auxiliary temperature input 4. | Byte temperature reading; exact signedness/invalid handling still needs dump verification before enablement. | S1 pp. 277-278 |
| Banked HM registers | bank `0x04`, indexes `0x90`-`0x95` | SYSTIN through AUXTIN3 temperature readings | Alternate banked readings for SYSTIN, CPUTIN, and AUXTIN0-3. AUXTIN4 is not listed in this table. | Byte temperature reading; exact signedness/invalid handling still needs dump verification before enablement. | S1 p. 176 |

### Fan tachometer / RPM registers

NCT6796D provides 13-bit fan count readings and 16-bit direct fan RPM
readings. For 13-bit count reads, the datasheet requires reading the
high byte first, then the low byte. `RPM = 1.35e6 / count`. For 16-bit
RPM reads, the datasheet requires reading the high byte first, then the
low byte; the combined 16-bit value is the RPM value in decimal. (S1
p. 66)

| Fan input | 13-bit count registers | 16-bit RPM registers | Read order / conversion | Source |
| --- | --- | --- | --- | --- |
| SYSFANIN | bank `0x04`, high `0xB0`, low `0xB1` | bank `0x04`, high `0xC0`, low `0xC1` | Count high then low; RPM high then low. | S1 pp. 66, 176, 180 |
| CPUFANIN | bank `0x04`, high `0xB2`, low `0xB3` | bank `0x04`, high `0xC2`, low `0xC3` | Count high then low; RPM high then low. | S1 pp. 66, 177, 180-181 |
| AUXFANIN0 | bank `0x04`, high `0xB4`, low `0xB5` | bank `0x04`, high `0xC4`, low `0xC5` | Count high then low; RPM high then low. | S1 pp. 66, 177-178, 181 |
| AUXFANIN1 | bank `0x04`, high `0xB6`, low `0xB7` | bank `0x04`, high `0xC6`, low `0xC7` | Count high then low; RPM high then low. | S1 pp. 66, 178, 181-182 |
| AUXFANIN2 | bank `0x04`, high `0xB8`, low `0xB9` | bank `0x04`, high `0xC8`, low `0xC9` | Count high then low; RPM high then low. | S1 pp. 66, 178-179, 182 |
| AUXFANIN3 | bank `0x04`, high `0xBA`, low `0xBB` | bank `0x04`, high `0xCA`, low `0xCB` | Count high then low; RPM high then low. | S1 pp. 66, 179, 182-183 |
| AUXFANIN4 | bank `0x04`, high `0xCC`, low `0xCD` per summary table | bank `0x04`, high `0xCE`, low `0xCF` per summary table; read-only HM offsets `0x46`/`0x47` also list AUXFANIN4 RPM | Count high then low; RPM high then low. Detailed banked-register sections for `0xCC`-`0xCF` were not found in S1 text extraction, so enablement needs hardware dump confirmation. | S1 pp. 66, 279 |

The HM read-only register table provides direct count/RPM offsets for
the first six fan inputs and RPM offsets for AUXFANIN4:

| Read-only HM offset range | Meaning | Source |
| --- | --- | --- |
| `0x2E`-`0x39` | SYSFANIN, CPUFANIN, AUXFANIN0, AUXFANIN1, AUXFANIN2, AUXFANIN3 count high/low readings. | S1 p. 279 |
| `0x3A`-`0x47` | SYSFANIN, CPUFANIN, AUXFANIN0, AUXFANIN1, AUXFANIN2, AUXFANIN3, AUXFANIN4 RPM high/low readings. | S1 p. 279 |

## Read procedure and decode

### Discovery procedure (draft)

This procedure is not implementation-ready until the exact `0xD802`
chip is identified or matching hardware is validated for NCT6796D.

1. Acquire `Global\Access_ISABUS.HTP.Method` with a bounded timeout
   before any Super I/O index/data or hardware-monitor I/O transaction.
   If the mutex cannot be acquired, abort the transaction rather than
   proceeding unlocked. (S2)
2. Use the Phase 2 Nuvoton configuration-mode procedure to read the raw
   chip-id bytes from `0x20` / `0x21`. If the responder is absent under
   the Phase 2 absent-id rules, stop. (S2)
3. Treat `0xD421` as NCT6796D/NCT6796D-E candidate hardware. Treat
   `0xD802` as an unresolved Nuvoton-class responder and keep decode
   disabled. (S1, S3)
4. Select Logical Device B by writing `0x07` then `0x0B` in
   configuration mode. (S1)
5. Read CR30, CR60/61, and CR64/65. CR30 bit 0 indicates whether
   Hardware Monitor & SB-TSI is active; CR60/61 is the normal HM base;
   CR64/65 is the read-only HM base. (S1)
6. Exit configuration mode using the Phase 2 Nuvoton exit sequence.
   (S2)
7. Cache the detected slot, chip-id bytes, optional model mapping, and
   base addresses. Sampling paths must not re-enter configuration mode
   on every metrics tick.

### Hardware-monitor register read procedure (draft)

1. Acquire `Global\Access_ISABUS.HTP.Method` for the whole
   hardware-monitor transaction. (S2)
2. For banked reads, write internal register `0x4E` through the normal
   HM index/data ports to select the required bank. (S1)
3. Write the target register address to `base + 0x05`, then read the
   selected byte from `base + 0x06`. (S1)
4. For 13-bit fan count and 16-bit fan RPM reads, read the high byte
   first, then the low byte. (S1)
5. Release the mutex after all bytes for one coherent sample have been
   read.

### Temperature decode (draft)

- For high/low temperature-source registers, decode the 9-bit
  two's-complement value as 0.5 C units. (S1 p. 60)
- For HM read-only byte temperature offsets, do not enable decode until
  an exact hardware dump confirms the access path and plausible source
  labels for the board. (S1, S3, S6)
- Do not merge motherboard/board temperature inputs with Phase 1 CPU
  package temperature sources.

### Fan RPM decode (draft)

- For 13-bit count registers, combine the high and low fields into a
  13-bit count and calculate `RPM = 1.35e6 / count`. (S1 p. 66)
- For 16-bit RPM registers, combine high then low bytes and interpret
  the resulting 16-bit integer as RPM. (S1 p. 66)
- Do not surface zero, max-count, or otherwise implausible values until
  stopped/disconnected handling is verified by real hardware dump.

## Quirks

- No model-specific quirks are implementation-ready in this revision.
- Chip-id `0xD802` is observed on one Nuvoton-class board (S3), but S1
  identifies NCT6796D as `0xD421`. Treat `0xD802` as a draft-only
  applicability clue, not as a decode key.

## Safety notes

- This document is read-oriented. Required writes are limited to
  read-transaction plumbing: Nuvoton configuration-mode enter/exit
  keys documented by S2, logical-device selection required for base
  discovery, and hardware-monitor bank selection required for banked
  register reads.
- The implementation must never write fan-control registers, PWM duty
  registers, Smart Fan policy registers, sensor source-selection
  registers, threshold/limit registers, alarm-clear registers, GPIO
  registers, CR30 activation bits, or any other register that can alter
  board behavior.
- All Super I/O and hardware-monitor index/data sequences must hold
  `Global\Access_ISABUS.HTP.Method` for the complete multi-I/O
  transaction. Interleaving another monitor's index write between this
  client's index and data operations can corrupt the read. (S2)
- Fan RPM and temperature values must be plausibility-filtered before
  surfacing them to the metrics stream. This revision does not define
  the exact ranges yet; implementers must wait for the readied revision.

## Open questions

- Blocking for Phase 3 ready: Identify the observed `0xD802` responder
  with a primary Nuvoton source, board documentation, or another
  independent source. The NCT6796D datasheet fetched in this session
  maps NCT6796D to `0xD421`, so `0xD802` must not use the NCT6796D
  register map by assumption.
- Blocking for Phase 3 ready: Complete a Phase 3 hardware-monitor byte
  dump on the S3 board. S6 captured chip-id and LDN B `CR30/CR60-65`
  under elevation, but `ioctl_find_bars` returned `0x80070490` and the
  normal HM index-port write to `0x0295` returned `0x80070005`; the
  remaining dump must include normal HM banked temperature/RPM bytes
  and manual context for connected versus empty fan headers.
- Blocking for Phase 3 ready: Confirm whether the observed chip exposes
  the same normal HM behavior as S1 before enabling any register reads.
  S6 observed normal HM base `0x0290` but read-only HM base `0x0000` on
  the unresolved `0xD802` chip, so the S1 read-only HM path is not
  hardware-validated for this board.
- Blocking for Phase 3 ready: Define stopped, disconnected, max-count,
  zero, and implausible fan handling from primary-source text or
  hardware dump evidence.
- Blocking for Phase 3 ready: Confirm AUXFANIN4 count/RPM banked
  registers for the exact supported chip. S1's summary table lists
  Bank 4 `0xCC`-`0xCF`, but the extracted detailed register sections
  only reached AUXFANIN3 for the normal Bank 4 speed registers.
- Blocking for Phase 3 ready: Decide whether a future revision will
  support NCT6796D/NCT6796D-E only, the observed `0xD802` chip only, or
  a broader NCT67xx/NCT679x table with per-chip scoped enablement.

## Implementation-ready transition checklist

Before this document can become `Implementation-ready (rev N)`, the
Phase 3 spec-author PR must satisfy the directory-level checklist in
[`README.md`](README.md) and at minimum:

- resolve the exact chip-id/model mapping for every enabled scope,
- pin exact S1 or replacement primary-source section/page references
  for every register, conversion, and procedure fact,
- ensure no normative fact rests solely on a copyleft implementation,
- record the exact elevated real-hardware dump(s) used for validation,
- set scoped enablement so unsupported or unverified chip IDs remain
  disabled by default,
- resolve or explicitly annotate every open question according to the
  README status-transition rules, and
- bump the revision and status in the header and revision history.

## Provenance text for a future clean-room implementation PR

Do **not** use this text while the document is Draft. After a later
revision is flipped to implementation-ready, the implementation PR
should pin the ready revision and commit, for example:

```text
Implemented from docs/specs/sensors/superio-access.md revision 3 (commit a8c167b1), limited to Super I/O configuration access and chip-id diagnostic facts.
Implemented from docs/specs/sensors/superio-nuvoton-nct67xx.md revision <ready-revision> (commit <ready-commit>), limited to the Nuvoton scopes marked enabled in that revision.
No other external sensor documentation was used.
```

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-06-28 | Initial Phase 3 Nuvoton-family draft; records Phase 2 diagnostic linkage, expected hardware-monitor spec shape, safety constraints, and unresolved provenance/dump requirements. |
| 2 | 2026-06-28 | Fetched and verified the official NCT6796D V0.6 datasheet; pinned NCT6796D configuration, HM base, temperature, and fan RPM facts; recorded that S1 maps NCT6796D to `0xD421` while the observed S3 board is `0xD802`; recorded standard-rights PawnIO dump failure (`0x80070005`). Status remains Draft. |
| 3 | 2026-06-28 | Added elevated PawnIO `LpcIO` evidence for the observed `0xD802` board: LDN B active, normal HM base `0x0290`, read-only HM base `0x0000`, and blocked HM index/data access (`ioctl_find_bars=0x80070490`, `pio_outb(0x0295)=0x80070005`). Status remains Draft. |
