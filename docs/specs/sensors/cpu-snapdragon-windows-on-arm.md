# Spec: CPU temperature on Snapdragon and other Windows on Arm hosts

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Windows hosts whose native machine is ARM64, including Snapdragon X Elite, X Plus, X2 and 8cx-class PCs, whether HardwareVisualizer runs as an x64 process under emulation or as a native ARM64 process. This revision specifies one runtime mechanism gate, the native-machine query, and what it changes. On an ARM64 host no PawnIO path is attempted and no PawnIO guidance is offered. The headline CPU temperature is not derived from ACPI thermal zones. Thermal-zone readings keep the existing OS-API path and are listed under their firmware zone names. It records the Windows on Arm thermal interfaces, the firmware evidence for Snapdragon X and 8cx-class devices, the evidence each blocked candidate needs, and the build and distribution prerequisites. It enables no new hardware read. It defines no CPU, SoC or skin attribution for any zone. Kernel drivers, MSR access, port I/O and any other privileged hardware access on Arm are out of scope. |
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
| `IsWow64Process2` on the current process succeeds and returns `IMAGE_FILE_MACHINE_ARM64` (`0xAA64`) in `pNativeMachine` | mechanism: selects the Windows on Arm host, where no x86-64 hardware access path exists and the x86 CPU identity seen by an emulated process is not the hardware | M1, M2, M3, R1 |

- The gate is evaluated by every process that selects a CPU, power or
  Super I/O source, including the Windows sensor service, before any
  CPUID-based selection. An x64 process under emulation receives
  emulated processor details from processor queries and from
  `GetNativeSystemInfo` (M1), so neither CPUID nor
  `GetNativeSystemInfo` can make this decision. `IsWow64Process2` is
  one of the APIs that convey the Arm64 host (M1).
