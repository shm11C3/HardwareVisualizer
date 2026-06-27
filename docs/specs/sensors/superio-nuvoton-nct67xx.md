# Spec: Super I/O Nuvoton NCT67xx/NCT679x hardware-monitor reads

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Draft — not implementation-ready |
| Scope | Phase 3 Nuvoton-family register-map draft for motherboard temperature and fan RPM reads. Covers the intended clean-room facts that must be pinned before implementation: chip-id applicability, Hardware Monitor logical-device base discovery, banked hardware-monitor access, temperature registers, fan tachometer registers, and RPM decode. Excludes voltage decode, fan-control/PWM writes, threshold/limit writes, alarm clearing, UI integration, and any Rust implementation. |
| Issue phase | Phase 3 (#1635) — Nuvoton NCT67xx / NCT679x temperature and fan RPM specification |

This revision is intentionally a **Draft**. It records the required
shape of the Nuvoton-family spec and the facts already available from
the Phase 2 clean-room diagnostic, but it still contains
`TODO(provenance)` markers for the per-chip datasheet page/section
pinning and for real hardware register dumps. No implementation may use
this document as a ready clean-room input until those markers are
resolved and the status is flipped using the checklist in
[`README.md`](README.md).

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Nuvoton, *NCT6796D* datasheet, revision V0.6, Hardware Monitor / configuration-register sections | Primary candidate source for this document. TODO(provenance): pin exact section and page numbers for every NCT6796D-derived fact before flipping this document to implementation-ready. |
| S2 | [`superio-access.md`](superio-access.md) revision 3 | Primary clean-room source for the Phase 2 Nuvoton configuration-mode key sequence, global chip-id register reads (`0x20` / `0x21`), absent-id classification, standard Super I/O configuration port pairs, and ISA mutex policy. |
| S3 | HardwareVisualizer PR #1732 real-hardware diagnostic dump, captured 2026-06-27 on NZXT N7 B650E / AMD Ryzen 7 7800X3D / Windows 11 Pro | Independent hardware evidence for a reachable Nuvoton-class responder at slot 0 (`0x2E`/`0x2F`) with raw chip-id bytes `0xD8` / `0x02`; not sufficient by itself to classify the exact chip model or decode hardware-monitor registers. |
| S4 | HardwareVisualizer issue #1635 clean-room policy and [`README.md`](README.md) status-transition rules | Project policy source for keeping copyleft implementations non-normative and for leaving unresolved datasheet/dump gaps as Draft-only open questions. |

No LibreHardwareMonitor, OpenHardwareMonitor, Linux kernel, lm-sensors,
or decompiled monitoring-tool source is a normative source for this
revision. If such sources are consulted later as non-normative leads,
they must be added to this table as non-normative leads and no fact may
rest solely on them.

## Detection

### Applicability

| Fact | Source |
| --- | --- |
| This document applies only after a Nuvoton-compatible Super I/O responder is detected through the Phase 2 configuration-mode diagnostic path. The diagnostic reads global chip-id bytes from configuration registers `0x20` and `0x21` while holding `Global\Access_ISABUS.HTP.Method`. | S2 |
| A raw chip-id reading of `0x00`/`0x00` or `0xFF`/`0xFF` is treated as absent/no usable responder for the Phase 2 diagnostic. Mixed values must stay visible as raw bytes for triage. | S2 |
| The observed Nuvoton-class board in S3 responded on slot 0 (`0x2E` index / `0x2F` data) with `idHigh=0xD8`, `idLow=0x02`, combined `chipId=0xD802`. | S3 |
| This revision does not classify `0xD802` as a specific Nuvoton model. Model classification must wait for a vendor chip-id table, board documentation, or another independent source. | S1, S3 |

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| Nuvoton responder detection using Phase 2 raw chip-id bytes | Ready only through `superio-access.md` rev 3; this document references the diagnostic result but does not change the Phase 2 ready scope. | Existing diagnostic may remain enabled. |
| Chip-id `0xD802` model mapping | Draft. Observed in S3, but exact model is unresolved. TODO(provenance): pin a primary source that maps `0xD802` to a model/revision, or keep model-specific decode disabled. | Disabled for model-specific decode. |
| Hardware Monitor logical-device selection and base discovery | Draft. Intended mechanism recorded below, but page-level datasheet provenance and hardware dump confirmation are not yet pinned. | Disabled. |
| Temperature register reads | Draft. Register numbers and source-to-label mapping remain TODO(provenance). | Disabled. |
| Fan tachometer reads and RPM decode | Draft. Register numbers, counter width/order, stopped/disconnected values, and RPM formula remain TODO(provenance). | Disabled. |

## Register map (facts)

### Configuration-space fields needed before hardware-monitor access

These entries describe the intended facts that must be pinned from S1
before implementation. They are not implementation-ready while any
`TODO(provenance)` marker remains.

| Address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x20` | Chip ID high byte | `7:0` | High byte of the Super I/O chip-id value used by the Phase 2 diagnostic. | Raw byte | S2 |
| `0x21` | Chip ID low byte | `7:0` | Low byte of the Super I/O chip-id value used by the Phase 2 diagnostic. | Raw byte | S2 |
| `0x07` | Logical Device Number select | `7:0` | Selects the logical device whose device-scoped configuration registers are accessed. TODO(provenance): pin exact Nuvoton register name/page. | Raw logical-device number | S1 TODO(provenance) |
| `0x0B` | Hardware Monitor logical device number | `7:0` | Logical-device number expected to expose the Nuvoton hardware-monitor I/O base. TODO(provenance): pin exact Nuvoton device name/page. | Raw logical-device number | S1 TODO(provenance) |
| `0x60` | Hardware Monitor base-address high byte | `7:0` | High byte of the hardware-monitor I/O base address after selecting the Hardware Monitor logical device. TODO(provenance): pin exact register name/page. | Base address high byte | S1 TODO(provenance) |
| `0x61` | Hardware Monitor base-address low byte | `7:0` | Low byte of the hardware-monitor I/O base address after selecting the Hardware Monitor logical device. TODO(provenance): pin exact register name/page and alignment/disabled-value rules. | Base address low byte | S1 TODO(provenance) |

### Hardware-monitor I/O access fields

| Address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `base + 0x05` | Hardware Monitor address/index port | `7:0` | I/O port used to select a hardware-monitor register. TODO(provenance): pin exact Nuvoton naming/page. | Port offset from discovered base | S1 TODO(provenance) |
| `base + 0x06` | Hardware Monitor data port | `7:0` | I/O port used to read or write the selected hardware-monitor register. TODO(provenance): pin exact Nuvoton naming/page. | Port offset from discovered base | S1 TODO(provenance) |
| `0x4E` | Hardware Monitor bank-select register | `7:0` | Bank selector for the banked hardware-monitor register space. Writing this selector is required to read banked registers, but must be limited to the read transaction. TODO(provenance): pin bank bit layout and reset/side-effect notes. | Bank number / selector bits | S1 TODO(provenance) |

### Temperature sensor registers

Do not implement from this table yet. It is a placeholder for the
primary-source facts and hardware-dump validation still needed for
Phase 3.

| Address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| TODO(provenance) | TODO(provenance) | TODO(provenance) | Temperature input reading for one Nuvoton hardware-monitor source. Must distinguish board/socket/auxiliary sensors from CPU package temperature already covered by Phase 1 specs. | TODO(provenance): signedness, resolution, Celsius conversion, invalid values | S1 TODO(provenance), S3 TODO(register dump) |

### Fan tachometer registers

Do not implement from this table yet. It is a placeholder for the
primary-source facts and hardware-dump validation still needed for
Phase 3.

| Address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| TODO(provenance) | TODO(provenance) | TODO(provenance) | Fan tachometer count for one Nuvoton FANIN source. | TODO(provenance): counter width, byte order, divisor/prescale interaction, stopped/disconnected count values | S1 TODO(provenance), S3 TODO(register dump) |

## Read procedure and decode

### Discovery procedure (draft)

The discovery procedure below is not implementation-ready until every
S1 `TODO(provenance)` marker is pinned and a real hardware dump confirms
the decoded base on at least the target Nuvoton-class board.

1. Acquire `Global\Access_ISABUS.HTP.Method` with a bounded timeout
   before any Super I/O index/data or hardware-monitor I/O transaction.
   If the mutex cannot be acquired, abort the transaction rather than
   proceeding unlocked. (S2)
2. Use the Phase 2 Nuvoton configuration-mode procedure to read the raw
   chip-id bytes from `0x20` / `0x21`. If the responder is absent under
   the Phase 2 absent-id rules, stop. (S2)
3. Select the Hardware Monitor logical device by writing the logical
   device selector and the Hardware Monitor logical-device number.
   TODO(provenance): pin exact selector register, logical-device number,
   and any activation precondition from S1.
4. Read the hardware-monitor base-address high and low bytes and combine
   them into a base address.
   TODO(provenance): pin disabled-base values, required alignment, and
   validity rules from S1.
5. Exit configuration mode using the Phase 2 Nuvoton exit sequence.
   (S2)
6. Cache the detected slot, chip-id bytes, optional model mapping, and
   base address. Sampling paths must not re-enter configuration mode on
   every metrics tick.

### Hardware-monitor register read procedure (draft)

1. Acquire `Global\Access_ISABUS.HTP.Method` for the whole banked
   hardware-monitor transaction. (S2)
2. For each required register, write the required bank selector to the
   hardware-monitor bank-select register.
   TODO(provenance): pin the exact bank selector encoding and whether
   the bank register is itself banked.
3. Write the register address to `base + 0x05`, then read the selected
   byte from `base + 0x06`.
   TODO(provenance): pin address/data port offsets and any ordering or
   delay requirement from S1.
4. Release the mutex after all bytes for one coherent sample have been
   read.

### Temperature decode (draft)

Temperature decode is unresolved in this revision.

- TODO(provenance): pin each temperature register, source label,
  Celsius conversion, signedness, resolution, and invalid-value rules
  from S1.
- TODO(provenance): confirm at least one real hardware dump maps the
  observed board's meaningful motherboard/auxiliary temperatures to the
  selected source labels without relying on third-party monitoring-code
  heuristics.

### Fan RPM decode (draft)

Fan RPM decode is unresolved in this revision.

- TODO(provenance): pin fan tachometer register addresses, counter
  width, byte order, divisor/prescale handling, stopped/disconnected
  counter values, and the RPM conversion formula from S1.
- TODO(provenance): confirm with a real hardware dump that a changing
  fan speed changes the expected tachometer count and that a stopped or
  disconnected fan is filtered to "unavailable" rather than an
  implausible RPM.

## Quirks

- No model-specific quirks are implementation-ready in this revision.
- Chip-id `0xD802` is observed on one Nuvoton-class board (S3), but the
  exact model/revision is intentionally unresolved. Treat it as a
  draft-only applicability clue, not as a model-specific decode key.

## Safety notes

- This document is read-oriented. Required writes are limited to
  read-transaction plumbing: Nuvoton configuration-mode enter/exit
  keys documented by S2, logical-device selection required for base
  discovery once pinned from S1, and hardware-monitor bank selection
  required for banked register reads once pinned from S1.
- The implementation must never write fan-control registers, PWM duty
  registers, Smart Fan policy registers, sensor source-selection
  registers, threshold/limit registers, alarm-clear registers, GPIO
  registers, or any other register that can alter board behavior.
- All Super I/O and hardware-monitor index/data sequences must hold
  `Global\Access_ISABUS.HTP.Method` for the complete multi-I/O
  transaction. Interleaving another monitor's index write between this
  client's index and data operations can corrupt the read. (S2)
- Fan RPM and temperature values must be plausibility-filtered before
  surfacing them to the metrics stream. This revision does not define
  the exact ranges yet; implementers must wait for the readied revision.

## Open questions

- TODO(provenance): Pin a primary-source Nuvoton chip-id table for the
  NCT67xx/NCT679x family, including whether `0xD802` maps to one
  model, multiple revisions, or an OEM/board-specific variant.
- TODO(provenance): Pin the exact Hardware Monitor logical-device
  selector, logical-device number, activation requirements, base-address
  registers, disabled values, and base alignment rules.
- TODO(provenance): Pin the hardware-monitor address/data port offsets
  and bank-select register behavior, including whether bank writes have
  any side effects beyond selecting the bank.
- TODO(provenance): Pin the temperature register map and each source's
  unit conversion, signedness, resolution, invalid values, and source
  label.
- TODO(provenance): Pin the fan tachometer register map, byte order,
  counter width, divisor/prescale handling, stopped/disconnected
  handling, and RPM formula.
- TODO(provenance): Capture a Phase 3 hardware dump on the S3 board
  after adding a dump-only diagnostic in the same PR or a dedicated
  validation branch. The dump must include chip-id, decoded base, banked
  temperature bytes, banked fan tachometer bytes, and enough manual
  context to distinguish connected fans from empty headers.
- TODO(provenance): Decide whether model-specific support should default
  to disabled until both a vendor chip-id mapping and one board-level
  dump are available for the exact chip-id.

## Implementation-ready transition checklist

Before this document can become `Implementation-ready (rev N)`, the
Phase 3 spec-author PR must satisfy the directory-level checklist in
[`README.md`](README.md) and at minimum:

- remove every `TODO(provenance)` marker or move unresolved items to
  Open questions with the required non-blocking annotation,
- pin exact S1 section/page references for every register, conversion,
  and procedure fact,
- ensure no normative fact rests solely on a copyleft implementation,
- record the exact real-hardware dump(s) used for validation,
- set scoped enablement so unsupported or unverified chip IDs remain
  disabled by default, and
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
