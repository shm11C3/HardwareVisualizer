# Spec: MSI CoreLiquid USB cooler fan and pump RPM through the vendor HID status exchange

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Draft — not implementation-ready |
| Scope | Candidate Experimental read of radiator fan, water block or pump fan, and pump RPM from MSI CoreLiquid coolers that expose a USB HID control interface, through one host status request and its reply. The candidate is keyed by the USB identity that selects the channel-role map: `0x0DB0:0xB130` (MPG CORELIQUID, K360 evidence) and `0x0DB0:0x6A05` (MEG Core Liquid S360). It is Disabled, blocked by Open questions B1–B7. No in-tree or out-of-tree Linux hwmon driver exists for any CoreLiquid cooler, so no already-bound endpoint is available. Liquid temperature is Unsupported on every model. Fan, pump, lighting, LCD, firmware and configuration commands are out of scope. The Target inventory gives every [#2529](https://github.com/shm11C3/HardwareVisualizer/issues/2529) target a disposition. |
| Issue phase | [#2529](https://github.com/shm11C3/HardwareVisualizer/issues/2529) (parent [#2510](https://github.com/shm11C3/HardwareVisualizer/issues/2510)) |

MSI publishes no protocol documentation for these coolers. Every
device-protocol fact below rests on copyleft leads and is tagged
lead-only under
[ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md).
MSI's product datasheets and the MSI Center user guide are the primary
sources for which models have a USB interface and what the vendor
software does with it. Independent linux-hardware.org probes supply the
USB descriptors. Unlike the
[NZXT Kraken X42–X72](cooler-nzxt-kraken-x42-x72.md) and
[Corsair HXi/RMi](psu-corsair-hxi-rmi-linux-hwmon.md) specifications,
no kernel driver sits between the application and the device: the
application itself would have to send the status request.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | MSI product datasheets, `https://storage-asset.msi.com/datasheet/liquid-cooling/uk/<model>.pdf`, retrieved 2026-11-29, Specifications table and Features section. Files (generation date printed in the margin, SHA-256): `MPG-CORELIQUID-K360` (2022-04-21, `d4b02b58d4cb24c1f91fb5b6b1b7570da1c77c2b843bcf66cd7ea8f12c6d735f`); `MPG-CORELIQUID-K240` (2022-04-21, `86e665c93cbfa3c0a0ce75ed8bb3d4e87fbed5d9479d6a74d12532aa8b007ba5`); `MPG-CORELIQUID-K360-V2` (2023-01-17, `2cb8e977f0a1cbe9269034c2af32b530a6307a2654ebdcb08f7da99b64abe068`); `MEG-CORELIQUID-S360` (2025-07-04, `02b16a450744e546beb01e70077cd86524360ba33bfeb849362e3d22afd83667`); `MEG-CORELIQUID-S280` (2025-08-06, `9587157ed7e7a37fdad09049b0e92a76e85171be0188ecdd1e333cba68d326d6`); `MPG-CORELIQUID-D360` (2025-07-03, `d4429ed2aa65c1346edc496045a7957b1c87fa66ee57317c43619ea7156ab385`); `MPG-CORELIQUID-D240` (2025-08-06, `cc31b4dc1b6afa1580ec049f24e568d7c79b5cf30a320df2e78b9e3d0ecf6c2b`); `MPG-CORELIQUID-P13` (2025-10-02, `8e5f1df96366f32bf10dfbf86bcb87ace890787563c5d87c04051508bc73ca6e`); `MEG-CORELIQUID-E15-360` (2026-07-13, `ea927846fa2fefb0860665f33ed9f5c9968aeefc6ced235f27d66562c2225055`); `MAG-CORELIQUID-A13-360` (2024-11-01, `77f133ced658e71be3e6fe9b6743fac01a4b243069c223efab9288fe2868a021`); `MAG-CORELIQUID-I360` (2024-10-19, `bf1947aaf85d086926a1cd91a68511d773f732c079776529044efedb27c7e484`); `MAG-CORELIQUID-E360` (2025-11-21, `b3d003e9578b64ee70d14550b35f800b26ef9e187d80d6d068a26cf0aeced5d1`); `MAG-CORELIQUID-M360` (2025-07-04, `b3b1fef645847fc4ce1c470a5cc842ac49dd9706adffff2fc35fab2d4602885d`); `MAG-CORELIQUID-360R-V2` (2025-07-04, `84fb850cdbc65e16e5271c2730b93ef50e837f60b6da0f609a95309cb95cb4de`); `MAG-CORELIQUID-C360` (2025-06-09, `8cd5e1461b208e275b0752dae9b6470c02f9de902583654cd7ff2d4debcd006d`); `MAG-CORELIQUID-P360` (2025-02-06, `db0c214e058ccecb2a835feafc484f33d64f1bc9114d0adb1226174e46bc9f4b`) | Primary vendor statements: which models list a "USB 2.0 pin header" interface connector (S360, S280, D360, D240 do; no other listed datasheet does), the pump connector type, LCD panel sizes, the K360 and K240 60 mm water block fan, and pump and fan speed ratings. No datasheet lists a liquid or coolant temperature sensor. |
| S2 | MSI, *MSI Center User Guide*, `https://download-2.msi.com/archive/mnu_exe/mb/MSICENTER.pdf`, revision history through Version 1.7 (2024/04), retrieved 2026-11-29, SHA-256 `a5d337bf67ac65ced611364488d550ce5716e1351505a35bec4aecc0d7978df2`; section Coreliquid (optional), pp. 94–97, and Revision History (Version 1.1, 2021/05, "Add Coreliquid feature") | Primary vendor statement: MSI Center changes the cooler's LCD display items and its fan settings (Balance, Silent, Game Mode, Customize, GI Mode), has separate radiator fan, pump fan and water block fan control panels, and shows the current fan speed in a real-time chart. |
| S3 | linux-hardware.org device pages, retrieved 2026-11-29: [`usb:0db0-b130`](https://linux-hardware.org/?id=usb:0db0-b130), [`usb:0db0-6a05`](https://linux-hardware.org/?id=usb:0db0-6a05), [`usb:0db0-9ba6`](https://linux-hardware.org/?id=usb:0db0-9ba6), [`usb:0db0-6a04`](https://linux-hardware.org/?id=usb:0db0-6a04), [`usb:0db0-75b6`](https://linux-hardware.org/?id=usb:0db0-75b6); probe [`00ee6cd73f`](https://linux-hardware.org/?probe=00ee6cd73f) (Linux Mint 21.1, kernel `5.15.0-69-generic`, MSI MPG B550 GAMING EDGE WIFI) `lsusb` and `usb-devices` logs; probe [`bb72de54b6`](https://linux-hardware.org/?probe=bb72de54b6) (Ubuntu 22.04, kernel `5.19.0-38-generic`, MSI PRO X670-P WIFI) `lsusb` and `usb-devices` logs | Independent observation, not maintainer-accepted. Device names and computer lists; USB descriptors and the bound USB driver; the statement that LKDDb knows no kernel driver for `0db0:b130`, `0db0:6a05` or `0db0:6a04` up to Linux 7.1. Details under USB identity and descriptors. |
| S4 | Linux kernel documentation, [Hardware Monitoring Kernel Drivers index](https://docs.kernel.org/next/hwmon/index.html) and [`msi_mystic_light`](https://docs.kernel.org/next/hwmon/msi_mystic_light.html), linux-next build `next-20261127`, retrieved 2026-11-29; liquidtux [`README.md` at `7256666cf748244071c9440cee49299ab46df172`](https://github.com/liquidctl/liquidtux/blob/7256666cf748244071c9440cee49299ab46df172/README.md), Device support | Primary for the public driver list: no hwmon driver document names a CoreLiquid cooler; `msi_mystic_light` covers motherboard voltage rails on four MSI boards (USB `1462:7C94`, `1462:7D75`, `1462:7D76`, `0DB0:0076`). The liquidtux README lists only NZXT devices. |
| S5 | liquidctl at commit [`292dc561361cfbc0cdee155c26662131fe160989`](https://github.com/liquidctl/liquidctl/tree/292dc561361cfbc0cdee155c26662131fe160989): [`liquidctl/driver/msi.py`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/liquidctl/driver/msi.py) (SHA-256 `f94034fa7c43edcbe3382982790a1e31b3e60c26180203ff8d08cbb36a02bc7d`) and [`docs/msi-mpg-coreliquid-guide.md`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/docs/msi-mpg-coreliquid-guide.md) (SHA-256 `ec2261d5e475ebcb664791925a93292d3f903d911fe2269634d15d0b93c7e268`), GPL-3.0-or-later | **lead-only (copyleft)** for the `0DB0:B130` identity and its tested K360 status, the two opt-in identities `0DB0:CA00` and `0DB0:CA02`, the status request and reply layout, units, the `B130` channel roles, the excluded HID usage, the drain of queued input reports before each request, the other request and configuration command codes, the host-supplied temperature that drives fan profiles, persistence of configuration until power loss, and the LCD reset at suspend. |
| S6 | liquidctl [pull request #564](https://github.com/liquidctl/liquidctl/pull/564), "Add support for MSI Coreliquid K360 device", merged 2024-01-28, review discussion | **lead-only (copyleft)**: on the K360 the two byte-wide temperature fields always read 0 and the two 16-bit fields always read 125, in liquidctl and in MSI's own software; a K240 capture looked identical; the device enumerates with two HID usages; the protocol knowledge was obtained by decompiling MSI's .NET software (see Open question B6). |
| S7 | LibreHardwareMonitor at commit [`2808d80564b6761e1b01f42fabfd966ed695fcbc`](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/tree/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Controller/MSI), `Hardware/Controller/MSI` (`MsiConstants.cs` SHA-256 `0d5877ffce77e72ff9a5d48877050e3a79f3d879498ea4b287fb7b7ed78248f7`, `MsiCoreLiquidController.cs` `da5929276ccd725885165e44207332ca751494f41f446885e8514f3d97ff4ca0`, `MsiDevice.cs` `3d6fa98e19241aa58e8b94f82cec5c4ee921b84f15dac32e37b2b3922949ee10`, `MsiGroup.cs` `6dc26d1e1cc8b8555e1de3a7e8189c13eb7d87221bbdabed4842dcacb43aba7b`) and `Hardware/Mutexes.cs`, MPL-2.0 | **lead-only (copyleft)** for the identities it matches (`0x6A04`, `0x6A05`, `0x6A10`, `0x6A11`, `0x6A15`, `0x6A16`, vendor `0x0DB0`, and `0x6A05` also under vendor `0x1462`) with their product names, the same status framing, the `6A05` channel roles, the byte 14 liquid-temperature interpretation, the 10 ms wait between request and reply, and the cross-process mutex it takes around each exchange. |
| S8 | Driver Fusion hardware database, [MSI USB Input Device](https://treexy.com/products/driver-fusion/database/human-interface-devices/micro-star-international-msi/usb-input-device/), retrieved 2026-11-29 | Third-party listing, non-normative: Windows hardware IDs `USB\VID_0DB0&PID_B130&MI_01`, `USB\VID_0DB0&PID_6A04&MI_01` and `USB\VID_0DB0&PID_6A05&MI_01`. |
| S9 | [NZXT Kraken X42–X72 specification](cooler-nzxt-kraken-x42-x72.md) revision 2, Sources S4, S11 and S12, section Windows HID transport and Open questions 1–3 | Repository specification. Reused for the Linux HID device name and `HID_ID` formats (lead-only there), for the Microsoft Windows HID enumeration, identity, buffering, `HidD_GetInputReport` and `CreateFileW` sharing facts (primary there), and for the open question whether every open handle receives every input report. |
| S10 | Design pull request [#2827](https://github.com/shm11C3/HardwareVisualizer/pull/2827), `docs/design/external-sensor-devices.md` at head `e2246ad521f39120835ce5a588a339901f31add0` (open) | Repository design proposal: the External Sensor Device model, role-tagged channels, live-only on-demand sampling at most once per second, and Windows and macOS providers out of scope. |

## Target inventory and disposition

Ranking. No public installed-base figure exists. The only per-identity
count found is the number of linux-hardware.org computers that report
each USB identity (S3): `0DB0:6A05` (MEG Core Liquid S360) 15 rows,
`0DB0:B130` (MPG CORELIQUID) 5, `0DB0:6A04` (MEG Core Liquid S280) 5,
and no entry for `0DB0:6A10`, `6A11`, `6A15`, `6A16`, `CA00`, `CA02`
or `1462:6A05`. This is a Linux-user sample, not sales data. MAG
CoreLiquid models have no USB data interface (S1), so their likely
larger volume does not change which USB identity comes first. The
table is ordered by that count, then by evidence.

| Rank | Target | USB identity (vendor:product) | Evidence | Disposition in this revision |
| --- | --- | --- | --- | --- |
| 1 | MEG CORELIQUID S360 | `0DB0:6A05` (S3, S7, S8) | USB 2.0 pin header, 2.4" panel, 4-pin PWM pump connector (S1). Status framing and channel roles from S7 only. | Candidate Experimental row in Detection. Disabled pending B1–B7. |
| 2 | MPG CORELIQUID K360 | `0DB0:B130` (S3, S5, S8) | 60 mm water block fan and 2.4" LCD (S1). liquidctl tests it without an opt-in flag (S5). | Candidate Experimental row in Detection. Disabled pending B1–B7. |
| 3 | MEG CORELIQUID S280 | `0DB0:6A04` (S3, S7, S8) | USB 2.0 pin header (S1). S7 applies its generic five-slot map, with no channel roles. | Deferred. Follow-up: needs a channel-role map (Open question 10). Not attempted. |
| 4 | MPG CORELIQUID K240 | Not observed; S6 reports a capture identical to the K360 | 60 mm water block fan and 2.4" LCD (S1). | Covered by the `0DB0:B130` row only if it presents that identity; its name is evidence only (Open question 9). |
| 5 | MPG CORELIQUID D360 and D240 | `0DB0:6A15`, `0DB0:6A16` (S7 only) | USB 2.0 pin header (S1). No independent identity observation. | Deferred. Follow-up: identity observation and channel-role map (Open question 10). |
| 6 | "CoreLiquid X360" and "X240" | `0DB0:6A11`, `0DB0:6A10` (S7 only) | Names exist only in S7; no MSI datasheet with these names was retrievable. | Deferred. Same follow-up as rank 5. |
| 7 | MEG CORELIQUID S360 under vendor `0x1462` | `1462:6A05` (S7 only) | No independent observation. | Deferred (Open question 11). |
| 8 | Two identities liquidctl gates behind an opt-in flag | `0DB0:CA00`, `0DB0:CA02` (S5) | S5 names them only as suspected CoreLiquid devices. | Unsupported: no model, descriptor or role evidence. |
| 9 | MPG CORELIQUID K360 V2, MPG CORELIQUID P13 360, MEG CORELIQUID E15 360 | Unknown | K360 V2 and P13 datasheets list no interface connector; P13 has a 2.1" panel; the E15 datasheet's table is empty and states a 6.67" AMOLED screen (S1). | Unsupported: no USB identity or protocol evidence. Follow-up after a user-submitted USB descriptor dump (#2500). |
| 10 | MAG CORELIQUID A13 360, I360, E360, M360, 360R V2, C360, P360 | None | No USB interface connector listed. The pump connects to a 3-pin (A13, M360), 4-pin (E360) or PWM 4-pin (I360) header; 360R V2 lists a 4-pin PWM/DC connector (S1). | Nothing to read over USB. Pump and fan speed reach motherboard fan headers and are covered only where the board's motherboard fan path is supported, for example [`superio-nuvoton-nct6687-linux-hwmon.md`](superio-nuvoton-nct6687-linux-hwmon.md) or [`superio-nuvoton-nct67xx.md`](superio-nuvoton-nct67xx.md). |
| — | Liquid temperature on any model | — | No datasheet lists a temperature sensor (S1). S5 states the K360 has no internal temperature measurement and S6 observed constant values; S7 reads byte 14 as liquid temperature on its identities. | Unsupported (Open question 8). |
| — | Fan and pump duty, configuration, firmware, serial number, LCD | — | Readable or writable through other commands (S5, S7). | Out of scope; never requested. |

## USB identity and descriptors (facts)

| Item | Fact | Source |
| --- | --- | --- |
| `0DB0:B130` strings | Manufacturer `MSI`, product `MPG CORELIQUID` | S3 (probe `bb72de54b6`) |
| `0DB0:B130` configuration | One interface: interface 0, class 3 (HID), subclass 0, protocol 0, bound to `usbhid`; HID 1.10; one report descriptor of 154 bytes (contents not captured); interrupt IN endpoint `0x81` and interrupt OUT endpoint `0x02`, 64-byte packets, 5 ms interval; device `bcdUSB` 1.10, `bcdDevice` 0.01 | S3 |
| `0DB0:B130` on Windows | A listed hardware ID carries `MI_01`, an interface number different from the Linux probe | S8 |
| `0DB0:6A05` strings | Manufacturer `MSI`, product `MEG Core Liquid S360` | S3 (probe `00ee6cd73f`) |
| `0DB0:6A05` configuration | Two interfaces. Interface 0: class 255, subclass 255, no driver, bulk IN `0x83` and bulk OUT `0x04`, 64-byte packets. Interface 1: class 3 (HID), subclass 0, protocol 0, bound to `usbhid`; HID 1.10; report descriptor 154 bytes (not captured); interrupt IN `0x81`, interrupt OUT `0x02`, 64-byte packets, 5 ms interval. Device `bcdUSB` 2.00, `bcdDevice` 1.00, full speed (12 Mbit/s) | S3 |
| Companion device | Every computer that reports `0DB0:6A05` also reports `0DB0:9BA6` (manufacturer `nuvoTon`, product `USB HID`, one HID interface, 36-byte report descriptor), and every computer that reports `0DB0:6A04` also reports `0DB0:75B6` (`USB HID`, Nuvoton). S7 records `0x9BA6` and `0x75B6` with these coolers but matches neither. | S3, S7 |
| Linux binding | No hwmon or other cooler-specific kernel driver exists for these identities; the HID interface is bound only by the generic USB HID driver | S3, S4 |
| HID usages | The control interface enumerates as more than one top-level HID usage. liquidctl excludes handles with usage page `0x0001` and a usage other than `0x0000` and uses the rest | S5, S6 |

## Detection

### Candidate runtime gates

| Condition | Gate kind | Source |
| --- | --- | --- |
| A HID interface whose USB vendor and product are `0x0DB0:0xB130` or `0x0DB0:0x6A05` | mechanism: selects the status framing and the channel-role map below | S3, S5, S7 |
| On Linux, the HID device's sysfs name is `0003:0DB0:<PID>.<seq>` and its `HID_ID` is `0003:00000DB0:0000<PID>` | mechanism: same identity, Linux form | S9 (lead-only formats) |
| The handle is opened on the control collection, not on a collection with usage page `0x0001` and usage other than `0x0000` | mechanism: only the control collection accepts the status request | S5, S6 |
| A reply is attributed to the status request only when its byte 1 is `0x31` | mechanism: reply attribution | S5, S7 |
| Each slot value is decoded only for the roles of its identity's map | mechanism: the product identity selects the role map | S5, S7 |

The product name, the serial number, the firmware versions, the HID
sequence suffix, the interface number (`MI_01` on Windows, interface 0
or 1 on Linux) and the companion Nuvoton device are evidence or
session details. They never gate a read. The safety gates that would
exclude a competing requester do not exist yet (Open questions B1 and
B3).

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| HID status exchange on `0DB0:6A05`: radiator fan, pump fan and pump RPM | **Candidate Experimental.** Lead-only S7: identity, request and reply framing, the echo byte, the five speed slots, the slot roles, the 10 ms wait. Lead-only S5: units (RPM), little-endian 16-bit slots, the excluded HID usage. Primary S1: USB interface connector, pump connector, separate pump and fan. Primary S2: MSI Center reads fan speeds and controls the radiator fan, pump fan and water block fan. Independent S3: descriptors and 15 computers. No maintainer-accepted dump and no captured reply exist. | Disabled pending Open questions B1–B7. |
| HID status exchange on `0DB0:B130`: radiator fans 1–3, water block fan and pump RPM | **Candidate Experimental.** Lead-only S5: identity, framing, echo byte, slots, units, slot roles, the excluded HID usage; tested by liquidctl with the K360 (S5, S6). Lead-only S7: the same framing. Primary S1: K360 and K240 have a 60 mm water block fan. Independent S3: descriptors and 5 computers. No maintainer-accepted dump. | Disabled pending Open questions B1–B7. |
| Other CoreLiquid identities in the Target inventory | Role map or identity evidence missing. | Disabled or Unsupported as stated in the Target inventory. |
| Liquid temperature, any identity | Conflicting leads, no primary evidence (Open question 8). | Unsupported. Never published. |
| Linux hwmon endpoint | None exists (S3, S4). | Not applicable. |

## Status exchange (facts, lead-only)

The application would send these bytes itself. They are recorded so
that the candidate can be audited; nothing here is enabled.

| Item | Fact | Source |
| --- | --- | --- |
| Transport | HID output report of 64 bytes written to the control collection; the reply is the next matching 64-byte input report | S5, S7 |
| Request | Byte 0 `0xD0` (the first byte of the HID write buffer, which HID APIs treat as the report ID), byte 1 `0x31`, bytes 2–63 `0x00` | S5, S7 |
| Reply check | Byte 1 equals `0x31` | S5, S7 |
| Before the request | liquidctl discards input reports already queued for its handle | S5 |
| Wait | LibreHardwareMonitor waits 10 ms after the write before reading and states that MSI's software uses that interval | S7 |
| Cross-process lock | LibreHardwareMonitor holds the named mutex `Global\Access_USB_Sensors` for each exchange, waiting at most 500 ms for it | S7 |

### Reply layout

Offsets count from byte 0 of the 64-byte reply.

| Bytes | Meaning | Encoding | Source |
| --- | --- | --- | --- |
| 1 | Echo of the request code | `0x31` | S5, S7 |
| 2–3 | Speed slot 1 | unsigned 16-bit little-endian, RPM | S5, S7 |
| 4–5 | Speed slot 2 | same | S5, S7 |
| 6–7 | Speed slot 3 | same | S5, S7 |
| 8–9 | Speed slot 4 | same | S5, S7 |
| 10–11 | Speed slot 5 | same | S5, S7 |
| 12 | Temperature-named field | unsigned byte; always 0 on the K360 | S6, S7 |
| 14 | Temperature-named field that S7 calls the liquid temperature | unsigned byte; always 0 on the K360 | S6, S7 |
| 16–17, 18–19 | Two temperature-named fields | unsigned 16-bit little-endian; always 125 on the K360; S7 treats 125 as absent | S6, S7 |
| 22–31 | Duty of slots 1–5, 2 bytes each | unsigned 16-bit little-endian, percent | S5, S7 |

Bytes 0, 13, 15, 20–21 and 32–63 are not described by any source.

### Channel-role maps

| Identity | Slot 1 | Slot 2 | Slot 3 | Slot 4 | Slot 5 | Source |
| --- | --- | --- | --- | --- | --- | --- |
| `0DB0:B130` | Radiator fan 1 | Radiator fan 2 | Radiator fan 3 | Water block fan | Pump | S5; S1 for the water block fan |
| `0DB0:6A05` | Radiator fans | not used | not used | Pump fan | Pump | S7 |

### Other commands (never sent)

- Queries with request bytes `0xD0 0x32` (fan configuration) and
  `0xD0 0x33` (fan temperature configuration); both leads accept a
  reply to `0x33` whose byte 1 is `0x32`. Firmware-version queries use
  a different first byte. A 185-byte feature report `0x52` holds
  lighting state. (S5, S7)
- Writes: `0x40` (fan modes and duty points), `0x41` (fan temperature
  points), a host CPU-status command that the device uses as the
  temperature input for its fan curves, and lighting, LCD, clock and
  image commands. (S5, S7)
- The device has no internal temperature measurement for its fan
  curves; a host must keep sending the current CPU temperature.
  Configuration persists until the system loses standby power; the LCD
  returns to its default animation at suspend. (S5)

## Read procedure and decode (candidate)

Not enabled. Recorded so that a revision resolving B1–B7 can adopt it.

1. Enumerate HID interfaces. Keep those whose vendor and product match
   a candidate identity. If none exists, the scope is Unsupported on
   this machine and nothing is opened.
2. Open the control collection for reading and writing with full
   sharing. If the open fails (for example access denied or a sharing
   violation), the cooler is Unavailable for this sample. Never retry
   with different access, never change permissions and never detach a
   driver.
3. Write the 64-byte status request. This write is required to obtain
   any reading; it is the only write on the path.
4. Read input reports until one has byte 1 equal to `0x31`, or until
   the reply timeout expires. Discard every other report. The timeout
   value is product policy and is fixed by the revision that enables
   the scope.
5. On timeout or a read error, every channel of that cooler is
   Unavailable for the sample. Never substitute zero.
6. Decode only the slots that the identity's role map names. Each
   value is an unsigned 16-bit little-endian RPM: 0 is an Inactive Fan
   Reading, a positive value an Active Fan Reading. Unnamed slots,
   duties and the temperature-named bytes are not published.
7. Close the handle, or keep it for the next sample, as fixed by the
   enabling revision.

Cadence: at most one exchange per cooler per second, only while the
readings are requested (S10). Each exchange is one request and one
reply.

Attribution: each qualifying HID interface is one cooler, never merged
with another cooler or with motherboard fans. Identity is session-local
(S10).

## Safety notes

- Every reading requires the application to send an output report to
  the cooler. No kernel driver mediates it. The read-only policy admits
  a write only where the read transaction requires it and the document
  marks it; whether an application-originated request qualifies is
  Open question B4.
- Never send any command other than `0xD0 0x31`. Never send the
  fan-configuration, temperature-configuration, CPU-status, lighting,
  LCD, firmware or feature-report commands, on any platform.
- MSI Center sets the cooler's fan modes and curves and shows live fan
  speed (S2). The device's fan curves depend on a host that keeps
  sending the current temperature (S5). A second requester on the same
  request and reply channel can make either side read the other's
  replies (Open questions B1 and B2).
- On Linux, liquidctl and tools built on it talk to the same HID
  interface through `hidraw` (S5); the same interleaving applies
  (Open question B3).
- Never open the companion Nuvoton device (`0DB0:9BA6`, `0DB0:75B6`)
  or the vendor bulk interface of `0DB0:6A05`.

## Synthetic fixture acceptance (candidate)

All fixtures are synthetic. The values of the first case are taken
from the S5 status example.

| Case | Expected result |
| --- | --- |
| `0DB0:B130`, reply byte 1 `0x31`, slots 1–5 `1546`, `1562`, `1530`, `2400`, `2777` | Radiator fans 1546, 1562, 1530 RPM, water block fan 2400 RPM, pump 2777 RPM, all Active, one cooler. Exactly one 64-byte write `D0 31 00 …`. |
| `0DB0:6A05`, slots 1–5 `1200`, `0`, `0`, `2400`, `2700` | Radiator fans 1200 RPM, pump fan 2400 RPM, pump 2700 RPM. Slots 2 and 3 not published. |
| `0DB0:6A05`, slot 5 `0` | Pump is an Inactive Fan Reading; siblings unchanged. |
| Reply bytes 12–19 `30`, `0`, `31`, `0`, `125`, `0`, `125`, `0` | No temperature published. |
| First input report has byte 1 `0x32`, second `0x31` | First discarded; second decoded. |
| No report with byte 1 `0x31` before the timeout | All channels of that cooler Unavailable; no zero published. |
| Open fails with access denied or a sharing violation | Unavailable; no other access mode tried; nothing written. |
| `0DB0:6A04`, `0DB0:CA00` or `1462:6A05` present | Not opened; no request sent. |
| Collection with usage page `0x0001` and usage `0x0006` on a candidate identity | Not used. |
| Two candidate coolers | Two independent sets of channels; never merged. |
| `0DB0:9BA6` present | Not opened. |

## Open questions

B1. Blocking for the HID candidate on Windows.
    MSI Center coexistence. MSI Center controls the fans and shows live
    fan speed (S2), and the device needs a host to keep sending the
    temperature its curves use (S5). MSI publishes no arbitration
    rule. Unknown: whether MSI Center takes `Global\Access_USB_Sensors`
    or any lock; what it does when it reads a reply to another
    program's request; whether MSI Center is the host that sends that
    temperature, and if so whether an interleaved request can delay it
    and so change fan or pump duty. Enabling needs a
    handle and traffic capture of a running MSI Center, a controlled
    test with concurrent status requests, and a maintainer decision.

B2. Blocking for the HID candidate.
    Report delivery with several open handles. S9 records that the
    Windows HID class driver supports several open files on one
    collection but does not state that each handle receives every input
    report. If replies are shared out rather than copied, one program
    can take the other's reply. The Linux `hidraw` behavior for
    several readers is not recorded either.

B3. Blocking for the HID candidate on Linux.
    No bound driver owns the request and reply exchange. liquidctl and
    programs built on it use the same `hidraw` interface (S5). The
    application has no evidence that excludes such a requester, which
    the safety policy requires for a scope that depends on exclusive
    use. `hidraw` nodes also need a permission that a standard
    installation does not grant; the application installs no udev rule.

B4. Blocking for the HID candidate: maintainer decision.
    Every reading needs an application-originated output report
    (`0xD0 0x31`). The Corsair PSU precedent accepted query writes sent
    by a kernel driver. Only lead-only sources (S5, S7) state that this
    request changes nothing on the device. The maintainer decides
    whether a lead-only query of this kind is admissible under the
    read-only policy for an Experimental scope.

B5. Blocking for the HID candidate.
    The 154-byte report descriptor was not captured (S3). Unknown: the
    usage page and usage of the control collection, whether `0xD0` is a
    declared output report ID, whether the reply carries a report ID in
    byte 0, the input and output report lengths reported by
    `HidP_GetCaps`, and whether a standard user can open the
    collection for writing on Windows. Needed: a report descriptor dump
    or `HidP_GetCaps` capture, and one captured status reply, from each
    candidate identity.

B6. Blocking for every lead-only fact in this document: maintainer
    decision.
    S6 states that the liquidctl driver's protocol knowledge came from
    decompiling MSI's .NET software. The clean-room rules prohibit the
    implementer from using decompiled monitoring tools; ADR 0023 admits
    copyleft implementations as leads but does not address a lead whose
    own source is a decompiled vendor tool. The origin of the S7 facts
    is not stated. The maintainer decides whether S5 and S7 facts may
    back an Experimental scope, or whether independent evidence (for
    example a USB capture of MSI Center) is required.

B7. Blocking for the HID candidate: product design.
    The External Sensor Device model (S10) is Linux-only and keeps
    Windows providers out of scope. A Windows provider needs a design
    for HID transport, including which process opens the device (the
    application or the Windows sensor service of
    [ADR 0029](../../adr/0029-windows-sensor-service.md)).

8. Non-blocking for the RPM candidate: temperature stays Unsupported.
   Liquid temperature. S5 states the K360 has no internal temperature
   measurement, and S6 observed constant 0 and 125 values on the K360
   in liquidctl and MSI's software, with an identical K240 capture. S7
   publishes byte 14 as liquid temperature on its identities without
   any observation of a varying value. No MSI datasheet lists a
   temperature sensor (S1). Enabling needs a capture showing byte 14
   tracking coolant temperature on a named identity.

9. Non-blocking for the RPM candidate: the identity, not the name,
   selects the map.
   Whether the K240 presents `0DB0:B130` and shares its role map is
   unknown; S3 shows the product string `MPG CORELIQUID` without a
   model. A K240 with only two radiator fans may report slot 3 as 0,
   which the decode shows as Inactive.

10. Non-blocking for the RPM candidate: blocks only the listed
    Deferred rows.
    Channel-role maps for `0DB0:6A04`, `6A10`, `6A11`, `6A15` and
    `6A16`. S7 applies a generic five-slot map with no roles to them.

11. Non-blocking for the RPM candidate.
    S7 lists `0x6A05` under vendor `0x1462` as well as `0x0DB0`. No
    observation shows the `0x1462` form.

12. Non-blocking for the RPM candidate: readings stay attributed to
    their source.
    The S360 and D-series pump has a 4-pin PWM connector (S1). If it is
    connected to a motherboard header, the same pump can appear once
    here and once as a motherboard fan.

13. Non-blocking for the RPM candidate: a failed exchange is
    Unavailable.
    Behavior after suspend and resume is unknown beyond the LCD reset
    (S5). Whether the HID interface re-enumerates is not recorded.

14. Non-blocking for the RPM candidate: product policy at enablement.
    The reply timeout, handle lifetime and any upper plausibility bound
    for RPM are not hardware facts. S1 rates the pump at 2800 ±300 RPM
    (S360, D-series, K360 V2) and radiator fans up to 2500 RPM
    (K360 V2); the 60 mm water block fan has no published rating.

15. Non-blocking for the RPM candidate: the scope is live-only.
    The USB serial number would give a stable identity but is not used
    (S10).

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-29 | Initial Draft. Records that no Linux hwmon endpoint exists for any CoreLiquid cooler, the USB identities and descriptors, the lead-only status exchange and reply layout, and the channel-role maps for `0DB0:B130` and `0DB0:6A05`. Defines a Disabled Candidate Experimental HID row for those two identities, blocked by B1–B7 (MSI Center coexistence, handle report delivery, Linux requesters and permissions, the application-originated request, the uncaptured report descriptor, lead provenance and the Windows provider design). Keeps liquid temperature Unsupported and gives every #2529 target a ranked disposition. |