- The SoC model, the processor name string, the processor identifier
  (for example "ARMv8 (64-bit) Family 8 Model 2 Revision 201 Qualcomm
  Technologies Inc", reported in U1's thread), the OEM, the firmware
  OEM Table ID and the Windows build are evidence identities. They
  never gate a read.

| Scope | Status | Default enablement |
| --- | --- | --- |
| PawnIO CPU package temperature, core and CCD temperatures, CPU package power and Super I/O readings on an ARM64 host | Every module this project loads targets x86-64 hardware (R1). Kernel-mode components cannot run under emulation (M1). MSR, SMN and port I/O do not exist as an access path on this host. | Unsupported. Not attempted. |
| Headline CPU temperature derived from ACPI thermal zones on an ARM64 host (zone-name match or hottest zone) | No primary source and no dump identifies any zone as the CPU die temperature. A zone's device list names what the zone throttles, not where its sensor is (M5). On the observed Snapdragon X device, zones whose device lists name CPU cores read 0 or a constant (L1 with D1–D4), and a hottest-zone reading stayed at 86 °C on an X2 Elite Extreme (U1). | Unsupported. |
| ACPI thermal-zone readings on an ARM64 host, listed under the firmware zone name | Existing OS-API path and decode (R2), Verified as OS/firmware API readings under [ADR 0011](../../adr/0011-experimental-sensor-enablement.md). Counter units are primary (M5). This revision does not change the transport, decode, plausibility filter or names. Lead-only observation L1 reports 36 instances on one Snapdragon X Plus device; whether the current WMI transport returns them in the application process is Open question 3. | Enabled (unchanged). |
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

### PawnIO on an Arm host (R1, B4)

| Item | Fact | Source |
| --- | --- | --- |
| Module targets | `IntelMSR`: Intel x86-64 CPUs. `RyzenSMU`, `AMDFamily17`, `AMDFamily10`: AMD x86-64 CPUs; `AMDFamily17` and `AMDFamily10` return `STATUS_NOT_SUPPORTED` at load for other architectures. `LpcIO`: x86-64 systems. | R1 |
| Installer architectures | The winget manifest for PawnIO 2.2.0 lists the same installer under `x64` and `arm64`. A PawnIO installation on an Arm host therefore says nothing about a usable CPU or Super I/O path. | B4 |

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
| Sources | A sampler thread queries `MSAcpi_ThermalZoneTemperature` in `root\WMI` and `Win32_PerfFormattedData_Counters_ThermalZoneInformation` in `root\CIMV2` through WMI, preferring the source that last answered, every 2 s. | R2 `thermal_zone.rs` |
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

These observations back no enabled scope. They are recorded because
they explain the Unsupported rows and shape the open questions.

| Observation | Source |
| --- | --- |
| On a Surface-class Snapdragon X Plus X1P64100 with Windows 11 Pro build 26200 ARM64, `Win32_PerfFormattedData_Counters_ThermalZoneInformation` returned 36 zones named after ACPI paths (`\_SB.TZxx`). `MSAcpi_ThermalZoneTemperature` and `Win32_TemperatureProbe` returned no instances. The elevation state of the probe is not stated. | L1 (lead-only) |
| Under load, `TZ0`, `TZ1` and `TZ99` read 349 K. `TZ31` and `TZ32` read 255 K and `TZ33`–`TZ37` read 233 K. `TZ17`, `TZ04` and `TZ15` read 0. The author found no zone-to-component mapping and no per-core temperature. | L1 (lead-only) |
| `Temperature` is in kelvin and `HighPrecisionTemperature` in tenths of a kelvin. | L1 (lead-only) for `HighPrecisionTemperature`; M5 for `Temperature` |
| A WMI query through the Rust `wmi` crate from inside a Tauri process failed with `WBEM_E_NOT_FOUND`. The same counters were read through PDH. | L1 (lead-only) |
| On an X2 Elite Extreme X2EE-94-100, the tool's hottest-zone temperature stayed at 86 °C. Another user saw the same on an ASUS Zenbook A16 after resuming from sleep at a high temperature. | U1 |

The zone names in L1 also occur in D1–D4. There, `TZ17` lists one CPU
cluster in all four dumps, and `TZ32` lists all CPU cores in D1 and
D3. L1's device is not one of the dumped devices, so the match of
names does not prove identical firmware.

## Read procedure and decode

1. Once per process, before any CPU, power or Super I/O source is
   selected, call `IsWow64Process2` on the current-process handle with
   both output pointers (M2). The host is ARM64 when the call succeeds
   and `pNativeMachine` is `0xAA64` (M3). Cache the result for the life
   of the process; it describes the host, not a sample.
2. If the call fails, record the `GetLastError` value as a diagnostic
   and keep the existing behavior. Justification: x64 emulation exists
   only on Windows 11 on Arm (M1), which is newer than the call's
   minimum version (M2), so an x64 build on an Arm host can always make
   the call.
3. On an ARM64 host:
   1. Do not open the PawnIO driver, `PawnIOLib.dll` or any module, and
      do not execute CPUID-based source selection. CPU package
      temperature, core and CCD temperatures, CPU package thermal
      status, CPU package power and motherboard sensors are
      Unsupported, with a reason that names the Windows on Arm host.
   2. Do not offer external-component guidance for these readings:
      installing PawnIO, setting up the sensor service or Elevated
      Startup cannot produce them on this host (R1, M1).
   3. Keep the thermal-zone sampler, its sources, decode, plausibility
      filter and zone names unchanged (R2). List every reading it
      returns as a thermal-zone temperature under its zone name.
   4. Do not set the headline CPU temperature from any zone. Neither
      the zone-name match nor the hottest-zone fallback applies. The
      headline CPU temperature is Unsupported, and the reason says that
      no CPU temperature source is documented for Windows on Arm.
4. On every other host, behavior is unchanged.

No new value is decoded by this revision. The existing counter decode
is: `Temperature` K − 273.15 = °C (M5 for the unit), and
`HighPrecisionTemperature` / 10 − 273.15 = °C (lead-only L1 and R2;
the existing code already uses it).

## Quirks

- Zones whose device lists name CPU cores can read 0 or a constant on
  the observed Snapdragon X device (L1, lead-only; names matched
  against D1–D4). On such a host the existing -40 to 150 °C filter
  drops 0 K and 233 K readings but keeps 255 K, which displays as
  about -18 °C. The zone list can therefore include a constant
  non-temperature value. See Open question 4.
- A hottest-zone value can stay constant at 86 °C (U1). This is the
  reason the hottest-zone fallback is withdrawn on ARM64 hosts.

## Safety notes

- On an ARM64 host this revision removes reads: no PawnIO path is
  attempted. No kernel driver, PawnIO module, MSR, SMN, port I/O or
  other privileged hardware access is proposed for Arm.
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

## Synthetic fixture acceptance

All fixtures in this section are synthetic. A passing fixture does not
prove real-device behavior.

| Fixture | Expected result |
| --- | --- |
| `IsWow64Process2` succeeds with process `0x0000`, native `0xAA64` (x64 or Arm64 process on an Arm host) | ARM64 host. No PawnIO open. Headline CPU temperature Unsupported with the Windows on Arm reason. No guidance candidates. |
| Same host, zones `TZ99` 349 K, `TZ32` 255 K, `TZ17` 0 K from the perf-counter source | Zone list `TZ99` ≈ 75.9 °C and `TZ32` ≈ -18.1 °C (`TZ17` dropped by the existing filter). Headline CPU temperature absent. |
| Same host, a zone named `CPUZ` at 320 K | Zone listed. Headline CPU temperature still absent: the name match does not apply on ARM64 hosts. |
| Same host, no readable zones | Zone list empty. Headline CPU temperature Unsupported. No guidance candidates. |
| `IsWow64Process2` succeeds with process `0x0000`, native `0x8664` | Not ARM64. Existing behavior, including the PawnIO path and the zone fallback. |
| `IsWow64Process2` fails | Diagnostic recorded. Existing behavior. |

## Target dispositions

| Target | Evidence in this revision | Disposition |
| --- | --- | --- |
| Snapdragon X Elite (X1E) | D1–D4 firmware; no counter observation on a dumped device | The ARM64 rules apply: PawnIO Unsupported; zones listed; headline CPU temperature Unsupported. SoC junction candidate: Open question 1. `TZ99` candidate: Open question 2. |
| Snapdragon X Plus (X1P) | L1 counter observation (lead-only); further X Plus dumps exist in the same corpus at the D1 commit but were not analysed | Same as X Elite. |
| Snapdragon X2 Elite, X2 Elite Extreme, X2 Plus | U1 observation only; no firmware dump analysed | Same rules. Attribution needs a dump and a counter capture, as in Open questions 1 and 2. |
| Snapdragon 8cx Gen 3 and SQ3 | D5 firmware (8cx Gen 3); no counter observation | Same rules. `TZ99` exists with `QCOM065A`; no `SoCTj` zone. |
| Snapdragon 8cx Gen 1 and Gen 2, SQ1 and SQ2 | D6, D7 firmware; no counter observation | Same rules. `TZ99` exists with `QCOM045E`; no `SoCTj` zone. |
| Other Windows on Arm hosts (older Snapdragon, virtual machines) | None | Same rules by mechanism. Zones may be absent; the readings are then empty. |
| `MSAcpi_ThermalZoneTemperature` on Snapdragon X | L1: no instances on one X Plus device. D4 defines no `_TMP` zone; D1–D3 define seven each, read through an I2C controller. | Kept as the existing first source; it is not a CPU temperature source. |
| Windows thermal manager and `Thermal Zone Information` counters | M5 units; L1 observation | The existing zone-listing path. No zone is attributed to the CPU. |
| Qualcomm-published interfaces | None found (Open question 5) | Unsupported; revalidate when Qualcomm publishes a user-mode interface. |
| Kernel drivers, PawnIO modules, MSR, SMN, port I/O on Arm | Out of policy | Not candidates. |
| Native ARM64 Windows build and distribution | See below | Follow-up required; this revision does not depend on it. |

## Build and distribution prerequisites (informative)

These facts are for the native ARM64 build follow-up. The rules above
apply equally to the current x64 build running under emulation.

| Item | Fact | Source |
| --- | --- | --- |
| Current artifacts | Windows releases are x64 only. | R2 `publish.yml` |
| x64 build on Arm | Runs under emulation on Windows 11 on Arm only, not on Windows 10 on Arm. Its kernel-mode components cannot run. | M1 |
| Rust target | `aarch64-pc-windows-msvc` is a Tier 1 target with host tools. | B1 |
| Tauri build | Install the Visual Studio C++ ARM64 build tools, add the `aarch64-pc-windows-msvc` target, and build with `--target aarch64-pc-windows-msvc`. The NSIS installer itself stays x86 and runs under emulation; the application is native ARM64. The page states nothing about MSI for ARM64. | B2 |
| CI runner | Windows arm64 hosted runners (`windows-11-arm`) are generally available for public repositories. | B3 |
| WebView2 | The WebView2 runtime ships with Windows 11. | B2 |

## Open questions

1. Non-blocking for #2516 host-classification scope: the SoC junction row stays Unsupported; nothing in this revision reads it.
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
2. Non-blocking for #2516 host-classification scope: the `TZ99` row stays Unsupported.
   `TZ99` lists every CPU core and other SoC devices, with the same
   passive and critical trip points in all seven dumps (378.0 K and
   388.0 K read as tenths of a kelvin). Its
   sensor `_HID` (`QCOM0C5A`, `QCOM065A`, `QCOM045E`, `_UID` `0x64`)
   is not documented publicly, and M5 shows that a device list does not
   locate the sensor. In L1, `TZ99` equalled the hottest zones under
   load. Unblocking evidence: Qualcomm or OEM documentation of that
   sensor device, or a #2500 report that pairs the `TZ99` counter with
   a documented on-die reading under controlled load.
3. Non-blocking for #2516 host-classification scope: the zone list is an unchanged existing path; an empty result is reported as unavailable.
   It is unknown whether the current WMI transport returns
   `Win32_PerfFormattedData_Counters_ThermalZoneInformation` inside the
   HardwareVisualizer process on an Arm host, as an x64 process under
   emulation or as a native ARM64 process. L1 reports `WBEM_E_NOT_FOUND`
   from the `wmi` crate inside a Tauri process while PDH worked. The
   privilege the counters need is also unrecorded. Unblocking evidence:
   a #2500 report from an Arm device with the thermal-zone source
   result. A PDH transport for this counter set would be a separate
   revision.
4. Non-blocking for #2516 host-classification scope: it affects only which existing zone readings are shown, not CPU attribution.
   Some zones read a constant (255 K, 233 K) or 0 on the observed
   device (L1). The existing filter keeps 255 K. Whether such zones
   should be hidden on Arm hosts needs a time series showing they never
   change, plus a maintainer decision; a fixed floor such as 0 °C would
   be a new heuristic without a primary source.
5. Non-blocking for #2516 host-classification scope: no scope depends on a Qualcomm interface.
   No public Qualcomm user-mode temperature API for Windows on
   Snapdragon was found in searches of Qualcomm and Microsoft
   documentation on 2026-11-28. The Qualcomm sensor devices are
   reachable only as ACPI thermal-zone sensor drivers (M4, M6, D1–D7).
   Revalidation trigger: Qualcomm publishes such an interface, or
   Microsoft documents a user-mode thermal-zone API beyond the
   performance counters.
6. Non-blocking for #2516 host-classification scope: the gate needs only the documented call and constants.
   This author did not observe `IsWow64Process2` from an x64 process
   on Arm hardware. M1 lists it as conveying the Arm64 host and M2
   defines `pNativeMachine` as the host architecture. A #2500 report
   from an Arm device would confirm the value seen by the shipped x64
   build.
7. Non-blocking for #2516 host-classification scope: the native ARM64 build is a separate follow-up.
   Whether every native dependency builds for `aarch64-pc-windows-msvc`
   (the bundled DuckDB, `nvapi`, the sensor service and its WiX
   fragments), and whether the MSI bundler produces an ARM64 package,
   is not verified. The follow-up needs a `windows-11-arm` CI build
   (B3).

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-28 | Initial version. Native-machine ARM64 mechanism gate; PawnIO paths and guidance Unsupported on Arm; headline CPU temperature not derived from thermal zones on Arm; existing zone listing unchanged; firmware evidence for Snapdragon X Elite and 8cx-class devices; blocked SoC junction and `TZ99` candidates; target dispositions; build prerequisites. |
