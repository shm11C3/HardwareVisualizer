# Windows Sensor External Components

This document lists the external runtime components required by Windows native
sensor collection. It is an operational checklist, not a hardware fact source.
The clean-room implementation remains derived from the pinned
`docs/specs/sensors/**` implementation-ready specs and this repository.

## CPU Package Temperature

Windows CPU package temperature collection through PawnIO requires a local
PawnIO installation. HardwareVisualizer does not bundle PawnIO. It can install
it only through External Component Setup, an explicit user action that
downloads the pinned upstream release and verifies it before running the
installer; see [ADR 0024](../adr/0024-external-component-setup.md).

Required components:

- A working PawnIO driver installation that `pawnio_open` can open.
- `PawnIOLib.dll`, loaded dynamically from the existing installation.
- One CPU-specific PawnIO module blob:
  - Intel package temperature path: signed `IntelMSR.bin`.
  - AMD Family 17h / 19h package temperature path: signed `RyzenSMU.bin`.

The CPU-specific module blob is not installed by the PawnIO runtime itself.
External Component Setup places the missing module files from the pinned
PawnIO.Modules release; users who set up PawnIO by hand download a release
asset from <https://github.com/namazso/PawnIO.Modules/releases>, extract the
module blob, and place the required file under `C:\Program Files\PawnIO`.

Production uses known signed `.bin` module releases through the optional
LocalSystem Windows sensor service. The normal app authenticates that service
and receives typed readings over local IPC; it does not open PawnIO itself.
Initial service setup and repair require administrator consent through the
protected MSI feature. See [ADR 0029](../adr/0029-windows-sensor-service.md)
and the [service owner guide](../../sensor-service/README.md).

Installing PawnIO alone does not remove its access restriction: the original
Phase 1 probe returned `0x80070005` without elevation even with the driver
running. The service is the privileged caller now. An explicitly saved
Elevated Startup Mode remains until the user switches it off; service setup
never silently changes that preference.

The service provider discovers candidates in these locations, then validates
the final files and directory ACLs before loading them:

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
- `IntelMSR.bin or IntelMSR.amx not found`.
- `RyzenSMU.bin or RyzenSMU.amx not found`.
- `pawnio_open failed: ...`.
- `pawnio_load failed: ...`.

## CPU Package Power (RAPL)

Windows CPU package power uses the same local PawnIO runtime and is sampled
through the read-only RAPL MSR path. Intel temperature and power share one
`IntelMSR` executor. AMD package temperature and power use their separate
module paths.

Required module blobs are:

- Intel temperature and power: signed `IntelMSR.bin`.
- AMD package temperature: signed `RyzenSMU.bin`.
- AMD package power: signed `AMDFamily17.bin`.

The service loads only known signed `.bin` release hashes from protected
directories. Unknown files and unsigned `.amx` test modules are not loaded;
unknown files are never overwritten. Missing or unusable power remains
unavailable and does not populate derived `package_watts`.

## Scope Boundaries

The current implementation uses read-only CPU package temperature paths:

- Intel: `MSR_TEMPERATURE_TARGET` and `IA32_PACKAGE_THERM_STATUS` through
  `IntelMSR`.
- AMD: SMN `0x00059800` through `RyzenSMU`. Family 17h and 19h are verified;
  other families the `RyzenSMU` module recognizes (e.g. Family 1Ah / Zen 5) are
  enabled best-effort as experimental paths with the same plausibility gate.
  Successful values keep the existing Core sample, App event, and Dashboard
  presentation contracts. Verification status is maintained in the sensor
  specification; only a surfaced failure from an experimental attempt carries
  that context (see ADR 0011).

The motherboard sensor implementation also uses a read-only PawnIO LpcIO path
for the scoped Nuvoton NCT6799D Super I/O bank-4 temperature and direct RPM
registers. It requires a compatible signed `LpcIO.bin` and the service's
ability to open the driver. Service/transport failure leaves explicit
unavailability while other monitoring continues. Component guidance points to
service/module repair instead of elevating the resident app.

The following are not covered by this runtime checklist:

- The External Component Setup mechanism itself (pinned artifacts, download,
  verification, installer option); it is owned by
  [`docs/design/external-component-setup.md`](../design/external-component-setup.md).
- Bundling `PawnIOLib.dll` or module blobs.
- AMD Family 1Ah / Zen 5 *verified* temperature enablement (it is enabled
  experimentally in the current implementation; verification against a
  primary source is still future work).
- AMD per-CCD temperatures or SMU PM-table metrics.
- Threadripper / EPYC multi-die-specific behavior.
- Super I/O chips outside the scoped NCT6799D read path.
- Fan control, PWM writes, voltage sensors, and embedded-controller sensors.

HardwareVisualizer downloads PawnIO components at setup time and does not
redistribute them. If a future release bundles PawnIO components instead,
update the Windows third-party notices and release packaging documentation
before shipping those artifacts.
