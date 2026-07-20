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
  - Intel package temperature path: `IntelMSR.amx` or `IntelMSR.bin`.
  - AMD Family 17h / 19h package temperature path: `RyzenSMU.amx` or
    `RyzenSMU.bin`.

The CPU-specific module blob is not installed by the PawnIO runtime itself.
Users must download a release asset from
<https://github.com/namazso/PawnIO.Modules/releases>, extract the module blob,
and place the required file under `C:\Program Files\PawnIO`.

The collector prefers the implementation-ready spec names (`*.amx`) and then
falls back to the installed module names observed during local validation
(`*.bin`). The module extension is an operational compatibility detail only; it
does not change the CPU register decode path.

The process that opens PawnIO must have enough Windows privileges to access the
driver. On the local Phase 1 validation machine, the `PawnIO` kernel driver
service was installed and running, but a non-elevated process still failed at
`pawnio_open` with `0x80070005`. Running the same probe elevated allowed the
driver open, module load, and CPU package temperature sample to succeed. Until
HardwareVisualizer has an elevated helper or service, users who want
PawnIO-backed CPU package temperature collection across launches should enable
Elevated Startup Mode so the whole app process starts as administrator.

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
- `IntelMSR.amx or IntelMSR.bin not found`.
- `RyzenSMU.amx or RyzenSMU.bin not found`.
- `pawnio_open failed: ...`.
- `pawnio_load failed: ...`.

## Scope Boundaries

The Phase 1 implementation uses read-only CPU package temperature paths:

- Intel: `MSR_TEMPERATURE_TARGET` and `IA32_PACKAGE_THERM_STATUS` through
  `IntelMSR`.
- AMD: SMN `0x00059800` through `RyzenSMU`. Family 17h and 19h are verified;
  other families the `RyzenSMU` module recognizes (e.g. Family 1Ah / Zen 5) are
  enabled best-effort as experimental readings, plausibility-gated and labeled
  experimental. Core diagnostics and sensor samples retain the verification
  level, and the App event projection carries it as structured metadata in
  addition to the experimental sensor label (see ADR 0011).

The motherboard sensor implementation also uses a read-only PawnIO LpcIO path
for the scoped Nuvoton NCT6799D Super I/O bank-4 temperature and direct RPM
registers. It requires `LpcIO.bin` or `LpcIO.amx` and the same process-level
ability to open the PawnIO driver. A non-elevated process can fail at
`pawnio_open` with `0x80070005`; in that case the Dashboard surfaces
`pawnio:motherboard-sensors:v1` guidance and offers the existing elevated
startup action.

The following are not covered by this runtime checklist or by the Phase 1
implementation:

- Installing PawnIO.
- Bundling `PawnIOLib.dll` or module blobs.
- Driver installer integration or bootstrapper work.
- AMD Family 1Ah / Zen 5 *verified* enablement (it is enabled experimentally in
  the current implementation; verification against a primary source is still
  future work).
- AMD per-CCD temperatures or SMU PM-table metrics.
- Threadripper / EPYC multi-die-specific behavior.
- Super I/O chips outside the scoped NCT6799D read path.
- Fan control, PWM writes, voltage sensors, and embedded-controller sensors.

If a future release bundles PawnIO components, update the Windows third-party
notices and release packaging documentation before shipping those artifacts.
