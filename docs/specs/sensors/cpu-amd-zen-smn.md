# Spec: AMD Zen CPU temperature (Tctl/Tdie) via SMN thermal controller

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Draft — not implementation-ready |
| Scope | Package control temperature (Tctl) and die temperature (Tdie) on AMD Family 17h (Zen/Zen+/Zen 2) and Family 19h (Zen 3/Zen 4) processors, read from the SMU thermal controller (THM) over the System Management Network (SMN). Family 1Ah (Zen 5) is recognized but disabled by default pending verification (see Detection). Excludes: per-CCD temperatures, SMU PM-table metrics, pre-Zen (Family 15h/16h) thermal registers. |
| Issue phase | Phase 1 (#1635) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD, *Processor Programming Reference (PPR) for AMD Family 17h Models 00h-0Fh Processors*, document no. 54945, register `SMU::THM::THM_TCON_CUR_TMP` | Primary |
| S2 | AMD, *PPR for AMD Family 19h Model 01h, Revision B1 Processors*, document no. 55898 Vol 2 (same THM register) | Primary |
| S3 | AMD public statements on Tctl offsets: "AMD Ryzen™ Community Update #3" (R. Hallock, Apr 2017) and 2nd-gen Ryzen launch documentation | Primary for offsets |
| S4 | Linux `k10temp` hwmon driver (GPL-2.0) | **Non-normative corroboration only.** No fact in this document relies solely on this source. No code, structure, identifiers, or tables were copied. |
| S5 | PawnIO `RyzenSMU.p` module source (LGPL-2.1-or-later) | Upstream-published interface definition of the module this project calls across the IOCTL boundary (allowed SMN windows, family gate, mutex). Not used as a source for any hardware register fact. No code was copied. |

`TODO(provenance)`: pin PPR section/page numbers; the family 1Ah PPR
reference still needs to be identified (see Open questions).

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
| `0x17` | `THM_TCON_CUR_TMP` verified against AMD PPR (S1) | Enabled |
| `0x19` | `THM_TCON_CUR_TMP` verified against AMD PPR (S2) | Enabled |
| `0x1A` | Recognized by the PawnIO module (S5) but not yet verified by this spec | Disabled until PPR or hardware-dump verification |

## Register map (facts)

Access is performed with the PawnIO `RyzenSMU` module's
`ioctl_read_smu_register` (input: SMN address; output: 32-bit value).
The module's allowed SMN window `0x56000`–`0x5AFFF` contains this
register. See [`pawnio-interface.md`](pawnio-interface.md).

| SMN address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x00059800` | `SMU::THM::THM_TCON_CUR_TMP` | 31:21 | `CUR_TEMP` — current reported temperature | 0.125 °C per LSB, unsigned 11-bit | S1, S2 |
| `0x00059800` | `SMU::THM::THM_TCON_CUR_TMP` | 19 | `CUR_TEMP_RANGE_SEL` — selects the reporting range | 0 → 0…225 °C; 1 → −49…206 °C | S1, S2 |

Notes:

- The same register address and layout are documented for Family 17h
  (S1) and Family 19h (S2). Family 1Ah is not yet verified by this
  spec and stays disabled by default — see Detection and Open
  questions.
- SMN is reached through an index/data register pair in the host
  bridge PCI configuration space; the PawnIO module handles this and
  serializes on the `Access_PCI` mutant. The client performs no raw
  PCI configuration access. (S5)

## Read procedure and decode

1. Confirm detection facts above (including the per-family enablement
   table).
2. Read the 32-bit value of `THM_TCON_CUR_TMP` from SMN `0x00059800`.
3. Decoding rules (hardware semantics):
   - Extract `CUR_TEMP` from bits 31:21.
   - Tctl = `CUR_TEMP` × 0.125 °C.
   - If `CUR_TEMP_RANGE_SEL` (bit 19) is 1, subtract 49 °C from Tctl.
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
  rather than assume either range. (S1)
- A read returning all zeros indicates a failed SMN transaction rather
  than 0 °C (or −49 °C); treat as an invalid sample. (Project policy;
  consistent with the plausibility gate.)

## Safety notes

- Read-only: only `ioctl_read_smu_register` is used. The module's SMU
  write/command surfaces (`ioctl_write_smu_register`,
  `ioctl_send_smu_command`) are never invoked by this project.
- PCI/SMN serialization relies on the module's `Access_PCI` mutant
  handling; a single register read is one IOCTL, so no multi-call
  client-side critical section is required for Tctl. (S5)

## Future extensions (recorded, not yet specified)

- Per-CCD temperature registers (`THM_DIE*_TEMP`) exist in the same
  THM block; their addresses vary by model and need PPR verification
  before a spec revision adds them.
- SMU PM-table reads (`ioctl_*_pm_table`) expose richer telemetry
  (per-core power, voltages) but are version-dependent; out of scope
  for Phase 1.

## Open questions

- Family 1Ah (Zen 5): confirm `THM_TCON_CUR_TMP` address/layout from
  an AMD PPR when available, or validate against a Phase 2 register
  dump from real hardware before enabling by default.
- Threadripper / EPYC multi-die parts: whether `0x59800` reports the
  hottest die or a specific die needs verification on such hardware
  (Phase 2 dumps).

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-06-10 | Initial version |
