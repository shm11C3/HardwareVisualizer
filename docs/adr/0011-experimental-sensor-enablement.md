# Experimental Sensor Enablement for Recognized-but-Unverified Hardware

Status: accepted

We will prefer best-effort experimental enablement for hardware that the current
safe access path already recognizes, even when the register behavior has not yet
been fully verified against a primary hardware specification. The goal is to
increase the number of devices that can produce useful local readings and then
use user reports, diagnostics, and later primary-source or hardware-dump
verification to improve quality.

The concrete trigger is issue #1824: AMD Family 1Ah / Zen 5 (Ryzen 7 9800X3D)
is recognized by the PawnIO `RyzenSMU` module, but HardwareVisualizer previously
hard-disabled it with `AMD family 0x1a is disabled by the ready spec`. Under this
policy, that family is attempted as an **experimental** CPU package temperature
source instead of being blocked only because the Family 1Ah THM facts are not yet
verified.

## Decision

For native sensor collection, the implementation distinguishes three states:

1. **Verified** — the hardware/register path is covered by the implementation-
   ready spec and may be presented as the normal supported path.
2. **Experimental** — the access module or OS path already recognizes the device
   or family, and HardwareVisualizer has a plausible read-only decode to attempt,
   but the exact hardware facts are not yet fully verified. The app may collect
   and display the reading, but it must label or otherwise classify it as
   experimental.
3. **Unsupported** — the access module/path does not recognize the hardware, or
   attempting a read would require guessing an address, chip selection, register
   map, mutation, or behavior not already described by the current repository
   inputs. This remains disabled.

Verification is independent from `SensorAvailability`. A successfully collected
experimental value is still `Available`; `Experimental` describes the confidence
in the hardware/register path, not whether the current sample produced a value.

Lack of complete primary-source verification is not, by itself, a reason to
disable a reading. When the device is recognized and the existing read-only
decode can be applied without inventing hardware facts, `Experimental` is the
default. This policy applies across vendors; it is not an AMD-only exception.

For AMD CPU package temperature, this means:

- Family 17h and 19h remain **Verified** for the current `RyzenSMU` SMN
  `0x00059800` Tctl path.
- Family 1Ah / Zen 5 becomes **Experimental** when using the same read-only
  `RyzenSMU` path and existing plausibility-gated decode.
- Families not accepted by `RyzenSMU` remain **Unsupported**.

For the other current paths:

- Intel CPU package temperature has no family/model allowlist. A CPU that
  advertises the architectural DTS and package-thermal-management capability
  bits uses the existing Intel MSR path as **Verified**; the capability bits are
  existence gates, not verification allowlists.
- ACPI thermal zones are **Verified** OS/firmware API readings when they pass the
  existing plausibility filter.
- Nuvoton NCT6799D motherboard readings remain **Verified**. Other Super I/O
  chips remain **Unsupported** until an implementation-ready chip profile exists,
  because reusing a register map based only on a similar chip ID would guess
  hardware facts.
- Vendor and OS API providers that already attempt capabilities at runtime do not
  gain coverage from a family allowlist change; their existing success/failure
  contracts remain in place.

## Guardrails

This policy changes runtime enablement, not clean-room provenance:

- Do not consult prohibited monitoring implementations. Implementation still uses
  only `docs/specs/sensors/**`, this repository, and allowed platform/API docs.
- Do not add new register addresses, decode fields, chip selection logic, writes,
  fan control, limit changes, or power-state changes merely to widen coverage.
- Keep access read-only and preserve the existing mutex and optional-component
  rules.
- Keep plausibility gates. For CPU temperature, reject all-zero reads and values
  outside the accepted range rather than publishing them as experimental.
- Mark experimental readings in the data path or presentation path so support
  reports can distinguish them from verified readings.
- Do not show PawnIO installation/permission guidance when the only issue is
  that a source is experimental or unverified. Guidance remains for actual
  missing, permission, load, or runtime failures after an attempted source and
  all useful fallbacks are insufficient.
- Experimental support should graduate to Verified only through a later spec
  update backed by a primary source or maintainer-accepted hardware dump.

## Implementation

The implementation makes the following vertical change:

1. Core owns shared `SensorEnablement` (`Verified`, `Experimental`,
   `Unsupported`) and `SensorVerification` (`Verified`, `Experimental`) models.
2. Core temperature and motherboard readings retain verification independently
   from availability. The headline CPU value also retains the verification of
   the source selected for it.
3. The App event DTO carries structured verification metadata to the frontend.
   Experimental CPU readings also keep an explicit sensor label so the current
   Dashboard and support reports identify them without requiring a new view.
4. AMD Family `0x1A` selects the existing AMD SMN source as `Experimental`;
   families `0x17` and `0x19` remain `Verified`, and families rejected by the
   `RyzenSMU` module remain `Unsupported`.
5. Intel remains capability-driven rather than family/model-gated.
6. Existing plausibility checks, read-only access, mutexes, fallback behavior,
   and optional-component guidance rules remain unchanged.
7. Focused regression tests cover candidate classification, metadata
   propagation, presentation labeling, and guidance suppression.

The Windows-only providers still require a Windows runner or machine for final
runtime proof. Per-CCD temperatures, SMU PM-table metrics, new Zen 5 register
facts, installer bundling, and unverified Super I/O register maps remain out of
scope.
