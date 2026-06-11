# Spec: AMD Zen CPU temperature (Tctl/Tdie) via SMN thermal controller

| Field | Value |
| --- | --- |
| Revision | 2 |
| Status | Implementation-ready (rev 2) |
| Scope | Package control temperature (Tctl) and die temperature (Tdie) on AMD Family 17h (Zen/Zen+/Zen 2) and Family 19h (Zen 3/Zen 4) processors, read from the SMU thermal controller (THM) over the System Management Network (SMN). Family 1Ah (Zen 5) is recognized but disabled by default pending verification (see Detection). Excludes: per-CCD temperatures, SMU PM-table metrics, pre-Zen (Family 15h/16h) thermal registers. |
| Issue phase | Phase 1 (#1635) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD, *PPR for AMD Family 19h Model 01h, Revision B1 Processors*, document no. **55898 Rev 0.50 (May 27, 2021), Vol 2, §10.3 Table 124** (p. 2-179): `SMU::THM::THM_TCON_CUR_TMP` at SMUTHM aperture `0005_9800h` | Primary; register, address, Tctl description |
| S2 | AMD, *Open-Source Register Reference (OSRR) for AMD Family 17h*, **§4.2.1**: bits 31:21 current temperature; bit 19 clear → 0…225 °C, set → −49…206 °C. Verified via the verbatim citation carried in S8 | Primary (AMD document; quote carried by a non-copyleft source) |
| S3 | AMD public statements on Tctl offsets: "AMD Ryzen™ Community Update #3" (R. Hallock, Apr 2017) and 2nd-gen Ryzen launch documentation | Primary for offsets |
| S4 | Linux `k10temp` hwmon driver (GPL-2.0) | **Non-normative corroboration only.** No fact in this document relies solely on this source. No code, structure, identifiers, or tables were copied. |
| S5 | PawnIO `RyzenSMU.p` module source (LGPL-2.1-or-later) | Upstream-published interface definition of the module this project calls across the IOCTL boundary (allowed SMN windows, family gate, caller-mutex `@warning` docs). Not used as a source for any hardware register fact. No code was copied. |
| S6 | AMD, `asic_reg/thm/thm_9_0_sh_mask.h` (MIT, © 2017 AMD): `CUR_TEMP_MASK 0xFFE00000` (bits 31:21), `CUR_TEMP_RANGE_SEL_MASK 0x00080000` (bit 19), `CUR_TEMP_TJ_SEL` (bits 17:16), `CUR_TEMP_TJ_SLEW_SEL` (bit 18) | Primary; AMD-published, permissively licensed field masks |
| S7 | PPR 55898 Vol 2 **§6.2.5–6.2.6 (SB-TSI)**: SB-TSI delivers Tctl encoded in 0.125 °C increments spanning 0–255.875 | Primary; scaling corroboration (11-bit × 0.125 °C = 255.875) |
| S8 | FreeBSD `amdtemp(4)` driver, `sys/dev/amdtemp/amdtemp.c` (BSD-2-Clause) | Non-copyleft engineering source: carries the S2 OSRR quote, the `TJ_SEL` 49 °C note (attributed to an AMD-authored Linux patch), and CCD-register leads. Facts only; no code copied. |

## Detection

| Fact | Source |
| --- | --- |
| CPU vendor string is `AuthenticAMD` (CPUID leaf 0) | AMD APM / CPUID convention |
| Effective family = `BaseFamily + ExtendedFamily` (CPUID leaf 1; extended family is added when base family is `0xF`) | AMD CPUID convention |
| The PawnIO `RyzenSMU` module accepts families `0x17`, `0x19`, `0x1A` and rejects other vendors/families with an error status, providing a second layer of gating | S5 |

"Recognized by the PawnIO module" and "enabled by this project" are
separate decisions. This spec enables a family only once its THM
register facts are verified against a primary source:

| Family | Status | Default enablement |
| --- | --- | --- |
| `0x17` | Layout and ranges verified against AMD OSRR §4.2.1 (S2) and the AMD register header (S6) | Enabled |
| `0x19` | Register, address, and Tctl description verified against PPR 55898 Vol 2 §10.3 (S1) | Enabled |
| `0x1A` | Recognized by the PawnIO module (S5) but not yet verified by this spec | Disabled until PPR/OSRR or hardware-dump verification |

## Register map (facts)

Access is performed with the PawnIO `RyzenSMU` module's
`ioctl_read_smu_register` (input: SMN address; output: 32-bit value).
The module's allowed SMN window `0x56000`–`0x5AFFF` contains this
register. See [`pawnio-interface.md`](pawnio-interface.md).

| SMN address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x00059800` | `SMU::THM::THM_TCON_CUR_TMP` | 31:21 | `CUR_TEMP` — "current control temperature (Tctl) after the slew-rate controls have been applied" | 0.125 °C per LSB, unsigned 11-bit | S1, S2, S6, S7 |
| `0x00059800` | `SMU::THM::THM_TCON_CUR_TMP` | 19 | `CUR_TEMP_RANGE_SEL` — selects the reporting range | 0 → 0…225 °C; 1 → −49…206 °C | S2, S6 |
| `0x00059800` | `SMU::THM::THM_TCON_CUR_TMP` | 17:16 | `CUR_TEMP_TJ_SEL` — `11b` marks `CUR_TEMP` as software-writable, and the 49 °C range adjustment then applies (see Quirks) | flag pair | S6 (bits); S8 (semantics) |

Notes:

- S1 defines the register at SMUTHM offset `0` with the SMUTHM
  aperture at `0005_9800h`, i.e. absolute SMN address `0x00059800`,
  and describes it as providing "the current control temperature
  (Tctl) after the slew-rate controls have been applied".
- The 0.125 °C/LSB scaling follows from S7: SB-TSI delivers the same
  Tctl in 0.125 °C increments spanning 0–255.875, which is exactly
  the unsigned 11-bit field range (2047 × 0.125 °C).
- Layout verified for Family 17h via S2/S6 and for Family 19h via S1.
  Family 1Ah is not yet verified by this spec and stays disabled by
  default — see Detection and Open questions.
- SMN is reached through an index/data register pair in the host
  bridge PCI configuration space; the PawnIO module performs that
  access, and the **client must hold `Global\Access_PCI`** around
  each call (caller-mutex requirement, S5; see
  [`pawnio-interface.md`](pawnio-interface.md)). The client performs
  no raw PCI configuration access.

## Read procedure and decode

1. Confirm detection facts above (including the per-family enablement
   table).
2. Read the 32-bit value of `THM_TCON_CUR_TMP` from SMN `0x00059800`.
3. Decoding rules (hardware semantics):
   - Extract `CUR_TEMP` from bits 31:21.
   - Tctl = `CUR_TEMP` × 0.125 °C.
   - If `CUR_TEMP_RANGE_SEL` (bit 19) is 1, or `CUR_TEMP_TJ_SEL`
     (bits 17:16) equals `11b`, subtract 49 °C from Tctl. (S2, S8)
   - Tdie = Tctl − the model's Tctl offset (table below); models not
     listed have offset 0, so Tdie equals Tctl.
4. Publish Tdie as the CPU temperature. Plausibility gate (this
   project's own policy, not a PPR fact): accept only
   −40 ≤ Tdie ≤ 120 °C and drop all-zero register reads.

## Quirks

Tctl is a unitless control input deliberately offset above the
measured die temperature on some first/second-generation parts so that
cooling policies stay uniform across the lineup. AMD published the
offsets (S3, normative); `k10temp` corroborates them (S4,
non-normative):

| Product | Tctl − Tdie offset |
| --- | --- |
| Ryzen 7 1700X / 1800X | 20 °C |
| Ryzen Threadripper 1900X / 1920X / 1950X | 27 °C |
| Ryzen 7 2700X | 10 °C |
| Ryzen Threadripper 2920X / 2950X / 2970WX / 2990WX | 27 °C |
| Products without a primary-source documented positive Tctl offset (incl. Zen 2 and newer) | 0 °C, subject to per-family verification before this document reaches implementation-ready status |

- Offset matching is by product name (OPN), not by family/model
  numbers alone. (S3; corroborated by S4)
- `CUR_TEMP_RANGE_SEL = 1` (the −49 °C range) is the normal case on
  many desktop parts; the decode in step 3 must always honor the bit
  rather than assume either range. (S2)
- When `CUR_TEMP_TJ_SEL` (bits 17:16) reads `11b`, `CUR_TEMP` carries
  a software-written value and the 49 °C adjustment applies even with
  `RANGE_SEL` clear. S8 carries this as an engineering note
  attributed to an AMD-authored Linux patch; Phase 2 dumps will
  exercise it on real hardware.
- A read returning all zeros indicates a failed SMN transaction rather
  than 0 °C (or −49 °C); treat as an invalid sample. (Project policy;
  consistent with the plausibility gate.)

## Safety notes

- Read-only: only `ioctl_read_smu_register` is used. The module's SMU
  write/command surfaces (`ioctl_write_smu_register`,
  `ioctl_send_smu_command`) are never invoked by this project.
- The module performs no locking itself; every SMU ioctl is
  documented with a caller-must-hold warning for
  `\BaseNamedObjects\Access_PCI`. The client therefore holds
  `Global\Access_PCI` (bounded timeout; on timeout the sample is
  skipped) around each `ioctl_read_smu_register` call. (S5)

## Future extensions (recorded, not yet specified)

- Per-CCD temperature registers exist in the same THM block; their
  addresses vary by model and need PPR/OSRR or dump verification
  before a spec revision adds them. (S8 records
  experimentally-derived bases `0x59954` (Zen 2) / `0x59b08` (Zen 4)
  as non-normative leads.)
- SMU PM-table reads (`ioctl_*_pm_table`) expose richer telemetry
  (per-core power, voltages) but are version-dependent; out of scope
  for Phase 1.

## Open questions

- Non-blocking for Phase 1: family 1Ah stays disabled by default via
  the Detection enablement table. Confirm `THM_TCON_CUR_TMP`
  address/layout from an AMD PPR/OSRR when available, or validate
  against a Phase 2 register dump, before enabling it.
- Non-blocking for Phase 1: the reported value is the package control
  temperature that drives cooling policy regardless of die-selection
  details, and Phase 1 targets single-die consumer parts. Whether
  `0x59800` reports the hottest die or a specific die on
  Threadripper / EPYC multi-die parts is verified via Phase 2 dumps.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-06-10 | Initial version |
| 2 | 2026-06-11 | Provenance pinned: PPR 55898 Vol 2 §10.3 (p. 2-179), AMD OSRR 17h §4.2.1, AMD MIT register header, SB-TSI scaling corroboration. Added `CUR_TEMP_TJ_SEL` field and decode rule. Corrected `Access_PCI` ownership to caller-held. Open questions resolved or annotated non-blocking. Status → Implementation-ready. |
