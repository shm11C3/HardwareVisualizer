# Spec: Intel CPU package power via RAPL energy MSRs

| Field | Value |
| --- | --- |
| Revision | 2 |
| Status | Implementation-ready (rev 2) |
| Scope | CPU package power (Watts) on Intel x86-64 CPUs, derived from the RAPL package-domain energy counter MSRs (`MSR_RAPL_POWER_UNIT`, `MSR_PKG_ENERGY_STATUS`). Covers Sandy Bridge-and-newer Core/Xeon parts with the standard RAPL unit semantics. Excludes: power-limit programming, PP0/PP1/DRAM/PSys domains, Atom parts with deviant RAPL unit semantics (see Quirks), pre-Sandy-Bridge CPUs. |
| Issue phase | Phase 5 (#1635) — sensor model extension beyond temperature |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel SDM, Volume 3B, **§14.10 "Platform Specific Power Management Support"**: §14.10.1 "RAPL Interfaces" (Figure 14-35, `MSR_RAPL_POWER_UNIT` layout; pp. 14-46–14-47) and §14.10.3 "Package RAPL Domain" (Figure 14-37, `MSR_PKG_ENERGY_STATUS` layout; pp. 14-48–14-49) | Primary; semantics and layouts |
| S2 | Intel SDM, Volume 4, **Table 2-20** "MSRs Supported by Intel Processors Based on Sandy Bridge Microarchitecture": rows `606H` `MSR_RAPL_POWER_UNIT` (Scope: Package, "Unit Multipliers used in RAPL Interfaces (R/O)", p. 2-188) and `611H` `MSR_PKG_ENERGY_STATUS` (Scope: Package, "PKG Energy Status (R/O)", p. 2-189) | Primary; register rows, package scope |
| S3 | Intel SDM, Volume 4, **Table 2-8** "Specific MSRs Supported by Intel Atom Processors with CPUID Signatures 06_37H, 06_4AH, 06_5AH, 06_5DH": rows `606H` (p. 2-100) and `611H` (p. 2-101) | Primary; documents the deviant Silvermont unit semantics (see Quirks) |
| S4 | PawnIO `IntelMSR.p` module source at PawnIO.Modules tag `0.2.8` (commit `754635b`, LGPL-2.1-or-later) | Upstream-published interface definition of the module this project calls across the IOCTL boundary (read allow-list membership of `0x606`/`0x611`). Not used as a source for any hardware register fact. No code was copied. |

All SDM section/figure/table identifiers above were verified against
the combined-volume revision **325462-076US (December 2021)** (PDF
retrieved via an Internet Archive copy of the Intel-published
document); identifiers can shift between revisions. The cited RAPL
definitions are stable across recent revisions.

## Detection

| Fact | Source |
| --- | --- |
| CPU vendor string is `GenuineIntel` (CPUID leaf 0) | CPUID convention (see [`cpu-intel-dts-msr.md`](cpu-intel-dts-msr.md)) |
| The RAPL MSRs are **non-architectural**: no CPUID feature flag advertises them. Presence is documented per model in SDM Vol. 4, starting with the Sandy Bridge microarchitecture (Table 2-20 and later tables) | S1, S2 |
| Detection is therefore probe-based: read `0x606` and `0x611` once; a faulted/failed read of either means "unsupported" (no package-power source; no fallback exists for power) | S2; project policy |
| An all-zero `MSR_RAPL_POWER_UNIT` value (`ESU = 0`, i.e. 1 J units) is treated as a failed probe rather than a valid configuration | Project policy (defensive; documented defaults are nonzero, S1) |

Scoped enablement:

| Scope | Status | Default enablement |
| --- | --- | --- |
| Sandy Bridge-and-newer Core/Xeon parts (standard RAPL unit semantics per §14.10.1) | Layouts and package scope verified against S1, S2 | Enabled (probe-gated) |
| Atom parts with CPUID signatures `06_37H`, `06_4AH`, `06_5AH`, `06_5DH` (Silvermont) | Deviant unit semantics documented by S3; no decode specified by this document | Disabled — excluded by CPUID signature until a dedicated decode is specified |
| CPUs where the probe read faults | Out of scope | Disabled by the probe |

## Register map (facts)

Both registers are read with `RDMSR` (via the PawnIO `IntelMSR`
module, see [`pawnio-interface.md`](pawnio-interface.md); both MSRs
are on its read allow-list and neither is on its write allow-list,
S4).

| MSR | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x606` | `MSR_RAPL_POWER_UNIT` | 3:0 | Power Units (`PU`) | multiplier 1/2^PU W; default `0011b` = 1/8 W | S1 |
| `0x606` | `MSR_RAPL_POWER_UNIT` | 12:8 | Energy Status Units (`ESU`) | multiplier 1/2^ESU J; default `10000b` (16) = 15.3 µJ | S1 |
| `0x606` | `MSR_RAPL_POWER_UNIT` | 19:16 | Time Units (`TU`) | multiplier 1/2^TU s; default `1010b` = 976 µs | S1 |
| `0x611` | `MSR_PKG_ENERGY_STATUS` | 31:0 | Total Energy Consumed — "the total amount of energy consumed since that last time this register is cleared" | unsigned, in Energy Status Units (1/2^ESU J) | S1 |

Notes:

- `MSR_RAPL_POWER_UNIT` is read-only and its units apply "across all
  RAPL domains". (S1)
- `MSR_PKG_ENERGY_STATUS` is read-only, reports "the actual energy
  use for the package domain", is "updated every ~1msec", and "has a
  wraparound time of around 60 secs when power consumption is high,
  and may be longer otherwise". Bits 63:32 are reserved. (S1)
- Both MSRs are **package scope**: any logical CPU of the package
  reads the same value. (S2)
- Only `ESU` enters this document's decode; `PU`/`TU` are listed for
  completeness of the register layout and are not consumed.

## Read procedure and decode

1. Confirm detection facts above (vendor, probe reads, Silvermont
   signature exclusion).
2. Once per session: read `MSR_RAPL_POWER_UNIT` (`0x606`), extract
   `ESU` from bits 12:8. The energy unit is `2^-ESU` joules. Always
   use the read value; never assume the default (see Quirks).
3. Each sampling tick: read `MSR_PKG_ENERGY_STATUS` (`0x611`) and
   capture a monotonic timestamp immediately adjacent to the read.
   Keep the counter's low 32 bits (bits 63:32 are reserved).
4. The first sample only establishes the baseline; it publishes no
   power value.
5. **Wrap-safe gap check (normative).** Derive once per session the
   maximum wrap-safe sampling gap
   `T_max = (2^32 × 2^-ESU J) / P_gate`, where `P_gate = 1000 W` is
   the same assumed maximum package power as the plausibility gate in
   step 7 (at the default `ESU = 16`: `65 536 J / 1000 W ≈ 65 s`).
   If `t_now − t_prev > T_max`, do **not** compute or publish a power
   value: treat the tick as a missing sample and restart by taking
   the current reading as the new baseline. Rationale: the modular
   difference cannot detect `k ≥ 1` complete counter wraps, so an
   oversized gap (system sleep, timer suspension, collector delay)
   would otherwise yield a plausible-looking but understated power
   value that the range gate in step 7 cannot catch. Dropping the
   sample instead of publishing a fabricated number follows DP-02 in
   [`docs/design-principles.md`](../../design-principles.md).
   (Project policy; arithmetic from the S1 units and counter width)
6. Otherwise compute, in 32-bit unsigned modular arithmetic:
   - `delta = (counter_now − counter_prev) mod 2^32`
   - `energy_J = delta × 2^-ESU`
   - `power_W = energy_J / (t_now − t_prev)` with the monotonic
     timestamps in seconds.
   The modular difference is exact across a single counter wrap;
   validity requires the true energy delta to be below `2^32` energy
   units, which the step 5 bound guarantees (see Quirks for the
   sampling-interval numbers).
7. Publish `power_W` as the CPU package power. Plausibility gate
   before publishing (this project's own policy, not an SDM fact):
   accept only `0 ≤ power_W ≤ 1000` and `t_now − t_prev > 0`;
   otherwise drop the sample and re-baseline from the current
   reading.
8. A failed MSR read invalidates the baseline: skip the sample and
   re-baseline on the next successful read (an unknown gap may span
   multiple wraps).

## Quirks

- **32-bit wraparound.** The package energy counter is 32 bits and
  free-runs; the SDM states a wrap time of "around 60 secs when
  power consumption is high" (S1). Quantitatively, the full 32-bit
  range at the default `ESU = 16` is `2^32 × 2^-16 J = 65 536 J`:
  ~327 s (≈ 5.5 min) at a continuous 200 W, ~65 s at 1000 W. The
  modular-difference decode is exact for any single wrap; a sampling
  interval of at most 30 s keeps the decode unambiguous below
  ~2 185 W average package power and is therefore safe for any
  plausible package. Gaps exceeding the `T_max` bound of Read
  procedure step 5 (e.g. after system sleep) are rejected and
  re-baselined instead of decoded. (S1; arithmetic)
- **`ESU` varies by product.** For example, the Goldmont Atom table
  documents a default of `01110b` (61 µJ) instead of 15.3 µJ (SDM
  Vol. 4 Table 2-12, same 1/2^ESU semantics). The decode must always
  use the `ESU` value read from `0x606`. (S1; Table 2-12)
- **Silvermont Atom deviation.** For CPUID signatures `06_37H`,
  `06_4AH`, `06_5AH`, `06_5DH`, Vol. 4 Table 2-8 defines `0x606`
  with *inverted* unit semantics: energy unit = `2^ESU` **micro**joules
  (default `00101b` = 32 µJ) and power unit = `2^PU` milliwatts.
  Applying the standard 1/2^ESU-joule decode to these parts would be
  wrong by orders of magnitude, so they are excluded by CPUID
  signature (see Detection). (S3)
- The counter value is "since the last time this register is
  cleared" (cleared at processor reset); treat it purely as a
  free-running unsigned counter, never as an absolute energy total.
  (S1)

## Safety notes

- Read-only: `RDMSR` of `0x606` and `0x611` only. No `WRMSR`.
  `0x611` is not on the `IntelMSR` module's write allow-list (S4);
  the write surface is never invoked by this project.
- MSR reads have no bus-level side effects requiring the ISA or PCI
  mutex conventions.
- **Execution context:** both MSRs are package scope (S2), so the
  read may execute on any logical CPU of the package. PawnIO executes
  `RDMSR` on the calling thread's current processor (see
  [`pawnio-interface.md`](pawnio-interface.md)); no affinity control
  is needed on the single-package consumer machines this phase
  targets.

## Open questions

- Non-blocking for Phase 5: multi-package systems require one
  baseline per package with the reading thread affinity-pinned to a
  core of each package; this phase targets single-package consumer
  machines (package 0), matching
  [`cpu-intel-dts-msr.md`](cpu-intel-dts-msr.md).
- Non-blocking for Phase 5: Silvermont Atom parts stay disabled;
  specify the Table 2-8 decode only if such hardware ever matters to
  this project.
- Non-blocking for Phase 5: PP0/PP1/DRAM/PSys energy counters
  (`0x639`, `0x641`, `0x619`, `0x64D`) exist and are on the module's
  read allow-list, but no decode is specified here; package domain
  only.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-08-30 | Initial version, authored with all provenance pinned against SDM 325462-076US (Vol 3B §14.10.1/§14.10.3, Vol 4 Tables 2-20 and 2-8) and PawnIO.Modules tag 0.2.8. Proposed Implementation-ready per the README status-transition checklist; effective upon maintainer approval of the introducing PR. |
| 2 | 2026-08-30 | PR #2033 review follow-up. Normative addition: wrap-safe gap check — gaps exceeding `T_max = 2^32 × 2^-ESU J / 1000 W` (≈ 65 s at `ESU = 16`) publish no power value and re-baseline (DP-02), because the modular difference cannot detect complete wraps across oversized gaps (sleep, timer suspension, collector delay). No register facts changed. Status remains Implementation-ready. |
