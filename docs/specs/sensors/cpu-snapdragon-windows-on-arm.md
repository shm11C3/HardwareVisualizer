# Spec: CPU temperature on Snapdragon and other Windows on Arm hosts

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Draft — not implementation-ready |
| Scope | Disposition and evidence record for CPU temperature on Windows hosts whose native machine is ARM64, including Snapdragon X Elite, X Plus, X2 and 8cx-class PCs. No ARM64 build is published. The x64 build runs under emulation on Windows 11 on Arm; Windows 10 on Arm emulates x86 apps only, so the x64 build does not run there (M1). CPU die temperature on Snapdragon is Unsupported. This revision changes no product behavior and adds no runtime condition. It records why the existing x64 build is inferred, not observed on Arm hardware, to fail safely on the PawnIO path. It records the existing thermal-zone fallback as unchanged behavior that is not attributed to the CPU die, with its known risks accepted. It also records the Windows on Arm thermal interfaces, the firmware evidence for Snapdragon X and 8cx-class devices, the evidence that would unblock the `SoCTj` and `TZ99` candidates, and the native ARM64 build prerequisites. Kernel drivers, MSR access, port I/O and any other privileged hardware access on Arm are out of scope. |
| Issue phase | [#2516](https://github.com/shm11C3/HardwareVisualizer/issues/2516) (epic [#2510](https://github.com/shm11C3/HardwareVisualizer/issues/2510)) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| M1 | Microsoft Learn, [How emulation works on Arm](https://learn.microsoft.com/en-us/windows/arm/apps-on-arm-x86-emulation), `ms.date` 2025-11-06, introduction and sections Prism, How emulation works, Detecting emulation | Primary. Which apps Windows 10 and Windows 11 on Arm emulate, user-mode-only emulation, the Arm64X system binaries, emulated processor details, and the host-aware APIs. |
| M2 | Microsoft Learn, [`IsWow64Process2` function (wow64apiset.h)](https://learn.microsoft.com/en-us/windows/win32/api/wow64apiset/nf-wow64apiset-iswow64process2), sections Syntax, Parameters, Return value, Requirements | Primary for the native-machine query. |
| M3 | Microsoft Learn, [Image File Machine Constants (Winnt.h)](https://learn.microsoft.com/en-us/windows/win32/sysinfo/image-file-machine-constants), `ms.date` 2018-05-31 | Primary for the `IMAGE_FILE_MACHINE_*` values. |
| M4 | Microsoft Learn, [ACPI-defined Devices](https://learn.microsoft.com/en-us/windows-hardware/drivers/bringup/acpi-defined-devices), `ms.date` 2024-09-26, sections Thermal zones, Thermal management in Windows, ACPI thermal zones | Primary for the Windows thermal-zone model and the child objects a zone uses. |
| M5 | Microsoft Learn, [Examples, Requirements and Diagnostics](https://learn.microsoft.com/en-us/windows-hardware/design/device-experiences/examples--requirements-and-diagnostics) (thermal management), `ms.date` 2017-05-02, sections Skin temperature sensors, Zones with different trip points, HCK test requirements for modern standby PCs, Thermal management solutions, Performance counters, Providers | Primary for zone-to-sensor cardinality, the skin-sensor example, the SoC-vendor thermal solutions note, the `Thermal Zone Information` counters and their units. |
| M6 | Microsoft Learn, [`IOCTL_THERMAL_READ_TEMPERATURE`](https://learn.microsoft.com/en-us/windows-hardware/design/device-experiences/ioctl-thermal-read-temperature), `ms.date` 2017-05-02 | Primary. The ACPI driver's request to a temperature sensor driver and the tenths-of-a-kelvin output. |
| R1 | This repository, [`pawnio-interface.md`](pawnio-interface.md) revision 9, module sections `IntelMSR`, `RyzenSMU`, `AMDFamily17`, `AMDFamily10`, `LpcIO` ("Target" lines) | Implementation-ready repository specification. Every module this project loads targets x86-64 hardware. |
| P1 | namazso, PawnIO driver repository, [`pawn/include/native.inc` at tag `2.2.0`](https://github.com/namazso/PawnIO/blob/2.2.0/pawn/include/native.inc) (commit `5cdf470831fdfff3f7f1d06363ca6b230f3bf35a`, Git blob `fef769eebc7bed337ad0d986929c9fc62a58cdbc`); same declarations at commit `9d52965895588b0ffa3703b72eec75ba19f4ccc0` (2026-08-14), declarations `get_arch`, `msr_read`, `msr_write` | Upstream-published interface definition of the natives the modules this project loads call. Interface facts only; no code was copied. |
| P2 | namazso, PawnIO.Modules module sources `IntelMSR.p`, `RyzenSMU.p`, `AMDFamily17.p`, `AMDFamily10.p`, `LpcIO.p` at tags `0.2.11` and `0.2.12` | Upstream-published interface definitions of the modules this project loads (the S5 and S13 sources of R1), LGPL-2.1-or-later. Load-time behavior only; no code was copied. |
| R2 | This repository at commit `c1dfd6547c96414b41d8f7207e29c5c7ae5bf513`: `hardviz/core/src/infrastructure/providers/windows/thermal_zone.rs`, `hardviz/core/src/utils/thermal.rs` (`select_cpu_temperature`), `hardviz/core/src/platform/windows/sensors.rs` (`sample_temperatures`, `build_temperature_sample`), `hardviz/core/src/infrastructure/providers/windows/pawn_io/cpu/identity.rs` (CPUID access), and `.github/workflows/publish.yml` (`publish-tauri` matrix) | Repository facts for the current Windows thermal-zone path, its headline fallback, and the release matrix. |
| B1 | Rust project, [Platform Support](https://doc.rust-lang.org/nightly/rustc/platform-support.html), table Tier 1 with Host Tools, row `aarch64-pc-windows-msvc` "ARM64 Windows MSVC", retrieved 2026-11-28 | Primary for the toolchain tier. |
| B2 | Tauri, [Windows Installer](https://v2.tauri.app/distribute/windows-installer/), section Building for 32-bit or ARM, retrieved 2026-11-28 | Primary for the ARM64 build steps and the NSIS installer architecture. |
| B3 | GitHub changelog, [Arm64 hosted runners for public repositories are now generally available](https://github.blog/changelog/2025-08-07-arm64-hosted-runners-for-public-repositories-are-now-generally-available/), 2025-08-07 | Primary for the `windows-11-arm` hosted runner in public repositories. |
| B4 | `microsoft/winget-pkgs`, [`manifests/n/namazso/PawnIO/2.2.0/namazso.PawnIO.installer.yaml`](https://github.com/microsoft/winget-pkgs/blob/master/manifests/n/namazso/PawnIO/2.2.0/namazso.PawnIO.installer.yaml), Git blob `8f2cc9cab36b631950cba51037babd37c8c9e7d0` | Package-manager metadata. It lists one installer URL and SHA-256 (`1F519A22E47187F70A1379A48CA604981C4FCF694F4E65B734AAA74A9FBA3032`) under both `x64` and `arm64`. |
| D1 | aarch64-laptops/build at commit [`2e58842f5fa2f87771c2df017ae4d8c65225ef10`](https://github.com/aarch64-laptops/build/tree/2e58842f5fa2f87771c2df017ae4d8c65225ef10/misc), `misc/qualcomm-snapdragon-dev-kit/acpi/dsdt.dsl`, Git blob `46375ae9c50d7e915594660628d6c6904724886c` (SHA-256 `40ef2d577b094da66302c037d9cf641929ca8daee1255ee116c527fdfd53e3ac`) | Independent firmware dump, not maintainer-accepted. Snapdragon Dev Kit for Windows (Snapdragon X Elite); DSDT OEM Table ID `SDM8380`. |
| D2 | Same commit, `misc/lenovo-yoga-slim-7x/acpi/dsdt.dsl`, Git blob `bd881ddfb832f25abcbcdb396ea2c8d33b601f29` (SHA-256 `3ddd4e0903ce4473ef0172e6f7969ad150044ceb86b567ddd9f7c256ed961ea5`) | Independent firmware dump, not maintainer-accepted. Lenovo Yoga Slim 7x (Snapdragon X Elite); DSDT OEM Table ID `CB-01`. |
| D3 | Same commit, `misc/acer-sfa14-11/dsdt.dsl`, Git blob `4804d8190da53daba119c4d426b51fde4cfcca18` (SHA-256 `5dc071d3d19c342221dd4974bbdcf7b3b5dec44a1cfd7c5e6f1ce05234c6561f`), and `misc/acer-sfa14-11/HWiNFO-ARM64.LOG` | Independent firmware dump, not maintainer-accepted. Acer SFA14-11; the log names Qualcomm Snapdragon X Elite X1E-78-100 and Windows 11 ARM64 build 26200.7171. DSDT OEM Table ID `SDM8380`. |
| D4 | Same commit, `misc/microsoft-surface-pro-11/acpi/dsdt.dsl`, Git blob `387fa1e89c1764cd4f1175fda38a1bc7e69d8d44` (SHA-256 `106bd9d1842b67a4d92bd2f7044026e33a89e1a931ad92e6f2992fb530b1bf00`); `misc/microsoft-surface-pro-11/acpi/ssdt.dsl`, Git blob `3583529421bab8f4800c8710311655970df549d5` (SHA-256 `5e4038de48a457f5535ec70317763a1377a149fd4d96a5834a285dd67395100b`); `note.txt` ("X1E80100 variant") | Independent firmware dump, not maintainer-accepted. Surface Pro 11th Edition, X Elite variant. DSDT OEM Table ID `SDM8380`; SSDT OEM Table ID `SsdtSurf`. |
| D5 | Same commit, `misc/lenovo-thinkpad-x13s/dsdt.dsl`, Git blob `eb5ad6f4e3efeef742a8f13956ad5f420b1153d7` (SHA-256 `e83edbb4896fdfd60d26b6adc7afb0d83d3fdd58a01622cf64639e4b447fefd1`), and `misc/lenovo-thinkpad-x13s/HWiNFO-ARM64.LOG` | Independent firmware dump, not maintainer-accepted. ThinkPad X13s Gen 1; the log names Qualcomm Snapdragon 8cx Gen 3 and Windows 11 ARM64 build 26100.3476. DSDT OEM Table ID `SDM8280`. |
| D6 | Same commit, `misc/microsoft-surface-prox/dsdt.dsl`, Git blob `f9a68702285958094d1c6b736af197a42c2ac220` (SHA-256 `99ae117c02a57c7ac1edbeffed5c2386331eaf9f4d23528ac0b97718eba9efb5`) | Independent firmware dump, not maintainer-accepted. Surface Pro X (SQ1 or SQ2, both 8cx-class; the dump does not say which). DSDT OEM Table ID `SDM8180`. |
| D7 | Same commit, `misc/lenovo-flex-5g/dsdt.dsl`, Git blob `1c659e6593a81711162594d8dc205beaf6ba7275` (SHA-256 `90c013bc9f1a49385644bf19cd40107698f6cc61237ad97b07609ab92e54cfd5`) | Independent firmware dump, not maintainer-accepted. Lenovo Flex 5G (Snapdragon 8cx); DSDT OEM Table ID `CB-01`. |
| L1 | spenceriam/armtemp at commit [`aa89a67c554ee6468a9a326672d4579a630fee7b`](https://github.com/spenceriam/armtemp/tree/aa89a67c554ee6468a9a326672d4579a630fee7b), `README.md` (Git blob `d1033b264d55407ec529601623a6a1c2c7a5c190`, data sources) and `SENSORS.md` (Git blob `a22441041c1336fa652e466fe9bd0cd144b0b2b0`, probe results), AGPL-3.0 | **lead-only (copyleft)**: a monitoring implementation's own description of its probe results. Only its prose observations were read; no code was read or copied. |
| U1 | spenceriam/armtemp [issue #8](https://github.com/spenceriam/armtemp/issues/8) "86 degrees on X2EE", opened 2026-11-06, and its comment of 2026-11-26 | User observations of that tool's output, not code. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| None. This revision adds no runtime condition; current behavior is unchanged. | — | — |

- Maintainer decision on PR #2804 (2026-11-28): keep the current
  behavior on Arm hosts. The application is not used on Arm today, and
  the current behavior does no practical harm. The PawnIO section
  below infers from interface definitions and module sources that the
  existing x64 build fails safely there; this is not observed on Arm
  hardware (Open question 6).
- If a later revision changes Arm behavior, the host is identified by
  `IsWow64Process2`, not by CPUID or `GetNativeSystemInfo`, because an
  emulated process sees an emulated processor (M1, M2, M3).
- The SoC model, the processor name string, the processor identifier
  (for example "ARMv8 (64-bit) Family 8 Model 2 Revision 201 Qualcomm
  Technologies Inc", reported in U1's thread), the OEM, the firmware
  OEM Table ID and the Windows build are evidence identities. They
  never gate a read.

| Scope | Status | Default enablement |
| --- | --- | --- |
| CPU die temperature on Snapdragon and other Windows on Arm hosts | No documented OS, Microsoft or Qualcomm route reaches it. No primary source and no dump identifies any zone as the CPU die temperature. | Unsupported. |
| PawnIO CPU package temperature, core and CCD temperatures, CPU package power and Super I/O readings on an ARM64 host | Existing behavior, unchanged. The attempt fails before any hardware access: a kernel driver on this host must be ARM64 (M1), and every module rejects a non-x86-64 architecture with `STATUS_NOT_SUPPORTED` at load, before any hardware access (P1, P2; inferred from the sources, not observed on Arm hardware, Open question 6). See PawnIO on an Arm host. | Unsupported; the existing attempt fails safely at driver open or module load. |
| Headline CPU temperature from the existing thermal-zone fallback (zone-name match or hottest zone) on an ARM64 host | Existing behavior, unchanged (R2), kept by maintainer decision. The value is a thermal-zone reading that no source attributes to the CPU die: a zone's device list names what the zone throttles and does not locate its sensor (M5), and no dump or document identifies the sensor. Accepted risks: see Accepted risks of the unchanged behavior. | Unchanged existing behavior. Not a CPU die temperature scope. |
| ACPI thermal-zone readings on an ARM64 host, listed under the firmware zone name | Existing OS-API path and decode (R2), Verified as OS/firmware API readings under [ADR 0011](../../adr/0011-experimental-sensor-enablement.md). Counter units are primary (M5). Lead-only observation L1 reports 36 instances on one Snapdragon X Plus device; whether the current WMI transport returns them in the application process is Open question 3. | Unchanged existing behavior. |
| SoC junction temperature from the zone whose firmware `_STR` reads "SoCTj Thermal-zone for Fan Control" | Candidate. The string is in D1, D2 and D3, not in D4, D5, D6 or D7. No observation joins it to a counter instance or a varying value. See Open question 1. | Unsupported. |
| CPU or SoC temperature from the zone whose device list names every CPU core together with the GPU and other SoC devices (named `TZ99` in D1–D7) | Candidate. The sensor behind it is a Qualcomm device of unknown meaning. See Open question 2. | Unsupported. |
| A Qualcomm-published Windows temperature interface | None found. See Open question 5. | Unsupported. |

## Interface facts

### Windows on Arm host (M1–M3)

| Item | Fact | Source |
| --- | --- | --- |
| Emulated apps | Windows 11 on Arm emulates x86 and x64 apps. Windows 10 on Arm emulates x86 apps only. | M1 introduction |
| Prism | Windows 11 24H2 includes the Prism emulator. It is available on all supported Windows 11 on Arm devices with 24H2. | M1 Prism |
| User mode only | Emulation supports user-mode code only and does not support drivers. Every kernel-mode component must be compiled as Arm64. | M1 How emulation works |
| x64 OS access | x64 apps have no WOW64 layer. System binaries are Arm64X files that load into x64 and Arm64 processes, so x64 apps access the whole file system and registry without redirection. | M1 How emulation works |
| Emulated identity | Apps under emulation that query processor details receive the emulated virtual processor's details. `GetNativeSystemInfo` also returns emulated details to them. `IsWow64Process2` conveys the Arm64 host. | M1 Detecting emulation |
| Native-machine query | `IsWow64Process2(hProcess, pProcessMachine, pNativeMachine)`. The handle needs `PROCESS_QUERY_INFORMATION` or `PROCESS_QUERY_LIMITED_INFORMATION`. `pProcessMachine` is `IMAGE_FILE_MACHINE_UNKNOWN` when the process is not a WOW64 process. `pNativeMachine` receives the native architecture of the host. A zero return is failure, with details from `GetLastError`. Minimum client Windows 10, version 1709. | M2 |
| Machine constants | `IMAGE_FILE_MACHINE_UNKNOWN` = `0`, `IMAGE_FILE_MACHINE_I386` = `0x014C`, `IMAGE_FILE_MACHINE_AMD64` = `0x8664`, `IMAGE_FILE_MACHINE_ARM64` = `0xAA64`. | M3 |

### PawnIO on an Arm host (R1, P1, P2, B4)

| Item | Fact | Source |
| --- | --- | --- |
| Module targets | `IntelMSR`: Intel x86-64 CPUs. `RyzenSMU`, `AMDFamily17`, `AMDFamily10`: AMD x86-64 CPUs. `LpcIO`: x86-64 systems. | R1 |
| Driver architecture | Emulation does not run kernel-mode code, so a PawnIO driver that runs on an Arm host is an ARM64 build (M1). The `get_arch` native reports the driver's architecture: `ARCH_X64` = 1 (x86-64) or `ARCH_A64` = 2 (ARM64). | M1, P1 |
| ARM64 `msr_read` | On ARM64, `msr_read` and `msr_write` take no MSR index. They take an assembled `mrs x0, <reg>` instruction word that selects an ARM system register. An x86 MSR index has no meaning there. | P1 |
| Module load check | `IntelMSR`, `RyzenSMU`, `AMDFamily17`, `AMDFamily10` and `LpcIO` each reject a non-x86-64 architecture with `STATUS_NOT_SUPPORTED` at module load, before any hardware access. This is read from the module sources at tags `0.2.11` and `0.2.12`; it is inferred, not observed on Arm hardware (Open question 6). | P2 |
| Installer architectures | The winget manifest for PawnIO 2.2.0 lists the same installer under `x64` and `arm64`. A PawnIO installation on an Arm host therefore says nothing about a usable CPU or Super I/O path. | B4 |

Consequence for the existing x64 build on an Arm host (R2): CPU source
selection runs on the emulated CPUID identity (M1) and may choose an
Intel or AMD path. The attempt then ends at one of three points:
`PawnIOLib.dll` is missing or does not load into the x64 process; the
driver cannot be opened; or the driver is the ARM64 build and the
module's load fails with `STATUS_NOT_SUPPORTED` (P1, P2). None of these
reaches a hardware access, by inference from these sources; no Arm
observation confirms it (Open question 6). The existing PawnIO failure handling then
falls back to the thermal-zone path (R2). The x86 MSR indexes that the
CPU paths would pass never reach the ARM64 `msr_read`, because the
module never loads (P1, P2).

### Windows thermal-zone model (M4–M6)

| Item | Fact | Source |
| --- | --- | --- |
| Model | The Windows thermal model is based on ACPI thermal zones. Sensor devices track the temperature in each zone. | M4 Thermal zones |
| Zone children | `_TZD` lists the nonprocessor devices in the zone, `_PSL` the processors. `_PSV`, `_HOT` and `_CRT` are the passive, hibernate and shutdown thresholds. A zone reports its temperature either through `_TMP` (firmware-reported) or through `_HID` and `_CRS`, which load a temperature sensor driver and allocate its resources. | M4 ACPI thermal zones |
| Sensor driver request | The ACPI driver sends `IOCTL_THERMAL_READ_TEMPERATURE` to a temperature sensor driver. Its output is a `ULONG` zone temperature in tenths of a kelvin. Microsoft documents it only as a request from the ACPI driver to the sensor driver. | M6 |
| Sensors per zone | A thermal zone has no more than one thermal sensor. Several zones can share one sensor, for example to throttle gradually with different trip points. | M5 Zones with different trip points |
| Device list versus sensor location | In Microsoft's skin-sensor example, a sensor placed between two devices feeds a zone that throttles both devices. A zone's device list is the set of devices it throttles; the sensor can be a skin sensor away from them. | M5 Skin temperature sensors |
| SoC vendor solutions | SoC vendors may use their own thermal solutions that Windows supports, for example PEP implementations on Arm. | M5 Thermal management solutions |
| Zone requirement | Every modern standby PC has at least one thermal zone, and each zone must report the actual temperature of its sensor. | M5 HCK test requirements |
| Counters | For each zone, the `Thermal Zone Information` counters report the percent passive limit (100 means not throttled), the temperature in kelvin, and the throttle reason (`0x0` not throttled, `0x1` thermal, `0x2` current limit). They are polled only when requested. | M5 Performance counters |
| Tracing | The `Microsoft-Windows-Thermal-Polling` provider polls every zone's temperature in a trace session. | M5 Providers |

### Current repository path (R2)

| Item | Fact | Source |
| --- | --- | --- |
| Sources | A sampler thread queries `MSAcpi_ThermalZoneTemperature` in `root\WMI` and `Win32_PerfFormattedData_Counters_ThermalZoneInformation` in `root\CIMV2` through WMI, preferring the source that last answered, every 2 s. After five consecutive failed or empty reads it polls every 60 s instead, and keeps retrying so that a transient WMI outage recovers. | R2 `thermal_zone.rs` |
| Decode | `CurrentTemperature` and `HighPrecisionTemperature` as tenths of a kelvin, `Temperature` as whole kelvin; readings outside -40 to 150 °C are dropped. Zone names are the last path segment with a numeric `_<n>` suffix removed. | R2 `thermal_zone.rs`, `thermal.rs` |
| Headline fallback | When no PawnIO CPU temperature is available, the headline CPU temperature is the first zone whose name contains `CPU` or `PROC`, otherwise the hottest zone. | R2 `thermal.rs`, `sensors.rs` |
| CPU identity | The PawnIO CPU source is chosen from CPUID. CPUID is executed only in x86 and x86-64 builds; other builds get no CPUID leaf. An x64 build under emulation therefore selects a source from the emulated identity (M1). | R2 `pawn_io/cpu/identity.rs`, M1 |
| Release matrix | The Windows release job builds on `windows-latest` with no target argument, which produces x64 artifacts only. No ARM64 Windows artifact is published. | R2 `publish.yml` |

## Firmware evidence (D1–D7)

All seven dumps are ACPI tables of shipping or reference Windows on
Arm devices. Temperatures and thresholds below are the raw AML
integers. Read as tenths of a kelvin, the encoding Windows uses for
zone temperatures (M6), `0x0EC4` is 378.0 K and `0x0F28` is 388.0 K.

| Dump | Thermal zones | Zones with `_TMP` | Zones with `_HID` | `TZ99` `_HID` / `_UID` | `TZ99` device list | `TZ99` `_PSV` / `_CRT` | `TZ98` with `_STR` "SoCTj Thermal-zone for Fan Control" |
| --- | --- | --- | --- | --- | --- | --- | --- |
| D1 Snapdragon Dev Kit (X Elite) | 35 | 7 (`TZ39`–`TZ45`) | 28 | `QCOM0C5A` / `0x64` | 12 CPU cores in three clusters, `PEP0`, `GPU0`, `GPU0.MON0`, `WLTM`, `PMBM`, `CSW0` | `0x0EC4` / `0x0F28` | Yes (`QCOM0C5A`, `_UID` 1; fan in `_AL0`) |
| D2 Yoga Slim 7x (X Elite) | 35 | 7 | 28 | `QCOM0C5A` / `0x64` | Same as D1 | `0x0EC4` / `0x0F28` | Yes |
| D3 Acer SFA14-11 (X1E-78-100) | 35 | 7 | 28 | `QCOM0C5A` / `0x64` | Same as D1 | `0x0EC4` / `0x0F28` | Yes |
| D4 Surface Pro 11 (X Elite) | 27 in the DSDT and 13 in the Surface SSDT | 0 | 40 | `QCOM0C5A` / `0x64` | Same as D1 | `0x0EC4` / `0x0F28` | No |
| D5 ThinkPad X13s (8cx Gen 3) | 49 | 6 (`TZ70`–`TZ75`) | 43 | `QCOM065A` / `0x64` | 8 CPU cores, `PEP0`, `GPU0`, `GPU0.MON0`, `WLTM`, `PMBM`, `CSW0` | `0x0EC4` / `0x0F28` | No |
| D6 Surface Pro X (SQ1 or SQ2) | 20 | 0 | 20 | `QCOM045E` / `0x64` | 8 CPU cores, `PEP0`, `AMSS`, `MPTM`, `GPU0`, `GPU0.MON0`, `COEX`, `PMBM` | `0x0EC4` / `0x0F28` | No |
| D7 Flex 5G (8cx) | 18 | 0 | 18 | `QCOM045E` / `0x64` | Same as D6 | `0x0EC4` / `0x0F28` | No |

Further facts from the dumps:

- In D1–D3 the zones that use `_TMP` (`TZ39`–`TZ45`) obtain their
  value through an I2C controller's `_DSM`, one channel per zone,
  retrying up to five times with a 50 ms sleep. The same `_TMP` can
  also write threshold values to that device when an internal flag is
  set. In D1 and D3 their device lists name only `PEP0`; in D2, `TZ39`
  and `TZ43` also list CPU cores and others list `GPU0` or `PMBM`. In
  D1, `TZ39`'s `_STA` returns 0 for some platform values. (D1, D2, D3)
- The CPU cores appear in the device lists of several zones with
  different sensor `_HID`s. In D1–D4: `TZ16` (all cores), `TZ17`,
  `TZ18` and `TZ19` (one four-core cluster each) and `TZ99`. `TZ32`
  lists all cores in D1 and D3, and `PEP0` with `GPU0` in D2 and D4.
  (D1–D4)
- The Surface SSDT zones `TZ01`–`TZ0C` use `_HID` `MSHW0188`, with
  `_STR` values `RTS1`–`RTS8` and `VTS1`–`VTS4`. The dump does not
  explain the strings. (D4)
- Every DSDT in D1–D7 refers to `TZ99` by the path `\_SB.TZ99`. L1
  reports counter instance names of this `\_SB.<zone>` form. (D1–D7,
  L1)

## Lead and user observations (L1, U1)

These observations back no scope. They explain the Unsupported rows,
describe the accepted risks of the unchanged behavior, and shape the
open questions.

| Observation | Source |
| --- | --- |
| On a Surface-class Snapdragon X Plus X1P64100 with Windows 11 Pro build 26200 ARM64, `Win32_PerfFormattedData_Counters_ThermalZoneInformation` returned 36 zones named after ACPI paths (`\_SB.TZxx`). `MSAcpi_ThermalZoneTemperature` and `Win32_TemperatureProbe` returned no instances. The elevation state of the probe is not stated. | L1 (lead-only) |
| Under load, `TZ0`, `TZ1` and `TZ99` read 349 K. `TZ31` and `TZ32` read 255 K and `TZ33`–`TZ37` read 233 K. `TZ17`, `TZ04` and `TZ15` read 0. The author found no zone-to-component mapping and no per-core temperature. | L1 (lead-only) |
| `Temperature` is in kelvin and `HighPrecisionTemperature` in tenths of a kelvin. | L1 (lead-only) for `HighPrecisionTemperature`; M5 for `Temperature` |
| A WMI query through the Rust `wmi` crate from inside a Tauri process failed with `WBEM_E_NOT_FOUND`. The same counters were read through PDH. | L1 (lead-only) |
| On an X2 Elite Extreme X2EE-94-100, the tool's hottest-zone temperature stayed at 86 °C. Another user saw the same on an ASUS Zenbook A16 after resuming from sleep at a high temperature. | U1 |

Whether the zones in L1 list the same devices as the zones with the
same names in D1–D4 is unconfirmed (Open question 8).

## Current behavior on an Arm host (unchanged)

This revision prescribes no implementation change. It records what the
existing x64 build does on an Arm host, from the repository at R2 and
the facts above.

1. The application runs under emulation on Windows 11 on Arm only
   (M1).
2. PawnIO CPU, power and Super I/O sources are attempted as on x86.
   They fail before any hardware access (PawnIO on an Arm host). The
   existing failure handling applies, so PawnIO guidance can be offered
   when the thermal-zone path also yields no reading.
3. The thermal-zone sampler queries the same two WMI sources with the
   same decode, plausibility filter and zone names (R2).
4. The headline CPU temperature comes from the existing fallback: the
   first zone whose name contains `CPU` or `PROC`, otherwise the hottest
   zone (R2). This value is a thermal-zone reading. No source attributes
   it to the CPU die.

The existing counter decode is: `Temperature` K − 273.15 = °C (M5 for
the unit), and `HighPrecisionTemperature` / 10 − 273.15 = °C (lead-only
L1 and R2; the existing code already uses it). No new value is decoded.

## Accepted risks of the unchanged behavior

The maintainer accepted these risks with the decision on PR #2804.
They are properties of existing behavior, not a new scope.

| Risk | Evidence |
| --- | --- |
| The hottest zone can stay at a constant value, such as 86 °C. When no zone name contains `CPU` or `PROC`, that zone becomes the headline CPU temperature; otherwise the first matching named zone is used (R2). | U1 (X2 Elite Extreme; Zenbook A16 after resume), R2 |
| A zone can read a constant 255 K. The existing -40 to 150 °C filter keeps it, so the zone list shows about -18 °C. | L1 (lead-only) |
| Zones can read 0. The existing filter drops them, together with the constant 233 K zones. | L1 (lead-only) |
| No zone name in D1–D7 contains `CPU` or `PROC`, so on those firmware tables the headline is the hottest of the readable zones the transport returns; when it returns none, there is no headline temperature. That zone's sensor is not identified as the CPU die by any source. | D1–D7, M5 |
| Which devices the L1 zones list is unconfirmed. | Open question 8 |

## Safety notes

- This revision adds no read and removes none. The existing PawnIO
  attempt on an Arm host ends before any hardware access, by inference
  from the interface definitions and module sources (P1, P2); this is
  not observed on Arm hardware (Open question 6). No
  kernel driver, PawnIO module, MSR, SMN, port I/O or other privileged
  hardware access is proposed for Arm.
- A later revision must not load a module that lacks the load-time
  architecture check, and must not pass x86 MSR indexes to an ARM64
  driver: there `msr_read` interprets its argument as an ARM
  system-register instruction word (P1).
- The thermal-zone path stays an OS API query. Windows obtains each
  value from the zone's `_TMP` or sensor driver (M4, M6). On D1–D3,
  the `_TMP` zones run firmware I2C transactions with retries, and the
  same method can write thresholds (D1–D3). This revision does not
  change how often the existing path queries these zones. Any later
  revision that queries `MSAcpi_ThermalZoneTemperature` more often on
  Arm must account for this firmware cost first.
- `IOCTL_THERMAL_READ_TEMPERATURE` is a request from the ACPI driver
  to a sensor driver (M6). The application must not send it, or any
  other IOCTL, to thermal or sensor devices.

## Target dispositions

In every row, the current behavior described above is unchanged.

| Target | Evidence in this revision | Disposition |
| --- | --- | --- |
| Snapdragon X Elite (X1E) | D1–D4 firmware; no counter observation on a dumped device | CPU die temperature Unsupported. PawnIO attempt fails safely (inferred, Open question 6). Headline from the existing zone fallback, not attributed to the CPU die. SoC junction candidate: Open question 1. `TZ99` candidate: Open question 2. |
| Snapdragon X Plus (X1P) | L1 counter observation (lead-only); further X Plus dumps exist in the same corpus at the D1 commit but were not analysed | Same as X Elite. |
| Snapdragon X2 Elite, X2 Elite Extreme, X2 Plus | U1 observation only; no firmware dump analysed | Same as X Elite. Attribution needs a dump and a counter capture, as in Open questions 1 and 2. |
| Snapdragon 8cx Gen 3 and SQ3 | D5 firmware (ThinkPad X13s, 8cx Gen 3); no SQ3 dump analysed; no counter observation | Same as X Elite. In D5, `TZ99` exists with `QCOM065A` and there is no `SoCTj` zone. SQ3 firmware: Open question 9. |
| Snapdragon 8cx Gen 1 and Gen 2, SQ1 and SQ2 | D6 firmware (Surface Pro X, SQ1 or SQ2, not identified) and D7 firmware (Flex 5G, 8cx); no 8cx Gen 2 dump analysed; no counter observation | Same as X Elite. In D6 and D7, `TZ99` exists with `QCOM045E` and there is no `SoCTj` zone. Per-variant firmware: Open question 9. |
| Other Windows on Arm hosts (older Snapdragon, virtual machines) | None | Same as X Elite. Zones may be absent; the readings are then empty. |
| `MSAcpi_ThermalZoneTemperature` on Snapdragon X | L1: no instances on one X Plus device. D4 defines no `_TMP` zone; D1–D3 define seven each, read through an I2C controller. | Kept as the existing first source; it is not a CPU temperature source. |
| Windows thermal manager and `Thermal Zone Information` counters | M5 units; L1 observation | The existing zone path. No zone is attributed to the CPU die. |
| Qualcomm-published interfaces | None found (Open question 5) | Unsupported; revalidate when Qualcomm publishes a user-mode interface. |
| Kernel drivers, PawnIO modules, MSR, SMN, port I/O on Arm | Out of policy | Not candidates. |
| Native ARM64 Windows build and distribution | See below | Follow-up candidate; nothing in this revision depends on it. |

## Build and distribution prerequisites (informative)

These facts are for a native ARM64 build follow-up. The current
behavior above describes the x64 build running under emulation.

| Item | Fact | Source |
| --- | --- | --- |
| Current artifacts | Windows releases are x64 only. | R2 `publish.yml` |
| x64 build on Arm | Runs under emulation on Windows 11 on Arm only, not on Windows 10 on Arm. Its kernel-mode components cannot run. | M1 |
| Rust target | `aarch64-pc-windows-msvc` is a Tier 1 target with host tools. | B1 |
| Tauri build | Install the Visual Studio C++ ARM64 build tools, add the `aarch64-pc-windows-msvc` target, and build with `--target aarch64-pc-windows-msvc`. The NSIS installer itself stays x86 and runs under emulation; the application is native ARM64. The page states nothing about MSI for ARM64. | B2 |
| CI runner | Windows arm64 hosted runners (`windows-11-arm`) are generally available for public repositories. | B3 |
| WebView2 | The WebView2 runtime ships with Windows 11. | B2 |

## Open questions

Each entry names the row or behavior it affects. None blocks the
current behavior, which this revision leaves unchanged.

1. Blocks the SoC junction candidate row.
   The firmware describes `TZ98` as "SoCTj Thermal-zone for Fan
   Control" in D1–D3. Unknown: whether Windows creates a
   `Thermal Zone Information` instance for a zone that has only active
   cooling objects (`_AC0`, `_AL0`) and no `_PSV`; whether its value
   varies with load; whether "SoCTj" means the SoC junction maximum;
   and how a user-mode process can read a zone's `_STR` and ACPI path
   so that selection uses the firmware's own description rather than
   the zone name. Unblocking evidence: an opt-in report through
   [#2500](https://github.com/shm11C3/HardwareVisualizer/issues/2500)
   from a device with this string, containing the counter instance
   list with idle and load values over time, and the thermal-zone PnP
   device list with each device's hardware ID, ACPI path and
   description properties. A maintainer decision on the label ("SoC"
   versus "CPU") is also needed.
2. Blocks the `TZ99` candidate row.
   `TZ99` lists every CPU core and other SoC devices, with the same
   passive and critical trip points in all seven dumps (378.0 K and
   388.0 K read as tenths of a kelvin). Its sensor `_HID` (`QCOM0C5A`,
   `QCOM065A`, `QCOM045E`, `_UID` `0x64`) is not documented publicly,
   and M5 shows that a device list does not locate the sensor. In L1,
   `TZ99` equalled the hottest zones under load. Unblocking evidence:
   Qualcomm or OEM documentation of that sensor device, or a #2500
   report that pairs the `TZ99` counter with a documented on-die
   reading under controlled load.
3. Affects whether the unchanged zone path returns anything on Arm.
   It is unknown whether the current WMI transport returns
   `Win32_PerfFormattedData_Counters_ThermalZoneInformation` inside the
   HardwareVisualizer process on an Arm host, as an x64 process under
   emulation or as a native ARM64 process. L1 reports `WBEM_E_NOT_FOUND`
   from the `wmi` crate inside a Tauri process while PDH worked. The
   privilege the counters need is also unrecorded. Evidence: a #2500
   report from an Arm device with the thermal-zone source result. A PDH
   transport for this counter set would be a separate revision.
4. Accepted risk; revisit trigger for the maintainer decision.
   Some zones read a constant (255 K, 233 K) or 0 on the observed
   device (L1), and a hottest-zone value can stay constant (U1).
   Hiding such zones or withdrawing the hottest-zone headline on Arm
   was proposed and declined on PR #2804 because the application is
   not used on Arm today. Revisit when #2500 reports or user issues show
   Arm use. Hiding constant zones would also need a time series showing
   they never change; a fixed floor such as 0 °C would be a new
   heuristic without a primary source.
5. Blocks any Qualcomm-interface row.
   No public Qualcomm user-mode temperature API for Windows on
   Snapdragon was found in searches of Qualcomm and Microsoft
   documentation on 2026-11-28. The Qualcomm sensor devices are
   reachable only as ACPI thermal-zone sensor drivers (M4, M6, D1–D7).
   Revalidation trigger: Qualcomm publishes such an interface, or
   Microsoft documents a user-mode thermal-zone API beyond the
   performance counters.
6. Affects a later revision that changes Arm behavior.
   This author did not observe `IsWow64Process2` from an x64 process
   on Arm hardware. M1 lists it as conveying the Arm64 host and M2
   defines `pNativeMachine` as the host architecture. A #2500 report
   from an Arm device would confirm the value seen by the shipped x64
   build. The PawnIO failure points in PawnIO on an Arm host are also
   derived from interface definitions, not observed on Arm hardware.
7. Affects the native ARM64 build follow-up.
   Whether every native dependency builds for `aarch64-pc-windows-msvc`
   (the bundled DuckDB, `nvapi`, the sensor service and its WiX
   fragments), and whether the MSI bundler produces an ARM64 package,
   is not verified. The follow-up needs a `windows-11-arm` CI build
   (B3).
8. Affects any CPU attribution from L1.
   The zone names that L1 reports also occur in D1–D4. There, `TZ17`
   lists one CPU cluster in all four dumps, and `TZ32` lists all CPU
   cores in D1 and D3. L1's device is not one of the dumped devices,
   so it is unconfirmed whether its zones with those names have the
   same device lists. Unblocking evidence: a #2500 report or an ACPI
   dump from the observed device model that pairs each zone's counter
   instance with its device list.
9. Affects the per-variant 8cx-class dispositions.
   The firmware facts for 8cx-class devices come from three dumps only:
   D5 (8cx Gen 3), D6 (SQ1 or SQ2, not identified) and D7 (8cx). No
   SQ3 or 8cx Gen 2 dump was analysed, and D6 does not show which of
   SQ1 and SQ2 it covers. Whether the other variants define `TZ99`
   with the same sensor `_HID`, and whether any defines a `SoCTj`
   zone, is unknown. Evidence: ACPI dumps from those variants, for
   example the Surface Pro 9 5G (SQ3) and 8cx Gen 2 dumps in the same
   corpus as D1.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-28 | Initial version, a disposition and evidence record: CPU die temperature on Snapdragon and other Windows on Arm hosts is Unsupported, and current behavior is unchanged. Records why the existing PawnIO attempt is inferred to fail safely on Arm (driver architecture and module load checks; not observed on Arm hardware), the existing thermal-zone headline fallback as unchanged behavior not attributed to the CPU die with its accepted risks, firmware evidence for Snapdragon X Elite and 8cx-class devices, the blocked SoC junction and `TZ99` candidates, target dispositions and native ARM64 build prerequisites. Corrections made in review before merge: the L1 zone-to-CPU correlation is recorded as unconfirmed (Open question 8); the sampler's 60 s idle interval is documented; and, after the maintainer decision on PR #2804, the proposed native-machine gate, the Arm headline withdrawal and their fixtures were removed. A later review pass limited x64 emulation to Windows 11 on Arm in the scope, stated the zone fallback as not attributed to the CPU die instead of ruling the die out, restated the PawnIO module load check as behavior only, added the named-zone condition to the hottest-zone risk, and limited the 8cx-class firmware facts to the dumped devices (Open question 9). |
