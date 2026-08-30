# Spec: AMD Zen CPU package power via RAPL energy MSRs

| Field | Value |
| --- | --- |
| Revision | 3 |
| Status | Implementation-ready (rev 3) |
| Scope | CPU package/socket power (Watts) on AMD Family 17h (Zen/Zen+/Zen 2), 19h (Zen 3/Zen 4), and 1Ah (Zen 5) processors, derived from the RAPL energy counter MSRs (`MSRC001_0299` RAPL Power Unit, `MSRC001_029B` Package Energy Status). Excludes: per-core energy (`MSRC001_029A`, recorded as a future extension), power-limit interfaces, SMU PM-table power telemetry, pre-Zen families. |
| Issue phase | Phase 5 (#1635) — sensor model extension beyond temperature |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD, *PPR for AMD Family 17h Models 01h,08h B2*, document no. **54945 Rev 3.03 (Jun 14, 2019)**: §2.1.14.3 "MSRs - MSRC001_0xxx" pp. 149–150 (`MSRC001_0299/029A/029B`); §2.1.13.1 p. 76 (`CPUID_Fn80000007_EDX`) | Primary |
| S2 | AMD, *PPR for AMD Family 19h Model 21h B0*, document no. **56214-B0 Rev 3.05 (Apr 22, 2021)**: §2.1.14.3 pp. 181–182; §2.1.13.1 p. 84 (`CPUID_Fn80000007_EDX`) | Primary |
| S3 | AMD, *PPR for AMD Family 19h Model 61h B1*, document no. **56713-B1 Rev 3.05 (Mar 8, 2023)**: §2.2.2 "L3 Clocks and Test (CT) MSR Registers" pp. 291–292 (`MSRC001_0299/029B`); §2.1.11.1 p. 99 (`CPUID_Fn80000007_EDX`) | Primary |
| S4 | AMD, *PPR Vol 1 for AMD Family 1Ah Model 02h C1*, document no. **57238 Rev 0.24 (Sep 29, 2024)**: §2.2.2 pp. 321–322; §2.1.14.1 p. 120 (`CPUID_Fn80000007_EDX`) | Primary |
| S5 | AMD, *PPR for AMD Family 1Ah Model 44h B0*, document no. **57896-B0 Rev 3.00 (Aug 28, 2024)**: §2.2.2 pp. 295–296; §2.1.12.1 p. 98 (`CPUID_Fn80000007_EDX`) | Primary |
| S6 | PawnIO `AMDFamily17.p` module source at PawnIO.Modules tag `0.2.8` (commit `754635b`, LGPL-2.1-or-later) | Upstream-published interface definition of the module this project calls across the IOCTL boundary (family gate, read/write allow-lists). Not used as a source for any hardware register fact. No code was copied. |

AMD removed these PPR PDFs from its live documentation site (the
former TechDocs URLs return 404 at authoring time). Each document was
retrieved as the AMD-published PDF via Internet Archive snapshots and
verified directly; the document number + revision above are the
canonical identifiers.

## Detection

| Fact | Source |
| --- | --- |
| CPU vendor string is `AuthenticAMD`; effective family = `BaseFamily + ExtendedFamily` (CPUID conventions, see [`cpu-amd-zen-smn.md`](cpu-amd-zen-smn.md)) | AMD CPUID convention |
| `CPUID_Fn80000007_EDX[14]` (`RAPL`) = 1 — "Running average power limit" supported; documented as `Read-only. Reset: Fixed,1` on **all five pinned models** | S1 (§2.1.13.1 p. 76), S2 (§2.1.13.1 p. 84), S3 (§2.1.11.1 p. 99), S4 (§2.1.14.1 p. 120), S5 (§2.1.12.1 p. 98) |
| The PawnIO `AMDFamily17` module accepts AMD families `0x17`–`0x1A` only and rejects other vendors/families/architectures with an error status, providing a second layer of gating | S6 |
| Detection is probe-based on top of the gates above: read `0xC0010299` and `0xC001029B` once; a failed read of either means "unsupported" | Project policy |
| An all-zero `MSRC001_0299` value (`ESU = 0`, i.e. 1 J units) is treated as a failed probe rather than a valid configuration | Project policy (defensive; the documented default is `10h`, S1–S5) |

Scoped enablement (per ADR 0011: recognized-but-unverified scopes are
enabled best-effort as **experimental**, reusing the verified decode
and plausibility gate; successful readings use the normal
presentation contract):

| Scope | Status | Default enablement |
| --- | --- | --- |
| Family `0x17`, models 01h/08h | Register layout and 32-bit counter verified (S1) | Enabled |
| Family `0x17`, other models | Not verified by this spec; the decode below is counter-width-agnostic | Enabled best-effort as **experimental** (ADR 0011) |
| Family `0x19`, model 21h | Register layout and 32-bit counter verified (S2) | Enabled |
| Family `0x19`, model 61h | Register layout and 64-bit counter verified (S3) | Enabled |
| Family `0x19`, other models | Not verified by this spec | Enabled best-effort as **experimental** (ADR 0011) |
| Family `0x1A`, model 02h | Register layout, 64-bit counter, and socket-domain semantics verified (S4) | Enabled |
| Family `0x1A`, model 44h | Register layout and 64-bit counter verified (S5), but the PPR titles `MSRC001_029B` "L3 CCX Energy Status", leaving the reported power domain (socket vs. CCX) ambiguous | Enabled best-effort as **experimental** (ADR 0011); graduates via hardware-dump verification (see Open questions) |
| Family `0x1A`, other models | Not verified by this spec | Enabled best-effort as **experimental** (ADR 0011) |

## Register map (facts)

Both registers are read with `RDMSR` via the PawnIO `AMDFamily17`
module's `ioctl_read_msr` (see
[`pawnio-interface.md`](pawnio-interface.md); both MSRs are on its
read allow-list and neither is on its write allow-list, S6).

