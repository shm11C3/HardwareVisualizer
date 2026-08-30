# Spec: ITE IT8728F/EX Super I/O Environment Controller temperatures

| Field | Value |
| --- | --- |
| Revision | 2 |
| Status | Implementation-ready (rev 2) |
| Scope | Phase 4 read-only motherboard-temperature access for exact raw chip ID `0x8728` / IT8728F/EX through the normal Environment Controller I/O block. Revision 2 enables only generic `TMPIN1`-`TMPIN3` byte temperatures as Experimental. It excludes every other IT86xx/IT87xx ID, fan RPM, voltages, FAN4/5 high-index access, PWM/fan control, limits, alarm/status reads, GPIO, board-specific physical labels, UI work, and Rust/TypeScript implementation. |
| Issue phase | Phase 4 (#1635) - ITE Super I/O temperature specification |

Revision 2 is implementation-ready for a deliberately narrow Experimental
scope. The IT8728F register path is defined by a vendor-authored primary
document, but no matching independent hardware dump is available yet. Per
[ADR 0011](../../adr/0011-experimental-sensor-enablement.md), a successful
read uses the existing presentation contract without an Experimental badge;
only a surfaced failure may identify the attempted path as experimental.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | ITE Tech., Inc., *IT8728F Environment Control - LPC I/O*, Preliminary Specification V0.4.2, E version; public third-party mirror: <https://www.rom.by/files/it8728f_datasheet.pdf>; SHA-256 `5393224C1E497E0F5DED27CF8609D16D3857003F8235C5128E903A1438680C0B` | Vendor-authored primary document. Relevant pins: MB PnP mode §8.1, printed pp. 39-40; global configuration table and chip ID §8.3.3-8.3.5, printed pp. 41, 48; LDN 4 Environment Controller table and base/activation registers §8.8.1-8.8.3, printed pp. 43, 61; Environment Controller LPC access §9.5.1-9.5.2.1, printed p. 87; register table and definitions §9.5.2.2, printed pp. 88-98; temperature format §9.5.3.3, printed pp. 108-109; fan counter formula §9.5.3.5, printed p. 109; update cadence §9.5.3.6, printed p. 110. The host is not an ITE-controlled distribution endpoint and the PDF carries a `CONFIDENTIAL` watermark; the pinned bytes are used as the publicly retrievable vendor document available for this clean-room revision. |
| S2 | ITE Tech., Inc., IT8728 product page, <https://www.ite.com.tw/en/product/cate2/IT8728> (retrieved 2026-08-30) | Official public primary product source. Identifies IT8728F as a Legacy LPC Super I/O with hardware monitoring, three thermal inputs, and up to five fan tachometer inputs. It does not supply a register map. |
| S3 | PawnIO.Modules upstream, `LpcIO` public IOCTL interface at commit `52a7e536dff3e53c96917a28caac5e0fa6510696`, <https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/LpcIO.p>; retrieved 2026-08-30; SHA-256 of the raw `LpcIO.p` bytes `43F0EAE8CBCFBB5D1DD7492C45F343061A8D7C81970AF56C15010ECE2DA1678E` | Upstream-published interface definition for the API HardwareVisualizer calls: slot selection, Super I/O configuration reads/writes, BAR discovery, restricted port I/O, and the caller-held ISA mutex. For a non-absent chip-ID byte, BAR discovery can return success after scanning without proving that it authorized any usable BAR window; subsequent PIO independently denies a port outside the selected configuration pair or discovered 8-byte windows. Therefore success from `ioctl_find_bars` is necessary but not sufficient evidence that the EC ports are usable. It is not used as a hardware-register source, and no implementation code or structure is reproduced here. |
| S4 | [`superio-access.md`](superio-access.md) revision 3 and [`pawnio-interface.md`](pawnio-interface.md) revision 5 | Existing implementation-ready clean-room dependencies for raw ITE chip-ID probing, slot mapping, configuration enter/exit, PawnIO `LpcIO` calls, elevation behavior, and mutex ownership. |
| S5 | [ADR 0011](../../adr/0011-experimental-sensor-enablement.md) and [design principle DP-02](../../design-principles.md#dp-02-represent-partial-capability-honestly) | Project policy source for Experimental enablement, success/failure presentation, plausibility-gated partial readings, manual feedback, and no new telemetry. It supplies no hardware fact. |

No external hardware-monitor implementation, source code, binary,
disassembly, or decompiled tool was consulted for this revision. Normative
hardware facts below rest on S1/S2; PawnIO interoperability facts rest on S3.

## Detection

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| Verified ITE temperature profile | No ITE hardware profile has a matching maintainer-accepted hardware dump in this revision. | None enabled as Verified. |
| Exact raw chip ID `0x8728` / IT8728F/EX, LDN `0x04`, normal Environment Controller base, generic `TMPIN1`-`TMPIN3` | **Experimental**: chip identity, configuration path, base registers, and temperature decode are primary-source pinned by S1/S2, but not yet independently hardware-verified. | Enabled best-effort for read-only, plausibility-gated temperature reads. Successful values use the normal presentation without a badge; an exposed failure may say the path was experimental. |
| FAN1-3 on exact raw chip ID `0x8728` | Register and formula facts exist, but the active divisor, split-counter consistency, and stopped/invalid values cannot be closed safely from S1 alone. | Disabled. No fan register reads or RPM publication in revision 2. |
| FAN4/5, voltages, PWM/control, limits, alarms/status, GPIO, board-specific physical labels | Outside the revision 2 ready scope. | Disabled. |
| Every IT86xx/IT87xx raw chip ID other than exact `0x8728`, including a raw `0x8721` response | **Unsupported**: no compatible exact-chip profile is defined by this document. | Disabled; retain only the existing Phase 2 raw diagnostic result. Do not choose a profile by family resemblance. |

### Chip identity and configuration facts

| Fact | Source |
| --- | --- |
| Probe both standard configuration slots and obtain raw `CR20`/`CR21` bytes using `superio-access.md` revision 3. This document applies only when the combined raw value is exactly `0x8728`. | S4 |
| For the `0x2E`/`0x2F` pair, enter ITE MB PnP mode by writing `0x87`, `0x01`, `0x55`, `0x55` to the index port. For `0x4E`/`0x4F`, the fourth byte is `0xAA`. Exit by selecting configuration register `0x02` and writing `0x02`, which sets its bit 1. | S1 §8.1, printed pp. 39-40; S4 |
| Select a logical device by writing its number to global configuration register `0x07`. The Environment Controller is LDN `0x04`. | S1 Table 8-6 and §8.8, printed pp. 43, 61 |
| Within LDN `0x04`, `CR30` bit 0 is the activation state. Revision 2 reads and requires this bit; it never writes `CR30`. | S1 §8.8.1, printed p. 61 |
| Within LDN `0x04`, `CR60`/`CR61` hold the Environment Controller base. The base is formed from the two bytes with `CR61` bits 2:0 read as zero, so the I/O block is 8-byte aligned. The documented reset default is `0x0290`, but runtime discovery must use the register value rather than the default. | S1 §8.8.2-8.8.3, printed p. 61 |
| S1 is internally inconsistent about the chip-ID low byte: the global table and product identity support `0x28`, while the prose in §8.3.4 says `0x21`. Existing `superio-access.md` rev 3 pins the representative IT8728F raw value as `0x87`/`0x28`. Revision 2 therefore enables only an actually read `0x8728`; it does not reinterpret `0x8721`, repair a byte, or infer a model from the prose discrepancy. | S1 Table 8-1 and §8.3.3-8.3.5, printed pp. 41, 48; S4 |

### Base validity and PawnIO authorization

After reading `CR60`/`CR61`, calculate:

```text
ec_base = ((CR60 & 0x0F) << 8) | (CR61 & 0xF8)
```

The revision 2 path is usable only when all of the following hold:

- LDN `0x04` `CR30` bit 0 is set. (S1 §8.8.1, printed p. 61)
- `CR60` bits 7:4 and `CR61` bits 2:0 are zero and the resulting base
  is 8-byte aligned. (S1 §8.8.2-8.8.3, printed p. 61)
- The result is neither `0x0000` nor the value obtained from raw
  `CR60/61 = 0xFF/0xFF`, and `ec_base + 0x06` fits in the 16-bit I/O
  port space. These are conservative project validity guards, not
  vendor-defined absent sentinels; a rejected value remains available
  as raw diagnostic data and is never replaced with the reset default.
- PawnIO `ioctl_find_bars` succeeds after slot selection, ITE
  configuration entry, exact chip-ID confirmation, LDN selection, and
  base discovery. This return alone does not prove that a usable BAR was
  authorized. (S3)
- After a successful ITE configuration exit and while still holding the
  ISA mutex, an authorization probe writes only ready register index
  `0x00` to `ec_base + 0x05` and reads one byte from
  `ec_base + 0x06`. Both PIO operations must succeed. This read-only EC
  transaction proves that the required index and data ports are accepted
  by the PawnIO allow-list without reading a side-effecting status
  register. If either operation or configuration exit fails, do not
  cache the executor/base state; report the error and require full
  rediscovery before another sample. (S1 §9.5.1 and §9.5.2.2.1,
  printed pp. 87, 91; S3)

## Register map (ready temperature scope)

Revision 2 uses no bank-selection write: every enabled register is in the
documented 7-bit index range. Write an Environment Controller index to
`ec_base + 0x05`, then read the selected byte at `ec_base + 0x06`.
The address port's bit 7 is read-only `Outstanding` and bits 6:0 are the
register index. (S1 §9.5.1-9.5.2.1, printed p. 87)

| Address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `ec_base + 0x05` | EC Address Port | `6:0` | Selects the Environment Controller register read at the data port; bit 7 is read-only `Outstanding`. | 7-bit register index | S1 §9.5.1-9.5.2.1, printed p. 87 |
| `ec_base + 0x06` | EC Data Port | `7:0` | Reads the currently selected Environment Controller register. | Raw byte | S1 §9.5.1, printed p. 87 |
| EC `0x00` | Configuration Register | bits `3`, `0` | Bit 0 `Start`: `1` starts monitoring and `0` selects standby. Bit 3 `INT_Clear`: `1` clears interrupt lines and stops the monitoring loop; the loop resumes after it is cleared. Revision 2 only reads and requires bit 0 = 1 and bit 3 = 0. | Monitoring state | S1 §9.5.2.2.1 and §9.5.3.6, printed pp. 91, 110 |
| EC `0x29` | Temperature Reading Register 1 | `7:0` | Current `TMPIN1` reading. | Signed 8-bit two's complement, 1 degree C/LSB | S1 Table 9-2, §9.5.2.2.24, and §9.5.3.3, printed pp. 89, 97, 108-109 |
| EC `0x2A` | Temperature Reading Register 2 | `7:0` | Current `TMPIN2` reading. | Signed 8-bit two's complement, 1 degree C/LSB | S1 Table 9-2, §9.5.2.2.24, and §9.5.3.3, printed pp. 89, 97, 108-109 |
| EC `0x2B` | Temperature Reading Register 3 | `7:0` | Current `TMPIN3` reading. | Signed 8-bit two's complement, 1 degree C/LSB | S1 Table 9-2, §9.5.2.2.24, and §9.5.3.3, printed pp. 89, 97, 108-109 |
| EC `0x51` | ADC Temperature Channel Enable Register | bits `7:6`, `5:3`, `2:0` | Bits 5/4/3 enable TMPIN3/2/1 in resistor mode and bits 2/1/0 enable TMPIN3/2/1 in diode mode; one TMPIN cannot use both physical modes. Bits 7:6 route an SST/PECI host value to none/TMPIN1/TMPIN2/TMPIN3 for encodings `00`/`01`/`10`/`11`. | Per-channel source/enable flags | S1 §9.5.2.2.30, printed p. 98 |

S1's temperature examples encode `+125` as `0x7D`, `+25` as `0x19`,
`+1` as `0x01`, `0` as `0x00`, `-1` as `0xFF`, `-25` as `0xE7`, and
`-55` as `0xC9`. (S1 §9.5.3.3, printed pp. 108-109)

## Read procedure and decode

Detection and base authorization may be cached on the configured PawnIO
`LpcIO` executor only after the explicit post-exit EC port authorization
probe below succeeds; `ioctl_find_bars` success alone is not cacheable
evidence. Do not enter configuration mode for every sample after that
proof succeeds.

Initial discovery and any rediscovery use this order:

1. Acquire `Global\Access_ISABUS.HTP.Method` with a bounded timeout
   before any slot/configuration or Environment Controller I/O. Abort
   rather than proceeding unlocked. (S3, S4)
2. Select the PawnIO `LpcIO` slot. Enter ITE MB PnP mode with the
   slot-specific sequence. Ensure the matching ITE exit write runs on
   every path after a successful enter attempt. (S1 §8.1; S4)
3. Read raw `CR20`/`CR21`. Continue only for exact `0x87`/`0x28`.
   Mixed, absent, `0x8721`, and every other ID remain diagnostic-only.
4. Select LDN `0x04` through `CR07`; read and require `CR30` bit 0.
   Read `CR60/61`, derive `ec_base`, and apply all base-validity guards.
   Do not write `CR30`, `CR60`, or `CR61`. (S1 §8.8, printed p. 61)
5. While the selected chip and its discovered configuration remain
   valid, call `ioctl_find_bars`; require success before any EC PIO.
   Treat success only as permission to attempt the following port proof,
   not as proof that any usable BAR was found. (S3)
6. Exit ITE configuration mode while retaining the ISA mutex. Require
   the exit write to succeed before attempting normal EC PIO. On every
   earlier error path, make the same exit best effort in `finally` and
   do not cache discovery state. (S1 §8.1; S4)
7. After the successful exit, write EC index `0x00` to
   `ec_base + 0x05` using `ioctl_pio_outb`, then read the selected
   Configuration Register byte from `ec_base + 0x06` using
   `ioctl_pio_inb`. Require both calls to succeed. The index-port write
   is read-transaction plumbing only; the EC data port is not written.
   (S1 §9.5.1 and §9.5.2.2.1, printed pp. 87, 91; S3)
8. Only after step 7 succeeds, cache the exact chip ID, selected slot,
   validated base, and port-proven executor state. Release the ISA mutex
   in `finally`. If either PIO fails, cache nothing, report the exact
   error, and require full rediscovery. Rediscover as well if the
   executor/slot state is later replaced or invalidated.

Each sample uses the cached, port-proven executor and this order:

1. Acquire `Global\Access_ISABUS.HTP.Method` with the same bounded
   timeout. Abort the sample rather than proceeding unlocked. (S3, S4)
2. Select EC `0x00` through `ec_base + 0x05`, read it through
   `ec_base + 0x06`, and require `Start` bit 0 = 1 and `INT_Clear` bit
   3 = 0. Do not start or resume the monitor in software. (S1
   §9.5.2.2.1 and §9.5.3.6, printed pp. 91, 110)
3. Read EC `0x51`. A physical channel is eligible only when its
   documented resistor-mode bit or diode-mode bit is set: TMPIN1 uses
   bit 3 or 0, TMPIN2 bit 4 or 1, and TMPIN3 bit 5 or 2. Omit a channel
   with neither bit set or with both physical-mode bits set. Also omit
   the one register selected as the external SST/PECI report target by
   bits 7:6 (`01` = TMPIN1, `10` = TMPIN2, `11` = TMPIN3); revision 2
   publishes only enabled physical inputs. Do not change the
   configuration. (S1 §9.5.2.2.30, printed p. 98)
4. For each eligible channel at EC `0x29`, `0x2A`, and `0x2B`, write
   only the index to `ec_base + 0x05` and read one byte from
   `ec_base + 0x06`. Decode `raw <= 0x7F` as `raw` degrees C and
   `raw >= 0x80` as `raw - 256` degrees C. Preserve raw bytes in manual
   diagnostics. One failed temperature-register read invalidates that
   input, not every successfully read sibling input.
5. Apply the revision 2 plausibility policy: accept an individual
   decoded temperature only when `-55 <= temperature <= 125` degrees C,
   the range bounded by S1's stated encoding examples. Zero degrees C
   remains a valid numeric sample for an enabled physical channel and
   must not by itself be treated as an absent sentinel. Omit an
   implausible or failed input; do not publish zero, a clamped boundary,
   or a whole-device failure in its place. (S1 §9.5.3.3; S5/DP-02)
6. Release the ISA mutex in `finally`. Failure to read gating register
   `0x00` or `0x51` invalidates the whole sample. A failed temperature
   index write or data read invalidates only that channel. Report each
   failure diagnostically without inventing replacement values.

The Environment Controller takes 1.5 seconds to update all of its
registers safely between completed read operations. Do not sample this
path more often than once every 1.5 seconds; a longer product cadence is
allowed. (S1 §9.5.3.6, printed p. 110)

Experimental success remains a normal available sensor reading. The
implementation must not attach verification metadata, an Experimental
badge, or a source-label suffix to successful values. If an existing
diagnostic surface reports a failed attempt, its detail may identify the
path as experimental. Hardware feedback is collected only through a
user-initiated manual dump; this specification adds no telemetry. (S5)

## Disabled fan facts

S1 documents FAN1-3 count low bytes at EC `0x0D`-`0x0F`, corresponding
extended bytes at `0x18`-`0x1A`, a 22.5 kHz counter, two pulses per
revolution, and the formula `RPM = 1,350,000 / (Count * Divisor)` with
default divisor 2. (S1 Table 9-2, §9.5.2.2.14, §9.5.2.2.19, and
§9.5.3.5, printed pp. 88, 94, 96, 109)

Those facts are **not** a revision 2 implementation surface. S1 does not
close all of the following for a safe current-value decode:

- where to read the active FAN1-3 divisor rather than assuming the
  documented default;
- whether or how the split high/low counter bytes are latched for a
  consistent sample; and
- a stopped, disconnected, overflow, or otherwise invalid counter
  encoding.

Consequently revision 2 performs no FAN1-3 register reads and publishes
no ITE fan RPM. FAN4/5 registers are also excluded: S1 places them at EC
indexes `0x80`-`0x83`, while the same document defines this LPC address
port's writable index field as bits 6:0. Revision 2 does not invent a
high-index mechanism.

## Quirks

- S1's `0x28` versus `0x21` chip-ID low-byte contradiction is contained
  by exact raw-ID enablement. A real `0x8721` responder is Unsupported,
  not treated as IT8728F. (S1, S4)
- The product can route or configure temperature inputs in board-specific
  ways. Revision 2 exposes only generic `TMPIN1`-`TMPIN3` names and does
  not infer CPU, system, auxiliary, socket, or motherboard placement from
  register order. (S1 §9.5.2.2; S2)
- The datasheet's reset base `0x0290` is not a fallback. Any unusable
  discovered base makes the path unavailable. (S1 §8.8.2-8.8.3)
- `ioctl_find_bars` success is not a positive count or a guarantee that
  EC PIO is authorized. Revision 2 requires the post-exit `0x00`
  read-transaction proof before caching the path. (S3)

## Safety notes

- Required writes are limited to ITE configuration enter/exit, LDN
  selection through `CR07`, and EC register-index selection at
  `ec_base + 0x05`.
- Do not write `CR30`, `CR60`, `CR61`, EC `0x00`, any EC data register,
  monitoring-start state, temperature configuration, interrupt/alarm
  state, fan-control/PWM, divisor, threshold/limit, GPIO, or power state.
- Do not read EC interrupt-status registers `0x01`-`0x03`; S1 documents
  read-to-clear behavior for this group. (S1 §9.5.2.2.2-9.5.2.2.4,
  printed p. 92)
- Hold the ISA mutex across the complete multi-step transaction. PawnIO
  does not acquire it for the caller. Abort on timeout or port/IOCTL
  failure; never retry through unrestricted raw I/O. (S3, S4)
- PawnIO permission or component failures remain normal availability
  failures. Experimental classification alone must not trigger PawnIO
  installation/elevation guidance. (S5)

## Open questions

- Non-blocking for Phase 4: No accepted IT8728F hardware dump exists;
  revision 2 intentionally classifies the primary-source-defined,
  read-only temperature path as Experimental and can graduate it to
  Verified after a maintainer-accepted manual dump confirms ID, base,
  EC state, and plausible changing temperatures.
- Non-blocking for Phase 4: S1 conflicts internally on chip-ID low byte;
  exact `0x8728` enablement follows the formal table and existing ready
  Phase 2 spec, while `0x8721` remains Unsupported rather than guessed.
- Non-blocking for Phase 4: FAN1-3 divisor state, split-counter
  consistency, and stopped/invalid encodings remain unresolved; fan
  reads and RPM publication are explicitly disabled.
- Non-blocking for Phase 4: FAN4/5 high-index access remains unresolved;
  those registers are excluded and never selected.
- Non-blocking for Phase 4: Board-specific physical temperature labels
  remain unknown; the ready output is deliberately generic
  `TMPIN1`-`TMPIN3`.

## Manual feedback dump for Experimental -> Verified

A user-initiated dump should capture the following under Administrator
PowerShell without changing hardware state:

1. Board model, BIOS version, Windows version, PawnIO version, and the
   SHA-256 of the signed `LpcIO.bin` used.
2. Under one bounded ISA-mutex transaction per slot, capture raw ITE
   `CR20/21/22`; for exact `0x8728`, LDN `0x04` `CR30/60/61`; every
   `ioctl_find_bars` result; the guaranteed ITE exit outcome; and the
   post-exit EC `0x00` index-write/data-read authorization-probe results.
3. For a valid active base only, capture raw EC `0x00`, `0x51`, and
   `0x29`-`0x2B` in three snapshots separated by at least 1.5 seconds,
   acquiring and releasing the ISA mutex for each snapshot. Do not read
   status `0x01`-`0x03`, fan registers, or any register at/above
   `0x80`; do not write the EC data port.
4. A same-session BIOS hardware-monitor temperature capture when
   practical, retaining generic TMPIN labels unless the board manual
   supplies an explicit mapping.

Submission is manual and opt-in. Do not add automatic upload, outbound
telemetry, or persistent portable hardware identity.

## Implementation-ready transition checklist

Revision 2 satisfies the directory-level ready gate for the scoped
Experimental temperature path:

- no unresolved provenance marker remains;
- all enabled hardware facts are pinned to S1/S2, and PawnIO interface
  ordering plus the required post-exit EC port proof is pinned to S3;
- no normative fact rests on an external monitor implementation;
- unverified or excluded scopes are disabled in Scoped enablement;
- every remaining open question is annotated as non-blocking for Phase
  4; and
- revision, status, and revision history record the ready boundary.

## Provenance text for a future clean-room implementation PR

```text
Implemented from docs/specs/sensors/superio-access.md revision 3 and docs/specs/sensors/pawnio-interface.md revision 5 (commit <pin>), limited to existing ITE configuration access and PawnIO LpcIO interface facts.
Implemented from docs/specs/sensors/superio-ite-it86xx-it87xx.md revision 2 (commit <ready-commit>), limited to the exact 0x8728 / IT8728F/EX Experimental TMPIN1-TMPIN3 scope marked enabled in that revision.
No other external sensor documentation was used.
```

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-08-30 | Initial implementation-ready Phase 4 revision: exact `0x8728` / IT8728F/EX only, Experimental read-only TMPIN1-3 with primary-source-pinned discovery/decode and conservative validity handling; all fans and other IT86xx/IT87xx scopes disabled pending later evidence. |
| 2 | 2026-08-30 | Required a successful post-configuration-exit EC `0x00` index-write/data-read authorization probe before caching the PawnIO executor/base, because `ioctl_find_bars` success alone does not guarantee that a usable BAR window was authorized. Status remains Implementation-ready for the same Experimental temperature-only scope. |
