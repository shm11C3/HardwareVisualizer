# Super I/O Hardware Monitor dump evidence - 2026-06-28

This is independently collected local hardware evidence for #1635 Phase 3/4
spec authoring. It is not production decode logic and does not make the
Nuvoton spec implementation-ready by itself.

## Capture command

    powershell -ExecutionPolicy Bypass -File .\scripts\diagnostics\capture-superio-hm-dump.ps1 -IncludeBaseDiscovery -IncludeHmRead -OutputPath tmp\superio-hm-dump-admin.json

Full raw JSON from this run is committed alongside this file as `2026-06-28-superio-hm-dump-admin.json` (the working copy was produced at `tmp/superio-hm-dump-admin.json`).

## Environment

| Field | Value |
| --- | --- |
| Captured at | 2026-06-28T02:14:37.4489581+09:00 |
| Elevated | True |
| Baseboard | NZXT N7 B650E |
| CPU | AMD Ryzen 7 7800X3D 8-Core Processor |
| OS | Microsoft Windows 11 Pro 10.0.26200 build 26200 |
| PawnIO DLL | C:\Program Files\PawnIO\PawnIOLib.dll |
| PawnIO LpcIO module | C:\Program Files\PawnIO\LpcIO.bin |
| PawnIO version | 0x00020000 |
| pawnio_open | 0x00000000 / succeeded=True |
| pawnio_load | 0x00000000 / succeeded=True |
| ISA mutex | acquired=True, source=opened-existing, timeoutMs=500 |

## Slot and chip-id outcome

| Slot | Ports | Vendor attempt | Chip ID | Absent | Error | Exit |
| --- | --- | --- | --- | --- | --- | --- |
| 0 | 0x002E/0x002F | Nuvoton | 0xD802 | False |  | 0x00000000 / succeeded=True |
| 0 | 0x002E/0x002F | ITE | null | True |  | 0x00000000 / succeeded=True |
| 1 | 0x004E/0x004F | Nuvoton | null | True |  | 0x00000000 / succeeded=True |
| 1 | 0x004E/0x004F | ITE | null | True |  | 0x00000000 / succeeded=True |

## Nuvoton slot 0 base-discovery outcome

| Field | Value |
| --- | --- |
| LDN | 0x0B |
| CR30 | 0x09, activeBitSet=True |
| CR60/CR61 normal HM base | 0x02 / 0x90 -> 0x0290, valid=True |
| CR64/CR65 read-only HM base | 0x00 / 0x00 -> 0x0000, valid=False |
| ioctl_find_bars before config | 0x80070490 / succeeded=False |
| ioctl_find_bars in config mode | 0x00000000 / succeeded=True |
| ioctl_find_bars before HM read | 0x00000000 / succeeded=True |
| HM index port | 0x0295 |
| HM data port | 0x0296 |
| Bank select index write | 0x00000000 / succeeded=True |
| Bank select data write | 0x00000000 / succeeded=True |

## Raw bank 4 Hardware Monitor bytes

| Register | Value | Index write HRESULT | Data read HRESULT |
| --- | --- | --- | --- |
| 0x90 | 0x27 | 0x00000000 | 0x00000000 |
| 0x91 | 0x23 | 0x00000000 | 0x00000000 |
| 0x92 | 0x29 | 0x00000000 | 0x00000000 |
| 0x93 | 0x0F | 0x00000000 | 0x00000000 |
| 0x94 | 0x13 | 0x00000000 | 0x00000000 |
| 0x95 | 0x10 | 0x00000000 | 0x00000000 |
| 0xB0 | 0xFF | 0x00000000 | 0x00000000 |
| 0xB1 | 0x1F | 0x00000000 | 0x00000000 |
| 0xB2 | 0x0E | 0x00000000 | 0x00000000 |
| 0xB3 | 0x1E | 0x00000000 | 0x00000000 |
| 0xB4 | 0xFF | 0x00000000 | 0x00000000 |
| 0xB5 | 0x1F | 0x00000000 | 0x00000000 |
| 0xB6 | 0x39 | 0x00000000 | 0x00000000 |
| 0xB7 | 0x05 | 0x00000000 | 0x00000000 |
| 0xB8 | 0x33 | 0x00000000 | 0x00000000 |
| 0xB9 | 0x1D | 0x00000000 | 0x00000000 |
| 0xBA | 0xFF | 0x00000000 | 0x00000000 |
| 0xBB | 0x1F | 0x00000000 | 0x00000000 |
| 0xC0 | 0x00 | 0x00000000 | 0x00000000 |
| 0xC1 | 0x00 | 0x00000000 | 0x00000000 |
| 0xC2 | 0x0B | 0x00000000 | 0x00000000 |
| 0xC3 | 0x08 | 0x00000000 | 0x00000000 |
| 0xC4 | 0x00 | 0x00000000 | 0x00000000 |
| 0xC5 | 0x00 | 0x00000000 | 0x00000000 |
| 0xC6 | 0x02 | 0x00000000 | 0x00000000 |
| 0xC7 | 0xE2 | 0x00000000 | 0x00000000 |
| 0xC8 | 0x03 | 0x00000000 | 0x00000000 |
| 0xC9 | 0x2C | 0x00000000 | 0x00000000 |
| 0xCA | 0x00 | 0x00000000 | 0x00000000 |
| 0xCB | 0x00 | 0x00000000 | 0x00000000 |
| 0xCC | 0xFF | 0x00000000 | 0x00000000 |
| 0xCD | 0x1F | 0x00000000 | 0x00000000 |
| 0xCE | 0x00 | 0x00000000 | 0x00000000 |
| 0xCF | 0x00 | 0x00000000 | 0x00000000 |

## Interpretation boundary

This run resolves the local hardware-monitor byte-dump blocker that was still
open after the earlier elevated probe: ioctl_find_bars succeeded after
Nuvoton config/base discovery, HM bank selection succeeded, and all 34 requested
bank 4 temperature/fan-adjacent bytes were read successfully.

This local dump does not independently resolve the exact chip-id mapping: it
proves that the local responder is raw chip ID 0xD802 and exposes the normal HM
path, but it does not name the chip model. The scoped 0xD802 / NCT6799D mapping
used by the rev 5 spec is provided by the separate AIDA64 dump evidence in
`2026-06-28-nuvoton-0xd802-aida64-dump.md`.