| MSR | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0xC0010299` | `Core::X86::Msr::RAPL_PWR_UNIT` (17h/19h ≤ 21h) / `L3::L3CT::L3RAPLPowerUnit0` (19h 61h, 1Ah) | 19:16 | `TU`: Time Units | multiplier 1/2^TU s; default `1010b` = 976 µs | S1–S5 |
| `0xC0010299` | (same) | 12:8 | `ESU`: Energy Status Units | multiplier 1/2^ESU J; default `10h` (16) = 15.3 µJ | S1–S5 |
| `0xC0010299` | (same) | 3:0 | `PU`: Power Units | multiplier 1/2^PU W; default `0011b` = 1/8 W | S1–S5 |
| `0xC001029B` | `Core::X86::Msr::PKG_ENERGY_STAT` (17h/19h ≤ 21h) / `L3::L3CT::L3PackageEnergyStatus` (19h 61h, 1Ah 02h) / `L3::L3CT::L3CCXEnergyStatus` (1Ah 44h) | see width table | `TotalEnergyConsumed` | unsigned, in Energy Status Units (1/2^ESU J) | S1–S5 |

Counter width of `MSRC001_029B` per pinned model (the only
model-dependent fact in this document):

| Family / model | Counter bits | Reserved bits | Source |
| --- | --- | --- | --- |
| 17h 01h/08h | 31:0 | 63:32 | S1 |
| 19h 21h | 31:0 | 63:32 | S2 |
| 19h 61h | 63:0 | — | S3 |
| 1Ah 02h | 63:0 | — | S4 |
| 1Ah 44h | 63:0 | — | S5 |

Notes:

- Both MSRs are read-only. The `ESU`/`TU`/`PU` field positions and
  the 1/2^ESU-joule unit formula are identical across all five pinned
  PPRs; only the counter width and the register title vary. (S1–S5)
- The 64-bit-counter PPRs describe `MSRC001_029B` as: "Total Energy
  consumed since the last time the register is cleared", "updated
  every ~1ms", "Energy status is free running", "Users calculate
  power for a given domain by calculating dEnergy/dTime", and "Users
  must ensure successive reads contain at least one, but preferably
  many energy status updates by hardware". (S3, S4, S5)
- S4 (Family 1Ah Model 02h) states the register "reports the actual
  energy use for the socket". S5 (Family 1Ah Model 44h) instead
  titles it "L3 CCX Energy Status" while otherwise using the same
  "respective power domain" wording as S3 — see Open questions.
- In the L3CT-form PPRs (S3–S5) the listed *Reset* value of the
  `ESU` field is 0 while the field description states the default
  `10000b`; the decode must always use the value read from the
  register (matching the defensive all-zero probe rule in
  Detection). (S3–S5)
- `MSRC001_0299`/`MSRC001_029B` carry per-CCD/CCX instance
  specifiers in the PPRs (e.g. `_ccd[1:0]_lthree0`), while
  `MSRC001_029A` is additionally per-core (`…_core[7:0]`). (S1–S5;
  see Open questions on multi-CCD instance behavior)

## Read procedure and decode

1. Confirm detection facts above (vendor, family, CPUID RAPL bit,
   probe reads, scoped-enablement table).
2. Once per session: read `MSRC001_0299`, extract `ESU` from bits
   12:8. The energy unit is `2^-ESU` joules.
3. Each sampling tick: read `MSRC001_029B` and capture a monotonic
   timestamp immediately adjacent to the read. **Keep only the low
   32 bits of the returned value**, regardless of model.
4. The first sample only establishes the baseline; it publishes no
   power value.
5. **Wrap-safe gap check (normative).** Derive once per session the
   maximum wrap-safe sampling gap
   `T_max = (2^32 × 2^-ESU J) / P_gate`, where `P_gate = 1000 W` is
   the same assumed maximum package power as the plausibility gate in
   step 7 (at the default `ESU = 16`: `65 536 J / 1000 W ≈ 65 s`).
   If `t_now − t_prev ≥ T_max`, do **not** compute or publish a power
   value: treat the tick as a missing sample and restart by taking
   the current reading as the new baseline. At equality, a package
   averaging exactly `P_gate` consumes `2^32` energy units, whose low
   32-bit modular difference is zero; validity therefore requires a
   strict gap below `T_max`. At or beyond that boundary, the modular
   difference cannot detect `k ≥ 1` complete counter wraps, so a gap
   caused by system sleep, timer suspension, or collector delay could
   otherwise yield a plausible-looking but understated power value
   that the range gate in step 7 cannot catch. Dropping the sample
   instead of publishing a fabricated number follows DP-02 in
   [`docs/design-principles.md`](../../design-principles.md).
   (Project policy; arithmetic from the S1–S5 units and widths)
6. Otherwise compute, in 32-bit unsigned modular arithmetic:
   - `delta = (counter_now − counter_prev) mod 2^32`
   - `energy_J = delta × 2^-ESU`
   - `power_W = energy_J / (t_now − t_prev)` with the monotonic
     timestamps in seconds.
7. Publish `power_W` as the CPU package power. Plausibility gate
   before publishing (this project's own policy, not a PPR fact):
   accept only `0 ≤ power_W ≤ 1000` and `t_now − t_prev > 0`;
   otherwise drop the sample and re-baseline from the current
   reading.
8. A failed MSR read invalidates the baseline: skip the sample and
   re-baseline on the next successful read.

Width-agnostic decode rationale (why step 3 truncates to 32 bits):

- On 32-bit-counter models the counter itself wraps modulo `2^32`;
  the modular difference of the low 32 bits is exactly the hardware
  behavior.
- On 64-bit-counter models the low 32 bits of a free-running 64-bit
  counter advance identically modulo `2^32`; the same modular
  difference is exact provided the true energy delta between samples
  is below `2^32` energy units.
- Both cases therefore share one correctness condition: the energy
  consumed between two samples must be below
  `2^32 × 2^-ESU J = 65 536 J` at the default `ESU = 16` — ~327 s
  (≈ 5.5 min) of headroom at a continuous 200 W, ~65 s at 1000 W. A
  sampling interval of at most 30 s keeps the decode unambiguous
  below ~2 185 W average package power, and any gap at or exceeding
  the `T_max` bound of step 5 is rejected and re-baselined instead of
  decoded. This removes the counter width as a runtime dependency,
  which is what makes the experimental (width-unverified) scopes in
  Detection safe to attempt. (S1–S5 for the widths and units;
  arithmetic)

## Quirks

- The counter width of `MSRC001_029B` changed from 32 to 64 bits
  between Zen 3 (19h 21h, S2) and Zen 4 (19h 61h, S3); the pinned
  per-model table in the Register map is the authority. The decode
  above is deliberately width-agnostic, so unpinned models decode
  correctly either way.
- On 19h 61h and later, the RAPL unit/package-energy registers are
  documented under the L3 "Clocks and Test" register chapter with
  L3CT mnemonics instead of the Core MSR chapter — same MSR
  addresses, same field layout. (S3–S5)
- `MSRC001_029B` reads as 0 after reset and free-runs from there;
  treat it purely as a free-running unsigned counter, never as an
  absolute energy total. (S1–S5)

## Safety notes

- Read-only: `ioctl_read_msr` of `0xC0010299` and `0xC001029B` only.
  No `WRMSR`; neither MSR is on the `AMDFamily17` module's write
  allow-list, and this project never invokes the module's write
  surface. (S6)
- MSR reads require no mutex: the `AMDFamily17` module documents a
  caller-held `Access_PCI` mutant only for its SMN ioctl, which this
  document does not use. (S6; see
  [`pawnio-interface.md`](pawnio-interface.md))
- **Execution context:** the package/socket energy counter is read
  from any logical CPU on the single-socket consumer machines this
  phase targets (S4 describes the value as socket-scope). PawnIO
  executes `RDMSR` on the calling thread's current processor with no
  affinity control (see
  [`pawnio-interface.md`](pawnio-interface.md)).

## Future extensions (recorded, not yet specified)

- `MSRC001_029A` (`Core::X86::Msr::CORE_ENERGY_STAT`) provides a
  per-core `TotalEnergyConsumed` with the same unit register and the
  same per-model width progression (31:0 on S1/S2; 63:0 on S3–S5,
  §2.1.14.3 of each PPR; page pinned for S1 at p. 150). It is on the
  `AMDFamily17` read allow-list (S6) but out of scope for this
  phase.

## Open questions

- Non-blocking for Phase 5: on Family 1Ah Model 44h the PPR titles
  `MSRC001_029B` "L3 CCX Energy Status" (S5), while the sibling
  Family 1Ah Model 02h PPR says "actual energy use for the socket"
  (S4) and the Family 19h Model 61h PPR titles it "Package Energy
  Status" (S3). Whether 1Ah 44h reports socket or CCX energy is
  therefore unverified; the scope stays experimental (ADR 0011) with
  the plausibility gate applied. Graduation path: a
  maintainer-accepted hardware dump on a 1Ah 44h part comparing
  `0xC001029B` deltas under all-core load against the sum of
  per-core `0xC001029A` deltas (a socket-scope counter must exceed
  the core sum; a single-CCX counter on a two-CCD part would track
  roughly half), or a future PPR revision clarifying the domain.
- Non-blocking for Phase 5: the PPR instance specifiers
  (`_ccd[…]_lthree0`) leave open whether every per-CCD instance of
  `MSRC001_029B` reports identical package energy on multi-CCD
  parts; the register title (Package/socket, S1–S4) says what is
  reported, not from where it is readable. Single-CCD parts are
  unaffected. Verify via a hardware dump reading the counter from
  cores of different CCDs on a multi-CCD part.
- Non-blocking for Phase 5: family/model combinations without a
  pinned PPR row are enabled as experimental with the width-agnostic
  decode (see Detection); each graduates to verified when its PPR is
  pinned or a hardware dump confirms the counter behavior.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-08-30 | Initial version, authored with all provenance pinned against AMD PPRs 54945 Rev 3.03, 56214-B0 Rev 3.05, 56713-B1 Rev 3.05, 57238 Rev 0.24, 57896-B0 Rev 3.00 and PawnIO.Modules tag 0.2.8. Proposed Implementation-ready per the README status-transition checklist; effective upon maintainer approval of the introducing PR. |
| 2 | 2026-08-30 | PR #2033 review follow-up. Provenance: `CPUID_Fn80000007_EDX[14]` (`RAPL`) pinned for all five models — added S2 §2.1.13.1 p. 84, S4 §2.1.14.1 p. 120, S5 §2.1.12.1 p. 98; corrected the S3 CPUID section number to §2.1.11.1 (p. 99 unchanged). Normative addition: wrap-safe gap check — gaps exceeding `T_max = 2^32 × 2^-ESU J / 1000 W` (≈ 65 s at `ESU = 16`) publish no power value and re-baseline (DP-02), because the modular difference cannot detect complete wraps across oversized gaps. Status remains Implementation-ready. |
| 3 | 2026-08-30 | Corrected the normative wrap-safe boundary from `> T_max` to `≥ T_max`: at equality and `P_gate`, the true delta is exactly `2^32` energy units and decodes as a zero low-32-bit modular difference. Maintainer approved the revision 3 status transition; status is Implementation-ready. |
