# PawnIO.Modules 0.2.8 vs 0.2.11 hardware validation - 2026-09-26

Local hardware evidence for moving the External Component Setup pin from
PawnIO.Modules 0.2.8 to 0.2.11 (PR #2283). It compares the two signed module
releases on one AMD Family 19h machine. It is not a register-fact source for
any sensor specification.

## Method

1. The four installed blobs in `C:\Program Files\PawnIO`
   (`IntelMSR.bin`, `RyzenSMU.bin`, `AMDFamily17.bin`, `LpcIO.bin`) were
   replaced with the files from `release_0_2_8.zip`, then from
   `release_0_2_11.zip`, with SHA-256 checked after each copy. The machine
   was restored to 0.2.8 afterwards.
2. For each version, a temporary, uncommitted example binary took eight
   samples at 1 s intervals through the public
   `hardviz_core::platform::windows::sensors` functions
   (`sample_temperatures`, `sample_power_draw`,
   `sample_motherboard_sensors`), then printed
   `cpu_temperature_diagnostics()` and `cpu_power_diagnostics()`.
3. For each version, the LpcIO diagnostic was captured with:

       powershell -ExecutionPolicy Bypass -File .\scripts\diagnostics\capture-superio-hm-dump.ps1 -IncludeBaseDiscovery -IncludeHmRead

All steps ran in one elevated PowerShell session. Another elevated
HardwareVisualizer instance was running during the capture; no mutex timeout
or access error was observed.

## Environment

| Field | Value |
| --- | --- |
| Captured at | 2026-09-26T17:36+09:00 |
| CPU | AMD Ryzen 7 7800X3D 8-Core Processor (Family 19h, Model 61h) |
| Baseboard | NZXT N7 B650E |
| Super I/O | Nuvoton, raw chip ID `0xD802` |
| OS | Microsoft Windows 11 Pro 10.0.26200 |
| PawnIO runtime | 2.2.0 (`pawnio_version` = `0x00020000`) |
| Elevated | True |
| ISA mutex | `Global\Access_ISABUS.HTP.Method`, opened-existing, acquired |

## Provider results

| Module | Result | 0.2.8 | 0.2.11 |
| --- | --- | --- | --- |
| `RyzenSMU` | Module load, source / enablement | loaded, `AmdZenSmnTctl` / Verified | loaded, `AmdZenSmnTctl` / Verified |
| `RyzenSMU` | CPU package temperature (8 samples) | 58.5–62.4 °C | 58.9–63.3 °C |
| `AMDFamily17` | Module load, source / enablement | loaded, `AmdZenRaplPackageMsr` / Verified | loaded, `AmdZenRaplPackageMsr` / Verified |
| `AMDFamily17` | CPU package power (samples 2–8; the first sample sets the baseline) | 37.9–41.5 W | 39.1–47.0 W |
| `LpcIO` | Motherboard availability | Available, `NCT6799D` | Available, `NCT6799D` |
| `LpcIO` | Channels | SYSTIN, CPUTIN, AUXTIN0–3; Fan 1–6 | same channels |

Neither version produced an unavailable sample or a fallback reason.

## LpcIO diagnostic comparison

| Field | 0.2.8 | 0.2.11 |
| --- | --- | --- |
| Module bytes | 17,388 | 18,076 |
| `pawnio_open` / `pawnio_load` | succeeded | succeeded |
| Slot 0 Nuvoton chip ID | `0xD802` | `0xD802` |
| `ioctl_find_bars` before config | `0x80070490` (not found) | `0x80070490` (not found) |
| `ioctl_find_bars` in config mode | `0x00000000` | `0x00000000` |
| Bank 4 byte reads | all succeeded | all succeeded |

Of 808 flattened JSON fields, 17 differ. They are the module size and live
bank 4 temperature and fan-count bytes (`0x91`, `0xB3`, `0xB6`, `0xB7`,
`0xB9`, `0xC3`, `0xC7`, `0xC9`). The failure of `ioctl_find_bars` before
configuration is the known ordering behavior recorded in
`2026-06-28-superio-hm-dump-admin.md`, and it is the same in both versions.

## Not covered

- Intel CPUs (`IntelMSR`)
- AMD Family 1Ah
- ITE Super I/O chips
