# Nuvoton 0xD802 independent AIDA64 dump evidence - 2026-06-28

This is spec-author evidence for #1635 Phase 3. It records a public,
independently collected AIDA64 text dump that ties the raw Nuvoton
Super I/O chip ID `0xD802` to `NCT6799D` and corroborates normal
hardware-monitor bank 4 temperature/fan reads. It is not implementation
source code and no AIDA64 binary, disassembly, or source was consulted.

## Source

The dump is attached to LibreHardwareMonitor issue #1720, "Temperatures
readings on Asus ROG X670E-F (Nuvoton NCT6799D and EC)":
<https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/issues/1720>

Relevant public attachments:

- `AIDA64 - superiodump.txt`:
  <https://github.com/user-attachments/files/20190283/AIDA64.-.superiodump.txt>
- `AIDA64 - isasensordump.txt`:
  <https://github.com/user-attachments/files/20190284/AIDA64.-.isasensordump.txt>

## Environment

| Field | Value |
| --- | --- |
| Dump tool | AIDA64 Extreme v7.65.7400 |
| Board | Asus ROG Strix X670E-F Gaming WiFi |
| DMI product | ROG STRIX X670E-F GAMING WIFI |
| BIOS | 2704 |
| OS | Microsoft Windows 11 Pro 10.0.26100.3775 |

## Key raw Super I/O facts

The `superiodump.txt` and `isasensordump.txt` summaries report the
Nuvoton/Winbond-compatible Super I/O responder at configuration port
`0x002E` with normal HM base `0x0290`.

| Fact | Evidence |
| --- | --- |
| Raw chip ID | `Winbond SuperIO Device ID = D802h (D802h / 0000h) (NCT6799D / ---)` |
| Global ID registers | The per-LDN config dumps show CR20/CR21 as `D8 02`. |
| Normal HM base | `Winbond SuperIO HWMonitor Port/60 = 0290h` |
| Read-only HM base on this board | `Winbond SuperIO HWMonitor Port/64 = 0A00h` |
| LDN B base registers | In logical device `0Bh`, CR60/CR61 are `02 90`; CR64/CR65 are `0A 00`. |

This differs from the local NZXT N7 B650E capture only for the
read-only HM base: the local board reported `0x0000`, while the AIDA64
dump board reported `0x0A00`. The rev 5 ready scope therefore uses the
normal banked HM path only.

## Bank 4 temperature evidence

The `isasensordump.txt` bank 4 dump records these bytes at the same
temperature registers used by the local NZXT N7 B650E dump:

| Bank | Register | Raw value |
| --- | --- | --- |
| `0x04` | `0x90` | `0x20` |
| `0x04` | `0x91` | `0x24` |
| `0x04` | `0x92` | `0x11` |
| `0x04` | `0x93` | `0x16` |
| `0x04` | `0x94` | `0x16` |
| `0x04` | `0x95` | `0x0E` |

The repeated sampling table later in the dump labels the same indexes
as `0490` through `0495` and repeatedly reports decimal values in the
same range (`32`, `36`, `17/18`, `22`, `22`, `14`), which corroborates
that these are byte temperature readings.

## Bank 4 fan/RPM evidence

The same bank 4 dump records fan count-adjacent bytes at `0xB0` through
`0xBB` and direct high/low RPM-style bytes at `0xC0` through `0xCB`.
The rev 5 ready scope enables the direct high/low RPM registers only.

| Bank | Registers | Raw high/low | Direct RPM value if interpreted as high/low |
| --- | --- | --- | --- |
| `0x04` | `0xC0`/`0xC1` | `0x03`/`0x59` | `857` RPM |
| `0x04` | `0xC2`/`0xC3` | `0x03`/`0x2D` | `813` RPM |
| `0x04` | `0xC4`/`0xC5` | `0x03`/`0x40` | `832` RPM |
| `0x04` | `0xC6`/`0xC7` | `0x03`/`0x55` | `853` RPM |
| `0x04` | `0xC8`/`0xC9` | `0x03`/`0x42` | `834` RPM |
| `0x04` | `0xCA`/`0xCB` | `0x03`/`0x8A` | `906` RPM |

The bytes at `0xCC` through `0xCF` are also present in the dump, but the
rev 5 ready scope leaves those registers disabled because the supported
scope does not need the seventh fan path.

## Conclusion

This independent dump is strong enough to resolve the `0xD802`
exact-chip blocker for a scoped implementation: `0xD802` maps to
`NCT6799D` for the normal HM banked-read scope. It does not prove the
package suffix (`-R`), an OEM variant, or a public Nuvoton datasheet for
NCT6799D. Implementations must key support to raw chip ID `0xD802` and
must not claim an exact package suffix.
