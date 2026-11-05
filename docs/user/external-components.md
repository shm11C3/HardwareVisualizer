# External Components

[English](external-components.md) | [Japanese](external-components.ja.md)

This page is the source content for External Component Guidance. For now, the
app should open the GitHub-rendered version of this document directly. The
HardwareVisualizer website should later publish content derived from this page,
and the app can switch to that website URL when it exists.

Temporary in-app detail links:

- PawnIO:
  `https://github.com/shm11C3/HardwareVisualizer/blob/develop/docs/user/external-components.md#pawnio`
- smartctl:
  `https://github.com/shm11C3/HardwareVisualizer/blob/develop/docs/user/external-components.md#smartctl`

HardwareVisualizer can collect many hardware signals without optional external
components. Some deeper sensor paths depend on tools or drivers that are
installed separately. HardwareVisualizer never installs, bundles, or enables
those components automatically. On Windows, External Component Setup can
install PawnIO when you ask for it in Settings; see
[Installing PawnIO from HardwareVisualizer](#installing-pawnio-from-hardwarevisualizer).

External Component Guidance appears only after HardwareVisualizer tries to use
an optional component, cannot use it, and fallback collection still leaves
user-visible hardware data unavailable. If fallback collection obtains the
needed information, the app should not show guidance.

## What The Dialog Means

The dialog tells you:

- which optional component could not be used;
- which hardware information remains unavailable;
- what action may make that information available.

The dialog is informational. Hardware monitoring continues with the best
available fallback data.

## Windows Sensor Service

The MSI includes the **Windows sensor service** when installed under Program
Files. Fresh installs and updates, including silent installs, need no separate
service selection. Installation and repair ask for administrator approval;
ordinary launch, tray residence and reconnect do not need to elevate the app.
**Settings → Advanced → Windows sensor service → Repair service** restores the
bundled service if needed.

For a **new installation**, CPU and motherboard sensors need both PawnIO (with
its modules) and the bundled service. PawnIO remains a separate optional driver:
select PawnIO in the interactive MSI, or use **Settings → Advanced → Optional
component setup**. Silent MSI driver setup still requires
`EXTERNAL_COMPONENT_PAWNIO=1`. Without PawnIO, other monitoring continues.

For an **upgrade from direct PawnIO collection**, an existing PawnIO installation
is maintained and the MSI adds the service during the normal update. If sensor
readings remain unavailable
after fallback collection, **Open sensor settings** takes you to the service
setup section. Existing guidance dismissals are respected.

Sensor guidance appears as a nonmodal notice when actual collection failures
leave readings on the current screen unavailable. It follows the normal sensor
updates and disappears on recovery; there is no setup dialog, startup deadline
or separate periodic sensor-guidance check. Closing it hides sensor guidance
for this launch. **Never show again** remembers the currently shown collection
gaps. Settings always remains available for checking or repairing the service.

If **Run as administrator on startup** was enabled, the update keeps that choice.
To use normal permissions, turn that existing setting off, then exit and launch
normally. CPU temperature/power and motherboard readings do not require the
setting; other privileged providers may expose less data. User-created highest-privilege
scheduled tasks or compatibility settings must be changed by their owner.
Release notes and the startup setting explain the change. A successful update
does not show a migration CTA merely because elevated startup remains enabled.
The sensor failure notice is not shown on the Settings screen.

Without the service, ordinary monitoring and available ACPI fallback continue.
The NSIS/portable app may use a compatible installed service; first-time setup
requires the protected MSI. Product uninstall removes the app-owned service
while retaining shared PawnIO. A compatible signed module from the app's known
release catalog is required; unsigned test modules and unrecognized files are
not loaded by the SYSTEM service. Unknown files are kept rather than replaced.

## PawnIO

PawnIO is used on Windows for CPU package temperature and CPU package power
collection when the CPU supports the implemented PawnIO-backed path.

PawnIO may be relevant when the CPU temperature is unavailable in the dashboard.
If HardwareVisualizer can still show a CPU temperature through ACPI thermal
zones, it should not show PawnIO guidance.

### What PawnIO Enables

PawnIO can provide Windows CPU readings from CPU-specific sensor paths:

- Intel package temperature through the Intel MSR module.
- Intel CPU package power through the Intel MSR RAPL registers.
- AMD Family 17h and Family 19h package temperature through the Ryzen SMU
  module.
- AMD CPU package power through the AMDFamily17 RAPL module.

### Installing PawnIO from HardwareVisualizer

On Windows, **Settings → Advanced → Optional component setup** shows whether
the PawnIO runtime and the module files HardwareVisualizer uses are present and
offers installation or an update when needed. When you choose it, HardwareVisualizer:

1. downloads the pinned PawnIO runtime installer and the pinned PawnIO.Modules
   release from their official GitHub releases;
2. verifies each download against the size and SHA-256 digest recorded in the
   app, and discards it on a mismatch;
3. asks Windows for administrator approval, then runs the runtime installer
   unattended if the runtime is not installed yet;
4. copies missing module files into the PawnIO install directory and replaces
   files whose SHA-256 matches an older upstream release. Current and
   unrecognized files, including newer versions, are kept.

An MSI upgrade under Program Files also refreshes outdated module files after
the product installation succeeds, including unattended updates. This refresh
never installs the runtime or adds missing files. A failed refresh does not
fail the upgrade; use Settings to retry. NSIS upgrades use the Settings action.

Restart HardwareVisualizer afterwards. If Windows reports that a restart is
required, restart Windows first.

Uninstalling HardwareVisualizer never removes PawnIO or its module files.
Remove PawnIO from Windows "Apps & features" if you no longer want it.

### Required User Setup

If you prefer to set up PawnIO yourself, for PawnIO-backed CPU package
temperature or power collection the machine needs:

- a working PawnIO driver/runtime installation;
- `PawnIOLib.dll`, which is provided by the PawnIO runtime installation;
- signed CPU-specific module blobs from the PawnIO.Modules release assets:
  - `IntelMSR.bin` for Intel package temperature and power;
  - `RyzenSMU.bin` for AMD package temperature;
  - `AMDFamily17.bin` for AMD package power;
- a connected Windows sensor service to open PawnIO on behalf of the app.

The Windows service loads known signed `.bin` releases only; it does not use
unsigned `.amx` or unrestricted test modules.

To set up the module blob:

1. Install the PawnIO runtime.
2. Download a release asset from
   [namazso/PawnIO.Modules](https://github.com/namazso/PawnIO.Modules/releases).
3. Extract the archive and copy the CPU module file you need into
   `C:\Program Files\PawnIO`.
   - For Intel package temperature or power, copy `IntelMSR.bin`.
   - For supported AMD package temperature, copy `RyzenSMU.bin`.
   - For AMD package power, copy `AMDFamily17.bin`.
   - If you are not sure which CPU path applies, copying all applicable signed
     module files is acceptable.
4. Restart HardwareVisualizer.

Writing to `C:\Program Files\PawnIO` usually requires administrator
permission.

If PawnIO is installed but readings remain unavailable, check or repair the
Windows sensor service in Advanced settings. Elevating the app does not repair
the separate service.

### Official Sources

- PawnIO runtime: <https://github.com/namazso/PawnIO>
- PawnIO modules: <https://github.com/namazso/PawnIO.Modules/releases>

More implementation detail is recorded in
[`../architecture/windows-sensor-external-components.md`](../architecture/windows-sensor-external-components.md).

### PawnIO Motherboard Sensors

PawnIO is also used on Windows for supported Super I/O motherboard temperature
and fan-speed reads. This path currently depends on the PawnIO LpcIO module and
the Windows sensor service being able to open the PawnIO driver.

For PawnIO-backed motherboard sensors, the machine needs:

- a working PawnIO driver/runtime installation;
- `PawnIOLib.dll`, which is provided by the PawnIO runtime installation;
- signed `LpcIO.bin` from a compatible PawnIO.Modules release;
- a connected Windows sensor service, which opens the PawnIO driver for the app.

If PawnIO is installed but motherboard readings remain unavailable, check or
repair the Windows sensor service and its compatible signed LpcIO module.

This guidance only covers access to the read-only motherboard sensor path. It
does not enable fan control, PWM writes, voltages, unsupported Super I/O chips,
or vendor-specific embedded-controller sensors.

## smartctl

`smartctl` is part of smartmontools. HardwareVisualizer can use it as one
Storage Health collection source when native OS paths cannot provide enough
device-health signals.

`smartctl` guidance may be relevant when Storage Health cannot collect important
health signals for at least one storage device. It should not appear if native
or fallback sources already collect the important health signals.

The `smartctl:storage-health:v1` guidance key is cross-platform. Windows,
Linux, and macOS can all surface it when HardwareVisualizer actually attempts
to use `smartctl` and still cannot collect the important Storage Health
signals through the available sources.

Live Storage Health uses the cheap native read path where available and does
not use `smartctl` on its live polling cadence.

### What smartctl Can Improve

For Storage Health, HardwareVisualizer treats these as important health signals:

- SMART overall health;
- temperature;
- NVMe percentage used;
- NVMe available spare;
- reallocated sector count;
- current pending sector count;
- offline uncorrectable sector count;
- NVMe media errors.

Other values, such as power-on hours, power cycle count, unsafe shutdown count,
or error-log entry count, may still be useful in the UI. Missing only those
values should not trigger external component guidance.

### Required User Setup

To use `smartctl`, install smartmontools for your operating system and make
sure the `smartctl` executable is available on `PATH`.

Some storage devices or operating systems may also require elevated privileges
to read SMART data. If `smartctl` is installed but HardwareVisualizer still
cannot collect Storage Health signals, try running HardwareVisualizer with the
permissions required by your operating system and device.

After installing or fixing `smartctl`, run Storage Device Refresh when
available, or restart HardwareVisualizer. Daily Storage Health collection will
also retry on its next scheduled collection.

### Official Source

- smartmontools: <https://www.smartmontools.org/>

## Guidance Reasons

The app may classify an unavailable component as:

- `missing`: the executable, DLL, or module file was not found;
- `permission`: the component exists but cannot be opened with current
  privileges;
- `misconfigured`: the runtime exists but a required driver, service, or module
  is missing;
- `failed`: the component ran but did not return usable data for the target
  device or sensor.

The website should use those classifications to show specific next steps rather
than displaying raw error strings as the main user-facing message.
