# Spec: Super I/O configuration access and chip-id diagnostics

| Field | Value |
| --- | --- |
| Revision | 3 |
| Status | Implementation-ready (rev 3) |
| Scope | Phase 2 raw Super I/O chip-id diagnostics only: PawnIO `LpcIO` slot selection for the standard configuration port pairs, Nuvoton/ITE configuration-mode enter/exit sequences, chip-id register reads (`0x20` / `0x21`), absent-id classification for raw diagnostics, and the ISA mutex requirement. Excludes: chip-model tables, logical-device hardware-monitor base discovery, `ioctl_find_bars`, temperature/fan/voltage register maps, and all hardware-monitor data reads. |
| Issue phase | Phase 2 (#1635) — sensor report / chip-id diagnostic mechanism |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Nuvoton, *NCT6779D* datasheet (representative NCT67xx document), "Extended Function Registers" chapter | Primary for the Phase 2 Nuvoton configuration-mode enter/exit sequence and global chip-id configuration registers used by this revision. Per-chip Nuvoton ID tables and hardware-monitor registers remain out of scope for this revision and must be re-pinned in Phase 3 documents before implementation. |
| S2 | ITE, *IT8728F Preliminary Specification* (representative IT87xx document), "MB PnP Mode" / configuration-register section | Primary for the Phase 2 ITE MB PnP enter/exit sequence and global chip-id configuration registers used by this revision. Per-chip ITE ID tables and Environment Controller registers remain out of scope for this revision and must be re-pinned in Phase 4 documents before implementation. |
| S3 | [`pawnio-interface.md`](pawnio-interface.md) revision 4, `LpcIO` module IOCTL contracts and mutex conventions | Primary clean-room interface source for PawnIO `LpcIO` slot mapping, `ioctl_superio_inb` / `ioctl_superio_outb` / `ioctl_pio_outb`, and the caller-held `Global\Access_ISABUS.HTP.Method` mutex. |
| S4 | HardwareVisualizer issue #1635 and Draft PR #1732 scope statement | Project policy source for Phase 2 behavior: return raw per-slot/per-vendor bytes for hardware triage, do not classify chip models yet, and do not read hardware-monitor data in the chip-id diagnostic PR. |

Facts below are implementation-ready only for the Phase 2 diagnostic
scope named in the header. Future Phase 3/4 work must not implement
chip-model mapping, hardware-monitor base discovery, temperatures, fans,
or voltages from this revision; those facts need separate
implementation-ready per-chip documents.

## Phase 2 readiness and default enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| Raw chip-id diagnostic over PawnIO `LpcIO` slot 0 / slot 1 | Implementation-ready for Phase 2 from S1–S4 | Enabled for the diagnostic command; safe to expose raw bytes and errors |
| Chip-id to model mapping | Deferred; no ready ID table in this revision | Disabled until a Phase 3/4 per-chip spec pins the table |
| Hardware-monitor base discovery and `ioctl_find_bars` | Deferred; logical-device/base-register facts are not part of this revision's ready scope | Disabled until a later Super I/O mechanism revision or per-chip spec is implementation-ready |
| Temperature, fan RPM, voltage decode | Deferred; no register maps in this revision | Disabled until per-chip decode specs are implementation-ready |

## Configuration port pairs

| Fact | Source |
| --- | --- |
| A Super I/O chip's configuration interface uses an index/data port pair. The standard primary pair is `0x2E` (index) / `0x2F` (data); the standard secondary pair is `0x4E` (index) / `0x4F` (data). A board straps the chip to one pair. | S1, S2 |
| PawnIO `LpcIO` exposes those pairs as slot 0 (`0x2E`/`0x2F`) and slot 1 (`0x4E`/`0x4F`) via `ioctl_select_slot`. | S3 |
| A Phase 2 diagnostic probes both slots independently and reports the raw result for each; it must not assume slot 0 is the only valid pair. | S3, S4 |

## Vendor key sequences (configuration mode)

These are **writes**, permitted because reading the global chip-id
configuration registers requires entering configuration mode. They are
not hardware-monitor/control writes and they do not alter sensor values,
fan control, limits, alarms, or power state.

| Vendor | Enter configuration mode | Exit configuration mode | Source |
| --- | --- | --- | --- |
| Nuvoton NCT67xx / Winbond lineage | Write `0x87`, then `0x87`, to the selected index port. | Write `0xAA` to the selected index port. | S1 |
| ITE IT86xx/IT87xx | For slot 0 (`0x2E`/`0x2F`): write `0x87`, `0x01`, `0x55`, `0x55` to the selected index port. For slot 1 (`0x4E`/`0x4F`): write `0x87`, `0x01`, `0x55`, `0xAA` to the selected index port. | Set bit 1 of configuration register `0x02` by writing register index `0x02`, then value `0x02`. | S2 |

## Phase 2 configuration registers

Once in configuration mode, the Phase 2 diagnostic reads only the global
chip-id registers by writing the register index to the selected index
port and reading the selected data port. It does not select logical
devices and it does not read any hardware-monitor I/O block.

| Index | Meaning in this revision | Access | Source |
| --- | --- | --- | --- |
| `0x02` | ITE configuration-mode control register; writing value `0x02` exits MB PnP mode. | ITE exit write only | S2 |
| `0x20` | Chip ID high byte | Read | S1, S2 |
| `0x21` | Chip ID low byte | Read | S1, S2 |

- ITE chip IDs encode the part number in the two raw bytes; for the
  representative IT8728F source, the raw bytes are `0x87` / `0x28`.
  A Phase 2 diagnostic may combine the bytes into `0x8728` for display,
  but it must still return the raw high/low bytes. (S2, S4)
- Nuvoton chip IDs are family/model-specific values. This revision does
  not carry a Nuvoton ID table; a Phase 2 diagnostic may combine and
  display the raw high/low bytes, but it must report the model as
  unknown until a Phase 3 Nuvoton spec pins an ID table. (S1, S4)
- For the Phase 2 diagnostic only, a raw reading of `0x00`/`0x00` or
  `0xFF`/`0xFF` is classified as "absent / no usable responder" for
  triage. Mixed values are not collapsed to absent; return them as raw
  responder bytes so they can be reviewed in hardware dumps. (S4)

## Phase 2 detection procedure

For each slot, in slot order 0 then 1:

1. Acquire `Global\Access_ISABUS.HTP.Method` with a bounded timeout.
   If acquisition times out or fails, report the error and do not issue
   any port I/O without the mutex.
2. Select the slot with PawnIO `LpcIO` `ioctl_select_slot`.
3. Nuvoton attempt:
   - enter configuration mode with the Nuvoton sequence,
   - read `0x20` and `0x21`,
   - best-effort exit with the Nuvoton sequence,
   - report raw bytes, combined `chipId`, absent classification, any
     read/enter error, and any exit error.
4. ITE attempt:
   - enter MB PnP mode with the ITE sequence matching the selected
     slot,
   - read `0x20` and `0x21`,
   - best-effort exit with the ITE exit register write,
   - report raw bytes, combined `chipId`, absent classification, any
     read/enter error, and any exit error.
5. Release the mutex after both vendor attempts for the selected slot.

The Phase 2 command is a diagnostic, not a sampling path. It may run on
demand and does not need re-detection caching. Future sampling paths must
cache detection results rather than entering configuration mode on every
sample.

## Concurrency and the ISA mutex

- The whole slot transaction — slot select, vendor enter key,
  chip-id register reads, and vendor exit — must execute under
  `Global\Access_ISABUS.HTP.Method`. Super I/O index/data access is
  stateful, so an interleaved write from another monitor can change the
  selected index between this client's write and read. (S3)
- PawnIO `LpcIO` does not acquire the mutex itself. The caller must hold
  the mutex before each port/config IOCTL, and the same mutex guard must
  cover the multi-IOCTL sequence. (S3)
- Mutex acquisition uses a bounded timeout; timeout means the diagnostic
  reports a skipped/failed attempt and never proceeds unlocked. (S3)

## Safety notes

Writes allowed by this Phase 2 revision are limited to exactly:

- vendor enter key sequences on the selected index port,
- Nuvoton exit key `0xAA` on the selected index port,
- ITE exit write to configuration register `0x02` with value `0x02`.

This revision does **not** allow writes to hardware-monitor data
registers, fan-control registers, PWM registers, limits, alarms, bank
selection registers, or any other device-scoped register. It also does
not allow reads from hardware-monitor data registers.

## Deferred / not ready in this revision

The following items were present in earlier drafts but are intentionally
not implementation-ready in rev 3. They must be specified in later
revisions or per-chip documents before implementation:

- logical-device selection for Nuvoton Hardware Monitor or ITE
  Environment Controller,
- hardware-monitor base-address registers and absent-base rules,
- PawnIO `ioctl_find_bars` usage for Super I/O hardware-monitor ports,
- Nuvoton bank selection and banked hardware-monitor register space,
- ITE EC register map,
- chip-id-to-model tables,
- temperature, fan RPM, and voltage registers or conversions.

## Open questions

- Non-blocking for Phase 2: Some boards' embedded controllers or
  firmware may claim or mirror the `0x2E`/`0x2F` or `0x4E`/`0x4F` pairs.
  The Phase 2 diagnostic is designed to return both slots' raw bytes and
  errors for triage instead of making chip-specific decisions.
- Non-blocking for Phase 2: Whether an ITE exit sequence is required or
  harmful after a Nuvoton enter sequence matched, and vice versa, still
  needs hardware validation. The Phase 2 procedure exits only with the
  same vendor sequence it entered with, records exit errors, and performs
  no further device access based on the result.
- Non-blocking for Phase 2: Per-chip ID tables, hardware-monitor base
  discovery, and hardware-monitor register maps are deferred to the
  Phase 3 Nuvoton and Phase 4 ITE documents. The Phase 2 diagnostic does
  not require them because it reports raw chip-id bytes only.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-06-10 | Initial version |
| 2 | 2026-06-11 | Corrected `Access_ISABUS.HTP.Method` ownership: the LpcIO module documents caller-held acquisition and takes no mutex itself. Still Draft — Phase 3/4 datasheet pinning outstanding. |
| 3 | 2026-06-27 | Narrowed the ready scope to the Phase 2 raw chip-id diagnostic used by PR #1732; removed hardware-monitor base discovery and decode facts from the implementation-ready surface; added scoped enablement, bounded Phase 2 procedure, non-blocking open-question annotations, and status → Implementation-ready. |
