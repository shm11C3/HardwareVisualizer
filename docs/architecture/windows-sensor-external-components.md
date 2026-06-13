# Windows Sensor External Components

This document lists the external runtime components required by Windows native
sensor collection. It is an operational checklist, not a hardware fact source.
The clean-room implementation remains derived from the pinned
`docs/specs/sensors/**` implementation-ready specs and this repository.

## Phase 1 CPU Package Temperature

Windows CPU package temperature collection through PawnIO requires an existing
local PawnIO installation. HardwareVisualizer does not install, bundle, or
bootstrap PawnIO in Phase 1.

Required components:

- A working PawnIO driver installation that `pawnio_open` can open.
- `PawnIOLib.dll`, loaded dynamically from the existing installation.
- One CPU-specific PawnIO module blob:
  - Intel package temperature path: `IntelMSR.amx`.
  - AMD Family 17h / 19h package temperature path: `RyzenSMU.amx`.

The current collector searches these locations:

- `InstallLocation` under
  `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO`.
- `%ProgramFiles%\PawnIO`.
- `%ProgramW6432%\PawnIO`.
- `%ProgramFiles(x86)%\PawnIO`.

Within each candidate root, the collector searches recursively for
`PawnIOLib.dll` and the selected module file. If the DLL or module is missing,
if the DLL cannot be loaded, if the driver cannot be opened, or if the module
cannot be loaded, CPU package temperature falls back to the existing ACPI
thermal-zone path.

If both PawnIO and ACPI thermal zones are unavailable, the collector reports an
unavailable reason instead of publishing a CPU temperature. Example reasons
include:

- `PawnIOLib.dll not found`.
- `IntelMSR.amx not found`.
- `RyzenSMU.amx not found`.
- `pawnio_open failed: ...`.
- `pawnio_load failed: ...`.

## Scope Boundaries

The Phase 1 implementation only uses read-only CPU package temperature paths:

- Intel: `MSR_TEMPERATURE_TARGET` and `IA32_PACKAGE_THERM_STATUS` through
  `IntelMSR.amx`.
- AMD: SMN `0x00059800` through `RyzenSMU.amx`, enabled only for Family 17h and
  Family 19h.

The following are not covered by this runtime checklist or by the Phase 1
implementation:

- Installing PawnIO.
- Bundling `PawnIOLib.dll` or `*.amx` module blobs.
- Driver installer integration or bootstrapper work.
- AMD Family 1Ah / Zen 5 enablement.
- AMD per-CCD temperatures or SMU PM-table metrics.
- Threadripper / EPYC multi-die-specific behavior.
- Super I/O, fan RPM, voltage, and motherboard sensors.

If a future release bundles PawnIO components, update the Windows third-party
notices and release packaging documentation before shipping those artifacts.
