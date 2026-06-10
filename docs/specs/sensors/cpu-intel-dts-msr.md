# Spec: Intel CPU temperature via digital thermal sensor MSRs

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Draft |
| Scope | Package (and per-core) temperature on Intel x86-64 CPUs using the architectural digital thermal sensor (DTS) MSRs. Covers Nehalem-and-newer Core/Xeon/Atom parts that expose `MSR_TEMPERATURE_TARGET`. Excludes: pre-Nehalem TjMax estimation, thermal interrupt configuration, RAPL. |
| Issue phase | Phase 1 (#1635) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel, *Intel® 64 and IA-32 Architectures Software Developer's Manual*, Volume 3B, chapter "Thermal Monitoring and Protection" (section "Reading the Digital Sensor"). Combined-volume order no. 325462. | Primary; semantics |
| S2 | Intel SDM Volume 4, *Model-Specific Registers*, Table 2-2 (architectural MSRs) and per-model tables for `MSR_TEMPERATURE_TARGET`. Order no. 335592. | Primary; register layouts |
| S3 | Intel SDM Volume 2, `CPUID` instruction reference (leaf `06H`). | Primary; feature detection |

`TODO(provenance)`: pin SDM revision number and page numbers at
implementation review time (the SDM is revised quarterly; field
layouts cited here are stable architectural definitions).

## Detection

| Fact | Source |
| --- | --- |
| CPU vendor string is `GenuineIntel` (CPUID leaf 0) | S3 |
| `CPUID.06H:EAX[0] = 1` — digital thermal sensor (DTS) supported; `IA32_THERM_STATUS` readout is valid to use | S3 |
| `CPUID.06H:EAX[6] = 1` — package thermal management (PTM) supported; `IA32_PACKAGE_THERM_STATUS` exists | S3 |
| `MSR_TEMPERATURE_TARGET` (`0x1A2`) is model-specific; treat a faulted read or a zero `Temperature Target` field as "not supported" and fall back | S2 |

## Register map (facts)

All registers are read with `RDMSR` (via the PawnIO `IntelMSR` module,
see [`pawnio-interface.md`](pawnio-interface.md); all three MSRs are on
its read allow-list).

| MSR | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x19C` | `IA32_THERM_STATUS` | 22:16 | Digital Readout: temperature **below** the TCC activation temperature | °C, unsigned | S1, S2 |
| `0x19C` | `IA32_THERM_STATUS` | 30:27 | Resolution of the readout | °C | S2 |
| `0x19C` | `IA32_THERM_STATUS` | 31 | Reading Valid (1 = digital readout is valid) | flag | S2 |
| `0x1B1` | `IA32_PACKAGE_THERM_STATUS` | 22:16 | Package Digital Readout: package temperature below package TCC activation | °C, unsigned | S2 |
| `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Temperature Target: TCC activation temperature (commonly called TjMax) | °C | S2 |
| `0x1A2` | `MSR_TEMPERATURE_TARGET` | 27:24 or 29:24 (model-dependent width) | TCC Activation Offset: lowers the throttle activation point below the Temperature Target | °C | S2 |

Notes:

- `IA32_THERM_STATUS` is a **per-core** register; the readout reflects
  the core the `RDMSR` executes on. (S1)
- `IA32_PACKAGE_THERM_STATUS` is **package-scope**: any logical CPU of
  the package reads the same value. It has **no** Reading Valid bit.
  (S2)
- The `TCC Activation Offset` field width differs between models
  (4-bit on many client models, 6-bit on some); consult the SDM Vol. 4
  row for each supported model before using it. (S2)

## Read procedure and decode

1. Confirm detection facts above.
2. Read `MSR_TEMPERATURE_TARGET` (`0x1A2`) once per session; let
   `t_target = bits[23:16]`. If the read faults or `t_target == 0`,
   report "unsupported" (caller falls back to ACPI zones per #1633).
3. Package temperature (preferred for Phase 1): read
   `IA32_PACKAGE_THERM_STATUS` (`0x1B1`); let
   `readout = bits[22:16]`; temperature in °C:

   ```text
   temp_pkg = t_target - readout
   ```

4. Per-core temperature (optional, later): read `IA32_THERM_STATUS`
   (`0x19C`) on the target core; use the value only when bit 31
   (Reading Valid) is 1; decode with the same subtraction.
5. Plausibility gate before publishing (defensive, this project's own
   policy, not an SDM fact): accept only `0 < temp ≤ t_target` and
   `t_target` within 50–120 °C; otherwise drop the sample.

## Quirks

- Pre-Nehalem parts (e.g. Core 2) do not document a usable
  `MSR_TEMPERATURE_TARGET`; tools of that era guessed TjMax. Such
  parts are **out of scope**: the `t_target == 0` / fault rule above
  excludes them. (S2)
- The digital readout saturates: values at or above TCC activation
  read as 0 °C below target. A readout of 0 therefore means "at TjMax",
  which on an idle machine indicates a bad read rather than a real
  temperature — the plausibility gate drops `temp == t_target` only if
  load context makes it implausible; implementers may publish it as-is
  with the valid bit set. (S1)
- Whether the digital readout is referenced to `Temperature Target`
  alone or to `Temperature Target − TCC Activation Offset` is not
  explicitly stated by the SDM; common practice is to use bits 23:16
  without subtracting the offset. Recorded as an open question. (S1,
  S2)
- Multi-package systems: `0x1B1` must be read once per package, with
  the reading thread affinity-pinned to a core of each package. Phase
  1 targets single-package consumer machines (package 0). (S2)

## Safety notes

- Read-only: `RDMSR` of the three listed MSRs only. No `WRMSR` in any
  phase. The PawnIO `IntelMSR` module's write surface is not used.
- MSR reads have no bus-level side effects requiring the ISA or PCI
  mutex conventions.

## Open questions

- TCC Activation Offset interaction with the readout reference (see
  Quirks). Resolution requires checking the SDM "Setting Thermal
  Targets" subsection wording against an offset-programmed machine.
- Which logical CPU PawnIO executes `RDMSR` on (tracked in
  [`pawnio-interface.md`](pawnio-interface.md)); package-scope reads
  make this moot for Phase 1 on single-package machines.
- Hybrid (P/E-core) parts: per-core readouts differ per core type;
  package readout is defined identically. Verify no additional
  enumeration is needed beyond CPUID leaf `06H` on a hybrid test
  machine.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-06-10 | Initial version |
