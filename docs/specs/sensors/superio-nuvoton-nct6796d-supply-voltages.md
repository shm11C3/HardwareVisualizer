# Spec: Nuvoton NCT6796D internally divided supply voltages and #2534 voltage dispositions

| Field | Value |
| --- | --- |
| Revision | 2 |
| Status | Implementation-ready (rev 2) |
| Scope | Windows PawnIO `LpcIO` read-only voltages of the four Super I/O supply pins whose divider is inside the chip (`AVSB`, `3VCC`, `3VSB`, `VBAT`). Exact raw chip ID `0xD421` (NCT6796D / NCT6796D-E) only, as Experimental. Every fact comes from the vendor datasheet. Values are the voltage at the chip pin. They are labelled with the vendor pin mnemonic and never with a motherboard rail name. Recorded but disabled: `CPUVCORE`, `VIN0`–`VIN8`, `VTT`, every other Nuvoton ID, and every ITE, Fintek, EC and Linux hwmon voltage route (see [Disposition of #2534 targets](#disposition-of-2534-targets)). Excludes limits, alarms, voltage or limit writes, calibration, fault diagnosis, board-specific resistor scaling and rail naming. |
| Issue phase | [#2534](https://github.com/shm11C3/HardwareVisualizer/issues/2534) (epic [#2510](https://github.com/shm11C3/HardwareVisualizer/issues/2510)) |

Revision 1 is the first implementable voltage slice of #2534. Most
hardware-monitor voltage inputs measure a signal that the motherboard
first scales with its own resistor network, on a net that the board
chooses. A value from such an input names no rail and has no known scale
without board evidence. This document enables only the inputs for which
the datasheet defines the measured net and the scale independently of
the board. The measured net is one of the chip's own supply pins. The
scale is the chip's integrated divider. The slice extends the existing
`0xD421` Experimental transaction of
[`superio-nuvoton-nct67xx.md`](superio-nuvoton-nct67xx.md) (S2) by two
bank 0 gate reads and four bank 4 value reads. It does not change any
temperature or fan behavior of that document.

Revision 2 adds the [freshness rule](#freshness-rule). `0x88` can hold a
byte that is not yet updated for one monitor cycle after the battery
monitor is enabled, and a read-only sampler cannot see an enable that
happened before its first sample. A voltage is therefore published only
after its gates have been observed satisfied for at least 704 ms across
samples. The same rule covers the restart of monitoring for all four
inputs.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Nuvoton, *NCT6796D LPC/eSPI SI/O Datasheet*, version 0.6, publication release date July 6, 2017; official PDF `https://www.nuvoton.com/resource-files/NCT6796D_Datasheet_V0_6.pdf`, fetched 2026-11-28, SHA-256 `94466053308082305a9d656211aa6b9e82b78c3116a2b466c1f27213e897506d`. Printed page = PDF page − 16. | Primary. The same document is S1 of S2. Pins (printed pages): power pins §5.13 p. 17; hardware-monitor overview (fifteen analog voltage inputs, six of them internal) §8 p. 54; analog inputs §8.6 p. 58; internal and external dividers §8.6.1 p. 59; voltage data format §8.6.2 p. 59; voltage reading locations §8.6.2.1 p. 59; ADC conversion and cycle time §8.6.4 p. 62; Configuration Register bank 0 index `0x40` §9.20 pp. 105–106; read-clear Interrupt Status registers §9.21 p. 106, §9.22 p. 107, §9.25 p. 108, §9.191 p. 168, §9.241 p. 186, §9.452 p. 261, §9.453 p. 262; Bank Select Register index `0x4E` §9.30 p. 111; VBAT Monitor Control Register bank 0 index `0x5D` §9.34 p. 113; bank 4 Value RAM `0x80`–`0x96` §9.208 pp. 175–176; global chip ID p. 369. |
| S2 | [`superio-nuvoton-nct67xx.md`](superio-nuvoton-nct67xx.md) revision 10 | Clean-room source for the exact `0xD421` profile: the split between the discovery attempt (steps 1–7 and 11) and the sample transaction on the cached path (steps 8–10), chip identity, the read-only `CR28` bit 4 lock gate, LDN B activation and base discovery, `ioctl_find_bars` ordering, normal-HM index/data ports, bank 4 selection, the abort rule, the input identity and ordering rule, and configuration-mode exit. This document adds to that transaction and changes none of it. |
| S3 | [`superio-access.md`](superio-access.md) revision 5 (its revision 4 transport facts unchanged) and [`pawnio-interface.md`](pawnio-interface.md) revision 9, `LpcIO` section | Clean-room transport sources: Nuvoton configuration keys, slot mapping, PawnIO `LpcIO` calls and the caller-held `Global\Access_ISABUS.HTP.Method` mutex. |
| S4 | [ADR 0011](../../adr/0011-experimental-sensor-enablement.md), [ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md), [ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md) | Project policy: Experimental enablement with plausibility gates; lead-only facts confined to Experimental rows; runtime gates limited to mechanism and safety gates. No hardware facts. |
| S5 | Linux kernel `Documentation/ABI/testing/sysfs-class-hwmon` at tag `v6.18`, `https://raw.githubusercontent.com/torvalds/linux/v6.18/Documentation/ABI/testing/sysfs-class-hwmon`, fetched 2026-11-29, SHA-256 `dde265e65b9308a0f1a2a4bfa34389d966313ec7b94a453aaf4c923ff8922888`; and `Documentation/hwmon/sysfs-interface.rst` at the same tag, SHA-256 `97304e541af2980bb2d963a1333e14fc04844be1f62376295c1f63a68f73c750` | Upstream-published definition of the sysfs ABI that the project's Linux hwmon paths read (primary for ABI semantics, as in [`superio-ite-trx50-secondary-linux-hwmon.md`](superio-ite-trx50-secondary-linux-hwmon.md)). `inY_input` is a read-only voltage in millivolts, measured on the chip pin. The actual voltage depends on board scaling resistors that vary by chip and motherboard. Drivers generally do not apply that scaling; some apply a chip's internal dividers. `inY_label` is a suggested label that a driver creates only when it has a hint about what the channel is used for. The interface document states that conversions for board resistors cannot be hard-coded in a driver and belong to user space. Used only for the Linux dispositions; no driver source was read for them. |
| S6 | Nuvoton, *NCT6776F / NCT6776D Nuvoton LPC I/O* datasheet, Revision 1.2, July 12, 2011, the copy pinned as S1 of [`superio-nuvoton-nct6775-nct6776-nct610x.md`](superio-nuvoton-nct6775-nct6776-nct610x.md) (SHA-256 `d5eec9d9bdc1212c619fd36995a2f1d964482079fbbf7aa55311a50ebd6e90fb`) | Primary. Disposition only: §8.5 p. 59 and §§8.5.1–8.5.2.1 p. 60. |
| S7 | Fintek, *F71882 Super Hardware Monitor + LPC I/O* datasheet V0.29P, July 2009, the copy pinned as S1 of [`superio-fintek-f71882-f71889.md`](superio-fintek-f71882-f71889.md) (SHA-256 `d70fc91e4284496a0834b0dc49653b0197965b4a1293a2060b4efe5680ffa4a1`); printed page = PDF page − 6 | Primary. Disposition only: §7.5 p. 36; §8.6.2.16 pp. 72–73. |
| S8 | Fintek, *F71889 Super Hardware Monitor + LPC I/O* datasheet V0.28P, April 2010, the copy pinned as S2 of the same Fintek document (SHA-256 `4628125a331da06a29e9750eb2f0508e32fb39923d6b3a9315c90b563ea1df66`); printed page = PDF page − 7 | Primary. Disposition only: §7.2 p. 16. |
| S9 | ITE Tech., Inc., *IT8728F Environment Control - LPC I/O* Preliminary Specification V0.4.2, the copy pinned as S1 of [`superio-ite-it86xx-it87xx.md`](superio-ite-it86xx-it87xx.md) (SHA-256 `5393224C1E497E0F5DED27CF8609D16D3857003F8235C5128E903A1438680C0B`); printed page = PDF page − 18 | Primary. Disposition only: EC Configuration Register index `0x00` §9.5.2.2.1 p. 91; voltage reading registers §§9.5.2.2.21–9.5.2.2.23 pp. 96–97; ADC range p. 108. |
| S10 | ITE Tech., Inc., *IT8716F Environment Control - LPC I/O* Preliminary Specification V0.3, the mirror pinned as S8 of [`superio-ite-primary-lpc-expansion.md`](superio-ite-primary-lpc-expansion.md), fetched 2026-11-28, SHA-256 `37c37afa3607d610b4a8ba6e4a4e44f0b66fc9d54a6edfa1bee9a64f922479ac` | Primary. Disposition only: global `CR2C` §8.3.15 p. 46. |
| S11 | ITE Tech., Inc., *IT8718F* Preliminary Specification V0.3, the copy pinned as S22 of the same ITE document (SHA-256 `838038b0218eabba54deeb57432126e27e72f7d4321ae7e21e3c54e449201a80`); printed page = PDF page − 18 | Primary. Disposition only: global `CR2C` §8.3.15 p. 43. |
| S12 | This repository at commit `50035f40`: `hardviz/core/src/models/metrics.rs` | Repository fact: one round of motherboard readings carries a list of temperatures and a list of fan speeds and no voltage kind. Used only for the model requirement. |
| S13 | Repository specifications for the other motherboard routes, at the revisions in the [Current documents](README.md#current-documents) table on commit `50035f40`. | Repository facts about what each existing route reads, excludes or leaves draft. Cited per disposition row. |
| S14 | LibreHardwareMonitor tree listing of `LibreHardwareMonitorLib/Hardware/Motherboard/` at `2808d80564b6761e1b01f42fabfd966ed695fcbc` (the #2534 comparison inventory), retrieved 2026-11-29 through the GitHub tree API, MPL-2.0 | **lead-only (copyleft)**. File list only, used to enumerate the comparison inventory's voltage-bearing groups: Super I/O drivers for ITE, Nuvoton, Fintek and Winbond parts; embedded-controller readers including ChromeOS; an IPMI reader; a Linux hwmon reader; Gigabyte controller access; per-board voltage descriptors and board identification. No file content was read, and no fact in this document rests on S14. |

| S15 | [`superio-access.md`](superio-access.md) revision 5, [Terms](superio-access.md#terms) and [Rules](superio-access.md#rules) of the sampling-path rediscovery cadence | Project policy used by the read procedure and the freshness rule: the terms discovery attempt, cached path, discarded path, sample transaction and monitoring session, and rule R1 (a discovery attempt is its own mutex-guarded transaction, never runs inside a sample transaction, and never runs while a path is cached) with the backoff rules R2–R8. S15 states that a monitoring session ends after a gap between samples and that this covers suspend and resume. This document relies on that only for a suspend long enough to cause the gap (see [Freshness rule](#freshness-rule)). The service requests a motherboard reading at most every 1.6 s. Supplies no hardware fact. S3 stays the pin for the transport facts. |

S1 is the only source of any enabled fact. S15 supplies only the
sampler lifetime terms of the freshness rule. S5–S13 support dispositions
only. S14 is a copyleft lead listed for the inventory. No code, code
structure or implementation identifier from any monitoring
implementation is reproduced.

## Validation outcome

Revision 2 keeps the implementation-ready proposal of revision 1 for one
Experimental scope: the four internally divided supply pins on exact raw
ID `0xD421`.

- Measured net. S1 §5.13 p. 17 defines `AVSB`, `3VCC`, `3VSB` and `VBAT`
  as chip power pins. S1 §8.6 p. 58 names them, with `CPUVCORE` and
  `VTT`, as the six internal signals connected to the power supplies.
  The reading is therefore the voltage at a chip pin whose function the
  vendor defines. It does not depend on which board net the
  manufacturer connects.
- Scale. S1 §8.6.1 p. 59 states that `AVSB`, `VBAT`, `3VSB` and `3VCC`
  each pass through an integrated divider of two equal 34 kΩ resistors,
  so the ADC sees one half of the pin voltage. S1 §8.6.2 p. 59 gives
  8 mV per code. No board resistor enters this scale.
- Location. S1 §8.6.2.1 p. 59 and §9.208 p. 175 place the four readings
  at bank 4 `0x82`, `0x83`, `0x87` and `0x88`.
- Gates. S1 §9.20 p. 106 defines the monitor start and stop bits. S1
  §9.34 p. 113 defines the battery-monitor enable bit and its one-cycle
  update delay. Both are read, never written, and the freshness rule
  withholds values until the gates have held for 704 ms across samples.
- Transport. The `0xD421` chip identity, lock gate, discovery, port and
  bank mechanism come from S2, which is Implementation-ready at
  revision 10.
- Evidence state. No hardware dump of these registers exists for any
  `0xD421` board. The scope is Experimental under ADR 0011 and depends on
  no lead-only fact (S4).

## Detection

### Runtime gates

| Condition | Gate kind | Source |
| --- | --- | --- |
| Raw chip ID `0xD421`, with every gate S2 applies to that profile: `CR28` bit 4 clear, LDN B `CR30` bit 0 set, a valid `CR60/61` base, and `ioctl_find_bars` after discovery | mechanism (chip identity and register map) and safety (HM I/O-space lock), as stated in S2 | S1 p. 369; S2 |
| The S2 `0xD421` temperature and fan register set of the same sample was read without failure | mechanism: the voltage reads continue an established normal-HM session and use its bank mechanism | S2 |
| Bank 0 index `0x40` bit 0 (`START`) is `1` | mechanism: `0` puts the hardware monitor in standby, so the value RAM is not being converted | S1 §9.20 p. 106 |
| Bank 0 index `0x40` bit 3 (`INT_Clear`) is `0` | mechanism: `1` stops monitoring until the bit is cleared | S1 §9.20 p. 106 |
| For `VBAT` only: bank 0 index `0x5D` bit 0 (`EN_VBAT_MNT`) is `1` | mechanism: `0` disables the battery-voltage monitor | S1 §9.34 p. 113 |
| The input's gate run has lasted at least 704 ms ([Freshness rule](#freshness-rule)) | mechanism: the `VBAT` reading updates one monitor cycle after `EN_VBAT_MNT` changes from `0` to `1`, and the cycle is about 352 ms | S1 §9.34 p. 113, §8.6.4 p. 62; S15 |

Board strings, BIOS versions and board vendor are not runtime conditions
for this scope. The measured net and scale are chip facts, so no board
identity selects a mechanism here (S4, ADR 0031).

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| `0xD421` `AVSB`, `3VCC`, `3VSB`, `VBAT` supply-pin voltages | **Experimental.** Measured net, divider, data format, register locations and gates all come from S1. There are no lead-only facts. No observed member: no `0xD421` board has a dump of bank 4 `0x80`–`0x8E` or bank 0 `0x40`/`0x5D`. | Enabled as Experimental after the S2 `0xD421` sample, behind the gates above. |
| `0xD421` `CPUVCORE` (bank 4 `0x80`) | Candidate. S1 §8.6.1 p. 59 states that the pin reaches the ADC with no internal divider, so the pin voltage is defined. Which board net drives the pin, and through what network, is a board fact. | Disabled. |
| `0xD421` `VIN0`–`VIN8` (bank 4 `0x81`, `0x84`–`0x86`, `0x8A`–`0x8E`) | Unsupported. S1 §8.6.1 p. 59 requires an external divider for any input above 2.048 V; resistor values and the connected net are board facts. S1 §9.208 pp. 175–176 lists no value register for `VIN9`. | Disabled. |
| `0xD421` `VTT` (bank 4 `0x89`) | Unsupported. S1 §5.13 p. 17 describes the pin as Intel CPU VTT power. S1 §8.6.1 p. 59 lists no integrated divider for it and states no direct path either (see Open questions). | Disabled. |
| Every other raw chip ID, chip family or route | See [Disposition of #2534 targets](#disposition-of-2534-targets). | Disabled. |

## Register map (facts)

### Bank selection (read-transaction plumbing)

| Address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| HM index `0x4E` | Bank Select Register | `3:0` | Bank number for the following banked reads. Data `0x00` selects bank 0; data `0x04` selects bank 4. | Binary bank number | S1 §9.30 p. 111; S2 |
| HM index `0x4E` | `HBACS` | `7` | Selects the high or low byte of the vendor-ID register at index `0x4F`. That register is not read here, so the bit has no effect on this scope. Both bank values above write `0`, as the existing S2 bank 4 write does. | Boolean | S1 §9.30 p. 111 |

### Gate registers (bank 0, read only by this scope)

| Address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| Bank 0 `0x40` | Configuration Register, `START` | `0` | `1` starts monitoring; `0` puts the part in standby. Register default `0x03`. | Boolean | S1 §9.20 pp. 105–106 |
| Bank 0 `0x40` | `INT_Clear` | `3` | `1` disables the SMI# output and stops monitoring until the bit is cleared. | Boolean | S1 §9.20 p. 106 |
| Bank 0 `0x40` | `Initialization` | `7` | Writing `1` restores power-on defaults to some registers; the bit clears itself. Never written by this scope. | Self-clearing control | S1 §9.20 p. 106 |
| Bank 0 `0x5D` | VBAT Monitor Control, `EN_VBAT_MNT` | `0` | `1` enables the battery-voltage monitor; `0` disables it. After a change from `0` to `1`, it takes one monitor cycle to update the VBAT reading value register. Register default `0x04` (bit 0 clear), reset by `PWROK` or by `LRESET#` when LDN A `CR E7` bit 0 is `1`. Bits `7:1` select temperature sensor types and are not interpreted here. | Boolean | S1 §9.34 p. 113 |

### Voltage value registers (bank 4)

| Bank | Register | Vendor input | Pin function (S1 §5.7 pp. 12–13 for analog inputs, §5.13 p. 17 for power pins) | Internal divider | Read by this scope | Source |
| --- | --- | --- | --- | --- | --- | --- |
| `0x04` | `0x80` | `CPUVCORE` | Analog input for voltage measurement, 0 to 2.048 V | None | No | S1 pp. 12–13; §8.6.1, §8.6.2.1 p. 59; §9.208 p. 175 |
| `0x04` | `0x81` | `VIN1` | Analog input for voltage measurement, 0 to 2.048 V | External only | No | S1 pp. 12–13, 59, 175 |
| `0x04` | `0x82` | `AVSB` | Analog +3.3 V power input that supplies all analog circuits | 34 kΩ / 34 kΩ (one half) | **Yes** | S1 pp. 17, 59, 175 |
| `0x04` | `0x83` | `3VCC` | +3.3 V power supply for driving 3 V on the host interface | 34 kΩ / 34 kΩ (one half) | **Yes** | S1 pp. 17, 59, 175 |
| `0x04` | `0x84` | `VIN0` | Analog input for voltage measurement, 0 to 2.048 V | External only | No | S1 pp. 12–13, 59, 175 |
| `0x04` | `0x85` | `VIN8` | Analog input for voltage measurement, 0 to 2.048 V | External only | No | S1 pp. 12–13, 59, 175 |
| `0x04` | `0x86` | `VIN4` | Analog input for voltage measurement, 0 to 2.048 V | External only | No | S1 pp. 12–13, 59, 175 |
| `0x04` | `0x87` | `3VSB` | +3.3 V standby power supply for the digital circuits | 34 kΩ / 34 kΩ (one half) | **Yes** | S1 pp. 17, 59, 175 |
| `0x04` | `0x88` | `VBAT` | +3 V on-board battery for the digital circuits | 34 kΩ / 34 kΩ (one half) | **Yes**, when `EN_VBAT_MNT` is `1` | S1 pp. 17, 59, 113, 175 |
| `0x04` | `0x89` | `VTT` | Intel CPU VTT power | Not stated | No | S1 pp. 17, 58–59, 175 |
| `0x04` | `0x8A`–`0x8E` | `VIN5`, `VIN6`, `VIN2`, `VIN3`, `VIN7` | Analog inputs for voltage measurement, 0 to 2.048 V | External only | No | S1 pp. 12–13, 59, 175–176 |
| `0x04` | `0x8F` | Reserved | — | — | No | S1 p. 176 |

Every row above is a Value RAM byte. S1 does not give any of them a
read side effect. The read-clear registers of the chip are listed in
[Safety notes](#safety-notes).

### Conversion facts

| Fact | Source |
| --- | --- |
| The ADC is 8 bits with 8 mV per code, so the input range is 0 to 2.048 V (256 × 8 mV). Every input must be held below 2.048 V by a divider; the power-supply inputs have internal resistors and the external pins need external ones. | S1 §8.6 p. 58 |
| `AVSB`, `VBAT`, `3VSB` and `3VCC` each use an integrated divider of two 34 kΩ resistors, giving one half of the supply. For a 3.3 V supply the ADC input is 1.65 V; for the worst case of 10 % above, 3.63 V, the ADC input is 1.815 V. | S1 §8.6.1 p. 59 |
| The `CPUVCORE` pin feeds the ADC directly with no divider. | S1 §8.6.1 p. 59 |
| Detected voltage = reading × 0.008 V. When the source was reduced by a divider, the detected voltage must be scaled accordingly. | S1 §8.6.2 p. 59 |
| Each channel conversion takes about 2 ms. With 15 voltage channels, 7 temperature channels and 8-fold averaging, the stated update period is about 352 ms. | S1 §8.6.4 p. 62 |

## Read procedure and decode

### Sampling

S2 separates two transactions, and this scope keeps that split (S2
[Discovery procedure](superio-nuvoton-nct67xx.md#discovery-procedure);
S15 rule R1).

- **Discovery attempt.** S2 steps 1–7 and 11, unchanged: mutex,
  configuration entry, chip ID, `CR28` lock gate, LDN B, base,
  `ioctl_find_bars`, configuration exit and mutex release. It runs only
  while no path is cached, as its own ISA-mutex transaction under the
  S15 rediscovery cadence. This scope adds no read to it and reads no
  voltage register in it.
- **Sample transaction.** Every sample on the cached `0xD421` path is
  one ISA-mutex transaction with no configuration-mode entry key or exit
  write. It runs the S2 sample steps and then the voltage steps below.
  No discovery attempt runs while the path stays cached, so ordinary
  samples follow one another on the same path.

Steps of one sample transaction:

1. Acquire the ISA mutex as S2 step 1 requires, then run S2 steps 8–9
   for `0xD421` unchanged: normal-HM ports, bank 4 selection, and the
   bank 4 temperature and fan reads with the S2 abort rule. If S2
   aborts the sample, no voltage is read and all four voltages are
   unavailable for that sample. (S2)
2. Select bank 0: write index `0x4E`, then data `0x00`. (S1 p. 111)
3. Read bank 0 index `0x40`. If bit 0 is `0` or bit 3 is `1`, mark all
   four voltages unavailable (monitoring stopped) and go to step 6.
   (S1 p. 106)
4. Read bank 0 index `0x5D` and keep bit 0. (S1 p. 113)
5. Select bank 4: write index `0x4E`, then data `0x04`. Read `0x82`,
   `0x83` and `0x87`. Read `0x88` only when the step 4 bit was `1`;
   otherwise mark `VBAT` unavailable (battery monitor disabled) without
   reading it. Apply the [freshness rule](#freshness-rule) to every byte
   read, then go to step 7. (S1 pp. 59, 113, 175)
6. Select bank 4 (write index `0x4E`, then data `0x04`), so that the
   transaction ends in the same bank state as the S2 sample. (S2)
7. Release the ISA mutex. The sample transaction issues no
   configuration-mode exit, because it entered none. (S2 step 11; S15
   R1)

Failure rule. If any index write or data read in steps 2–6 fails, stop
the voltage steps. Every byte already read successfully in that sample
still goes through the normal decode rules (including the raw `0x00` and
`0xFF` rule), whether the implementation decodes it before the next read
or from a buffer afterwards. Additionally mark every voltage whose byte was
not read successfully as unavailable and identify the failed read as experimental in the
diagnostic detail (S4). Attempt the step 6 bank 4 selection once, unless
the failed operation was that selection, and then continue with step 7.
The temperatures and fans that step 1 already read stay valid for that
sample. A voltage failure does not fail the device or the other inputs.

The gate bits are read on every sample. S1 gives both registers the
Read/Write attribute (pp. 105, 113), so firmware can change them at run
time.

### Freshness rule

S1 §9.34 p. 113 states that after `EN_VBAT_MNT` changes from `0` to `1`,
it takes one monitor cycle to update the VBAT reading value register.
S1 §8.6.4 p. 62 gives that cycle as about 2 ms per channel ×
(15 voltage + 7 temperature channels) × 8 averages = 352 ms. S1 does not define what
`0x88` holds before that update, so the byte can be any value, including
a nonzero one that decodes. A read-only sampler sees a gate bit only
when it reads it. It cannot tell how long before its first observation
the bit became `1`, and it never writes the bit to force a known state.
S1 states no update time after `START` returns to `1` or `INT_Clear`
returns to `0`. The scope applies the same rule to that gate instead of
assuming that the value RAM is current when monitoring restarts.

The rule is scope policy built on those S1 facts. It needs no write and
no wait.

- **Runs.** The sampler keeps two runs for its cached path (S15): a
  monitor run and a battery run. A sample extends the monitor run when
  its step 3 read succeeded with `START` = `1` and `INT_Clear` = `0`. A
  sample extends the battery run when, in addition, its step 4 read
  succeeded with bit 0 = `1`. Every other sample ends the run it does
  not extend: an S2 abort, a failed gate read, or a gate read with the
  gate not satisfied. A failed value read in step 5 does not end a run,
  because the gates of that sample were observed.
- **Other run ends.** Both runs also end when the cached path is
  discarded, when the monitoring session ends (S15), and when the gate
  reads of two consecutive samples are more than 5 s apart on the
  sampler's monotonic clock. A discovery attempt runs only while no path
  is cached (S15 R1). It therefore never falls between two samples of a
  run: the runs of the old path ended when that path was discarded, and
  the path the attempt caches starts with no runs. Ordinary sample
  transactions on a cached path never end a run by themselves. A `PWROK` reset
  returns `0x5D` bit 0 to `0` (S1 p. 113), so a bit read as `1` after
  power returns can come from a new enable. The gap rule and the session
  end catch a suspend and resume only when they leave more than 5 s
  between gate reads, or end the session. The repository has no
  suspend or resume signal that the sampler could observe, so a shorter
  suspend and resume is not detected and is the residual case in Open
  questions.
- **Run start.** The first sample that extends a run after the run
  ended, or after the path was cached, starts it. The run start
  time is the monotonic time of that sample's gate read: `0x40` for the
  monitor run, `0x5D` for the battery run.
- **Freshness interval: 704 ms**, twice the S1 cycle. The margin covers
  the "about" of the S1 conversion time. It is scope policy, not a chip
  fact.
- **Publication.** `AVSB`, `3VCC` and `3VSB` are published only when the
  current sample's `0x40` read is at least 704 ms after the monitor run
  start. `VBAT` is published only when the current sample's `0x5D` read
  is at least 704 ms after the battery run start. A byte read before
  that point is not decoded. Its input is unavailable with the reason
  that the value update is pending. Diagnostics may keep the raw byte.
- **Why it holds.** The battery run starts with bit 0 read as `1`, so
  every `0` → `1` change the sampler could have missed happened at or
  before the run start, unless the bit toggled between two samples of
  the run (see Open questions). A `0x88` read at least 704 ms after the
  run start therefore follows that change by at least two S1 cycles.
  The monitor run argues the same way for `START` and `INT_Clear`.
- **Consequence.** The first sample transaction on a newly cached path,
  and the first sample after any run end, publishes none of the inputs
  that run covers. At the S15 service cadence of at most one reading
  every 1.6 s, the next sample that extends the run is already past the
  interval and publishes. From then on the run survives every ordinary
  sample on the cached path, so each later sample publishes as long as
  its gates hold. A sampler must not wait inside the ISA mutex
  transaction to satisfy the interval; the rule spans samples.

### Decode

For each voltage byte read in step 5 that the freshness rule allows:

- A raw byte `0x00` or `0xFF` is unavailable for that input. `0xFF` is
  the top code of the 8-bit range (S1 p. 58), so the pin voltage may be
  at or above full scale and is not known. `AVSB`, `3VCC` and `3VSB`
  power the operating chip (S1 p. 17), so they cannot read 0 V while it
  monitors. For `VBAT`, `0x00` is the bottom code and is not published
  as a measurement either. The update window after an enable is handled
  by the freshness rule, not by this rule. The battery case is an open
  question below. (S1 pp. 17, 58–59; S4)
- Otherwise the pin voltage is raw × 16 mV: 8 mV per code (S1 §8.6.2
  p. 59) times the factor of 2 that reverses the integrated one-half
  divider (S1 §8.6.1 p. 59). The result is an exact whole number of
  millivolts from 16 mV (`0x01`) to 4064 mV (`0xFE`), so no rounding
  is needed in Core.
- Publish the value as the voltage at the chip's supply pin. Do not
  scale it further, compare it with a nominal, or classify it as within
  or outside tolerance. Diagnostics may keep the raw byte.

### Input identity and ordering

| Identity label | Register | Description that may accompany the label |
| --- | --- | --- |
| `AVSB` | bank 4 `0x82` | Super I/O analog +3.3 V supply pin |
| `3VCC` | bank 4 `0x83` | Super I/O +3.3 V supply pin |
| `3VSB` | bank 4 `0x87` | Super I/O +3.3 V standby supply pin |
| `VBAT` | bank 4 `0x88` | Super I/O battery supply pin |

- The identity label is the S1 pin mnemonic and is bound to one register
  in every revision. The source is the chip label of the S2 `0xD421`
  row. (S1 p. 17; S2)
- The description column restates the S1 §5.13 pin function. Neither
  the label nor the description may be replaced by a motherboard rail
  name (for example a +3.3 V rail, a standby rail or a CMOS battery)
  unless a later revision adds board evidence that identifies that net.
- Order is register order: `AVSB`, `3VCC`, `3VSB`, `VBAT`. A consumer
  that keys inputs by label may omit an unavailable input in a sample. A
  consumer that depends on list position reports an explicit unavailable
  slot at that position, as the S2 identity rule requires. (S2)
- Voltage inputs are a separate kind from S2 temperatures and fans and
  never share their labels or positions.

### Implementation model requirements

This section records what the implementation needs. It does not design
the model.

- One round of motherboard readings has no voltage kind today (S12). The
  scope needs one, separate from temperatures and fan speeds.
- Each voltage reading needs a stable channel identity (the identity
  label above), a source (the chip label), a value with an explicit unit
  (volts or millivolts at the chip pin), and per-input availability with
  a reason. A reading must be able to be explicitly unavailable.
- Three facts stay separate. The raw ADC code is diagnostic only. The
  chip-pin voltage is the value of this scope. A board rail name has no
  data in this revision and must not be derived from the label.
- Successful readings carry no verification metadata (ADR 0011). Unit
  formatting is App presentation. Limits, alarms and nominal-value
  comparisons are out of scope.
- New channel identities and their joins go through the repository's
  identity-contract verification before implementation (#2534 delivery
  requirements).

### Synthetic cases for implementation review

All cases are synthetic fixtures, not hardware observations. Times are
monotonic gate-read times of consecutive sample transactions on one
cached path. A discovery attempt cached the path just before the sample
at 0 ms, and no discovery attempt runs while it stays cached. Unless a case
says otherwise, the case's sample is at least 704 ms after the start of
both runs.

| Case | Inputs | Expected result |
| --- | --- | --- |
| Nominal | `0x40` = `0x03`, `0x5D` bit 0 = `1`; `0x82` = `0xCE`, `0x83` = `0xCF`, `0x87` = `0xD0`, `0x88` = `0xBC` | `AVSB` 3296 mV, `3VCC` 3312 mV, `3VSB` 3328 mV, `VBAT` 3008 mV |
| Range ends | a byte `0x01`; a byte `0xFE` | 16 mV; 4064 mV |
| Invalid codes | a byte `0x00`; a byte `0xFF` | That input unavailable; the others unaffected |
| First sample | Nominal bytes at 0 ms | All four unavailable, update pending; bytes not decoded |
| Second sample | Nominal bytes at 0 ms and at 1600 ms | At 1600 ms all four decoded as in Nominal |
| Battery monitor enabled between samples | 0 ms and 1600 ms: `0x5D` bit 0 = `0`. 3200 ms: bit 0 = `1`, `0x88` = `0xA0`. 4800 ms: bit 0 = `1`, `0x88` = `0xBC` | At 3200 ms `VBAT` unavailable, update pending, and `0xA0` not decoded. At 4800 ms `VBAT` 3008 mV. The other three decode from 1600 ms on |
| Short interval | Gates satisfied at 0, 300, 600 and 900 ms | All four unavailable at 0, 300 and 600 ms; published at 900 ms |
| Steady cached sampling | Gates satisfied in sample transactions every 1600 ms from 0 ms to 60 000 ms on the same cached path | No discovery attempt and no configuration-mode entry or exit after 0 ms. Both runs started at 0 ms and never end. All four unavailable at 0 ms and published in every sample from 1600 ms on |
| Path discarded and rediscovered | Gates satisfied at 0 ms and 1600 ms; the path is discarded after 1600 ms; a discovery attempt caches it again and its first sample is at 4000 ms, the next at 5600 ms | Published at 1600 ms. The runs end with the discard. All four unavailable at 4000 ms (first sample on the new path); published at 5600 ms |
| Sampling gap | Gates satisfied at 0 ms, 1600 ms and 7600 ms | Published at 1600 ms; all four unavailable at 7600 ms (gap above 5 s, new runs) |
| Battery gate read fails | At 1600 ms the `0x5D` read fails; at 3200 ms and 4800 ms it succeeds with bit 0 = `1` | At 1600 ms the failure rule stops the voltage steps, so all four are unavailable; the battery run ends and the monitor run does not. At 3200 ms `AVSB`, `3VCC` and `3VSB` published, `VBAT` unavailable (new battery run). At 4800 ms all four published |
| Battery monitor disabled | `0x5D` bit 0 = `0` | `0x88` not read; `VBAT` unavailable; the other three decoded |
| Monitor stopped | `0x40` bit 0 = `0`, or bit 3 = `1` | No value register read; all four unavailable; both runs end; temperatures and fans of the sample unchanged; bank 4 selected again |
| Partial failure | The bank 0 selection or the `0x40` read fails | All four unavailable with an experimental diagnostic; both runs end; temperatures and fans of the sample unchanged |
| Late failure | The `0x87` read fails after `0x82` and `0x83` succeeded | `AVSB` and `3VCC` keep their values; `3VSB` and `VBAT` unavailable; neither run ends |
| S2 abort | A temperature or fan read fails in S2 step 9 | S2 aborts the sample; no voltage read; both runs end |
| Other chip | Any raw ID other than `0xD421`, including `0xD423` and `0xD802` | No voltage read |

## Quirks

- `VBAT` starts disabled at power-on. The `0x5D` default has bit 0
  clear (S1 p. 113), so a board whose firmware never enables the battery
  monitor reports `VBAT` unavailable. This scope never sets the bit.
- `0x5D` bits `7:1` select temperature sensor types (S1 p. 113). The
  scope reads the whole byte but interprets only bit 0.
- The value RAM updates about every 352 ms (S1 p. 62). Samples taken
  more often can repeat a value.
- A board whose firmware enables the battery monitor only shortly before
  the operating system starts sampling still shows `VBAT` only from the
  second sample on. The [freshness rule](#freshness-rule) withholds the
  first observation of every run.

## Safety notes

- Writes in this scope are read-transaction plumbing only: normal-HM
  index writes and the bank selection writes `0x00` and `0x04` to index
  `0x4E` (S1 p. 111). Everything S2 writes is unchanged.
- Never write bank 0 `0x40`. Bit 7 restores register defaults, bit 0
  stops or starts monitoring, and bit 3 stops monitoring. (S1 p. 106)
- Never write bank 0 `0x5D`, any voltage limit, any SMI or alarm control,
  or any register outside the read set. (S1 pp. 106, 113)
- Never read the read-clear interrupt status registers: bank 0 `0x41`
  (S1 p. 106), `0x42` (p. 107) and `0x45` (p. 108); bank 4 `0x50`
  (p. 168); bank 5 `0x67` (p. 186); bank C `0x02` (p. 261) and `0x03`
  (p. 262). None of them is in the read set.
- Hold the ISA mutex for the whole transaction (S2, S3).

## Disposition of #2534 targets

#2534 defines its inventory as read-only voltage channels on already
supported or newly specified Super I/O and EC profiles, with the
LibreHardwareMonitor motherboard tree as the comparison inventory (S14).
Each row is one of three kinds: **Implemented scope** (this document),
**Evidenced existing coverage**, or **Open follow-up**. An open
follow-up stays tracked in #2534 until the maintainer splits it out. A
"candidate" has primary evidence located but needs its own revision.
"Unsupported" lacks the evidence named in its last column. No enabled
row rests on a lead-only fact.

### Nuvoton Super I/O (Windows PawnIO)

| Target | Disposition | Evidence and reason | Unblock evidence |
| --- | --- | --- | --- |
| `0xD421` NCT6796D/-E `AVSB`, `3VCC`, `3VSB`, `VBAT` | Implemented scope (Experimental) | This document, from S1 | A maintainer-accepted `0xD421` dump of bank 0 `0x40`/`0x5D` and bank 4 `0x80`–`0x8E`, with an independent measurement of the same pins, graduates the row to Verified. |
| `0xD421` `CPUVCORE` | Open follow-up (candidate) | Pin voltage defined, no internal divider (S1 p. 59); board net and network unknown | A board schematic or vendor board document showing what drives the `CPUVCORE` pin, or a maintainer-accepted dump with an independent measurement of that net, per board design. |
| `0xD421` `VIN0`–`VIN8` | Open follow-up (Unsupported) | External dividers and nets are board facts (S1 p. 59) | Per board design: resistor values and the connected net from a schematic or vendor document, or a maintainer-accepted dump with an independent reference. |
| `0xD421` `VTT` | Open follow-up (Unsupported) | The divider is not stated (S1 pp. 58–59) | A Nuvoton statement of the `VTT` input path. |
| `0xD423` NCT6796D and unrecognized `0xD420`–`0xD427` versions | Open follow-up (candidate) | S1 defines the voltage facts for NCT6796D. The S2 rows attach these IDs to NCT6796D through lead-only facts only. | A maintainer decision that a lead-only chip identity with primary voltage facts may back an Experimental voltage row (ADR 0023 permits it), or a maintainer-accepted dump. |
| `0xD802` NCT6799D (Verified temperatures and fans in S2) | Open follow-up (candidate) | No NCT6799D datasheet (S2 Open questions). S1 does not cover `0xD802`. Neither S2 dump record in this repository includes bank 4 `0x80`–`0x8E` bytes. | A Nuvoton NCT6799D document defining the voltage value RAM and integrated dividers, or a maintainer-accepted `0xD802` dump of bank 0 `0x40`/`0x5D` and bank 4 `0x80`–`0x8E` with an independent measurement of the supply pins. |
| S2 lead-only rows: NCT6779D range, `0xC803`, `0xC911`, `0xC913`, `0xD121`, `0xD352`, `0xD429`, `0xD42A`, `0xD42B`, `0xD451`, `0xD801`, `0xD806`, and unrecognized versions of the eight S2 ranges | Open follow-up (Unsupported) | No primary voltage map or divider statement for these parts. | A primary datasheet per part, or a maintainer-accepted dump with an independent reference. |
| NCT6776F/D `0xC330`–`0xC337` | Open follow-up (candidate) | S6 §8.5.1 p. 60 states one-half integrated dividers for `AVCC`, `VBAT`, `3VSB` and `3VCC`. S6 §8.5.2.1 p. 60 places `AVCC` and `3VCC` at bank 0 `0x22`/`0x23` and `3VSB` and `VBAT` at bank 5 `0x50`/`0x51`. | A revision of this kind for NCT6776F that pins bank 5 access, its monitor-start and battery-monitor gates, and its read-clear registers. |
| NCT6775F/NCT6771F `0xB470`–`0xB477` | Open follow-up (Unsupported) | Lead-only chip facts only (S13: [`superio-nuvoton-nct6775-nct6776-nct610x.md`](superio-nuvoton-nct6775-nct6776-nct610x.md)) | A primary datasheet. |
| NCT6102D/NCT6104D/NCT6106D | Open follow-up (Unsupported) | Every ID stays disabled because of its chip-ID conflict (S13) | Resolution of that conflict first. |
| NCT668x direct EC Space (MSI `0xD592`) on Windows | Open follow-up (Unsupported) | Draft access (S13: [`superio-nuvoton-nct668x.md`](superio-nuvoton-nct668x.md)); EC firmware defines channels | A ready access path plus board evidence of each channel's net and scale. |

### ITE and Fintek Super I/O (Windows PawnIO)

| Target | Disposition | Evidence and reason | Unblock evidence |
| --- | --- | --- | --- |
| IT8728F `0x8728` | Open follow-up (candidate) | S9 gives a 12 mV code over 0 to 3.072 V (p. 108). `3VSB` and `VBAT` read as reading × 2 × 12 mV (pp. 96–97). `AVCC3` reads through `VIN3` as reading × 2 × 12 mV when LDN 7 index `0x2C` bit 0 is `1` (p. 96). EC index `0x00` bit 6 is a read/write "Update VBAT Voltage Reading" control (p. 91). | A revision that pins the `VIN3` selection as a read-only gate, states when `VBAT` refreshes without that write (the write is excluded), and lists the EC read-clear registers for the voltage path. |
| IT8716F D and IT8718F | Open follow-up (candidate) | Global `CR2C` bits 1:0 switch `VIN7` and `VIN3` to an internal divider for VCCH5V and VCC5V, together with alternate pin functions (S10 §8.3.15 p. 46; S11 §8.3.15 p. 43). | Divider ratio, ADC code size and `3VSB`/`VBAT` handling pinned from S10/S11, with the `CR2C` bits as read-only gates. |
| IT8620E, IT8628E, IT8686E, IT8688E, IT8689E | Open follow-up (Unsupported) | Their existing rows rest on lead-only chip facts (S13: [`superio-ite-primary-lpc-expansion.md`](superio-ite-primary-lpc-expansion.md)) | A primary datasheet or a maintainer-accepted dump with an independent reference. |
| IT8772E FX | Open follow-up (candidate review) | A primary document is pinned in the ITE expansion specification (S13). Its voltage section was not reviewed for this revision. | Review of that document's voltage section. |
| IT8721F DX | Open follow-up (Unsupported) | Its fan row is disabled for configuration-pair ownership (S13) | Resolution of that ownership question first. |
| Gigabyte secondary ITE controllers, IT8705F/AF, IT8783E/F, IT8720F | Open follow-up (Unsupported) | Secondary access is draft, or the chip has no enabled profile (S13: [`superio-ite-gigabyte-secondary.md`](superio-ite-gigabyte-secondary.md) and the ITE expansion specification) | A ready access profile first. |
| Fintek F71882 `0x0541` and F71889 `0x0723` | Open follow-up (candidate) | S7 §7.5 p. 36 and S8 §7.2 p. 16 state that `3Vcc` reaches the monitor through two internal 150 kΩ resistors, giving one half. S7 §8.6.2.16 pp. 72–73 places `VCC3V`, `VSB3V` and `VBAT` at HM `0x20`, `0x27` and `0x28` with an 8 mV reading unit. S7 does not state whether that unit applies before or after the divider. It states no divider for `VSB3V` or `VBAT`. | Those two points resolved from a primary source, and the F71889 register locations pinned. |
| Other Fintek IDs (S13: [`superio-fintek-f71882-f71889.md`](superio-fintek-f71882-f71889.md) disposition table) | Open follow-up (Unsupported) | No enabled profile | An enabled profile first. |

### Embedded controllers and Linux hwmon routes

| Target | Disposition | Evidence and reason | Unblock evidence |
| --- | --- | --- | --- |
| Any Linux hwmon `inY_input` used as a rail reading | Open follow-up (Unsupported) | S5 defines `inY_input` as millivolts on the chip pin, generally without board scaling, and `inY_label` as a driver hint. A driver label or value therefore establishes neither the rail nor its scale. | Per board: evidence of the net and scale, or a driver whose readings come from firmware that reports board-scaled values, with that scaling backed by primary evidence. |
| MSI NCT6687D-family endpoint (S13: [`superio-nuvoton-nct6687-linux-hwmon.md`](superio-nuvoton-nct6687-linux-hwmon.md)) | Open follow-up (Unsupported) | The ready scope excludes voltages. EC firmware chooses the channel assignment. S5 applies. | Per-board channel, net and scale evidence. |
| ASRock NCT6686D endpoint (S13: [`superio-nuvoton-nct6686-asrock-linux-hwmon.md`](superio-nuvoton-nct6686-asrock-linux-hwmon.md)) | Open follow-up (Unsupported) | The ready scope excludes voltages; S5 applies. | Same as above. |
| ASUS ROG CROSSHAIR VIII EC CPU core voltage (S13: [`ec-asus-crosshair-viii-linux-hwmon.md`](ec-asus-crosshair-viii-linux-hwmon.md)) | Open follow-up (candidate) | The EC register is unsigned big-endian millivolts, and that document routes this reading to #2534. Its EC map is lead-only, so the scale has no primary evidence. | An ASUS primary source for the EC field, or a maintainer-accepted dump with an independent measurement of CPU core voltage. |
| Gigabyte B850M AORUS ELITE WIFI6E ICE WMI endpoint (S13: [`superio-ite-b850m-wifi6e-ice-linux-wmi.md`](superio-ite-b850m-wifi6e-ice-linux-wmi.md)) | Evidenced existing coverage (no voltage channel on this route) | The scoped endpoint exposes temperatures only. | — |
| IT87 Linux routes on GA-B75M-D3H and TRX50 AI TOP (S13: [`superio-ite-b75m-d3h-linux-hwmon.md`](superio-ite-b75m-d3h-linux-hwmon.md), [`superio-ite-trx50-secondary-linux-hwmon.md`](superio-ite-trx50-secondary-linux-hwmon.md)) | Open follow-up (Unsupported) | Draft. Both record that a non-default battery-update option adds EC writes. S5 applies. | Those drafts' blockers first, then per-board evidence. |
| LattePanda Sigma EC (S13: [`superio-ite-lattepanda-sigma-linux-ec.md`](superio-ite-lattepanda-sigma-linux-ec.md)) | Open follow-up (Unsupported) | Draft. No voltage channel is in its evidence. | That draft's arbitration blocker first. |
| ASUS EC on Windows (S13: [`ec-asus-windows-transport.md`](ec-asus-windows-transport.md)) | Open follow-up (Unsupported) | Transport rejected or blocked | That document's unblock evidence. |
| Corsair HXi/RMi PSU rail voltages (S13: [`psu-corsair-hxi-rmi-linux-hwmon.md`](psu-corsair-hxi-rmi-linux-hwmon.md)) | Evidenced existing coverage (#2531) | PSU-reported AC input and +12 V, +5 V, +3.3 V output voltages are already specified there. They are PSU readings, not motherboard inputs. | — |

### Comparison-inventory groups with no repository specification (S14)

| Target | Disposition | Reason | Unblock evidence |
| --- | --- | --- | --- |
| Winbond W836xx Super I/O | Open follow-up (Unsupported) | No chip profile in this repository | A primary datasheet and an access profile. |
| Per-board voltage descriptors in the comparison inventory | Open follow-up (Unsupported) | Board resistor and rail knowledge from a copyleft implementation would make every scaled rail lead-only, and a scaled rail needs primary evidence. | Per-board primary evidence (schematic, vendor document) or a maintainer-accepted dump with an independent reference. |
| ChromeOS EC, other embedded-controller boards and Gigabyte controller access | Open follow-up (Unsupported) | No ready access path in this repository | A ready access specification first. |
| IPMI | Open follow-up (Unsupported) | A new BMC transport; #2510 excludes remote BMC access. | A product decision and an access specification. |
| Linux hwmon reader | Open follow-up (Unsupported), as the first Linux hwmon row above | S5 | As that row. |

## Open questions

- Non-blocking for the #2534 initial slice: no hardware dump exists for
  any `0xD421` board, so the scope is Experimental. The graduation
  evidence is in the disposition table.
- Non-blocking for the #2534 initial slice: a battery that truly reads
  0 V, for example an absent battery with the monitor enabled, is
  reported unavailable instead of 0 V. S1 defines the code but not what
  `0x88` holds before its first update, so the bottom code is not
  published. Only the code `0x00` is hidden; low non-zero values are
  published once the freshness rule allows them.
- Non-blocking for the #2534 initial slice: the residual stale case needs
  a disable or `PWROK` reset and a re-enable inside one sample interval
  of at most 5 s, and its value is still a VBAT-register byte filtered
  by the `0x00`/`0xFF` rule. A read-only sampler cannot see
  `EN_VBAT_MNT`, `START` or `INT_Clear` change and change back between
  two samples of a run. If firmware disables and re-enables the battery
  monitor between two samples that are at most 5 s apart, and the
  re-enable is less than 704 ms before the later sample, that sample
  publishes the `0x88` byte as it stood before the update. S1 defines
  that byte only as the VBAT reading, so the published value is the
  last VBAT conversion before the re-enable, or an undefined byte if the
  register was never updated. It still passes the `0x00`/`0xFF` rule and
  is labelled only as a chip-pin voltage. At the S15 cadence the two
  writes must fall inside one interval of at least 1.6 s. A suspend and
  resume that leaves at most 5 s between gate reads and keeps the
  monitoring session is the same case: a `PWROK` reset clears `0x5D`
  bit 0 (S1 p. 113), and a firmware re-enable less than 704 ms before
  the next sample leaves a mature battery run. That sample publishes the
  `0x88` byte as it stood after the reset and before the update. S1
  states no reset value for the Value RAM, so the byte is the last VBAT
  conversion or an undefined byte, filtered by the same `0x00`/`0xFF`
  rule and labelled the same way. The repository has no suspend or
  resume signal that would let the sampler end its runs (a search of
  `hardviz/app` and `hardviz/core` found no power-event handling). The
  covered paths are sampler start, a disabled monitor, a failed gate
  read, and a suspend and resume that leaves more than 5 s between gate
  reads or ends the session. An observable resume signal added later
  should end both runs. A maintainer-accepted dump taken across a
  firmware enable would show whether the residual case occurs.
- Non-blocking for the #2534 initial slice: the 704 ms interval is twice
  the only cycle time S1 states. S1 §8.6.4 p. 62 derives the 352 ms
  cycle as an example from about 2 ms per conversion, 15 voltage and 7
  temperature channels and eight averages. S1 describes averaging only
  there and gives no register that changes it. A cycle longer than
  704 ms on real hardware would shorten the protection after an enable,
  and the graduation dump in the disposition table would expose it.
- Non-blocking for the #2534 initial slice: S1 gives the integrated
  divider as two nominal 34 kΩ resistors and states no accuracy for the
  divider or the ADC. Values are published without an accuracy claim
  and are not compared with a nominal.
- Non-blocking for the #2534 initial slice: S1 states neither an
  integrated divider nor a direct path for `VTT`, so `VTT` is not read.
- Non-blocking for the #2534 initial slice: S1 §9.20 also governs
  whether temperatures are being converted, but S2 does not gate its
  temperature and fan reads on bank 0 `0x40`. This document gates only
  the voltages. Whether S2 should adopt the same gate is a separate S2
  revision.
- Non-blocking for the #2534 initial slice: a board may put a filter
  between its supply net and a chip supply pin. The published value is
  the pin voltage, which S1 defines, and makes no claim about the net
  beyond the pin.
- Non-blocking for the #2534 initial slice: Core has no voltage kind yet
  (S12). The scope cannot be shown until the implementation adds one
  that meets [Implementation model requirements](#implementation-model-requirements).

## Implementation-ready transition checklist

Revision 2 keeps the flip proposed by revision 1 and meets the checklist in
[`README.md`](README.md#status-transition-draft--implementation-ready):

- no provenance marker remains; every enabled fact is pinned to an S1
  section and printed page, and the transport facts are pinned to S2 and
  S3 at implementation-ready revisions, and the freshness rule takes only
  sampler lifetime terms from S15 at an implementation-ready revision;
- no enabled fact is lead-only; S14 is tagged `lead-only (copyleft)` and
  backs no fact;
- every runtime gate is tagged mechanism or safety, no evidence identity
  is a gate, and the Scope rows are keyed by chip ID and input mechanism;
- the scoped-enablement table matches the evidence state: one
  Experimental row and disabled rows with stated reasons;
- every open question is annotated non-blocking for the #2534 initial
  slice; and
- the revision, status and revision history record the proposal, and
  the Current documents row in `README.md` is added with it.

## Provenance text for a clean-room implementation PR

```text
Implemented from docs/specs/sensors/superio-nuvoton-nct6796d-supply-voltages.md revision 2 (commit <sha>), limited to the 0xD421 AVSB, 3VCC, 3VSB and VBAT supply-pin voltages, together with docs/specs/sensors/superio-nuvoton-nct67xx.md revision 10 (commit <sha>) for the 0xD421 transaction it extends, and docs/specs/sensors/superio-access.md revision 5 (commit <sha>) for the sampler lifetime terms of the freshness rule.
No other external sensor documentation was used.
```

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-29 | Initial version for #2534. Proposes Implementation-ready status for one Experimental scope: `AVSB`, `3VCC`, `3VSB` and `VBAT` supply-pin voltages on exact raw `0xD421` (NCT6796D/-E), all from the NCT6796D datasheet (S1). The scope reads bank 4 `0x82`, `0x83`, `0x87` and `0x88` after the existing revision 9 `0xD421` temperature and fan sample. It is gated on bank 0 `0x40` `START`/`INT_Clear` and, for `VBAT`, on `0x5D` `EN_VBAT_MNT`. Decode is raw × 16 mV; `0x00` and `0xFF` are unavailable. Labels are the pin mnemonics, with no rail names. Adds the failure rule, synthetic cases, read-clear registers that are never read, the model requirements and a disposition for every #2534 target. No lead-only fact backs any row. |
| 2 | 2026-11-29 | Review finding on #2828: S1 §9.34 p. 113 says the VBAT reading updates one monitor cycle (about 352 ms, §8.6.4 p. 62) after `EN_VBAT_MNT` changes from `0` to `1`, and a read-only sampler cannot see an enable before its first sample. Adds the [freshness rule](#freshness-rule): per cached path, a monitor run and a battery run of consecutive samples with satisfied gates; a run ends on an S2 abort, a failed or unsatisfied gate read, a discarded path, a discovery attempt, the end of the monitoring session, or a gap above 5 s between gate reads. `AVSB`, `3VCC` and `3VSB` publish only 704 ms or more after the monitor run start, and `VBAT` only 704 ms or more after the battery run start; earlier bytes are unavailable as update pending. Adds S15 (`superio-access.md` revision 5 terms), a runtime gate row, the `0x5D` reset sources, synthetic cases for startup, enable, short interval, gap and gate failure, a quirk, and Open questions for the residual between-sample toggle, including a short suspend and resume that the repository cannot observe, and the cycle margin. The `0x00` rationale no longer relies on the update window. The read procedure now follows the S2 revision 10 split: the discovery attempt (S2 steps 1–7 and 11) runs only while no path is cached, under the S15 cadence, and each sample transaction on the cached path runs S2 steps 8–9 and then the voltage reads with no configuration-mode entry or exit. Runs therefore survive ordinary cached samples, and a discovery attempt no longer appears as a run end. S2 is re-pinned to revision 10, S3 to `superio-access.md` revision 5, and S15 adds rule R1; fixtures for steady cached sampling and a discarded and rediscovered path are added. Registers, writes, decode scale and dispositions are unchanged. Status remains Implementation-ready (rev 2), proposed. |
