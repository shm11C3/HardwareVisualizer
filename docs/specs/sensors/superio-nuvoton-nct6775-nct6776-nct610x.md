# Spec: Nuvoton NCT6775F / NCT6776F / NCT610x Super I/O hardware monitor (source-selected temperature slots)

| Field | Value |
| --- | --- |
| Revision | 2 |
| Status | Implementation-ready (rev 2) |
| Scope | Windows PawnIO `LpcIO` read-only motherboard temperatures and fan RPM on the early Nuvoton parts whose hardware monitor reports temperatures through six source-selected slots (the datasheets call them SMIOVT1–SMIOVT6) instead of the fixed bank 4 bytes of [`superio-nuvoton-nct67xx.md`](superio-nuvoton-nct67xx.md). Enabled: NCT6776F/NCT6776D raw IDs `0xC330`–`0xC337` (temperatures and five direct RPM inputs, primary-source-backed, Experimental) and the NCT6775F/NCT6771F family raw IDs `0xB470`–`0xB477` (temperatures only, lead-only, Experimental). Recorded but disabled: every NCT6102D/NCT6104D/NCT6106D ID (chip-ID conflict), NCT6775F-family fans (conflicting leads), PECI, SMBus-master, PCH and BYTE_TEMP slot sources, voltages. Excludes every write other than the read-transaction plumbing listed in Safety notes, fan/PWM control, limits, alarms, the Linux and ACPI WMI paths, and every raw ID handled by `superio-nuvoton-nct67xx.md`. |
| Issue phase | #2521 (epic #2510) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Nuvoton, *NCT6776F / NCT6776D Nuvoton LPC I/O* datasheet, Revision 1.2, publication release date July 12, 2011. Copy hosted by DigiKey: `https://mm.digikey.com/Volume0/opasdata/d220001/medias/docus/532/NCT6776F,D.pdf`, fetched 2026-11-28, SHA-256 `d5eec9d9bdc1212c619fd36995a2f1d964482079fbbf7aa55311a50ebd6e90fb`. Pins (printed page numbers): configuration protocol §7, §7.1.1–7.1.3 pp. 52–55; HM access §8.3 pp. 56–57 and §9 intro p. 87; temperature data format Table 8-1 §8.5.3 p. 61; temperature reading table §8.5.3.4 pp. 62–63; PECI Tbase §8.6 p. 64; fan speed reading §8.7.1–8.7.3 p. 65; SMIOVT1 configuration (index 18h) and value RAM (index 27h) §9.12–9.14 pp. 92–93; configuration register (index 40h) §9.15 pp. 93–94; interrupt status 41h, 42h, 45h §9.16, 9.17, 9.20 pp. 94–96; FAN IN/OUT control (index 4Dh) §9.26 p. 98; bank select (4Eh) and vendor ID (4Fh) §9.27–9.28 p. 99; SMIOVT2 bank 1 50h–52h §9.92–9.94 pp. 116–117; SMIOVT3 bank 2 50h–52h §9.140–9.142 pp. 128–129; bank 4 interrupt status 41h and 50h pp. 150–151; bank 6 source select 21h–26h §9.251–9.256 pp. 158–162; bank 6 SMIOVT4–6 configuration 28h–2Ah §9.258–9.260 pp. 163–164; bank 6 SMIOVT4–6 readings 2Bh–2Eh §9.261–9.264 pp. 164–165; bank 6 fan counts 30h–39h §9.266–9.275 pp. 165–167; bank 6 direct RPM 56h–5Fh §9.295–9.304 pp. 173–175; global register defaults and the CR21 IC-version note §24.1 p. 304; CR1C p. 309–310; CR20/CR21 p. 310; CR24 pp. 311–313; CR26 p. 314; CR27 pp. 315–316; CR28 p. 320; Logical Device B CR30, CR60/61, CR62/63 §24.12 p. 374. | Primary (vendor datasheet; third-party-hosted copy pinned by hash). The section numbers of the table of contents and the body differ in places; the printed page numbers above are authoritative. |
| S2 | Nuvoton, *NCT6102D / NCT6106D Nuvoton LPC I/O* datasheet, Revision 1.0, publication release date January 11, 2012. Copy hosted by datasheet4u: `https://datasheet4u.com/pdf/1006022/NCT6106D.pdf`, fetched 2026-11-28, SHA-256 `3c56f97cd080e037c5ab66e14dc299f9a78395f86212490ecb271185f9ff1f83`. Pins: configuration protocol §7 pp. 46–47; HM access §8.3 pp. 50–51 and §9 intro p. 83 (indexes `0x00`–`0x4F` are global to all banks); Table 8-1 p. 55; temperature reading table §8.5.3.4 p. 57; PECI Tbase p. 58; fan speed reading §8.7.1–8.7.3 p. 60; value RAM §9.3 pp. 84–87; SMIOVT readings §9.4–9.10 pp. 87–89; fan counts §9.15–9.20 pp. 90–92; bank select §9.21 p. 92; interrupt status 70h–76h §9.51–9.57 pp. 100–103; source select B0h–B5h §9.93–9.98 pp. 116–120; SMIOVT1 control B7h §9.100 p. 121; FAN IN/OUT control F8h §9.146 p. 137; vendor ID FEh §9.149 p. 138; bank 3 configuration register 17h §9.293 p. 187; CR20/CR21 p. 303; CR24 pp. 304–306; CR26 p. 307; Logical Device B §23.12 p. 362. | Primary for the disabled NCT610x candidate profile only. Revision 1.0 does not name the NCT6104D; a later revision 1.52 (July 29, 2013) that adds it could not be obtained (see Open questions). |
| S3 | [`superio-access.md`](superio-access.md) revision 5 | Clean-room source for the Nuvoton configuration-mode enter/exit keys, slot mapping, chip-ID reads at `0x20`/`0x21`, absent-ID classification and the ISA mutex, which are unchanged from revision 4, and for the [sampling-path rediscovery cadence](superio-access.md#sampling-path-rediscovery-cadence), which is project policy. |
| S4 | [`pawnio-interface.md`](pawnio-interface.md) revision 8, `LpcIO` section | Interface source: `ioctl_select_slot`, `ioctl_find_bars`, `ioctl_superio_inb`/`outb`, `ioctl_pio_inb`/`outb`; port access is limited to the configuration pair and the BAR windows found by `ioctl_find_bars`, clamped to 8-byte-aligned windows; the caller holds `Global\Access_ISABUS.HTP.Method`. |
| S5 | PawnIO.Modules `LpcIO.p` at tag `0.2.11` (commit `52a7e536dff3e53c96917a28caac5e0fa6510696`), LGPL-2.1-or-later | Upstream-published interface definition of the module this project calls. `ioctl_find_bars` reads global `0x20` and returns not-found if it is `0x00` or `0xFF`; otherwise it selects every logical device and records the bases at `0x60/61` and `0x62/63` that read the same twice. No chip-ID allow-list. Not used for any hardware register fact; no code copied. |
| S6 | [`superio-nuvoton-nct67xx.md`](superio-nuvoton-nct67xx.md) revision 9 | Project source for the shared Nuvoton normal-HM procedure and its evidence: `ioctl_find_bars` must run after configuration-mode entry and LDN B base discovery (its S7 hardware evidence), the -55 to +125 °C plausibility policy, zero-RPM handling, and the input identity and ordering rule. Its raw-ID ranges do not overlap any ID in this document. |
| S7 | [ADR 0011](../../adr/0011-experimental-sensor-enablement.md), [ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md), [ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md) | Project policy: Experimental enablement, lead-only facts confined to Experimental rows, runtime gates limited to mechanism and safety gates. No hardware facts. |
| S8 | Linux kernel `drivers/hwmon/nct6775-platform.c` (torvalds/linux commit `7cd8495c0f16275c4f9abe1cef09a7b5fcd6d046`, retrieved 2026-11-28), GPL-2.0 | **lead-only (copyleft)**. Super I/O detection classifies the raw ID after masking its low three bits (`0xFFF8`) against `0xB470` (the NCT6775F kind), `0xC330` (the NCT6776F kind) and `0xC450` (the NCT6106D kind); hardware monitor is logical device `0x0B`, base `CR60/61` with the low three bits cleared, base `0` refused; the `CR28` I/O-mapping step is applied only to NCT6791D and later kinds; on an inactive LDN B the driver writes `CR30` bit 0 (not adopted). For the NCT6776F kind it decides whether fan inputs 3–5 are wired from LDN B `CR30` bits 7–5, `CR27` bit 7, `CR24` bit 6 and `CR1C` bits 0–1; for the NCT6106D kind fan input 3 from `CR24` bit 7. Restated in this document's own words; no code, structure or identifiers copied. |
| S9 | Linux kernel `drivers/hwmon/nct6775-core.c` and `drivers/hwmon/nct6775.h` (same commit), GPL-2.0 | **lead-only (copyleft)**. For the NCT6775F kind: temperature slot registers bank 0 `0x27`, bank 1 `0x50`, bank 2 `0x50`, bank 6 `0x2B`, `0x2C`, `0x2D`; bank 1 and bank 2 `0x50` are read as 16-bit (high byte then bit 7 of `0x51`), bank 6 `0x2B`–`0x2D` as 8-bit; slot source-select registers bank 6 `0x21`–`0x26` (bits 4:0); slot configuration registers bank 1 `0x52`, bank 2 `0x52`, bank 6 `0x28`–`0x2A` (none for the bank 0 slot), bit 0 clear means monitoring; source codes 1 SYSTIN, 2 CPUTIN, 3 AUXTIN, 4 AMD SB-TSI, 5–12 PECI agents 0–7, 13–20 PCH sources; four fan inputs; fan counts at bank 6 `0x30`, `0x32`, `0x34`, `0x36` read as 16-bit values decoded with a per-fan divisor from bank 5 `0x06`/`0x07` (the driver also rewrites the divisor, a write not adopted); hardware-monitor configuration register `0x40` bit 0 starts monitoring; vendor ID `0x5CA3` at `0x4F` and HM chip ID `0xC1` at `0x58` for the NCT6775F, NCT6776F and NCT6106D kinds. For the NCT6106D kind: temperature slots at bank 0 `0x10`–`0x15`, sources at bank 0 `0xB0`–`0xB5`, slot configuration at bank 0 `0xB7`–`0xBC`, three fan counts at bank 0 `0x20`, `0x22`, `0x24`, and the same source-code table as the NCT6776F kind. Restated; nothing copied. |
| S10 | Linux kernel `Documentation/hwmon/nct6775.rst` (same commit), GPL-2.0 | **lead-only (copyleft)**. Documentation text: one driver kind covers NCT5572D/NCT6771F/NCT6772F/NCT6775F/W83677HG-I (datasheet "available from Nuvoton upon request"), another NCT5573D/NCT5577D/NCT6776D/NCT6776F, another NCT6102D/NCT6104D/NCT6106D (datasheet "available from the Nuvoton web site"). |
| S11 | LibreHardwareMonitor `LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/` `Chip.cs`, `LpcIO.cs`, `Nct677X.cs` (commit `9e2d677cefff5c614ecc38da88fbc447eb478e2a`, retrieved 2026-11-28), MPL-2.0 | **lead-only (copyleft)**. Detection: chip-ID high byte `0xB4` with `CR21` high nibble `7` = NCT6771F; `0xC3` with high nibble `3` = NCT6776F; `0xC4` with high nibble `5` = one entry for NCT6102D/NCT6104D/NCT6106D, whose enumeration value is `0xC452`; no `CR28` lock step for these chips. Hardware monitor: requires vendor ID `0x5CA3` read from `0x4F` (NCT6771F, NCT6776F) or `0xFE` (NCT610x) with bank-select bit 7 choosing the byte; NCT6771F has four fan inputs read from direct RPM pairs bank 6 `0x56`–`0x5D` with a 16-bit-counter floor; NCT6776F five inputs from bank 6 `0x56`–`0x5F`, values at or below 1 350 000 / `0x1FFF` treated as zero; NCT610x three inputs from bank 0 `0x30`–`0x35`; temperature slots read with the source register and the half-degree bit from bank 6 `0x2E` for both NCT6771F and NCT6776F; source codes for NCT6771F 1 SYSTIN, 2 CPUTIN, 3 AUXTIN, 5 PECI 0. Its NCT610x source-code table and its NCT610x half-bit register assignments differ from S2 (see Open questions). Restated; nothing copied. |
| S12 | coreboot `util/superiotool/nuvoton.c` (coreboot commit `95b01cee5e20d5916fcbf50f85c3baba011e4a4a`, retrieved 2026-11-28), GPL-2.0-or-later | **lead-only (copyleft)**. Super I/O register-dump tool ID table: `0xB472` "NCT6775F (A)", `0xB473` "NCT6775F (B) / NCT5572D (B)", `0xC332` "NCT6776F (B)", `0xC333` "NCT6776F/D (C)", `0xC452` "NCT6102D / NCT6106D". Restated; nothing copied. |

S8–S12 are copyleft implementations and documentation consulted as
leads under ADR 0023. Every fact taken from them is restated, carries its
source ID and backs only rows whose default enablement is Experimental,
or is recorded in Open questions. The NCT6776 rows rest on S1 and on the
project sources S3–S7; S8, S9, S11 and S12 only corroborate them.
The implementer works from this document only and does not open S12,
which is a copyleft lead like S8–S11.

## Target inventory disposition (#2521)

| Target | Disposition in this revision | Basis |
| --- | --- | --- |
| NCT6776F | Enabled, Experimental: temperatures and fans (`0xC330`–`0xC337`) | Primary S1 |
| NCT6776D | Same row as NCT6776F (S1 covers both packages; the chip ID does not distinguish them) | Primary S1 |
| NCT6775F | Enabled, Experimental: temperatures only (`0xB470`–`0xB477`); fans Unsupported | Lead-only S8, S9, S11, S12; no datasheet obtained |
| NCT6771F | Same row as NCT6775F (S10 groups it with NCT6775F; S11 labels the ID range NCT6771F) | Lead-only |
| NCT6102D, NCT6106D | Unsupported: datasheet chip ID conflicts with every lead; candidate profile recorded below | S2 vs S8, S11, S12 |
| NCT6104D | Unsupported: not in S2 revision 1.0; leads group it with NCT6102D/NCT6106D | S10, S11 |
| NCT5572D, NCT5573D, NCT5577D, NCT6772F, W83677HG-I | Not separate targets; no label is surfaced for them. If one reports an enabled ID it is read with that row's profile (see Open questions) | S10, S12 |

## Detection

### Chip identity

| Fact | Source |
| --- | --- |
| Enter configuration mode with two writes of `0x87` to the index port, select registers by writing the index to the index port and reading the data port, exit with a write of `0xAA`. The configuration pair is `0x2E`/`0x2F` or `0x4E`/`0x4F` according to a strap. | S1 pp. 52–55; S2 pp. 46–47; S3 |
| NCT6776F/NCT6776D: `CR20` (read only) is `0xC3`. `CR21` is the low byte of the chip ID with value `3X`, where `X` is the IC version (the datasheet's examples: `31` = A, `32` = B, `33` = C; the CR21 description itself gives `33h`). | S1 p. 304, p. 310 |
| NCT6102D/NCT6106D revision 1.0 datasheet: `CR20` = `0x10`, `CR21` = `0x61`. | S2 p. 303 |
| Lead-only: NCT6775F-family IDs `0xB470`–`0xB477` (mask `0xFFF8`, S8); `0xB470`–`0xB47F` (S11); exact `0xB472` and `0xB473` (S12). | S8, S11, S12 |
| Lead-only: NCT6776F `0xC330`–`0xC337` (S8), `0xC330`–`0xC33F` (S11), exact `0xC332` and `0xC333` (S12). | S8, S11, S12 |
| Lead-only: NCT610x `0xC450`–`0xC457` (S8), `0xC450`–`0xC45F` (S11), exact `0xC452` (S11, S12). This conflicts with S2. | S8, S11, S12 |

### ID-matching policy

| Raw ID | Row | Chip label to surface |
| --- | --- | --- |
| `0xC331`, `0xC332`, `0xC333` | NCT6776 profile | `NCT6776F / NCT6776D` |
| `0xC330`, `0xC334`–`0xC337` | NCT6776 profile | `NCT6776 family (unrecognized version)` |
| `0xC338`–`0xC33F` | Unsupported | — |
| `0xB472`, `0xB473` | NCT6775 temperature profile | `NCT6775F / NCT6771F` |
| `0xB470`, `0xB471`, `0xB474`–`0xB477` | NCT6775 temperature profile | `NCT6775F family (unrecognized version)` |
| `0xB478`–`0xB47F` | Unsupported | — |
| `0x1061`, `0xC450`–`0xC45F` | Unsupported (candidate NCT610x profile, disabled) | — |

`0xC330`–`0xC337` is the intersection of the S1 statement (any `3X`) and
the narrower lead range (S8); `0xC338`–`0xC33F` stays Unsupported because
no lead or dump places a part there and one lead's family range ends at
`0xC337`. `0xB470`–`0xB477` is the range both leads accept. The raw ID
is always recorded in diagnostics.

### Runtime gates

| Condition | Gate kind | Rows | Source |
| --- | --- | --- | --- |
| Raw chip ID in an enabled range of the ID-matching table | mechanism | all | S1 pp. 304, 310; S8, S11, S12 (lead-only for `0xB47x`) |
| Global `CR26` bit 5 (LOCKREG) is `0`. If it is `1` the datasheet states that read/write of the configuration registers is disabled; stop and report the chip unavailable. Never write `CR26`. | safety | NCT6776 | S1 p. 314 |
| The `CR28` bit 4 HM I/O-space-lock gate of `superio-nuvoton-nct67xx.md` is **not** applied. On NCT6776F `CR28` bits 7–3 are reserved and bits 2–0 select the parallel-port mode; no lead applies the lock step to these chips. | (not a gate) | all | S1 p. 320; S8, S11 |
| LDN `0x0B` `CR30` bit 0 is `1` (hardware monitor active). Never write `CR30`. | mechanism | all | S1 p. 374; S8 (lead-only for `0xB47x`) |
| HM base from `CR60` (high) / `CR61` (low) lies in `0x0100`–`0x0FF8` and its low three bits are zero. S1 allows any two-byte-aligned base in `0x100`–`0xFFE`, but PawnIO only authorizes ports inside 8-byte-aligned BAR windows, so for a base whose low three bits are not zero, base + `0x06` (and for some values base + `0x05`) falls outside the window; such a base is reported unavailable, not worked around. | safety | all | S1 p. 374; S4; S5; S8 (lead-only, base aligned down by 8) |
| HM vendor ID reads `0x5CA3` (high byte `0x5C` with bank-select bit 7 = 1, low byte `0xA3` with bit 7 = 0). Otherwise report the chip unavailable. | mechanism | all | S1 p. 99; S9, S11 (lead-only for `0xB47x`) |
| HM configuration register bank 0 `0x40`: bit 0 (Start) is `1` and, on NCT6776, bit 3 (INT_Clear) is `0`. Bit 0 = 0 puts the part in standby; bit 3 = 1 stops monitoring. Either state makes every reading stale: report all inputs unavailable for that sample. Never write `0x40`. | mechanism | all (bit 3 NCT6776 only) | S1 pp. 93–94; S9 (lead-only, bit 0, for `0xB47x`) |
| Per temperature slot: its source-select field is 1, 2 or 3 | mechanism | all | S1 pp. 158–162; S9, S11 (lead-only for `0xB47x`) |
| Per temperature slot: its STOP bit is `0` | mechanism | all slots that have a configuration register in the row | S1 pp. 92, 117, 129, 163–164; S9 (lead-only for `0xB47x`) |
| Per fan input: the pin-function and input-control registers route the pin to the tachometer input (table below) | mechanism | NCT6776 | S1 pp. 98, 309–316, 374; S8 corroborates |

No brand, board, BIOS or package-suffix identity is a runtime condition.

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| NCT6776 source-selected temperatures (SYSTIN, CPUTIN, AUXTIN) on `0xC330`–`0xC337` | **Experimental**. Every register, decode, gate and invalid state is pinned to S1. No hardware dump is available; S12 names `0xC332`/`0xC333` as observed versions (lead-only evidence identities). | Enabled as Experimental |
| NCT6776 direct RPM fans 1–5 on `0xC330`–`0xC337` | **Experimental**, S1 only (S11 corroborates the registers and the floor value) | Enabled as Experimental, per-input behind the pin-routing gate |
| NCT6775F/NCT6771F source-selected temperatures on `0xB470`–`0xB477` | **Experimental, lead-only**: chip identity (S8, S11, S12), LDN B and base (S8), slot registers, 8/9-bit widths, source-select registers and codes 1–3, STOP bits (S9; S11 for registers and codes), Start bit (S9), vendor ID (S9, S11). The register layout matches S1 for NCT6776F except that the bank 6 slots are decoded as 8-bit and the bank 0 slot has no STOP gate. | Enabled as Experimental |
| NCT6775F/NCT6771F fans | **Unsupported**. S9 reads 16-bit counts with a divisor; S11 reads direct RPM pairs at the S1 NCT6776F addresses. No primary source; the leads do not describe the same register path (see Open questions). | Disabled |
| Slot sources other than codes 1–3 (SMBus master 0–7, PECI agent 0/1, PCH sources, BYTE_TEMP; on NCT6775F also AMD SB-TSI and PECI agents) | S1 p. 64 and S2 p. 58: the PECI slot value (Tbase + PECI) "should not be interpreted as the temperature" of the CPU. The SMBus-master, PCH and BYTE_TEMP values depend on firmware programming of the SMBus master, which this document does not specify. | Disabled (slot ignored) |
| NCT610x candidate profile (`0x1061` per S2; `0xC450`–`0xC45F` per leads) | **Unsupported**: the S2 chip ID conflicts with three leads; the lead value cannot be used (ADR 0023) and no observation of `0x1061` exists. | Disabled |
| Voltages, count-based fan decode, alarms, limits, PWM state | Out of scope | Disabled |
| Byte/9-bit temperature plausibility filter (-55 to +125 °C inclusive) | S1 Table 8-1 p. 61 lists the format from +125 °C to -55 °C; same policy as S6 | Enabled on every temperature |

## Register map (facts)

### Configuration space (read in configuration mode)

| Register | Bits | Meaning | Rows | Source |
| --- | --- | --- | --- | --- |
| `0x07` | 7:0 | Logical device select; indexes `0x30` and above address the selected logical device | all | S1 p. 52; S3 |
| `0x20` / `0x21` | 7:0 | Chip ID high / low byte (read only) | all | S1 p. 310; S2 p. 303 |
| `0x1C` | bit 0 | NCT6776: `1` = pin 4 is AUXFANIN1 regardless of `CR24` bit 6 | NCT6776 | S1 pp. 309–310 |
| `0x1C` | bit 1 | NCT6776: `1` = pin 5 is AUXFANIN2 regardless of `CR24` bit 6 | NCT6776 | S1 pp. 309–310 |
| `0x24` | bit 6 | NCT6776: `0` = pin 3 is AUXFANIN0 (and, with `CR1C` bit 0/1 = 0, pins 4/5 are GP01/GP02); `1` = floppy-interface functions (default) | NCT6776 | S1 pp. 311–313 |
| `0x26` | bit 5 | LOCKREG: `0` = configuration registers read/write enabled; `1` = disabled. Bit 6 (HEFRAS) reports the configuration-port strap. | NCT6776 | S1 p. 314 |
| `0x27` | bit 7 | NCT6776: `1` = pins 88–93 are GP75–GP70 (pin 93 GP70, pin 91 GP72, pin 90 GP73); `0` = VID/deep-sleep functions selected by straps | NCT6776 | S1 pp. 315–316 |
| `0x28` | 7:0 | NCT6776: bits 7–3 reserved, bits 2–0 parallel-port mode. Not an HM lock. Not read. | NCT6776 | S1 p. 320 |
| LDN B `0x30` | bit 0 | `1` = Hardware Monitor & SB-TSI active | all | S1 p. 374 |
| LDN B `0x30` | bits 7, 6, 5 | NCT6776: AUXFANIN0 / AUXFANIN1 / AUXFANIN2 source select; `0` = dedicated pin (3 / 4 / 5), `1` = GPIO pin (GP70 pin 93 / GP72 pin 91 / GP73 pin 90) | NCT6776 | S1 p. 374 |
| LDN B `0x60` / `0x61` | 15:0 | HM base address, high byte / low byte, `0x100`–`0xFFE` on a two-byte boundary | all | S1 pp. 87, 374 |
| LDN B `0x62` / `0x63` | 15:0 | SB-TSI base. Not used. | NCT6776 | S1 p. 374 |

NCT6776 LDN B has no read-only HM base register (`CR64/65`). (S1 p. 374)

### Hardware-monitor ports and bank select

| Item | Meaning | Source |
| --- | --- | --- |
| base + `0x05` | Address (index) port | S1 pp. 56–57, 87 |
| base + `0x06` | Data port | S1 pp. 56–57, 87 |
| Index `0x4E` (bank 0) | Bank select. Bits 2:0 select bank 0–7; bit 7 (HBACS, default `1`) selects the high (`1`) or low (`0`) byte of index `0x4F`; bits 6:3 reserved. A bank-select write writes the bank number in bits 2:0 and bit 7 as stated in the procedure; reserved bits are written as 0. | S1 p. 99 |
| Index `0x4F` (bank 0) | Nuvoton vendor ID, read only: `0x5C` when `0x4E` bit 7 = 1, `0xA3` when bit 7 = 0 | S1 p. 99 |

### NCT6776 temperature slots

| Slot | Source select (bits 4:0) | Reading | Width / decode | STOP bit (`1` = slot not monitored) | Source |
| --- | --- | --- | --- | --- | --- |
| SMIOVT1 | bank 6 `0x21` (default `0x01` SYSTIN) | bank 0 `0x27` | 8-bit two's complement, 1 °C | bank 0 `0x18` bit 0 | S1 pp. 62–63, 92–93, 158 |
| SMIOVT2 | bank 6 `0x22` (default `0x02` CPUTIN) | bank 1 `0x50` = temp<8:1>, bank 1 `0x51` bit 7 = temp<0> | 9-bit two's complement, 0.5 °C | bank 1 `0x52` bit 0 | S1 pp. 116–117, 159 |
| SMIOVT3 | bank 6 `0x23` (default `0x03` AUXTIN) | bank 2 `0x50`, bank 2 `0x51` bit 7 | 9-bit, 0.5 °C | bank 2 `0x52` bit 0 | S1 pp. 128–129, 159–160 |
| SMIOVT4 | bank 6 `0x24` (default `0x01`) | bank 6 `0x2B`, bank 6 `0x2E` bit 0 | 9-bit, 0.5 °C | bank 6 `0x28` bit 0 | S1 pp. 160–165 |
| SMIOVT5 | bank 6 `0x25` (default `0x01`) | bank 6 `0x2C`, bank 6 `0x2E` bit 1 | 9-bit, 0.5 °C | bank 6 `0x29` bit 0 | S1 pp. 161–165 |
| SMIOVT6 | bank 6 `0x26` (default `0x01`) | bank 6 `0x2D`, bank 6 `0x2E` bit 2 | 9-bit, 0.5 °C | bank 6 `0x2A` bit 0 | S1 pp. 162–165 |

Source-select codes (bits 4:0, identical in all six registers):
`0x01` SYSTIN, `0x02` CPUTIN, `0x03` AUXTIN, `0x04`–`0x0B` SMBus master
0–7, `0x0C` PECI agent 0, `0x0D` PECI agent 1, `0x0E` PCH_CHIP_CPU_MAX_TEMP,
`0x0F` PCH_CHIP_TEMP, `0x10` PCH_CPU_TEMP, `0x11` PCH_MCH_TEMP,
`0x12`–`0x15` PCH_DIM0–3_TEMP, `0x16` BYTE_TEMP. `0x00` and `0x17`–`0x1F`
are not defined. Bits 7:5 are reserved. (S1 pp. 158–162)

Table 8-1 examples (S1 p. 61): 8-bit `0x7D` = +125, `0x19` = +25,
`0x01` = +1, `0x00` = 0, `0xFF` = -1, `0xE7` = -25, `0xC9` = -55; 9-bit
`0x0FA` = +125, `0x032` = +25, `0x002` = +1, `0x001` = +0.5, `0x000` = 0,
`0x1FF` = -0.5, `0x1FE` = -1, `0x1CE` = -25, `0x192` = -55.

The datasheet also lists bank 0 `0x73`–`0x78` "monitor temperature"
registers; they report the fan-control temperature of each fan output,
whose source is chosen separately by bank 1/2/3 `0x00`. They are not read.
(S1 pp. 67, 101–103)

### NCT6776 fan inputs

| Fan label | Input | Direct RPM high / low (bank 6) | 13-bit count high / low (bank 6, not read) | Pin-routing gate | Source |
| --- | --- | --- | --- | --- | --- |
| Fan 1 | SYSFANIN | `0x56` / `0x57` | `0x30` / `0x31` | bank 0 `0x4D` bit 0 (FANINC1) = `1` | S1 pp. 65, 98, 165, 173 |
| Fan 2 | CPUFANIN | `0x58` / `0x59` | `0x32` / `0x33` | bank 0 `0x4D` bit 2 (FANINC2) = `1` | S1 pp. 65, 98, 166, 174 |
| Fan 3 | AUXFANIN0 | `0x5A` / `0x5B` | `0x34` / `0x35` | LDN B `CR30` bit 7 = 0 and `CR24` bit 6 = 0, or `CR30` bit 7 = 1 and `CR27` bit 7 = 1 | S1 pp. 65, 166, 174, 311–316, 374 |
| Fan 4 | AUXFANIN1 | `0x5C` / `0x5D` | `0x36` / `0x37` | LDN B `CR30` bit 6 = 0 and `CR1C` bit 0 = 1, or `CR30` bit 6 = 1 and `CR27` bit 7 = 1 | S1 pp. 65, 167, 175, 309–310, 315–316, 374 |
| Fan 5 | AUXFANIN2 | `0x5E` / `0x5F` | `0x38` / `0x39` | LDN B `CR30` bit 5 = 0 and `CR1C` bit 1 = 1, or `CR30` bit 5 = 1 and `CR27` bit 7 = 1 | S1 pp. 65, 167, 175, 309–316, 374 |

| Fact | Source |
| --- | --- |
| The 16-bit RPM reading is read high byte first, then low byte; its value is the RPM (example `0x09C4` = 2500 RPM). The datasheet documents no latch for the pair, so a change between the two reads can return a torn value for that sample (for example `0x00FF` → `0x0100` read as `0x0000`, which the decode reports as 0 RPM). | S1 p. 65 |
| Power-on default of every RPM pair is `0x00A4` (high `0x00`, low `0xA4`). | S1 pp. 65, 173–175 |
| Power-on default of every 13-bit count is `0x1FFF` (high `0xFF`, low bits 4:0 `0x1F`), the largest count. RPM = 1.35 × 10⁶ / count, so the largest count corresponds to 164.8 RPM; `0xA4` = 164. | S1 pp. 65, 165–167 |
| Lead-only corroboration: S11 treats a direct RPM value at or below 1.35 × 10⁶ / `0x1FFF` as zero on NCT6776F. | S11 |

### Status registers that clear on read (never read)

| Register | Attribute | Source |
| --- | --- | --- |
| NCT6776 bank 0 `0x41`, `0x42`, `0x45` (interrupt status 1, 2, 4) | Read Clear | S1 pp. 94–96 |
| NCT6776 bank 4 `0x41` (SMI_TEMP4–6 interrupt status), bank 4 `0x50` (interrupt status 3) | Read Clear | S1 pp. 150–151 |
| NCT610x bank 0 `0x70`–`0x76` (interrupt status 1–7) | Datasheet says Read Only and does not state whether reading clears them; treated as never read | S2 pp. 100–103 |

## NCT6775F/NCT6771F temperature profile (lead-only)

Every fact in this section is lead-only and backs only the Experimental
NCT6775 temperature row.

| Fact | Source |
| --- | --- |
| Hardware monitor at LDN `0x0B`, active bit `CR30` bit 0, base `CR60/61`, index/data at base + 5 / + 6, bank select at index `0x4E`. | S8, S11 |
| Vendor ID `0x5CA3` at bank 0 `0x4F`, byte selected by bank-select bit 7. | S9, S11 |
| Bank 0 `0x40` bit 0 starts monitoring. | S9 |
| Six slots with the NCT6776 addresses and source-select registers bank 6 `0x21`–`0x26` bits 4:0: bank 0 `0x27` (8-bit); bank 1 `0x50` + `0x51` bit 7 and bank 2 `0x50` + `0x51` bit 7 (9-bit); bank 6 `0x2B`, `0x2C`, `0x2D` (8-bit in S9; S11 adds a half bit from bank 6 `0x2E`, not adopted). | S9, S11 |
| STOP bits: bank 1 `0x52` bit 0, bank 2 `0x52` bit 0, bank 6 `0x28`–`0x2A` bit 0. No configuration register is used for the bank 0 slot. | S9 |
| Source codes 1 = SYSTIN, 2 = CPUTIN, 3 = AUXTIN. Code 4 is AMD SB-TSI and codes 5–12 PECI agents 0–7 (S9); S11 has PECI 0 at code 5. | S9, S11 |
| `CR28` lock step not applied; on an inactive LDN B the kernel lead writes `CR30` (not adopted). | S8 |

## Read procedure and decode

### Detection (cached; configuration mode)

1. Acquire `Global\Access_ISABUS.HTP.Method` with a bounded timeout; on
   failure skip the attempt. Hold it for the whole transaction. (S3, S4)
2. Select the slot (`ioctl_select_slot`), enter configuration mode, read
   `CR20`/`CR21`. Absent-ID rules of S3 apply. Match the ID-matching
   table; any other ID is outside this document. (S1, S3)
3. NCT6776 only: read `CR26`; if bit 5 is set, stop (chip unavailable).
   Read `CR1C`, `CR24` and `CR27` for the fan pin-routing gates. (S1)
4. Select LDN `0x0B` (write `CR07`), read `CR30`; require bit 0 = 1. On
   NCT6776 keep bits 7–5 for the pin-routing gates. (S1 p. 374; S8)
5. Read `CR60` and `CR61`; apply the base gate. (S1, S4, S5)
6. Call `ioctl_find_bars` (after steps 2–5, as required by S6). (S5, S6)
7. Through base + 5 / + 6: write `0x4E` = `0x80` (bank 0, high byte) and
   read `0x4F`; write `0x4E` = `0x00` and read `0x4F`; require `0x5C` /
   `0xA3`. (S1 p. 99; S9, S11 for `0xB47x`)
8. Exit configuration mode (`0xAA`) and release the mutex on every path,
   including failures. (S3)
9. Cache the profile, the base, and on NCT6776 the set of routed fan
   inputs. Configuration mode is not entered again for sampling. (S3)

### Rediscovery eligibility

Detection follows the S3
[sampling-path rediscovery cadence](superio-access.md#sampling-path-rediscovery-cadence).
A rediscovery is a separate detection transaction under its own ISA
mutex acquisition. No sample transaction enters configuration mode, and
a failed sample starts no detection. The S3 shared table covers the
mutex, the executor, `ioctl_find_bars` and configuration-register read
failures. The configuration reads include `CR1C`, `CR24`, `CR26` and
`CR27` in step 3.

| Detection failure | Class | Reason |
| --- | --- | --- |
| The `0x87`, `0x87` entry-key write or the `0xAA` exit write fails, so the exit is not confirmed (steps 2, 8) | Eligible | The chip may still be in configuration mode, and a later attempt is the only point at which the exit is issued again (S3). |
| Step 7: an HM port write or read for the vendor ID fails | Eligible | The ports were reached only after `ioctl_find_bars` in step 6. Under the S3 shared table, that success does not prove a usable window, and a later attempt runs step 6 again (S3). |
| Step 7: the vendor ID reads but is not `0x5CA3` | Not eligible | A value read successfully (S1 p. 99). |
| `CR26` bit 5 set (step 3), `CR30` bit 0 clear (step 4), or a refused base (step 5) | Not eligible | A value read successfully (S1). |
| Absent responder or a chip ID outside the ID-matching table | Not eligible | The probe completed. |

### Sampling (HM ports only, under the ISA mutex)

1. Bank 0: read `0x40`; apply the monitoring gate (bit 0 = 1; NCT6776
   also bit 3 = 0). If it fails, report every input unavailable for this
   sample and stop. (S1 pp. 93–94; S9)
2. Bank 0: read the SMIOVT1 reading `0x27`; NCT6776 also `0x18` (STOP)
   and `0x4D` (fan input control). Bank 1: `0x50`, `0x51`, `0x52`.
   Bank 2: `0x50`, `0x51`, `0x52`. Bank 6: `0x21`–`0x26`, `0x28`–`0x2A`,
   `0x2B`–`0x2D`; NCT6776 also `0x2E` and the RPM pairs of the routed
   fan inputs (`0x56`–`0x5F`, high byte before low byte). Select each
   bank by writing `0x4E` = bank number (bit 7 = 0) before its reads.
   No other register and no other bank is read. (S1; S9 for `0xB47x`)
3. Any failed index write, bank-select data write or data read aborts the
   sample; do not publish values from a partial transaction. (S6)
4. Release the mutex.

### Temperature decode

- A slot is a candidate only when its source field (bits 4:0) is 1, 2 or
  3 and its STOP bit (where the row has one) is 0. Bits 7:5 of the
  source register are ignored. (S1; S9)
- 8-bit slots: temperature = the reading byte as signed 8-bit, in °C.
- 9-bit slots: v = (high byte × 2) + bit; if v ≥ 256, v = v − 512;
  temperature = v × 0.5 °C. Equivalently, the high byte as signed 8-bit
  plus 0.5 when the low bit is set. (S1 pp. 61, 117, 129, 165)
- NCT6775 row: the bank 6 slots are 8-bit (the `0x2E` bits are not used).
  (S9)
- Each label (`SYSTIN`, `CPUTIN`, `AUXTIN`) is published once. If several
  candidate slots select the same source, use the first candidate in the
  order SMIOVT2, SMIOVT3, SMIOVT4, SMIOVT5, SMIOVT6, SMIOVT1 (9-bit slots
  before the 8-bit slot). On the NCT6775 row the order is SMIOVT2,
  SMIOVT3, then SMIOVT1, SMIOVT4, SMIOVT5, SMIOVT6. A source that no
  candidate slot selects is unavailable for that sample.
- Plausibility: publish only -55 ≤ t ≤ +125 °C inclusive; otherwise the
  input is unavailable for that sample (not clamped, not zero, not a
  device failure). If the preferred slot of a source is implausible, the
  source is unavailable; another slot is not substituted. (S1 p. 61; S6)
- The high byte and the low bit are separate reads; the datasheets
  document no latch, so a change between the two reads can combine bytes
  from different readings for that sample. No bound on that error is
  documented.

### Fan decode (NCT6776)

- RPM = high × 256 + low. (S1 p. 65)
- A value at or below `0x00A4` (164) is reported as 0 RPM
  (stopped, unconnected, or below the measurable range); this includes
  `0x0000` and the power-on default. (S1 pp. 65, 165–175; S11
  corroborates)
- A fan input whose pin-routing gate fails is not read and not reported
  as a measurement; a position-keyed consumer reports it as an explicit
  unavailable slot at its position, as required by S6.
- Do not read the count registers or the pulse-per-revolution registers.

### Input identity and ordering

Labels are bound to physical inputs for every row and revision:
temperatures `SYSTIN`, `CPUTIN`, `AUXTIN` (keyed by source code, not by
slot); fans `Fan 1`–`Fan 5` = SYSFANIN, CPUFANIN, AUXFANIN0, AUXFANIN1,
AUXFANIN2 (the same numbering as `Fan 1`–`Fan 5` in S6). The S6
unavailable-slot rule applies. Source label: the existing generic Nuvoton
Super I/O source label. (S6)

## NCT610x candidate profile (disabled)

Recorded from S2 so that a later revision can enable it once the chip-ID
conflict is resolved. Nothing in this section is enabled.

| Item | Fact | Source |
| --- | --- | --- |
| Chip ID | `CR20` = `0x10`, `CR21` = `0x61` (S2); leads: `0xC450`–`0xC457`, `0xC450`–`0xC45F`, exact `0xC452` | S2 p. 303; S8, S11, S12 |
| Lock / base | `CR26` bit 5 LOCKREG; LDN B `CR30` bit 0 active; `CR60/61` HM base `0x100`–`0xFFE`, two-byte boundary | S2 pp. 307, 362 |
| Banks | Bank select `0x4E` bits 1:0 (banks 0–3), bit 7 HBACS; indexes `0x00`–`0x4F` are global to all banks | S2 pp. 83, 92 |
| Vendor ID | Bank 0 `0xFE`, `0x5C`/`0xA3` by bank-select bit 7 | S2 p. 138 |
| Monitoring | Bank 3 `0x17`: bit 0 Start, bit 3 INT_Clear | S2 p. 187 |
| Temperature slots | Readings bank 0 `0x10`–`0x15` = temp<8:1> of SMIOVT1–6; bank 0 `0x16` bits 0–5 = temp<0> of SMIOVT1–6; 9-bit, 0.5 °C; sources bank 0 `0xB0`–`0xB5` bits 4:0 with the S1 code table (defaults SYSTIN, CPUTIN, AUXTIN, SYSTIN, SYSTIN, SYSTIN); slot control `0xB7`–`0xBC` | S2 pp. 57, 84–89, 116–121 |
| Fans | Three inputs; direct RPM bank 0 `0x30`/`0x31` SYSFANIN, `0x32`/`0x33` CPUFANIN, `0x34`/`0x35` AUXFANIN (high then low); counts `0x20`–`0x25`; AUXFANIN is on pin 6 only when `CR24` bit 7 = 0; FAN IN/OUT control bank 0 `0xF8` bits 0 and 2 | S2 pp. 60, 85, 90–92, 137, 304–306 |
| Never read | Bank 0 `0x70`–`0x76` | S2 pp. 100–103 |

## Quirks

- NCT6776 `0x00A4` RPM floor: a stopped or unconnected fan is expected
  to read the RPM of the saturated count rather than zero. (S1 pp. 65,
  165–175)
- Unconnected thermistor or diode inputs can read an in-range value; the
  plausibility filter only removes out-of-range bytes. Slot sources and
  wiring are firmware choices; labels name the chip input, not a board
  location. (S1 pp. 61–63)
- NCT6776 bank 4 `0x54`–`0x56` hold per-input temperature offsets that
  the chip adds to its readings; they are not read. (S1 pp. 151–152)
- The PECI value in a slot is Tbase + PECI counts and is not a
  temperature. (S1 p. 64; S2 p. 58)

## Safety notes

- Writes allowed, all read-transaction plumbing: configuration enter
  (`0x87`, `0x87`) and exit (`0xAA`); `CR07` = `0x0B`; HM index-port
  writes; HM bank-select writes to `0x4E` with value `0x80` or `0x00`
  (vendor ID) or the bank number `0x00`, `0x01`, `0x02`, `0x06`. The
  `ioctl_find_bars` call selects logical devices inside the module.
  (S1, S3, S4, S5)
- Never write `CR26`, `CR30`, any other configuration register, HM
  `0x40` (its bit 7 restores power-on defaults), STOP bits, source
  selects, fan-input control, divisors, limits, PWM, PECI or SMBus-master
  registers. (S1 pp. 93, 314, 374)
- Never read the read-to-clear status registers listed above, and never
  read banks 3, 4, 5 or 7 on NCT6776. (S1)
- Do not apply the `CR28` lock write or gate of the newer chips. (S1
  p. 320; S8)
- Hold the ISA mutex across every multi-step transaction; never proceed
  unlocked. (S3, S4)

## Open questions

- Non-blocking for #2521: NCT610x identity. S2 revision 1.0 states
  `0x1061`; S8, S11 and S12 place the parts at `0xC45x` (exact `0xC452`).
  The primary value wins over the leads, but no observation of `0x1061`
  exists, so the whole candidate profile stays disabled. Resolution: the
  revision 1.52 datasheet (July 29, 2013), whose third-party copies were
  unreachable on 2026-11-28, or a maintainer-accepted dump showing
  `CR20/21`, LDN B base and the bank 0 `0xFE` vendor ID.
- Non-blocking for #2521: NCT6775F/NCT6771F fans. S9 decodes 16-bit
  counts at bank 6 `0x30`–`0x37` with a divisor from bank 5 `0x06`/`0x07`;
  S11 reads direct RPM pairs at bank 6 `0x56`–`0x5D`. Neither is primary
  and they describe different register paths; S11 shares one table for
  NCT6771F and NCT6776F. Fans stay Unsupported until a datasheet or a
  dump with a known fan speed settles the path and the four-input pin
  routing.
- Non-blocking for #2521: NCT6775 bank 6 half-degree bit. S11 reads the
  bank 6 `0x2E` bits for NCT6771F; S9 treats those slots as 8-bit. The
  row uses 8-bit decode, which can only lose 0.5 °C of resolution.
- Non-blocking for #2521: other parts sharing the IDs. S10 lists
  NCT5573D/NCT5577D with NCT6776F and NCT5572D/NCT6772F/W83677HG-I with
  NCT6775F, and S12 labels `0xB473` NCT6775F (B) / NCT5572D (B). Such a
  part reporting an enabled ID is read with that row's profile and the
  NCT6776/NCT6775 label; its pin-routing registers may differ. The
  vendor-ID, Start, STOP, source-code, plausibility and RPM-floor rules
  bound the effect. A dump from such a board can add a row.
- Non-blocking for #2521: NCT6776 IDs `0xC338`–`0xC33F`. S1 assigns every
  `CR21` = `3X` to NCT6776F/D, but S8's family range ends at `0xC337` and
  no version above C is attested. They stay Unsupported; a dump can
  enable them.
- Non-blocking for #2521: whether configuration reads still return valid
  data while `CR26` bit 5 is set is not stated by S1. The gate stops the
  attempt in that state, so the answer changes no published value.
- Non-blocking for #2521: RPM floor and pulse setting. The `0x00A4` floor
  is derived for the default count range; S1 does not state how the
  per-input revolution-pulse setting (bank 6 `0x44`–`0x46`) scales the
  RPM register. A real fan below 165 RPM would be reported as 0.
- Non-blocking for #2521: SMBus-master, PCH and BYTE_TEMP slot sources
  carry values read from other devices by firmware-programmed SMBus
  transactions. They stay disabled; a later revision can specify them.
- Non-blocking for #2521: the bank-select register is left at the last
  bank written, as in S6. Whether firmware that touches the HM without
  the ISA mutex assumes bank 0 is unknown.
- Non-blocking for #2521: no hardware dump exists for any row. A row
  graduates to Verified only when a maintainer-accepted dump confirms the
  chip ID, `CR26`, LDN B base, vendor ID, `0x40`, the source selects and
  readings, and (NCT6776) the pin-routing registers and RPM pairs; the
  NCT6775 row additionally needs a primary source or dump for every
  lead-only fact listed in its Status.
- Non-blocking for #2521: LibreHardwareMonitor's NCT610x source-code and
  half-bit tables (S11) differ from S2 (for example code 1 = CPUTIN in
  S11, SYSTIN in S2). S2 wins; the candidate profile records S2.

## Implementation-ready transition checklist

- No unresolved provenance marker remains; every NCT6776 fact is pinned to a
  printed page of S1, every NCT610x fact to S2, and every access fact to
  S3–S6.
- Every lead-only fact is tagged (S8–S12 carry `lead-only (copyleft)`)
  and backs only the Experimental NCT6775 temperature row, whose Status
  names the facts and sources; the NCT6776 rows do not depend on any
  lead-only fact.
- Every runtime condition in Detection carries a gate kind; no evidence
  identity gates a read; rows are keyed by mechanism.
- Every open question is annotated non-blocking.
- Revision, status and history record the change. The flip to
  Implementation-ready takes effect with maintainer approval of the PR
  that introduces this revision.

Implementation PRs pin this revision, for example:

```text
Implemented from docs/specs/sensors/superio-nuvoton-nct6775-nct6776-nct610x.md revision 2 (commit <sha>), limited to the NCT6776 and NCT6775 rows marked enabled.
Implemented from docs/specs/sensors/superio-access.md revision 5 (commit <sha>), limited to the configuration access facts and the sampling-path rediscovery cadence.
No other external sensor documentation was used.
```

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-28 | Initial version for #2521. Experimental NCT6776F/NCT6776D (`0xC330`–`0xC337`) source-selected temperatures and five direct RPM fans from the vendor datasheet, with `CR26` LOCKREG, LDN B, aligned-base, vendor-ID, Start/INT_Clear, per-slot source and STOP, and per-fan pin-routing gates and the `0x00A4` RPM floor; lead-only Experimental NCT6775F/NCT6771F (`0xB470`–`0xB477`) temperatures; NCT6775-family fans and every NCT610x ID Unsupported with a recorded candidate profile; read-to-clear status registers listed. |
| 2 | 2026-11-28 | Re-pinned S3 to `superio-access.md` revision 5 for #2821, whose configuration access facts are unchanged, and adopted its sampling-path rediscovery cadence. Added [Rediscovery eligibility](#rediscovery-eligibility). An unconfirmed configuration exit and an HM vendor-ID port failure in step 7 are eligible. A vendor-ID mismatch, `CR26` LOCKREG, an inactive LDN B, a refused base and the shared configuration-read and `ioctl_find_bars` failures are not eligible. A rediscovery is a separate detection transaction, and a failed sample starts none. Profiles, gates, reads, writes and decodes are unchanged. Status remains Implementation-ready (rev 2). |
