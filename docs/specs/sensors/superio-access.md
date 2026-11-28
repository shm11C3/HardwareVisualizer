# Spec: Super I/O configuration access and chip-id diagnostics

| Field | Value |
| --- | --- |
| Revision | 5 |
| Status | Implementation-ready (rev 5) |
| Scope | Phase 2 raw Super I/O chip-id diagnostics: PawnIO `LpcIO` slot selection for the standard configuration port pairs, Nuvoton/ITE configuration-mode enter/exit sequences, chip-id register reads (`0x20` / `0x21`), absent-id classification for raw diagnostics, the ISA mutex requirement, and (rev 4) the slot-1 ITE configuration-mode exit withhold for secondary-pair embedded-controller chip IDs. Rev 5 adds the [sampling-path rediscovery cadence](#sampling-path-rediscovery-cadence). This product-policy rule says when a Super I/O motherboard sampling path may start a discovery transaction after a failure, and which failure classes may start one. It adds no register access. Excludes: chip-model tables, logical-device hardware-monitor base discovery, `ioctl_find_bars`, temperature/fan/voltage register maps, and all hardware-monitor data reads. The owning per-chip specifications define those. |
| Issue phase | Phase 2 (#1635) — sensor report / chip-id diagnostic mechanism |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Nuvoton, *NCT6779D* datasheet (representative NCT67xx document), "Extended Function Registers" chapter | Primary for the Phase 2 Nuvoton configuration-mode enter/exit sequence and global chip-id configuration registers used by this revision. Per-chip Nuvoton ID tables and hardware-monitor registers remain out of scope for this revision and must be re-pinned in Phase 3 documents before implementation. |
| S2 | ITE Tech., Inc., *IT8728F Environment Control - LPC I/O*, Preliminary Specification V0.4.2, E version (representative IT87xx document); public third-party mirror <https://www.rom.by/files/it8728f_datasheet.pdf>; SHA-256 `5393224C1E497E0F5DED27CF8609D16D3857003F8235C5128E903A1438680C0B` | Primary for the Phase 2 ITE MB PnP enter/exit sequence and global chip-id configuration registers used by this revision. Pins: MB PnP mode entry keys, port-pair selection, and exit §8.1, printed pp. 39–40; global configuration register table (Table 8-1) printed p. 40; Configure Control register `0x02` bit definitions §8.3.1, Chip ID bytes §8.3.3–8.3.4, Chip Version §8.3.5, printed p. 48 (printed page = PDF page − 18). Per-chip ITE ID tables and Environment Controller registers remain out of scope for this revision and must be re-pinned in Phase 4 documents before implementation. |
| S3 | [`pawnio-interface.md`](pawnio-interface.md) revision 4, `LpcIO` module IOCTL contracts and mutex conventions | Primary clean-room interface source for PawnIO `LpcIO` slot mapping, `ioctl_superio_inb` / `ioctl_superio_outb` / `ioctl_pio_outb`, and the caller-held `Global\Access_ISABUS.HTP.Method` mutex. |
| S4 | HardwareVisualizer issue #1635 and Draft PR #1732 scope statement | Project policy source for Phase 2 behavior: return raw per-slot/per-vendor bytes for hardware triage, do not classify chip models yet, and do not read hardware-monitor data in the chip-id diagnostic PR. |
| S5 | Linux kernel, `drivers/hwmon/it87.c` (torvalds/linux `master`, retrieved 2026-10-25), GPL-2.0 | **lead-only (copyleft)**. Source of the restated chip-ID values for IT8790E, IT8792E/IT8795E and IT87952E, the Gigabyte board list, the LPC-error/hang hazard attributed to leaving configuration mode on those chips, the read-before-enter probe order at `0x4E`, and the never-exit handling for those IDs. Read as a source of facts only; no code, structure, or identifiers reproduced. |
| S6 | Out-of-tree `it87` driver, `frankcrawford/it87` (`it87.c`, `master`, retrieved 2026-10-25), GPL-2.0 | **lead-only (copyleft)**. Corroborates S5: same three chip IDs, same never-exit handling with the stated reason (system hang-ups and access failures to the Super I/O chip at the second address), describes `0x4E` as the secondary configuration address. |
| S7 | LibreHardwareMonitor, `LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/LpcIO.cs` and `Lpc/LpcPort.cs` (`master`, retrieved 2026-10-25), MPL-2.0 | **lead-only (copyleft)**. Source of the restated facts that the ITE exit is never issued on port `0x4E`, that the chip ID at `0x4E` is read before any enter key is sent and the key is sent only when the read returns `0xFFFF`, that an IT8792E which receives the enter key while already in configuration mode reports `0x8883`, that `0x8733` / `0x8790` / `0x8695` map to IT8792E / IT8790E / IT87952E, and that its Gigabyte second-controller handling runs only on port `0x4E` with those three IDs. |
| S8 | Linux kernel documentation, `Documentation/hwmon/it87.rst` (kernel.org HTML, retrieved 2026-10-25), GPL-2.0 | **lead-only (copyleft)**. Documentation text only: lists IT8790E, "IT8792E/IT8795E" (one entry) and IT87952E as supported chips whose datasheets are "Not publicly available", found "from Super I/O config space". |
| S9 | HardwareVisualizer issue [#2821](https://github.com/shm11C3/HardwareVisualizer/issues/2821) and the review of PR [#2805](https://github.com/shm11C3/HardwareVisualizer/pull/2805) | Project policy source for the rediscovery cadence. It records the problem: while a retryable discovery failure is cached, every sampling request reruns the whole discovery transaction. It also records the constraints. Rediscovery must never run inside a sample transaction. A non-zero ITE bank at discovery needs an explicit eligibility decision. Supplies no hardware fact. |
| S10 | HardwareVisualizer repository at commit `75e9fcba`: `hardviz/core/src/sensor_service/windows/providers.rs`, `hardviz/core/src/sensor_service/protocol.rs` and `hardviz/core/src/infrastructure/providers/windows/pawn_io/super_io/motherboard.rs` | Repository facts the cadence is sized against. The service requests a motherboard reading at most every 1.6 s. Once 30 s have passed since a motherboard sampler was created, the service recreates it after any unavailable reading; it never recreates a sampler for an Unsupported reading. The service releases all sampling state when no sample is requested for 15 s, or when two samples are more than 5 s apart, which covers suspend and resume. The ITE sampling path keeps at least 1.5 s between sample bursts. One discovery transaction probes slot 0 and then slot 1, and on each slot runs the NCT67xx probe, then the source-selected Nuvoton probe, then the ITE probe, each with its own configuration-mode entry. Supplies no hardware fact. |

Facts below are implementation-ready only for the Phase 2 diagnostic
scope named in the header. Future Phase 3/4 work must not implement
chip-model mapping, hardware-monitor base discovery, temperatures, fans,
or voltages from this revision; those facts need separate
implementation-ready per-chip documents. The rediscovery cadence of rev 5
is the exception: it binds every Super I/O motherboard sampling path, and
it governs only when those paths may start a discovery transaction.

## Phase 2 readiness and default enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| Raw chip-id diagnostic over PawnIO `LpcIO` slot 0 / slot 1 | Implementation-ready for Phase 2 from S1–S4. Unchanged in rev 4: the slot-1 exit withhold below subtracts a write from this row's procedure and is not a dependency of any read, classification, or report this row produces. | Enabled for the diagnostic command; safe to expose raw bytes and errors |
| Slot-1 ITE configuration-mode exit withhold for raw IDs `0x8733` / `0x8790` / `0x8695` / `0x8883` | **Experimental** — rests on lead-only facts F1, F2, F3, F5, F6 and F8 (S5, S6, S7, S8). No vendor document or maintainer-accepted hardware dump covers these parts; the Experimental label records that verification state. | **Experimental** — applied whenever one of these IDs is read on slot 1 by the Phase 2 diagnostic or by any later path that reuses this document's ITE enter/read/exit sequence. The rule withholds a write and performs no read, decode, or attempt of its own, so there is no best-effort attempt to make and no experimental failure to surface; the diagnostic only reports that the exit was intentionally skipped. |
| Sampling-path rediscovery cadence and eligibility classes | Project policy from S9 and S10. Rests on no hardware fact and no lead-only fact. It only limits when a discovery transaction may start and which failures may start another one. | **Enabled** for every Super I/O motherboard sampling path, whatever the Verified or Experimental state of its rows. It never relaxes a gate, a read limit or a write limit of an owning specification. |
| Chip-id to model mapping | Deferred; no ready ID table in this revision. The ID values in F1–F3 exist only to scope the exit withhold; they do not make model mapping ready. | Disabled until a Phase 3/4 per-chip spec pins the table |
| Hardware-monitor base discovery and `ioctl_find_bars` | Deferred; logical-device/base-register facts are not part of this revision's ready scope | Disabled until a later Super I/O mechanism revision or per-chip spec is implementation-ready |
| Temperature, fan RPM, voltage decode | Deferred; no register maps in this revision | Disabled until per-chip decode specs are implementation-ready |

### Note on lead-only facts in this document (ADR 0023)

The lead-only facts F1–F8 are confined to the Experimental "exit
withhold" row above and are not depended on by the raw chip-id row.
The raw chip-id diagnostic produces exactly the same raw bytes,
absent classification, enter/read errors, and mutex behavior whether or
not F1–F8 are true; the only effect of the withhold is that one
non-required write (the ITE exit, see Safety notes) is not issued on
slot 1 for four exact ID values, and that this omission is reported.
Under the safety policy of the specs README, writes are documented only
where a read transaction requires them; the exit write has never been
required by the chip-id read transaction (it is clean-up after the
read), so withholding it narrows the set of hardware mutations this
document permits and introduces none. A lead-only fact that turns out
to be wrong therefore cannot cause a wrong reading or an unsafe write;
its worst case is a chip left in configuration mode, which is the state
S5/S6 report the firmware of the affected boards already leaves it in
(see Open questions for the residual uncertainty). This is why the rule
is expressed as a safety constraint with an Experimental verification
label rather than as a widening of hardware coverage, and why the raw
chip-id row keeps its rev 3 status.

## Configuration port pairs

| Fact | Source |
| --- | --- |
| A Super I/O chip's configuration interface uses an index/data port pair. The standard primary pair is `0x2E` (index) / `0x2F` (data); the standard secondary pair is `0x4E` (index) / `0x4F` (data). A board straps the chip to one pair. | S1, S2 |
| PawnIO `LpcIO` exposes those pairs as slot 0 (`0x2E`/`0x2F`) and slot 1 (`0x4E`/`0x4F`) via `ioctl_select_slot`. | S3 |
| A Phase 2 diagnostic probes both slots independently and reports the raw result for each; it must not assume slot 0 is the only valid pair. | S3, S4 |
| For the ITE MB PnP key sequence, the fourth key byte selects which pair becomes the configuration ports: `0x55` selects `0x2E`/`0x2F`, `0xAA` selects `0x4E`/`0x4F`. | S2 §8.1, printed p. 39 |

## Vendor key sequences (configuration mode)

These are **writes**, permitted because reading the global chip-id
configuration registers requires entering configuration mode. They are
not hardware-monitor/control writes and they do not alter sensor values,
fan control, limits, alarms, or power state.

| Vendor | Enter configuration mode | Exit configuration mode | Source |
| --- | --- | --- | --- |
| Nuvoton NCT67xx / Winbond lineage | Write `0x87`, then `0x87`, to the selected index port. | Write `0xAA` to the selected index port. | S1 |
| ITE IT86xx/IT87xx | For slot 0 (`0x2E`/`0x2F`): write `0x87`, `0x01`, `0x55`, `0x55` to the selected index port. For slot 1 (`0x4E`/`0x4F`): write `0x87`, `0x01`, `0x55`, `0xAA` to the selected index port. | Set bit 1 of configuration register `0x02` by writing register index `0x02`, then value `0x02` (exactly `0x02`; see Safety notes for bit 0). On slot 1 the exit is **withheld** for the IDs listed under "Slot-1 ITE exit withhold rule". | S2 §8.1, printed pp. 39–40; §8.3.1, printed p. 48; withhold: S5, S6, S7 (lead-only) |

## Phase 2 configuration registers

Once in configuration mode, the Phase 2 diagnostic reads only the global
chip-id registers by writing the register index to the selected index
port and reading the selected data port. It does not select logical
devices and it does not read any hardware-monitor I/O block.

| Index | Meaning in this revision | Access | Source |
| --- | --- | --- | --- |
| `0x02` | ITE Configure Control register (write-only, non-sticky). Bit 1 = return to the "Wait for Key" state, i.e. exit MB PnP mode; the exit write is value `0x02`. Bit 0 = reset all logical devices and restore configuration registers to their power-on state; this document never permits writing a value with bit 0 set. | ITE exit write only (value exactly `0x02`) | S2 §8.3.1, printed p. 48 |
| `0x20` | Chip ID high byte | Read | S1, S2 §8.3.3, printed p. 48 |
| `0x21` | Chip ID low byte | Read | S1, S2 §8.3.4, printed p. 48 |

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

## Secondary-pair ITE embedded controllers (lead-only facts)

Every fact in this table rests only on copyleft implementations and is
therefore lead-only (ADR 0023). These facts back only the Experimental
"exit withhold" row; they do not make chip-model mapping ready and they
are not ID-table facts for Phase 4.

| ID | Fact (restated) | Source |
| --- | --- | --- |
| F1 | Raw chip ID `0x8733` (bytes `0x87` / `0x33`) is reported by the IT8792E and also by the IT8795E; no source gives the IT8795E a distinct chip ID, and the kernel sources treat "IT8792E/IT8795E" as one chip type. The chip does not report `0x8792`. | S5, S6, S7, S8 — lead-only (copyleft) |
| F2 | Raw chip ID `0x8790` (bytes `0x87` / `0x90`) is reported by the IT8790E. | S5, S6, S7 — lead-only (copyleft) |
| F3 | Raw chip ID `0x8695` (bytes `0x86` / `0x95`) is reported by the IT87952E; S5 describes this as an early ID of that part. | S5, S6, S7 — lead-only (copyleft) |
| F4 | No public datasheet exists for IT8790E, IT8792E/IT8795E, or IT87952E; the kernel documentation lists them as detected from Super I/O configuration space with datasheets not publicly available. | S8 — lead-only (copyleft) |
| F5 | Placement convention: Gigabyte desktop boards that carry two ITE chips strap the primary hardware-monitor Super I/O to the primary pair `0x2E`/`0x2F` and a second ITE embedded controller — IT8790E, IT8792E/IT8795E, or IT87952E — to the secondary pair `0x4E`/`0x4F`. Boards named by S5: AM4 AB350 and AX370 (IT8792E), X299 (IT8795E), Z97X-Gaming and Z87X-OC (IT8790E). S7 applies its Gigabyte second-controller handling only when the port is `0x4E` and the ID is one of F1–F3. | S5, S7 — lead-only (copyleft) |
| F6 | Hazard: on the boards in F5, taking the second chip out of configuration mode (the register `0x02` exit write) has caused LPC bus access errors and system hangs, including access failures when the first chip at `0x2E` is used afterwards; S5 attributes the behavior to a defect in the IT8792E and reports the same symptom for IT8795E and IT8790E. S5 and S6 therefore never issue the exit for the IDs in F1–F3 and leave those chips in configuration mode. S7 never issues the ITE exit on port `0x4E` for any chip. | S5, S6, S7 — lead-only (copyleft) |
| F7 | Read-before-enter at `0x4E`: S5 and S7 read the chip-id registers at `0x4E` before sending any enter key and send the key only when that read returns `0xFFFF` (S7 treats `0xFFFF` as the only failure value; S5 gives up if the read after entering is still `0xFFFF`). The reason given is that the second chip is commonly already in configuration mode when the operating system starts. | S5, S7 — lead-only (copyleft) |
| F8 | An IT8792E that receives the `0x4E` enter key while it is already in configuration mode has been observed to report chip ID `0x8883` (bytes `0x88` / `0x83`) instead of `0x8733`. | S7 — lead-only (copyleft) |

## Slot-1 ITE exit withhold rule

This rule is normative for the Phase 2 diagnostic and for every later
path that reuses this document's ITE enter/read/exit sequence on
slot 1 (`0x4E`/`0x4F`). It rests on F1–F3, F5, F6 and F8 (lead-only);
see the ADR 0023 note above for why a lead-only fact may back it.

1. After the slot-1 ITE attempt has read registers `0x20` and `0x21`,
   combine the raw bytes into the 16-bit value high:low.
2. If that value is exactly one of `0x8733`, `0x8790`, `0x8695` or
   `0x8883`, the ITE exit write (register `0x02`, value `0x02`)
   **MUST NOT** be issued for that slot in that transaction. Matching is
   exact on the combined value; no mask or range generalization is
   permitted, and no substitute exit mechanism may be used.
3. The diagnostic reports the exit as *intentionally skipped* for that
   slot/vendor attempt — a state distinct from "exit succeeded" and
   "exit error" — together with the raw bytes that triggered it, so
   hardware dumps show which boards carry such a chip.
4. Everything else in the slot transaction is unchanged: the raw bytes,
   combined ID, absent classification, enter/read errors, and the mutex
   release after both vendor attempts.
5. On slot 0 (`0x2E`/`0x2F`) the rule does not apply; the ITE exit is
   issued as before regardless of the ID read.

`0x8883` is included because, per F8, it is the value a chip that would
otherwise report `0x8733` returns when the diagnostic's unconditional
enter key reaches it in the already-unlocked state the firmware leaves
it in (F7); withholding the exit for that value keeps the protection
effective without changing the Phase 2 enter step. Withholding the exit
for a different chip that legitimately reports `0x8883` costs only the
non-required clean-up write.

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
   - on slot 1, apply the "Slot-1 ITE exit withhold rule"; if it
     matches, skip the exit write and report the exit as intentionally
     skipped,
   - otherwise best-effort exit with the ITE exit register write,
   - report raw bytes, combined `chipId`, absent classification, any
     read/enter error, and either the exit error or the skipped-exit
     state.
5. Release the mutex after both vendor attempts for the selected slot.

The Phase 2 command is a diagnostic, not a sampling path. It may run on
demand and does not need re-detection caching. Sampling paths must cache
detection results rather than entering configuration mode on every
sample. After a failed discovery they follow the
[sampling-path rediscovery cadence](#sampling-path-rediscovery-cadence).

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

## Sampling-path rediscovery cadence

This section is product policy (S9, S10), not a hardware fact. It binds
every Super I/O motherboard sampling path that runs a discovery
transaction. The owning specifications are
[`superio-nuvoton-nct67xx.md`](superio-nuvoton-nct67xx.md),
[`superio-nuvoton-nct6775-nct6776-nct610x.md`](superio-nuvoton-nct6775-nct6776-nct610x.md),
[`superio-fintek-f71882-f71889.md`](superio-fintek-f71882-f71889.md),
[`superio-ite-it86xx-it87xx.md`](superio-ite-it86xx-it87xx.md),
[`superio-ite-it8728f-ex-ga-b75m-d3h-fans.md`](superio-ite-it8728f-ex-ga-b75m-d3h-fans.md)
and
[`superio-ite-primary-lpc-expansion.md`](superio-ite-primary-lpc-expansion.md).
The rule decides only when a discovery transaction may start. It does not
change the order, gates, reads or writes of any discovery or sample
transaction. It never permits an access that an owning specification
forbids. The Phase 2 diagnostic is not a sampling path. Its runs are not
discovery attempts here: they do not count toward the backoff and do
not make an attempt due.

Where an owning specification says a failure requires "rediscovery", or
leaves a value unavailable "until rediscovery", it refers to this
section.

### Terms

| Term | Meaning |
| --- | --- |
| Discovery attempt | One discovery transaction. It opens the PawnIO `LpcIO` executor, acquires `Global\Access_ISABUS.HTP.Method`, runs the slot and vendor probes of the owning specifications in order, and releases the mutex. It ends either with a cached path or with a failure. |
| Cached path | The slot, profile and port proof that a successful attempt cached, as the owning specification defines them. |
| Sample transaction | One acquisition of the ISA mutex on a cached path, with the reads the owning specification defines for a sample. |
| Successful sample | A sample transaction that returns a sample, even one in which some inputs are unavailable. A sample that the owning specification reports unavailable as a whole is not successful. |
| Discarded path | A cached path that is dropped while the session continues. Either the owning specification invalidates it, or the sampler holding it is recreated (S10). |
| Monitoring session | The period during which the sensor service keeps motherboard sampling state. It starts when that state is created. It ends when the service releases that state: after a period with no sample request, after a gap between samples, which covers suspend and resume, or when the service stops (S10). Recreating a sampler does not end the session. |
| Failure count `n` | The number of failed attempts and discarded paths in the session since the last successful sample, or since the session started. |

### Eligibility classes

Every discovery failure is in one of two classes.

- **Eligible for rediscovery.** A transient access condition stopped the
  attempt before it reached a stable conclusion. A later attempt under
  rule R2 may succeed.
- **Not eligible.** The attempt reached a stable conclusion. Either no
  supported responder was found, or PawnIO read hardware or firmware
  state that the owning specification refuses. The sampler starts no
  further attempt on its own. The failure stays reported until the
  sampler is recreated or the session ends, and rule R4 bounds the
  first attempt of a recreated sampler.

A failure is not eligible unless the table below or the owning
specification marks it eligible. An attempt that caches no path is
eligible when at least one of its probes ended with an eligible
failure, and it then reports that eligible failure.

An unconfirmed exit takes precedence over every other outcome of the
attempt. Once an entry-key or exit write fails on any slot, the attempt
makes no further probe on any slot and caches no path. This holds even
when a later probe would have qualified. The attempt ends with that
eligible failure. The slot and the vendor sequence whose write failed
are recorded as **owed cleanup**.

**Cleanup first.** Every later attempt begins, under its own ISA mutex
acquisition, with a cleanup step for each slot that owes one, in slot
order. The step comes before the normal ordered scan of the owning
specifications. It never depends on whether an earlier slot would
qualify.

- The cleanup selects the slot and writes the entry keys of the
  recorded vendor sequence. It then writes that sequence's exit and
  makes no other access.
- On slot 1 there is one exception, whatever the recorded sequence.
  Between entry and exit, the cleanup reads `0x20`/`0x21` and compares
  the combined ID with the withhold set of the
  [slot-1 ITE exit withhold rule](#slot-1-ite-exit-withhold-rule):
  `0x8733`, `0x8790`, `0x8695` and `0x8883`. This applies even when the
  failed attempt never read the ID.
  - **ID in the set:** no exit write of any kind is issued. The
    withheld exit counts as a completed cleanup.
  - **ID outside the set:** the recorded exit is written.
  - **ID read fails:** no exit write is issued. A cleanup without a
    complete ID cannot exclude a protected chip, so the exit stays
    withheld. The cleanup stays owed, and the attempt stops as an
    eligible failure under R2.

  An owning specification that withholds a non-ITE exit on slot 1, such
  as the Fintek `0xAA` exit, uses the same set and the same outcomes.
- A cleanup whose entry and exit writes both succeed clears that slot's
  owed cleanup. Only then does the attempt continue to the next owed
  slot, and after the last one to the normal scan.
- If a cleanup write fails, the attempt stops. It caches no path, keeps
  the cleanup owed and ends with an eligible failure under R2.

No path is cached while any cleanup is owed. Owed cleanup is not
cleared by a session start or by sampler recreation; only a completed
cleanup clears it. The cleanup writes are the entry keys and the exit
writes of this document's Safety notes.

The rule rejects the alternative of keeping a qualified path and owing
the failed slot a cleanup exit. That cleanup would be a configuration
write while a path is cached, which R1 forbids. It would also select
the failed slot on the shared executor, which invalidates the cached
BAR authorization
([`superio-fintek-f71882-f71889.md`](superio-fintek-f71882-f71889.md)
S11; [`superio-ite-primary-lpc-expansion.md`](superio-ite-primary-lpc-expansion.md) S11).
Stopping the attempt also avoids further configuration-mode entries on
a transport that has just failed a write.

An exit withheld under the
[slot-1 ITE exit withhold rule](#slot-1-ite-exit-withhold-rule), or
under an owning specification that applies it, is intentional. It is
not an unconfirmed exit and not a failure.

| Failure (every sampling path) | Class | Reason |
| --- | --- | --- |
| ISA mutex acquisition timed out, or the wait failed | Eligible | Another client holds the mutex for its own multi-step transaction and releases it when that transaction ends (S3). |
| ISA mutex could not be created or opened | Not eligible | This is not contention. Another attempt cannot change it. |
| PawnIO `LpcIO` executor cannot be opened | Not eligible | No hardware access took place. Recovery needs an install or a permission change, and sampler recreation picks that up (S10). |
| On a slot, a configuration-mode entry-key write failed, or the matching exit write failed | Eligible, for every vendor sequence and whether or not a profile matched | The chip on that slot may still be in configuration mode. A later attempt is the only point at which the sampling path issues the matching exit again. The attempt stops and caches no path (see above). For ITE, S2 §8.1, printed p. 39, warns that an undesired result may occur if MB PnP mode is not exited properly. |
| A PawnIO port read or write fails after `ioctl_find_bars` succeeded on a matched profile, before the path is cached | Eligible | `ioctl_find_bars` success does not prove that a usable window was authorized (S3; [`superio-ite-it86xx-it87xx.md`](superio-ite-it86xx-it87xx.md) S3). A later attempt runs the BAR discovery again. The owning specifications list the reads concerned. |
| `ioctl_find_bars` fails, or a configuration-register read fails, and the exit is confirmed | Not eligible | This keeps the existing classification (see [Open questions](#open-questions)). |
| A gate refuses a value that PawnIO read successfully | Not eligible, unless the owning specification marks the value eligible | The value describes the chip or its firmware state. Examples are a lock bit, an inactive logical device, a refused base, a vendor ID or vendor byte mismatch, or a revision nibble. |
| No responder, or no profile matches the chip ID | Not eligible | The probe completed. |

### Rules

- **R1 Separation.** A discovery attempt is its own transaction and
  never runs inside a sample transaction. It acquires and releases the
  ISA mutex itself. No sample transaction contains a configuration-mode
  entry key or exit write. One sampling request may run a due attempt
  and then, if that attempt cached a path, a sample transaction. The
  attempt must release the mutex before the sample transaction acquires
  it. While a path is cached, no discovery attempt runs. A failed sample
  on a cached path is not a discovery failure and never starts an
  attempt.
- **R2 Backoff after an eligible failure.** After an attempt that ends
  with an eligible failure, the next attempt must not start until
  `D(n) = min(2 s × 2^(n−1), 30 s)` has elapsed since the start of the
  failed attempt. The delays are therefore 2 s, 4 s, 8 s and 16 s, then
  30 s for every later failure.
- **R3 Not due.** A sampling request that arrives before the next
  attempt is due makes no PawnIO open, no mutex acquisition and no port
  I/O for discovery. It reports the session's most recent failure
  unchanged, with the same text, class and guidance. That failure is
  either a discovery failure or the sample failure that preceded a
  discarded path. If the session has no failure, as when a path that
  sampled successfully is invalidated, the request reports a
  *rediscovery pending* result. That result is unavailable, not
  Unsupported. It is eligible and offers no PawnIO install or permission
  guidance, because no PawnIO access failed. Its detail states that the
  previous path was discarded and that discovery will run again.
- **R4 Not eligible, and recreation.** After an attempt that ends with
  a not-eligible failure, the sampler starts no further attempt. Within
  one session, the first attempt of a recreated sampler waits for the
  delay that the session's last event requires:

  | Last event | Delay |
  | --- | --- |
  | Eligible failure | `D(n)` from the start of the failed attempt |
  | Discarded path | `D(n)` from the discard |
  | Not-eligible failure | 30 s from the start of the failed attempt |

  Until then the recreated sampler makes no attempt and reports what R3
  requires. Recreation does not reset `n`.
- **R5 Reset.** `n` returns to 0 at the first successful sample on a
  cached path, and when a session starts. The first attempt of a session
  runs at the session's first sampling request without delay. An attempt
  that caches a path stops further attempts (R1) but does not reset
  `n` by itself. A discovered path that never produces a successful
  sample therefore cannot reset the backoff.
- **R6 Discarded path.** A discarded path increments `n`. The next
  attempt waits `D(n)` from the discard.
- **R7 Clock.** Elapsed time comes from a monotonic clock. A delay is a
  lower bound. The attempt runs at the first sampling request at or
  after its due time, so the actual spacing rounds up to the request
  cadence. If the clock reads earlier than the recorded start, the
  elapsed time is zero, so the attempt is not due.
- **R8 Sample intervals.** A per-path minimum between sample bursts,
  such as the 1.5 s ITE minimum, governs only sample transactions on a
  cached path. Discovery attempts do not wait for it and do not restart
  it. The first sample transaction after a successful attempt may run in
  the same sampling request. The service's 1.6 s request interval only
  rounds the delays up (R7).

### Why these values

- **2 s first delay.** The value is just above the service's 1.6 s
  request interval and the 1.5 s ITE burst minimum (S10). Two attempts
  therefore never run on consecutive requests. At the service cadence the
  first retry falls on the second request after the failure, about 3.2 s
  later. The eligible conditions are expected to be short. Another
  monitor holds the ISA mutex for one multi-step transaction. An
  embedded controller changes the ITE bank for one serial-bus access
  ([`superio-ite-primary-lpc-expansion.md`](superio-ite-primary-lpc-expansion.md)
  S17, S18). A port denial clears on the next BAR discovery. Four retries
  in the first 16 s let these clear within seconds.
- **Doubling.** A short burst recovers quickly, and a persistent
  condition reaches the cap after four retries without a long fixed wait.
- **30 s cap.** The service already recreates an unavailable sampler
  30 s after it was created (S10). That recreation reruns discovery at
  this rate for not-eligible failures today. With the same cap, a
  persistent eligible condition costs no more configuration-mode entries
  than a persistent not-eligible failure already does. A condition that
  ends while the cap applies is picked up within 30 s.
- **Resulting bound.** With zero-length attempts, a persistent eligible
  condition starts attempts at 0, 2, 6, 14 and 30 s after the first
  failure, then every 30 s. That is five attempts in the first minute
  and two per minute after that. Each attempt enters configuration mode
  at most once per vendor probe and slot. With today's three probes on
  two slots that is six entries (S10), so at most 12 entries per minute
  in the steady state. Before this rule, the sampler made about 37
  attempts per minute at the 1.6 s request interval.
- **Mutex timeouts count.** Backing off while another monitor holds the
  mutex reduces contention with that monitor (S3).

### Fixtures

Use a controllable monotonic clock and a fake executor. The fake records
every PawnIO open, mutex acquisition and configuration-key write. An
attempt takes no time unless the fixture says otherwise. Times are
seconds after the first attempt starts.

| Fixture | Expected observation |
| --- | --- |
| The first attempt ends with an eligible failure (mutex timeout). Requests at 0.5, 1.0 and 1.999 | No executor open, mutex acquisition or port I/O. Each request reports the first failure with the same text, class and guidance. |
| Same, request at 2.0 | Exactly one attempt. |
| An eligible failure persists, with requests every 0.1 from 0 to 120 | Attempts start at 0, 2, 6, 14, 30, 60, 90 and 120, eight in total. |
| An eligible failure persists, with requests every 1.6 (service cadence) from 0 to 64 | Attempts start at 0, 3.2, 8.0, 16.0, 32.0 and 62.4, six in total. |
| Eligible failures at 0 and 2, a successful attempt and a successful sample at 6, then the path is discarded at 100 | No attempt between 6 and 100. The next attempt starts no earlier than 102. |
| A successful attempt and sample at 0, the path is invalidated at 50 with no failure in the session, then a request at 51 | No attempt at 51. The request reports the *rediscovery pending* result: unavailable, not Unsupported, eligible and without PawnIO guidance. A request at 52 runs an attempt. |
| A successful path whose sample fails at 50, after which the sampler is recreated | Until 52 the recreated sampler reports that sample failure with its text and guidance. |
| Requests every 0.1. Every attempt caches a path, its first sample fails as a whole, and the path is then discarded | Attempts start at 0, 2, 6, 14, 30 and 60. Discovery success alone never resets the backoff. |
| A not-eligible failure at 0, such as a refused lock bit or vendor-ID mismatch, with requests every 1.6 for 120 on the same sampler | Exactly one attempt. Every request reports the same failure. |
| Same failure, and the sampler is recreated at 10 in the same session | The recreated sampler makes no attempt at creation. It reports the carried-over failure and attempts at its first request at or after 30. |
| Eligible failures at 0, 2 and 6, and the sampler is recreated at 10 | The recreated sampler's first attempt starts no earlier than 14, and `n` continues from 3. |
| In one attempt, slot 0 ends not eligible and slot 1 eligible, and separately the reverse order | Both attempts are eligible, report the eligible failure, and are followed by `D(n)`. |
| Slot 0 reads an unmatched chip ID and its exit write fails, while slot 1 holds a chip that would qualify | The attempt issues no `ioctl_select_slot(1)` and no write to slot 1. It caches no path and ends eligible. The next attempt, no earlier than `D(n)`, starts again at slot 0 with its entry keys and exit. Slot 1 can qualify only in an attempt whose slot 0 exit is confirmed. |
| Attempt 1: slot 0 fails its base gate, then the exit write on slot 1 fails. Attempt 2 is due, and slot 0 now qualifies. | Attempt 2 first selects slot 1, writes the recorded entry keys and exit, and then scans slot 0. It caches slot 0 only after the slot 1 exit write succeeds. If the slot 1 cleanup write fails again, attempt 2 makes no slot 0 access, caches nothing and ends eligible. |
| Cleanup is owed on slot 1, and the service releases the session before the next attempt | The first attempt of the new session still begins with the slot 1 cleanup. |
| Cleanup owed on slot 1 for an ITE sequence. The cleanup ID read returns `0x87`/`0x33` | No exit write. Cleanup complete, and the attempt continues to the normal scan. |
| Cleanup owed on slot 1 for an ITE sequence, and the cleanup ID read fails | No exit write. The cleanup stays owed, no path is cached, and the attempt ends eligible with the next attempt after `D(n)`. |
| Cleanup owed on slot 1 for a Nuvoton-lineage sequence whose entry write failed before any ID read. The cleanup ID read returns `0x8790`, and separately an ID outside the set | For `0x8790`, no `0xAA` and no other exit write, and the cleanup is complete. For the other ID, `0xAA` is written. |
| An unconfirmed exit on a later probe of slot 0, after an earlier probe on the same slot found nothing | The attempt stops at that probe, caches no path and ends eligible. |
| A due request whose attempt caches a path, followed by a sample transaction in the same request | Two mutex acquisitions in order. The first contains every configuration-key write and is released before the second. The second contains no entry key and no exit write. |
| A cached path whose samples fail as a whole on several requests, for example a banked ITE opening check that reads EC `0x06` = `0x20`, while the path stays cached | No discovery attempt and no configuration-key write. Samples continue at their own cadence. |
| `n` = 5, then the service releases sampling state for idleness or a sampling gap, then a new request | An attempt runs at once. If it fails eligibly, the next waits 2. |
| ITE: a successful attempt and a first sample burst at 0, then requests at 1.0 and 1.5 | The 1.0 request reuses the burst from 0. The 1.5 request runs a new burst. Neither runs a discovery attempt. |
| The clock reads earlier than the start of the failed attempt | Not due. No attempt. |
| A Phase 2 diagnostic runs between two sampling requests while a failure is cached | The diagnostic does not change `n` or the due time. |

## Safety notes

Writes allowed by this Phase 2 revision are limited to exactly:

- vendor enter key sequences on the selected index port,
- Nuvoton exit key `0xAA` on the selected index port,
- ITE exit write to configuration register `0x02` with value exactly
  `0x02`, except where the "Slot-1 ITE exit withhold rule" withholds it.

Additional constraints:

- The ITE exit write is clean-up after the chip-id read, not a
  prerequisite of it. Withholding it never widens hardware mutation.
- Never write a value with bit 0 set to ITE register `0x02`: bit 0
  resets all logical devices and restores configuration registers to
  their power-on state (S2 §8.3.1, printed p. 48).
- Write the `0xAA`-terminated ITE key only to port `0x4E` and the
  `0x55`-terminated key only to port `0x2E`. Per S2 §8.1 the fourth
  byte selects which pair becomes the configuration ports, so a
  `0xAA`-terminated key reaching a chip on the primary pair could move
  it to the secondary pair, where a second chip may already respond.

This revision does **not** allow writes to hardware-monitor data
registers, fan-control registers, PWM registers, limits, alarms, bank
selection registers, or any other device-scoped register. It also does
not allow reads from hardware-monitor data registers.

## Deferred / not ready in this revision

The following items were present in earlier drafts but are intentionally
not implementation-ready in rev 5. They must be specified in later
revisions or per-chip documents before implementation:

- logical-device selection for Nuvoton Hardware Monitor or ITE
  Environment Controller,
- hardware-monitor base-address registers and absent-base rules,
- PawnIO `ioctl_find_bars` usage for Super I/O hardware-monitor ports,
- Nuvoton bank selection and banked hardware-monitor register space,
- ITE EC register map,
- chip-id-to-model tables (F1–F3 scope the exit withhold only),
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
- Non-blocking for Phase 2: the withhold set covers the ID the chip
  reports after the diagnostic's unconditional enter (F8), so the
  protection does not depend on changing the enter step; a changed enter
  step is a separate revision. Whether the slot-1 ITE attempt should
  read the chip-id registers *before* sending the enter key and skip
  the key when the read is not `0xFFFF` (the S5/S7 behavior in F7) is
  unresolved. Evidence so far: both leads do this because the second
  chip is commonly already in configuration mode; S7 reports that
  re-entering an IT8792E in that state makes it report `0x8883` (F8),
  which means the current Phase 2 enter step can hide the true
  `0x8733` identity in the raw report. Against: whether registers
  `0x20`/`0x21` are readable at `0x4E` without the key on every ITE part
  is known only from these leads, and S2 §8.1 describes configuration
  registers as accessible after entering MB PnP mode. A hardware dump
  from a Gigabyte two-chip board showing the slot-1 pre-enter read and
  the post-enter read would settle it.
- Non-blocking for Phase 2: the withheld write is not required by the
  read transaction, and the leads report that the affected boards' own
  firmware and the Linux driver leave these chips in configuration mode
  as a matter of course. Whether leaving an ITE chip in MB PnP mode has
  side effects is otherwise unverified for the parts in F1–F3. S2 §8.1
  (IT8728F, printed p. 39) states that an undesired result may occur if
  MB PnP mode is not exited properly, without specifying the result;
  that statement covers the IT8728F, for which the exit is still issued.
  No vendor statement exists for IT8790E, IT8792E/IT8795E or IT87952E
  (F4).
- Non-blocking for Phase 2: no hazard is reported for any other ID, the
  IT8728F primary source directs a proper exit, and the diagnostic
  records exit errors for triage. Whether other ITE IDs found on slot 1
  share the hazard is unresolved: S7 withholds the exit on `0x4E` for
  every chip, while S5/S6 withhold it only for `0x8733`, `0x8790`,
  `0x8695`. Rev 4 follows the ID-scoped set so that the IT8728F
  guidance governs everywhere the hazard is not reported. A report of
  LPC errors or hangs on a board whose slot-1 ITE ID is outside the set
  would be grounds to widen it.
- Non-blocking for Phase 2: the slot-1 key is only ever written to port
  `0x4E` and the diagnostic reports raw results per slot either way.
  S2 §8.1 describes both ITE key variants as writes to the special
  address port `0x2E`, with the fourth byte selecting the resulting
  pair, whereas S5, S6 and S7 write the `0xAA`-terminated key to port
  `0x4E` itself and the Phase 2 procedure does the same. Whether the
  second-chip parts in F1–F3 also accept the key on `0x2E` is unknown
  and must not be tested by the diagnostic (see Safety notes).
- Non-blocking for Phase 2: model mapping is deferred, and the withhold
  rule needs only the shared raw value. IT8792E and IT8795E share
  `0x8733` (F1); whether the Chip Version register `0x22` (S2 §8.3.5)
  distinguishes them is unknown — no lead uses it for that purpose.
- Non-blocking for Phase 2: the rule only withholds a write, and the
  skipped-exit report is the evidence-collection path. None of F1–F8 is
  backed by a vendor document or a maintainer-accepted hardware dump.
  A dump from a Gigabyte board with a second ITE chip, showing the
  slot-1 raw bytes and the skipped-exit state, would let the withhold
  row graduate from Experimental to Verified under ADR 0023.
- Non-blocking for the rediscovery cadence: the values are product
  policy sized against the current service cadence (S10), and no source
  reports a side effect of repeated configuration-mode entry and exit on
  any Nuvoton, ITE or Fintek part. The rule bounds the steady state at
  two attempts per minute. A report that an eligible condition regularly
  outlasts the backoff, or that configuration-mode entry has a side
  effect at that rate, would be grounds for a revision.
- Non-blocking for the rediscovery cadence: these failures keep their
  existing not-eligible classification, and sampler recreation still
  retries them at most every 30 s under R4. It is unknown whether an
  `ioctl_find_bars` failure, or a configuration-register read failure
  with a confirmed exit, is transient. The S7 observation in
  [`superio-nuvoton-nct67xx.md`](superio-nuvoton-nct67xx.md) shows
  `ioctl_find_bars` failing for an ordering reason, not a transient one.
  Evidence that such a failure clears within seconds would be grounds
  to make it eligible.
- Non-blocking for the rediscovery cadence: suspend and resume end the
  session through the service's sampling-gap release (S10), whatever
  the clock does. Whether the monotonic clock advances during suspend
  therefore does not affect the rule.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-06-10 | Initial version |
| 2 | 2026-06-11 | Corrected `Access_ISABUS.HTP.Method` ownership: the LpcIO module documents caller-held acquisition and takes no mutex itself. Still Draft — Phase 3/4 datasheet pinning outstanding. |
| 3 | 2026-06-27 | Narrowed the ready scope to the Phase 2 raw chip-id diagnostic used by PR #1732; removed hardware-monitor base discovery and decode facts from the implementation-ready surface; added scoped enablement, bounded Phase 2 procedure, non-blocking open-question annotations, and status → Implementation-ready. |
| 4 | 2026-10-25 | Read-only safety amendment: added lead-only sources S5–S8 (ADR 0023) and lead-only facts F1–F8 on the ITE secondary-pair embedded controllers (shared ID `0x8733` for IT8792E/IT8795E, `0x8790` IT8790E, `0x8695` IT87952E, `0x8883` re-enter report, Gigabyte `0x2E`/`0x4E` placement convention, LPC-error/hang hazard on exiting configuration mode); added the normative "Slot-1 ITE exit withhold rule" and its Experimental scoped-enablement row; the raw chip-id row keeps its rev 3 status. Pinned S2 pages; added S2 facts that register `0x02` bit 0 is a logical-device reset (never written) and that the fourth ITE key byte selects the configuration pair (key written only to its own port). Added six non-blocking open questions. Status remains Implementation-ready (rev 4). |
| 5 | 2026-11-28 | Added the sampling-path rediscovery cadence for [#2821](https://github.com/shm11C3/HardwareVisualizer/issues/2821). Its sampling-path timing policy is product policy from the project sources S9 and S10 and rests on no hardware or lead-only fact. Its slot-1 cleanup rule applies the existing exit-withhold set, so that part depends on the same lead-only facts F1–F3, F5, F6 and F8 as the withhold row. It defines eligible and not-eligible discovery failure classes with a shared table. Rules R1–R8 cover the following. A discovery attempt is a separate mutex-guarded transaction and never runs inside a sample transaction, and none runs while a path is cached. After an eligible failure the backoff is `min(2 s × 2^(n−1), 30 s)`. A not-eligible failure starts no further attempt from the same sampler. A recreated sampler inherits the backoff. The count resets at the first successful sample or at a new session. A discarded path counts as a failure. The ITE 1.5 s sample minimum is independent. An unconfirmed exit on any slot takes precedence over every other outcome: the attempt stops probing, caches no path and ends eligible, and records the slot as owed cleanup. Every later attempt, including the first of a new session, cleans up each owed slot with entry plus exit before the normal ordered scan. It caches no path until that cleanup succeeds. A slot-1 cleanup reads the chip ID before any exit. A withhold-set ID issues no exit, and the cleanup counts as complete. A failed ID read issues no exit, keeps the cleanup owed and ends the attempt eligible. With no failure in the session, for example after a successful path is invalidated, a request that is not due reports a defined *rediscovery pending* unavailable result. Added the reasoning for the values, fixtures, a scoped-enablement row and three non-blocking open questions. The Phase 2 diagnostic, the slot-1 exit withhold and every fact of revision 4 are unchanged. Status remains Implementation-ready (rev 5). |
