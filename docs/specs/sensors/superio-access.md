# Spec: Super I/O configuration access and chip detection

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Draft — not implementation-ready |
| Scope | The generic LPC/ISA access mechanism shared by PC Super I/O chips: configuration port pairs, vendor enter/exit key sequences, logical-device selection, chip identification, and locating the hardware-monitor I/O block. Applies to Nuvoton NCT67xx and ITE IT86xx/87xx families. Excludes: per-chip register maps (temperatures, fans, voltages) — those are the Phase 3/4 documents. |
| Issue phase | Phases 2–4 (#1635) — mechanism shared by all of them |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Nuvoton, *NCT6779D* datasheet (representative of the NCT67xx family): "Extended Function Registers" and "Hardware Monitor" chapters | Primary for Nuvoton facts. `TODO(provenance)`: pin section/page numbers per chip in the Phase 3 documents |
| S2 | ITE, *IT8728F Preliminary Specification* (representative of the IT87xx family): "MB PnP Mode" and "Environment Controller" chapters | Primary for ITE facts. `TODO(provenance)`: pin per chip in Phase 4 |
| S3 | PawnIO `LpcIO.p` module source (LGPL-2.1-or-later) | Upstream-published interface definition of the module this project calls (interoperability facts: slot mapping, allowed ports, mutex name). Not used as a source for any chip register fact. No code was copied. |

Facts below are uniform across the respective vendors' datasheet
series; each per-chip Phase 3/4 document must still re-verify them
against the exact datasheet for that chip before implementation relies
on chip-specific values.

## Configuration port pairs

| Fact | Source |
| --- | --- |
| Super I/O chips are configured through an index/data port pair: primary `0x2E` (index) / `0x2F` (data), secondary `0x4E` / `0x4F`. A board straps the chip to one pair | S1, S2 |
| PawnIO `LpcIO` exposes the pairs as slot 0 (`0x2E`/`0x2F`) and slot 1 (`0x4E`/`0x4F`) via `ioctl_select_slot` | S3 |

## Vendor key sequences (configuration mode)

These are **writes**, permitted because reading any configuration
register requires them. They alter no monitoring/control state.

| Vendor | Enter configuration (extended function / MB PnP) mode | Exit | Source |
| --- | --- | --- | --- |
| Nuvoton (NCT67xx; Winbond lineage) | write `0x87`, `0x87` to the index port | write `0xAA` to the index port | S1 |
| ITE (IT86xx/87xx) | at `0x2E`: write `0x87`, `0x01`, `0x55`, `0x55` to the index port; at `0x4E`: write `0x87`, `0x01`, `0x55`, `0xAA` | set bit 1 of configuration register `0x02` (write index `0x02`, then data `0x02`) | S2 |

## Common configuration registers

Once in configuration mode, registers are read by writing the register
index to the index port and reading the data port.

| Index | Meaning | Source |
| --- | --- | --- |
| `0x07` | Logical device select (write the logical device number; **write required** for device-scoped registers) | S1, S2 |
| `0x20` | Chip ID, high byte | S1, S2 |
| `0x21` | Chip ID, low byte | S1, S2 |
| `0x60` / `0x61` | Selected logical device's base address, high/low byte | S1, S2 |

- ITE chip IDs literally encode the part number (example: the IT8728F
  reads `0x87`/`0x28`). (S2)
- Nuvoton chip IDs are family-specific opaque values; the per-chip
  Phase 3 document carries the exact ID table with datasheet
  citations. (S1)
- Reading `0xFF` (or `0x00`) from both ID registers on both port pairs
  means no responding Super I/O chip; detection reports "absent".

## Hardware-monitor I/O block

| Vendor | Logical device | Register access | Source |
| --- | --- | --- | --- |
| Nuvoton NCT67xx | `0x0B` ("Hardware Monitor") | address port = base + `0x05`, data port = base + `0x06`; banked register space — write the bank number to hardware-monitor register `0x4E` (**write required**), then address registers within the bank | S1 |
| ITE IT86xx/87xx | `0x04` ("Environment Controller", EC) | address port = base + `0x05`, data port = base + `0x06`; flat register space (no banks) | S2 |

- The base address is read from configuration registers
  `0x60`/`0x61` of the selected logical device. A base of `0x0000`
  or `0xFFFF` means the block is not mapped; detection reports
  "absent". (S1, S2)
- PawnIO `LpcIO` only allows port reads/writes within the
  configuration pair and the BAR ranges it discovered via
  `ioctl_find_bars`; the hardware-monitor base must therefore be
  discovered through `LpcIO` itself before its ports are accessible.
  (S3)

## Detection procedure

For each slot (0, then 1):

1. Acquire the ISA mutex (below); select the slot
   (`ioctl_select_slot`).
2. Try each vendor's enter sequence in turn; after each, read chip ID
   registers `0x20`/`0x21`.
3. On a recognized ID: select the vendor's hardware-monitor logical
   device (`0x07` ← device number), read the base address
   (`0x60`/`0x61`), run `ioctl_find_bars`, then exit configuration
   mode.
4. On no recognized ID: exit configuration mode (best-effort with the
   matching vendor exit) and continue.
5. Release the mutex. Cache the detection result; re-detection on
   every sample is unnecessary.

## Concurrency and the ISA mutex

- The whole transaction — enter key, configuration reads, bank select,
  hardware-monitor index/data reads, exit — must execute under the
  ecosystem mutex `Global\Access_ISABUS.HTP.Method`, because the
  index/data port protocol is stateful: an interleaved write from
  another monitor changes the selected index/bank between our write
  and read. (Issue #1635 convention; see
  [`pawnio-interface.md`](pawnio-interface.md))
- The PawnIO `LpcIO` module additionally acquires the same named
  mutant per IOCTL (S3); that does not make multi-IOCTL sequences
  atomic, so the client-side mutex above is mandatory.
- Acquisition uses a bounded timeout; on timeout the sample is skipped
  (never proceed unlocked).

## Safety notes

Writes are limited to exactly:

- vendor enter/exit key sequences,
- logical device select (`0x07`),
- hardware-monitor/EC **address** port (index selection),
- Nuvoton bank select (hardware-monitor register `0x4E`).

No hardware-monitor data register is written except the documented
Nuvoton bank-select register (`0x4E`) above. No sensor value,
fan-control, limit, alarm, or other configuration data register is
written in any phase of #1635.

## Open questions

- Some boards' embedded controllers or firmware claim the `0x2E`/`0x2F`
  pair (or mirror a different device); the Phase 2 dump tool should
  capture both slots' raw ID bytes so unknown responders can be
  triaged from user dumps before any chip-specific logic runs.
- Whether ITE's exit (`0x02` bit 1) is required-or-harmful when the
  enter sequence matched a Nuvoton chip (and vice versa) — the
  detection loop above exits with the vendor sequence matching the
  attempted enter key; verify on hardware via Phase 2 dumps.
- Per-chip ID tables and hardware-monitor register maps: deferred to
  the Phase 3 (Nuvoton) / Phase 4 (ITE) documents.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-06-10 | Initial version |
