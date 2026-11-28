# Spec: Corsair HXi/RMi digital PSU telemetry through Linux hwmon

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental, read-only PSU temperatures, fan RPM, AC input voltage, total DC output power, and 12 V, 5 V and 3.3 V rail voltage, current and power for Corsair HXi and RMi PSUs (USB vendor `0x1B1C`, 15 product IDs) through an already bound Linux `corsair-psu` hwmon endpoint. Records the PSU requests that the kernel driver issues for each read. Input current, PWM and fan-mode attributes, limits, debugfs, derived input power and efficiency, Linux `hidraw` access, Windows direct HID access, AXi PSUs and Corsair Link dongle PSUs are out of scope or Unsupported. The Model inventory gives each one a disposition. No fan-mode, OCP-mode, fan-duty, limit or firmware command is in scope. |
| Issue phase | [#2531](https://github.com/shm11C3/HardwareVisualizer/issues/2531) (parent [#2510](https://github.com/shm11C3/HardwareVisualizer/issues/2510)) |

Corsair publishes no documentation of the PSU's USB protocol. Every
protocol, driver-behavior and channel-semantics fact below rests on
copyleft leads and is tagged lead-only under
[ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md).
The only enabled scopes are Experimental. The Corsair manual, the
Linux hwmon ABI documentation and the `corsair-psu` driver documentation
are the primary sources. This document follows the bound-endpoint
pattern of the
[NZXT Kraken X42–X72 specification](cooler-nzxt-kraken-x42-x72.md).
Unlike that cooler, this PSU sends nothing on its own: every reading is
a host request answered by the PSU. Every application read therefore
causes PSU traffic, issued by the kernel driver.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Corsair, [*HXi Series Power Supply Manual*](https://assets.corsair.com/image/upload/v1701372411/documents/49_002658_AA_WW_HXi_Series_QSG_Web.pdf), document 49-002658 AA (HX1000i and HX1500i), English section: PDF p. 3 (protection features), p. 4 (HX1000i power table), p. 5 (HX1500i power table), p. 8 (iCUE). File SHA-256 `421689370003d8bbcb56488ba7de427b7e7951dedf175a550bf3a85d79fa977c`. | Primary for the USB-C connection, iCUE's ability to monitor, switch between single-rail and multi-rail OCP on the fly and set custom fan profiles, the protection features (OVP, OCP, OTP, OPP between 115 % and 135 % of rated power), AC input rating 100–240 V, input current ratings and rail ratings. |
| S2 | Linux kernel documentation [`Documentation/ABI/testing/sysfs-class-hwmon` at v7.3](https://github.com/torvalds/linux/blob/v7.3/Documentation/ABI/testing/sysfs-class-hwmon), file SHA-256 `687fdee89a5370852322f35b543f2aab4b753a4505c2970254db5eaf3330579c`: `name`, `inY_input`, `fanY_input`, `tempY_input`, `currY_input`, `powerY_input`, `pwmY_enable` | Primary for the generic ABI: the chip name contains no dashes; units millivolt, RPM, millidegree Celsius, milliampere and microwatt. |
| S3 | Linux kernel documentation [`Documentation/hwmon/corsair-psu.rst` at v7.3](https://github.com/torvalds/linux/blob/v7.3/Documentation/hwmon/corsair-psu.rst), file SHA-256 `9f57bf1ea5feaeda09a9dbe2168c6daf8265f7ffc882f08c76662b4603cef41a` (identical on `master` as of 2026-11-28): Supported devices, Description, Sysfs entries, Usage Notes, Debugfs entries | Primary for the public driver ABI: supported models, the meaning of each attribute, read-only `pwm1` and `pwm1_enable`, automatic detection, hot swap and several devices at once, the fan starting at about 30 % of the wattage rating, and the debugfs entries. |
| S4 | Linux [`drivers/hwmon/corsair-psu.c` at v7.3](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/corsair-psu.c), Git blob `b545461d4ced3d0bafc11b019921145a2615047a` (file SHA-256 `22110759b0713568d864b31ad649c6df649c67466c5cc24f4289f329f58b450a`); compared with [v7.1](https://github.com/torvalds/linux/blob/v7.1/drivers/hwmon/corsair-psu.c) (SHA-256 `17b616345fd3724a0e9d0d823a777a80cd9af0e5a501c9e46211750b67db38dd`), [v6.18](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/corsair-psu.c) (SHA-256 `9876159e1b4da210e29fda8305afb9b93fe5c42ec1868d44c33cec1922e1b34e`) and [v5.11](https://github.com/torvalds/linux/blob/v5.11/drivers/hwmon/corsair-psu.c) (SHA-256 `9b43beba9be02187fdfb361862aa03237f90e1b581fa7070443a88b6186f283b`). `master` as of 2026-11-28 differs from v7.3 only in one allocation helper. | **lead-only (copyleft)** for the USB match table, hwmon name, channel labels, request framing, the command code behind each attribute, the rail selection before rail commands, the bind-time and resume-time requests, the reply check, the 250 ms timeout, the absence of caching, attribute visibility, the `hidraw` node, the decode, and the version differences listed under Quirks. |
| S5 | Linux commits [`d115b51e0e56`](https://github.com/torvalds/linux/commit/d115b51e0e567199c821fc39e13b6af7e78f247d) (driver added, 2020), [`7656cd217761`](https://github.com/torvalds/linux/commit/7656cd2177612aa7c299b083ecff30a4d3e9a587) (re-initialization at resume, 2021), [`741ed0856d5e`](https://github.com/torvalds/linux/commit/741ed0856d5ef94c2dbdbf58cb13d262d89505bb) (Series 2022/2023 reply sizes, 2023), [`4207069edbf0`](https://github.com/torvalds/linux/commit/4207069edbf01ae859946fe7f51e3fa03a3bb2f8) (locking moved to the hwmon core, 2025), [`2da6050809d4`](https://github.com/torvalds/linux/commit/2da6050809d437a805e7a3aa22398a15073b0234) (debugfs serialized against hwmon, 2026) and [`c6c4234928d2`](https://github.com/torvalds/linux/commit/c6c4234928d2eb4f61fecb61067e612d9bdbd2ff) (LINEAR11 sign fix, 2026), commit messages | **lead-only (copyleft)**: the PSU can also be configured over USB (fan mode, single or multi rail, over-current protection) and the driver deliberately supports no configuration; some PSUs power the microcontroller down in standby and need re-initialization at resume; replies use at most 24 bytes; an interleaved reader can make a rail value be reported for the wrong rail or take another reader's reply; before the sign fix a negative value decodes as a large positive one. |
| S6 | Linux [`drivers/hwmon/hwmon.c` at v7.3](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/hwmon.c), file SHA-256 `c42d168282e8dbbacb52e3b78fd1fa6f7a43a35a4600b2f619f88a4dcfb31e49`, device registration and attribute locking | **lead-only (copyleft)**: the hwmon core holds a per-device lock across each attribute read, and registers thermal zones only for a device with a device-tree node, so a USB PSU on an ACPI system gets no kernel thermal polling. |
| S7 | liquidctl at commit [`292dc561361cfbc0cdee155c26662131fe160989`](https://github.com/liquidctl/liquidctl/tree/292dc561361cfbc0cdee155c26662131fe160989): [`liquidctl/driver/corsair_hid_psu.py`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/liquidctl/driver/corsair_hid_psu.py) (SHA-256 `97249a0103aa76ab5da7dad7897edf7545babb7497fb39a89d83a35642bade58`) and [`docs/corsair-hxi-rmi-psu-guide.md`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/docs/corsair-hxi-rmi-psu-guide.md) (SHA-256 `c28b14cd7fa8caace52a8e910b83939f49e4efc67ed6a66722780d41c19709eb`), GPL-3.0-or-later | **lead-only (copyleft)**: the total power command reports output power; HXi and RMi PSUs report neither input power nor input current; input power and efficiency shown by tools are estimates from manual efficiency curves; the PSU must be initialized after power-on; liquidctl's own initialization writes the OCP mode and resets fan control; the first request byte distinguishes write (`0x02`) from read (`0x03`); reports are 64 bytes without report IDs; since 1.9.0 liquidctl reads through the bound hwmon driver by default and then leaves OCP and fan modes unchanged; direct access while the kernel driver is bound can race with it. |
| S8 | LibreHardwareMonitor at commit [`2808d80564b6`](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/tree/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Psu/Corsair), `Hardware/Psu/Corsair` (`CorsairPsuGroup.cs` SHA-256 `ec2261d5e475ebcb664791925a93292d3f903d911fe2269634d15d0b93c7e268`, `CorsairPsu.cs` `c2df39b2a3be8b8c4f81d69502d374a7288c07e61520b615a3206e1538c816dc`, `UsbApi.cs` `60e428aa604fd79833ae795b1e2ac0b8c0b4c6469cb0c407152d3a41ef375839`) | **lead-only (copyleft)**, used only for the Model inventory and the Windows notes: the product IDs it opens on Windows, the AX1600i ID present but disabled, and its shared HID stream access without a cross-process lock. |
| S9 | linux-hardware.org [device `usb:1b1c-1c0a`](https://linux-hardware.org/?id=usb:1b1c-1c0a) (name `Corsair RM650i C-Link Adapter`, class `03-00-00`) and [probe `01369642f4` sensors log](https://linux-hardware.org/?log=sensors&probe=01369642f4) (Gentoo, May 3, 2022) | Independent observation, not maintainer-accepted. Chip `corsairpsu-hid-3-1` on a HID adapter with all fourteen labels of the scoped channels and no input-current channel: `v_in` 230.00 V; rails 12.23 V, 5.05 V, 3.30 V; `psu fan` 0 RPM; `vrm temp` 47.5 °C; `case temp` 39.2 °C; `power total` 84.00 W; rail power 44.00 W, 24.00 W, 17.50 W; rail current 3.75 A, 4.88 A, 5.44 A. Kernel version not recorded in the retrieved log. |
| S10 | Keith Myers, [Einstein@Home forum, message 230595](https://einsteinathome.org/content/psu-not-displayed-sysinfo?page=1#comment-230592), December 5, 2024 | Independent observation, not maintainer-accepted. Chip `corsairpsu-hid-3-8` with the same labels and no input-current channel: `v_in` 115.00 V, `power total` 884.00 W, `power +12v` 902.00 W, `curr +12v` 71.75 A (crit max 168.75 A), `psu fan` 488 RPM. The poster names the PSU an AX-1500i; see Open questions. Kernel not recorded. |
| S11 | [`usb.ids`](http://www.linux-usb.org/usb.ids) version 2026.11.25, vendor `1b1c` | Public identifier database: `1c00` Controller for Corsair Link, `1c02` AX1500i Power Supply, `1c05`, `1c07`, `1c08`, `1c0b`, `1c0c` HXi and RMi Power Supply. |
| S12 | FanControl.CorsairLink [README at commit `4bf8cbb5bed2a354fd53ddebc2c86058c1d1c2b6`](https://github.com/EvanMulawski/FanControl.CorsairLink/blob/4bf8cbb5bed2a354fd53ddebc2c86058c1d1c2b6/README.md), compatibility, device and synchronization sections | Third-party statement; the repository declares no license. Non-normative: used only for the Model inventory and coexistence. It lists AXi product IDs, marks Corsair iCUE and the Corsair Service as not compatible, and synchronizes its own device access with the named mutex `Global\CorsairLinkReadWriteGuardMutex`. |
| S13 | [NZXT Kraken X42–X72 specification](cooler-nzxt-kraken-x42-x72.md) revision 2, Sources S4, S11 and S12 and section Windows HID transport | Repository specification. Reused for the Linux HID device name and `HID_ID` uevent formats (lead-only there) and for the Microsoft Windows HID and `CreateFileW` sharing facts (primary there). |

## Model inventory

Each row is keyed by the USB identity that selects a protocol and a
driver binding. Product names are evidence; they never select a path.

| Models | USB identity (vendor:product) | Linux `corsair-psu` binding at v7.3 | Disposition in this revision |
| --- | --- | --- | --- |
| HX550i, HX650i, HX750i, HX850i | `1B1C:1C03`, `1C04`, `1C05`, `1C06` | Bound since v5.11 (S3, S4) | This specification, Experimental. |
| HX1000i, HX1200i (first generation) | `1B1C:1C07`, `1C08` | Bound since v5.11 (S3, S4) | This specification, Experimental. |
| RM550i, RM650i, RM750i, RM850i, RM1000i | `1B1C:1C09`, `1C0A`, `1C0B`, `1C0C`, `1C0D` | Bound since v5.11 (S3, S4). RM650i observed (S9). | This specification, Experimental. |
| HX1000i Series 2022/2023 | `1B1C:1C1E` | Bound (S3, S4) | This specification, Experimental. |
| HX1500i first generation, Series 2023 and Series 2025 | `1B1C:1C1F` (shared) | Bound (S3, S4) | This specification, Experimental. |
| HX1200i Series 2023 (ATX 3.1, CP-9020281) | `1B1C:1C23` | Bound (S3, S4, S7) | This specification, Experimental. |
| HX1200i Series 2025 (CP-9020307) | `1B1C:1C27` | Bound (S3, S4, S7) | This specification, Experimental. |
| Later HXi or RMi models with an identity not listed above | Unknown | Not bound by the in-tree driver | Unsupported. A model that presents one of the listed identities and is bound is covered by its row; its name is evidence only. |
| AX1600i | `1B1C:1C11` (S8, S12) | Not bound (S4) | Unsupported. Follow-up: no evidence that it shares the HXi/RMi command set; S8 keeps it disabled. |
| AX1500i | `1B1C:1C02` (S11, S12) | Not bound (S4) | Unsupported. Follow-up. S10 reports a bound endpoint on a PSU named AX-1500i; see Open questions. |
| AX850i, AX1000i, AX1300i | `1B1C:1C0E`, `1C0F`, `1C10` (S12 only) | Not bound (S4) | Unsupported. Follow-up after independent identity evidence. |
| AX760i, AX860i, AX1200i | Reached through the Corsair Link controller `1B1C:1C00` (S11, S12) | Not bound (S4) | Unsupported. Different transport (a separate Corsair Link USB controller). Follow-up. |
| Corsair PSUs without a USB data interface | None | None | Nothing to read. Unavailable by design (#2531). |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| A HID device on the Linux HID bus whose sysfs name is `0003:1B1C:<PID>.<seq>` and whose `uevent` `HID_ID` is `0003:00001B1C:0000<PID>`, with `<PID>` one of `1C03`, `1C04`, `1C05`, `1C06`, `1C07`, `1C08`, `1C09`, `1C0A`, `1C0B`, `1C0C`, `1C0D`, `1C1E`, `1C1F`, `1C23`, `1C27` (four uppercase hex digits in the name, eight in `HID_ID`) | mechanism: selects the HXi/RMi HID command set that the bound driver's match table defines; excludes a device added to the driver through its `new_id` attribute | S3, S4, S13 (lead-only name and uevent formats), S9 |
| That HID device's `subsystem` link resolves to `/sys/bus/hid` and its `driver` link resolves to `/sys/bus/hid/drivers/corsair-psu` | mechanism: the bound driver defines the ABI and the PSU requests behind each read | S3, S4 |
| Exactly one hwmon child `<hid device>/hwmon/hwmonN` exists, its `name` is `corsairpsu`, its `device` link resolves to the same HID device, and `/sys/class/hwmon/hwmonN` resolves to that child | mechanism: hwmon chip kind and ownership | S2, S4, S9, S10 |
| Per channel, the label file exists and equals the label in Published attributes | mechanism: the channel metadata detects a changed assignment | S4, S9, S10 |
| Per value, the value is within the admitted range in Read procedure | mechanism (plausibility) | S1, S2, [ADR 0011](../../adr/0011-experimental-sensor-enablement.md) |

The model name, the vendor and product strings the PSU reports, the
kernel version, the `hwmonN` number and the HID sequence suffix are
evidence identities or session enumeration details. They never gate a
read. The vendor and product strings exist only in debugfs and are not
read.

| Scope | Status | Default enablement |
| --- | --- | --- |
| Linux, already bound `corsair-psu` endpoint on a listed identity: unselected readings `temp1`, `temp2`, `fan1`, `in0`, `power1` | Public ABI: S2, S3. Lead-only S4: USB match, hwmon name `corsairpsu`, labels, one query per read with the command codes in Per-read transactions, no caching, 250 ms timeout, reply check, attribute visibility. Lead-only S5: configuration exists but is never sent by the driver; resume re-initialization. Lead-only S6: per-device read lock, no thermal polling. Lead-only S7: `power1` is total output power, not input power. Independent observations S9 (RM650i) and S10 corroborate the chip name, HID adapter, labels and plausible values; neither is maintainer-accepted. No maintainer-accepted hardware dump exists. | Experimental |
| Linux, same endpoint: rail readings `in1`–`in3`, `curr2`–`curr4`, `power2`–`power4` | Everything in the row above, plus lead-only S4: each read sends a rail selection write before the query and the driver does not restore the selection; lead-only S5: the in-kernel lock keeps a selection and its query together; lead-only S7: the selection is the only write on this path, and the known configuration writes use other command codes. S9 and S10 corroborate labels and values. | Experimental |
| `curr1_input` (`curr in`) | Lead-only S4: the file exists only when the PSU answered the input-current query at bind. Lead-only S7: HXi and RMi PSUs do not report input current. Neither S9 nor S10 shows it. Its semantics are unverified. | Unsupported by this revision. Not read. |
| `pwm1`, `pwm1_enable`, `*_crit`, `*_lcrit`, debugfs entries | `pwm1` and `pwm1_enable` are read-only queries (S3, S4) but are not needed for the scoped readings. Limits are values cached at bind. Debugfs needs root and duplicates nothing in scope. | Not read. |
| Input power and efficiency | Not reported by the PSU; tools that show them estimate from efficiency curves (S7). | Unsupported. Never derived. |
| Linux `hidraw` access | The bound driver owns the request/reply exchange; a second host races with it (S5, S7). | Unsupported |
| Windows direct HID access | See Windows notes and Open questions 1–3. | Unsupported |
| Unbound identities and AXi, Corsair Link controller PSUs | See Model inventory. | Unsupported |

## Published attributes

| Attribute | Label (exact) | Meaning | Units | Source |
| --- | --- | --- | --- | --- |
| `temp1_input` | `vrm temp` | Temperature of the PSU VRM component | millidegree Celsius | S2, S3; S4 label |
| `temp2_input` | `case temp` | Temperature of the PSU case | millidegree Celsius | S2, S3; S4 label |
| `fan1_input` | `psu fan` | PSU fan speed | RPM | S2, S3; S4 label |
| `in0_input` | `v_in` | AC input voltage | millivolt | S2, S3; S4 label |
| `in1_input` | `v_out +12v` | +12 V rail output voltage | millivolt | S2, S3; S4 label |
| `in2_input` | `v_out +5v` | +5 V rail output voltage | millivolt | S2, S3; S4 label |
| `in3_input` | `v_out +3.3v` | +3.3 V rail output voltage | millivolt | S2, S3; S4 label |
| `curr2_input` | `curr +12v` | +12 V rail output current | milliampere | S2, S3; S4 label |
| `curr3_input` | `curr +5v` | +5 V rail output current | milliampere | S2, S3; S4 label |
| `curr4_input` | `curr +3.3v` | +3.3 V rail output current | milliampere | S2, S3; S4 label |
| `power1_input` | `power total` | Total DC output power reported by the PSU | microwatt | S2, S3; S4 label; S7 output semantics |
| `power2_input` | `power +12v` | +12 V rail output power | microwatt | S2, S3; S4 label |
| `power3_input` | `power +5v` | +5 V rail output power | microwatt | S2, S3; S4 label |
| `power4_input` | `power +3.3v` | +3.3 V rail output power | microwatt | S2, S3; S4 label |

- Every attribute has mode `0444`. The driver exposes no writable
  attribute; `pwm1` and `pwm1_enable` are documented read only.
  (S3, S4)
- The driver hides some files depending on what the PSU answered at
  bind: the `temp2` files exist only if the over-temperature limit
  query succeeded; the input, label and limit files of `in1`–`in3`
  exist only if that rail's over-voltage limit query succeeded; the
  label and limit files of `curr2`–`curr4` exist only if that rail's
  over-current limit query succeeded; `curr1_input` exists only if the
  input-current query succeeded. The labels of `temp1`, `fan1`, `in0`
  and all `power` channels always exist. (S4)
- Labels are constants in the driver and have been unchanged since
  v5.11. Reading a label, `name` or a limit causes no PSU traffic.
  (S4)
- The values are what the PSU microcontroller reports. Which of them
  are direct measurements and which are computed inside the PSU is not
  documented (Open question 6).

## Protocol facts (lead-only, for the safety review)

The application sends none of these requests. They are the requests
the bound kernel driver sends on the application's behalf, recorded so
that the read path can be audited.

| Item | Fact | Source |
| --- | --- | --- |
| Transport | USB HID; each request is one 64-byte output report and each reply the next 64-byte input report; no report IDs | S4, S7 |
| Request layout | Byte 0 is `0x03` for a query and `0x02` for a write; byte 1 is the command code; byte 2 is the parameter (the rail index for a selection, otherwise zero); the remaining bytes are zero. S4 and S7 describe byte 0 differently but agree on both values. | S4, S7 |
| Reply layout | Bytes 0 and 1 echo the request; data starts at byte 2. A command the PSU does not support is answered with byte 1 = `0`. Replies carry at most 24 data bytes. | S4, S5 |
| Numeric encoding | 16-bit little-endian value in bytes 2–3: bits 15–11 a signed 5-bit exponent, bits 10–0 a signed 11-bit mantissa; value = mantissa × 2^exponent, in °C, V, A, W or RPM | S4, S7 |
| Rail selection | Write with command `0x00` and parameter 0 (+12 V), 1 (+5 V) or 2 (+3.3 V); the following rail commands apply to the selected rail | S4, S7 |
| Initialization | Request bytes `0xFE`, `0x03`. Needed after power-on before the PSU returns non-zero data. | S4, S5, S7 |
| Configuration commands (never sent by the driver or the application) | Writes of `0xD8` (12 V OCP mode: 1 single rail, 2 multi rail), `0xF0` (fan control mode: 0 hardware, 1 software) and `0x3B` (fixed fan duty in percent) | S5, S7 |

## Per-read transactions

| Attribute read | PSU requests the driver sends | Source |
| --- | --- | --- |
| `temp1_input` / `temp2_input` | One query, `0x8D` / `0x8E` | S4 |
| `fan1_input` | One query, `0x90` | S4 |
| `in0_input` | One query, `0x88` | S4 |
| `power1_input` | One query, `0xEE` | S4, S7 |
| `in1_input`–`in3_input` | Rail selection (rail 0, 1, 2), then query `0x8B` | S4 |
| `curr2_input`–`curr4_input` | Rail selection (rail 0, 1, 2), then query `0x8C` | S4 |
| `power2_input`–`power4_input` | Rail selection (rail 0, 1, 2), then query `0x96` | S4 |
| Any label, `name`, `*_crit`, `*_lcrit` | None | S4 |
| `curr1_input`, `pwm1`, `pwm1_enable` (not read) | One query, `0x89`, `0x3B`, `0xF0` respectively | S4 |

- The driver caches no input. Every input read is a live exchange.
  Each request waits up to 250 ms for its reply; no reply fails the
  read with `ETIMEDOUT`, and a reply whose first two bytes do not echo
  the request fails it with `EOPNOTSUPP`. (S4)
- The selection write and its query run under one lock: the driver's
  own lock through v6.18, the hwmon core's per-device lock from v6.19
  (S4, S6). Other sysfs readers cannot split the pair. From v6.19
  through v7.1 a debugfs read could interleave with a sysfs read; v7.2
  fixed that (S5). The application reads no debugfs.
- The driver leaves the last selected rail selected after a read. (S4)
- At bind the driver sends the initialization request, queries the
  vendor and product strings (`0x99`, `0x9A`), queries the limits
  (`0x4F`, and per rail a selection followed by `0x40`, `0x44`,
  `0x46`) and probes `0x89`. At system resume it sends the
  initialization request again, because some PSUs power their
  microcontroller down in standby. (S4, S5) The application never
  causes a bind, unbind or resume.
- The kernel registers no thermal zone for this device on a system
  without a device-tree node, so the kernel itself does not poll it.
  (S6)

Why the read path is safe:

1. Every request caused by an application read is a query (byte 0 =
   `0x03`), except the rail selection that the rail queries require.
   The driver sends no other write, and the known configuration writes
   (OCP mode, fan control mode, fan duty) use other command codes that
   the driver never sends. (S4, S5, S7)
2. The rail selection chooses which output the next rail query
   addresses. It does not change OCP mode, fan mode, fan duty, limits
   or the output itself. It is the PSU-side analogue of a bank select,
   and this document marks it as the only write on the read path, as
   the specs README requires. (S4, S7)
3. The requests are the same ones `sensors` or liquidctl in its
   default hwmon mode cause through the same driver (S7). The
   application adds no new kind of request.
4. The application writes no sysfs attribute, opens no `hidraw` node,
   issues no HID ioctl, and loads, binds or unbinds no driver.

## Read procedure and decode

1. Find HID devices under `/sys/bus/hid/devices` whose name and
   `HID_ID` match the identity gate. If none exists, the scope is
   Unsupported on this machine and nothing else is read.
2. For each matching HID device, apply the driver and hwmon ownership
   gates. A matching HID device without a qualified endpoint is
   Unavailable for this sample. Do not fall back to `hidraw`, do not
   bind, unbind or load a driver, and do not write `new_id`.
3. Read each scoped channel's label once per qualification. A missing
   label file means the endpoint does not offer that channel: omit it,
   do not read its input, and do not report an error. A label that
   differs from Published attributes makes that channel Unavailable.
4. Read the inputs of the qualified channels one at a time, in this
   order: `temp1`, `temp2`, `fan1`, `in0`, `power1`, then `in1`,
   `curr2`, `power2`, `in2`, `curr3`, `power3`, `in3`, `curr4`,
   `power4`. A failed read makes only that channel Unavailable for the
   sample. Never substitute zero.
5. Timeout limit (product policy). When a read fails with
   `ETIMEDOUT`, mark every remaining channel of that PSU Unavailable
   for this sample and read nothing more from it until the next
   sample. This bounds the blocking time a non-responding PSU causes to
   about one 250 ms timeout per sample instead of one per channel.
6. Parse each input as a decimal integer. Malformed text is Invalid
   for a fan and Unavailable for any other channel. Convert and admit
   (product policy under ADR 0011; the bounds reject decode garbage and
   misattributed replies, not real operating points):

   | Channel | Conversion | Admitted range (inclusive) | Basis |
   | --- | --- | --- | --- |
   | `temp1`, `temp2` | ÷ 1000 → °C | −55 to 125 °C | Existing motherboard temperature policy |
   | `fan1` | RPM | 0 to 65535 | 0 is an Inactive Fan Reading, a positive value an Active Fan Reading, anything else an Invalid Fan Reading |
   | `in0` | ÷ 1000 → V | above 0 to 300 V | AC input rating 100–240 V (S1 p. 4, p. 5) |
   | `in1` / `in2` / `in3` | ÷ 1000 → V | above 0 to 18 V / above 0 to 7.5 V / above 0 to 4.95 V | 1.5 × nominal; OVP shuts the PSU down above a set level (S1 p. 3) |
   | `curr2` / `curr3` / `curr4` | ÷ 1000 → A | 0 to 200 A / 0 to 50 A / 0 to 50 A | Largest listed rail ratings 125 A (+12 V) and 25 A (+5 V, +3.3 V) (S1 p. 5); reported limits up to 168.75 A and 40 A (S9, S10) |
   | `power1`–`power4` | ÷ 1 000 000 → W | 0 to 2500 W | Largest listed model 1500 W; OPP shuts off between 115 % and 135 % of rated power (S1 p. 3, p. 5) |

   Omit a value outside its range for that sample.
7. A fan reading of 0 RPM is normal: the PSU's automatic plan keeps
   the fan stopped below about 30 % of the wattage rating (S3), as S9
   shows at 84 W. Present it as Inactive, never as a failure. A read
   failure is Unavailable, never 0.

Cadence and cost. A full sample is at most 5 single exchanges and 9
selection-plus-query pairs, 23 exchanges in total (S4). Sample at most
once per second per PSU, and only while PSU readings are requested
(visible, or an explicit background consumer). Run the reads off any
UI thread. Reading faster adds PSU traffic without new information.

Attribution and presentation.

- Each qualified HID device is one PSU. Readings from different PSUs
  are never merged. The `hwmonN` number and the HID sequence suffix
  change across reboots and re-plugs and identify a PSU only within a
  session. When an endpoint disappears (unplug, unbind), its channels
  are Unavailable until a qualified endpoint is found again.
- The channels of one sample are read at different instants and do not
  form a coherent snapshot. S10 shows `power total` 884 W below
  `power +12v` 902 W in one report. Never sum, subtract or divide
  channels of one sample (no derived total, input power or efficiency).
- `power1` is the PSU's total DC output power. It is not wall (input)
  power, and it is not CPU, GPU or package power. Never feed it into
  CPU or platform power readings or label it as power draw from the
  outlet.
- `in0` is the AC input voltage; its value depends on the mains supply
  (S9 230 V, S10 115 V).

## Quirks

- Kernels before v7.2 decode a negative mantissa as a large positive
  value (for example −1 becomes 2047 before the exponent and scaling
  are applied); v7.2 and later sign-extend correctly and clamp the
  result. **lead-only (copyleft)** S4, S5. A sub-zero temperature
  decoded this way lands far above 125 °C and is omitted by step 6.
  Negative currents and powers have not been observed (Open question
  5).
- At v5.11 and v5.12 the driver truncates each decoded value to whole
  units (V, A, °C, W) before scaling, so values arrive as exact
  multiples of 1000 or 1 000 000. The values are still valid and are
  published unchanged. **lead-only (copyleft)** S4.
- Kernels before the resume fix (commit `7656cd217761`, 2021) do not
  re-initialize the PSU after standby; its microcontroller can then
  stay unresponsive, and reads time out (S5). An uninitialized PSU
  answers with a reply that does not echo the request (S7), which the
  driver rejects with `EOPNOTSUPP` (S4), so it yields read failures,
  not zero values. Step 5 bounds the cost. **lead-only (copyleft)**
  S4, S5, S7.

## Coexistence

- Linux, liquidctl: since 1.9.0 liquidctl reads a bound PSU through
  this hwmon endpoint and skips its OCP and fan-mode changes. With
  `--direct-access` it sends its own requests through `hidraw`, and
  its initialization writes the OCP mode and resets fan control. Those
  are that tool's writes. Direct access races with the kernel driver:
  either side can take the other's reply or read a rail the other
  selected. (S5, S7) The application's samples can then show
  misattributed values, and can corrupt the other tool's readings;
  they cannot change PSU configuration. Step 6 omits gross
  misattributions.
- Linux, CoolerControl and other hwmon consumers: sysfs readers share
  the in-kernel lock (S4, S6) and do not interfere with each other
  beyond adding traffic.
- Corsair iCUE does not run on Linux. A Windows guest with the PSU
  passed through owns the USB device, and the host then has no bound
  endpoint, which this scope reports as Unavailable.

## Windows notes (scope Unsupported)

No Windows implementation is admitted by this revision. Recorded for
the follow-up:

- The PSU is a HID device with a vendor protocol and 64-byte reports
  without report IDs (S4, S7). Windows HID enumeration, identity
  queries, shared opening and `ERROR_SHARING_VIOLATION` behave as
  recorded in S13. LibreHardwareMonitor opens these PSUs with shared
  access on Windows and takes no cross-process lock (S8).
- With no kernel driver in between, the application itself would have
  to send every request, including the initialization request whose
  full effect is unknown (Open question 4) and the rail selection.
- Corsair's iCUE monitors these PSUs and switches OCP mode and fan
  profiles over the same USB connection (S1 p. 8). Corsair documents
  no arbitration with other programs. FanControl.CorsairLink declares
  iCUE and the Corsair Service incompatible and synchronizes only with
  programs that take `Global\CorsairLinkReadWriteGuardMutex` (S12).
  Two hosts exchanging requests with one PSU can take each other's
  replies and interleave rail selections (S5, S7), so the application
  could corrupt what iCUE shows and logs, and the reverse.

## Safety notes

- The Linux path reads sysfs attributes only. It writes no sysfs
  attribute, opens no `hidraw` node, issues no HID ioctl, reads no
  debugfs, loads, binds or unbinds no driver and needs no elevation.
- The only PSU write the read path causes is the driver's rail
  selection, marked above as required by the rail queries. Nothing on
  any platform may send the initialization request, an OCP-mode,
  fan-mode or fan-duty write, or any command not listed in Per-read
  transactions.
- Do not read `curr1_input`, `pwm1` or `pwm1_enable` in this revision.
  They are queries, but they are outside the scoped readings and add
  traffic.
- Windows: do not open the device.

## Synthetic fixture acceptance

All fixtures are synthetic. Values of the first case are taken from
the S9 observation.

| Case | Expected result |
| --- | --- |
| Qualified endpoint on `0003:1B1C:1C0A.0003`, all fourteen labels, inputs `47500`, `39200`, `0`, `230000`, `84000000`, `12230`, `3750`, `44000000`, `5050`, `4880`, `24000000`, `3300`, `5440`, `17500000` | VRM 47.5 °C, case 39.2 °C, fan 0 RPM Inactive, input 230.0 V, total output 84.0 W, +12 V 12.23 V / 3.75 A / 44.0 W, +5 V 5.05 V / 4.88 A / 24.0 W, +3.3 V 3.30 V / 5.44 A / 17.5 W, attributed to one PSU. Only labels and these inputs are read; nothing is written. |
| Same endpoint with `fan1_input` `488` | Fan 488 RPM Active. |
| `fan1_input` fails with `EIO` | Fan Unavailable, not 0; other channels preserved. |
| `in2_input` fails with `ETIMEDOUT` | `in2` and every later channel in the read order Unavailable for this sample; no further reads of that PSU until the next sample; earlier channels preserved. |
| `curr3_input` fails with `EOPNOTSUPP` | `curr3` Unavailable; siblings preserved. |
| `temp2_*` files absent | `temp2` omitted without error; other channels unchanged. |
| `curr4_label` absent, `curr4_input` present | `curr4` omitted; `curr4_input` not read. |
| `in1_label` `v_out +5v` | `in1` Unavailable; `in1_input` not read. |
| `temp1_input` `2047000` (pre-v7.2 negative decode) | `temp1` omitted. |
| `power1_input` `2600000000` | `power1` omitted. |
| `in0_input` `0` | `in0` omitted. |
| `curr2_input` `-125` | `curr2` omitted. |
| `fan1_input` `65536` or malformed | Invalid Fan Reading. |
| Values from a v5.11 kernel: `in1_input` `12000`, `power1_input` `84000000` | Published as 12.0 V and 84.0 W. |
| hwmon `corsairpsu` whose HID parent is `0003:1B1C:1C11.0005` | Not this scope; no input reads. |
| HID device `0003:1B1C:1C06.0004` bound to `hid-generic`, unbound, or without a hwmon child | Unavailable; no `hidraw` access and no driver action. |
| hwmon `device` link not resolving to the matching HID device, class link not resolving to the child, or two hwmon children | Unavailable; no input reads. |
| Two qualified HID devices | Two independent PSUs; never merged. |
| Endpoint removed, then present again with a new `hwmonN` and HID sequence | Unavailable while absent; readings resume after re-qualification. |
| `curr1_input` present | Not read; no channel published. |
| No listed `1B1C` HID device | Unsupported; no hwmon reads for this scope. |

## Open questions

1. Non-blocking for #2531 Linux hwmon scope: only the Windows scope
   depends on it, and that scope stays Unsupported.
   iCUE's open mode and arbitration are unknown. Corsair's help article
   "HWiNFO64 Conflict With Coolers and iCUE"
   (`help.corsair.com/hc/en-us/articles/360027502932`) could not be
   retrieved (HTTP 403 on 2026-11-28); only forum summaries of it
   exist. Enabling Windows needs evidence of how iCUE opens the device,
   whether iCUE honors any named mutex, a controlled test of the
   interleaving outcome and a maintainer decision.
2. Non-blocking for #2531 Linux hwmon scope: Windows only.
   The Windows top-level collection layout (usage page, usage, report
   lengths) and whether a standard user can open it are unrecorded.
   A report descriptor dump or `HidP_GetCaps` capture is needed.
3. Non-blocking for #2531 Linux hwmon scope: Windows only.
   Whether each open handle receives every input report or readers
   share one queue is not stated by Microsoft (S13, open question 3
   there). Either outcome breaks a request/reply exchange shared with
   iCUE, but the failure differs.
4. Non-blocking for #2531 Linux hwmon scope: the kernel sends the
   initialization request at bind and resume regardless of the
   application, which never triggers either.
   The full effect of the initialization request is undocumented.
   liquidctl sends OCP-mode and fan-mode writes separately after it
   (S7), which suggests it changes neither, but no evidence confirms
   that. A Windows path would have to send it itself.
5. Non-blocking for #2531 Linux hwmon scope: the admitted ranges reject
   negative values and the temperature form of the decode error.
   Whether the PSU ever reports a negative current or power (for
   example an offset near zero load on a lightly loaded rail) is
   unknown. On kernels before v7.2 such a value would decode as a large
   positive number that can fall inside the +12 V current range.
6. Non-blocking for #2531 Linux hwmon scope: every value is presented
   as PSU-reported, with its unit and channel role, and no value is
   derived from another.
   Which values the PSU measures directly and which it computes is
   undocumented. In S9 `power total` (84 W) is close to the rail sum
   (85.5 W); in S10 it is below the +12 V rail alone, consistent with
   sequential sampling under a varying load. Whether `0xEE` is a sum of
   rail measurements or a separate measurement is unknown.
7. Non-blocking for #2531 Linux hwmon scope: the row is Unsupported and
   the identity is not in the gate.
   S10 reports a bound `corsairpsu` endpoint on a PSU the poster names
   AX-1500i, whose `usb.ids` identity `1C02` (S11) is not in the
   driver's match table at any examined version (S4). Either the PSU
   is another model, or the device was added through `new_id`, or the
   identity differs. An `lsusb` line from such a unit would resolve
   it.
8. Non-blocking for #2531 Linux hwmon scope: the scope is live-only
   and stores no history, so session-local identity suffices.
   No stable per-PSU identity across reboots is established. Whether
   these PSUs report a USB serial number is not recorded.
9. Non-blocking for #2531 Linux hwmon scope: presentation is an App and
   frontend modelling decision, not a hardware fact; channel roles,
   units and attribution above are complete.
   Core has no PSU category. `PowerDraw` holds CPU, GPU, ANE and
   package power, and the motherboard temperature and fan types belong
   to the motherboard. Core has no voltage or current quantity. A PSU
   reading needs a per-PSU device entity with session-local identity,
   two temperature roles (VRM, case), one fan with the existing fan
   status classification, AC input voltage, per-rail voltage, current
   and power, and total output power, kept apart from CPU and platform
   power. The identity contract is decided in the implementation
   design review.
10. Non-blocking for #2531 Linux hwmon scope: the selection is
    restored by the next rail read, and the only other hosts on Linux
    are direct-access tools covered under Coexistence.
    Whether the PSU keeps the rail selection across standby or resets
    it at power-on is not recorded.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-28 | Initial version. Propose Implementation-ready for two Experimental Linux `corsair-psu` hwmon scopes on 15 listed Corsair HXi/RMi identities: unselected readings (two temperatures, fan, AC input voltage, total output power) and rail readings (voltage, current and power of +12 V, +5 V, +3.3 V). Record the per-read PSU requests and why they are safe, the timeout limit, admitted ranges, coexistence with liquidctl and iCUE, and the Windows blockers. Every model group in the #2531 inventory has a disposition. |
