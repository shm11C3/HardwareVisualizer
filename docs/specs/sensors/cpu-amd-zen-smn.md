# Spec: AMD Zen CPU temperature (Tctl/Tdie) via SMN thermal controller

| Field | Value |
| --- | --- |
| Revision | 4 |
| Status | Implementation-ready (rev 4) |
| Scope | Package control temperature (Tctl) and die temperature (Tdie) on AMD Family 17h (Zen/Zen+/Zen 2) and Family 19h (Zen 3/Zen 4) processors, read from the SMU thermal controller (THM) over the System Management Network (SMN). Family 1Ah (Zen 5) is recognized and enabled best-effort as experimental pending verification (see Detection and ADR 0011). Excludes: per-CCD temperatures, SMU PM-table metrics, pre-Zen (Family 15h/16h) thermal registers. |
| Issue phase | Phase 1 (#1635) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD, *PPR for AMD Family 19h Model 01h, Revision B1 Processors*, document no. **55898 Rev 0.50 (May 27, 2021), Vol 2, §10.3 Table 124** (p. 2-179): `SMU::THM::THM_TCON_CUR_TMP` at SMUTHM aperture `0005_9800h` | Primary; register, address, Tctl description |
| S2 | AMD, *Open-Source Register Reference (OSRR) for AMD Family 17h Processors, Models 00h–2Fh*, document no. **56255 Rev 3.03 (July 2018), §4.2.1 "Registers" (p. 243)**: `SMUTHMx00000000 (SMU::THM::THM_TCON_CUR_TMP)`, `SMUTHM=0005_9800h`; bits 31:21 `CUR_TEMP` "Provides current control temperature"; bit 19 `CUR_TEMP_RANGE_SEL` "0=Report on 0C to 225C scale range. 1=Report on -49C to 206C scale range."; bits 18:0 Reserved | Primary; the AMD PDF was fetched (GitHub mirror) and text-verified directly during authoring |
| S3 | AMD public statements on Tctl offsets: "AMD Ryzen™ Community Update #3" (R. Hallock, Apr 2017) and 2nd-gen Ryzen launch documentation | Primary for offsets |
| S4 | Linux `k10temp` hwmon driver (GPL-2.0) | **Non-normative corroboration only.** No fact in this document relies solely on this source. No code, structure, identifiers, or tables were copied. |
| S5 | PawnIO `RyzenSMU.p` module source (LGPL-2.1-or-later) | Upstream-published interface definition of the module this project calls across the IOCTL boundary (allowed SMN windows, family gate, caller-mutex `@warning` docs). Not used as a source for any hardware register fact. No code was copied. |
| S6 | AMD, `asic_reg/thm/thm_9_0_sh_mask.h` (MIT, © 2017 AMD): `CUR_TEMP_MASK 0xFFE00000` (bits 31:21), `CUR_TEMP_RANGE_SEL_MASK 0x00080000` (bit 19) | Primary; AMD-published, permissively licensed field masks, consistent with S2. The header (a GPU THM IP description) additionally defines `CUR_TEMP_TJ_SEL`/`TJ_SLEW_SEL` in bits 18:16, which the CPU OSRR (S2) marks Reserved — see Open questions |
| S7 | PPR 55898 Vol 2 **§6.2.5–6.2.6 (SB-TSI)**: SB-TSI delivers Tctl encoded in 0.125 °C increments spanning 0–255.875 | Primary; scaling corroboration (11-bit × 0.125 °C = 255.875) |
| S8 | FreeBSD `amdtemp(4)` driver, `sys/dev/amdtemp/amdtemp.c` (BSD-2-Clause) | **Non-normative engineering corroboration only** (non-copyleft): carries an OSRR quote (superseded by the direct S2 pin), a `TJ_SEL` 49 °C note attributed to an AMD-authored Linux patch (not adopted — see Open questions), and CCD-register leads. Facts only; no code copied. |

## Detection

| Fact | Source |
| --- | --- |
| CPU vendor string is `AuthenticAMD` (CPUID leaf 0) | AMD APM / CPUID convention |
| Effective family = `BaseFamily + ExtendedFamily` (CPUID leaf 1; extended family is added when base family is `0xF`) | AMD CPUID convention |
| The PawnIO `RyzenSMU` module accepts families `0x17`, `0x19`, `0x1A` and rejects other vendors/families with an error status, providing a second layer of gating | S5 |

"Recognized by the PawnIO module", "verified by this spec", and
"enabled by this project" are separate decisions. A family is marked
**Verified** only once its THM register facts are verified against a
primary source. Per ADR 0011, a family that the `RyzenSMU` module
recognizes but this spec has not yet verified is still enabled
best-effort as an **experimental** path (reusing the verified decode and
plausibility gate) rather than disabled. Successful readings use the normal
presentation contract; if an attempted experimental path fails and the failure
is surfaced, the diagnostic may identify it as experimental:

| Family | Status | Default enablement |
| --- | --- | --- |
| `0x17` | Layout and ranges verified against the directly pinned AMD OSRR 56255 Rev 3.03 §4.2.1, p. 243 (S2) and the AMD register header (S6) | Enabled |
| `0x19` | Register, address, and Tctl description verified against PPR 55898 Vol 2 §10.3 (S1) | Enabled |
| `0x1A` | Recognized by the PawnIO module (S5) but not yet verified by this spec | Enabled best-effort as **experimental** and plausibility-gated per ADR 0011; successful UI readings use the normal source label; graduates to verified once THM facts are pinned |

## Register map (facts)

Access is performed with the PawnIO `RyzenSMU` module's
`ioctl_read_smu_register` (input: SMN address; output: 32-bit value).
The module's allowed SMN window `0x56000`–`0x5AFFF` contains this
register. See [`pawnio-interface.md`](pawnio-interface.md).

| SMN address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x00059800` | `SMU::THM::THM_TCON_CUR_TMP` | 31:21 | `CUR_TEMP` — "Provides current control temperature" (S2); "the current control temperature (Tctl) after the slew-rate controls have been applied" (S1) | 0.125 °C per LSB, unsigned 11-bit | S1, S2, S6, S7 |
| `0x00059800` | `SMU::THM::THM_TCON_CUR_TMP` | 19 | `CUR_TEMP_RANGE_SEL` — "0=Report on 0C to 225C scale range. 1=Report on -49C to 206C scale range." | range select | S2, S6 |

Notes:

- S1 defines the register at SMUTHM offset `0` with the SMUTHM
  aperture at `0005_9800h`, i.e. absolute SMN address `0x00059800`,
  and describes it as providing "the current control temperature
  (Tctl) after the slew-rate controls have been applied".
- The 0.125 °C/LSB scaling follows from S7: SB-TSI delivers the same
  Tctl in 0.125 °C increments spanning 0–255.875, which is exactly
  the unsigned 11-bit field range (2047 × 0.125 °C).
- Layout verified for Family 17h via the directly pinned OSRR (S2)
  plus S6, and for Family 19h via S1. The OSRR marks bits 18:0 (other
  than the fields above, i.e. including 18:16) as **Reserved** on the
  CPU THM; the `TJ_SEL` fields named in the GPU-IP header (S6) are
  therefore not part of this spec's decode — see Open questions.
  Family 1Ah is not yet verified by this spec; per ADR 0011 it is enabled
  best-effort as an experimental path (reusing this verified decode and
  plausibility gate) rather than disabled — see
  Detection and Open questions.
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
   - If `CUR_TEMP_RANGE_SEL` (bit 19) is 1, subtract 49 °C from Tctl.
     (S2)
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
| Products without a primary-source documented positive Tctl offset (incl. Zen 2 and newer) | 0 °C — no positive offset is documented in the primary sources for these products; deviations discovered later (e.g. via Phase 2 dumps) require a spec revision before adoption |

- Offset matching is by product name (OPN), not by family/model
  numbers alone. (S3; corroborated by S4)
- `CUR_TEMP_RANGE_SEL = 1` (the −49 °C range) is the normal case on
  many desktop parts; the decode in step 3 must always honor the bit
  rather than assume either range. (S2)
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

- Non-blocking for Phase 1: the decode ignores bits 18:16. The CPU
  OSRR (S2) marks bits 18:0 other than `RANGE_SEL` as Reserved, while
  the GPU-IP header (S6) names `TJ_SEL`/`TJ_SLEW_SEL` there and an S8
  engineering note claims a 49 °C adjustment when `TJ_SEL` reads
  `11b`. Without AMD primary documentation for the CPU THM, this is
  not part of the Ready decode path; revisit only if Phase 2 dumps
  show readings explained by it (then a spec revision may adopt it as
  a future extension).
- Non-blocking for Phase 1: family 1Ah is enabled best-effort as an
  experimental path via the Detection enablement table (ADR 0011),
  not verified. Confirm `THM_TCON_CUR_TMP` address/layout from an AMD
  PPR/OSRR when available, or validate against a Phase 2 register dump,
  to graduate it from experimental to verified.
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
| 3 | 2026-06-11 | OSRR 56255 Rev 3.03 §4.2.1 (p. 243) fetched and pinned directly (GitHub mirror of the AMD PDF), replacing the FreeBSD-carried quote as the Family 17h primary; S8 demoted to non-normative corroboration. `TJ_SEL` removed from the register map and decode path (the CPU OSRR marks bits 18:0 Reserved) and recorded as a non-blocking open question. Fixed the contradictory zero-offset wording in the Tctl-offset table. Status remains Implementation-ready. |
| 4 | 2026-07-20 | Runtime enablement policy updated per ADR 0011: Family 1Ah remains unverified by this spec but is no longer hard-disabled by default; it is enabled best-effort as an experimental, plausibility-gated reading until THM facts are verified by a primary source or accepted hardware dump. No new register facts were added. Status remains Implementation-ready. |
