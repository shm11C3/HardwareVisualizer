# Spec: Aqua Computer cooling-device temperatures, RPM and flow through Linux hwmon

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental, read-only temperatures, pump and fan RPM, and coolant flow for the Aqua Computer D5 NEXT, QUADRO, OCTO, high flow NEXT and farbwerk 360 (USB vendor `0x0C70`) through an already bound Linux `aquacomputer_d5next` hwmon endpoint. On these devices every scoped read is served from the driver's cache of a status report that the device sends on its own, so no host-to-device transaction occurs on this path. Excludes software (virtual) sensors, power, voltage and current, water quality, conductivity and dissipated power, every attribute whose read queries the device (`pwmN`, `tempN_offset`, `fanN_pulses`), debugfs, Linux `hidraw` access and Windows access. Gives every #2528 target and every other identity of the driver a disposition in the Model inventory. No pump, fan, RGB, calibration or configuration command is in scope. |
| Issue phase | [#2528](https://github.com/shm11C3/HardwareVisualizer/issues/2528) (parent [#2510](https://github.com/shm11C3/HardwareVisualizer/issues/2510)) |

Aqua Computer publishes no documentation of the devices' USB protocol.
Every protocol, driver-behavior and channel-assignment fact below rests
on copyleft leads and is tagged lead-only under
[ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md).
The only enabled scopes are Experimental. The Aqua Computer manuals,
the Linux hwmon ABI documentation and the `aquacomputer_d5next` driver
documentation are the primary sources. This document follows the
bound-endpoint pattern of the
[NZXT Kraken X42–X72 specification](cooler-nzxt-kraken-x42-x72.md): like
that cooler, the scoped devices push their readings, and a read
returns a cached value without USB traffic. Unlike the
[Corsair HXi/RMi PSU specification](psu-corsair-hxi-rmi-linux-hwmon.md),
no read in scope causes a request to the device.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Linux kernel documentation [`Documentation/hwmon/aquacomputer_d5next.rst` at v7.3](https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/Documentation/hwmon/aquacomputer_d5next.rst?h=v7.3), file SHA-256 `264b4bebf6276de7c6f3b0b41e0dc6c1731eb122d599d344ebbbf5fa3d5fc4a8` (identical at v6.12 through v7.3 and on `master` as of 2026-11-29); compared with v5.15 (SHA-256 `55074b651fd3c1d91788e999a542b5fe6b9fcc6dac2f2e33eca21cc146da6a94`) and v6.6 (`1d2e26399de35fdf121c9848c2ef6fd3ca66aa5729291d78999489e4ac6e4434`). Sections Supported devices, Description, Usage notes, Sysfs entries, Debugfs entries. | Primary for the public driver ABI: the supported device list, the physical and virtual temperature sensors per device, the D5 NEXT fan readings reporting zeroes when no fan is attached, the Octo and Quadro flow sensors, the High Flow Next readings, `fan[1-9]_input` carrying pump and fan speed in RPM or flow speed in dL/h, temperature in millidegrees Celsius, the writable `pwmN`, `tempN_offset` and `fanN_pulses` entries, the debugfs entries, automatic loading and hot swap. |
| S2 | Linux kernel documentation [`Documentation/ABI/testing/sysfs-class-hwmon` at v7.3](https://github.com/torvalds/linux/blob/v7.3/Documentation/ABI/testing/sysfs-class-hwmon), file SHA-256 `687fdee89a5370852322f35b543f2aab4b753a4505c2970254db5eaf3330579c` (pinned in the Corsair specification, S2 there): `name`, `fanY_input`, `fanY_label`, `tempY_input`, `tempY_label` | Primary for the generic ABI: the chip name, one value per file, millidegree Celsius temperatures and channel labels. |
| S3 | Linux [`drivers/hwmon/aquacomputer_d5next.c` at v7.3](https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/drivers/hwmon/aquacomputer_d5next.c?h=v7.3), file SHA-256 `faf226042e8d46d6e85b045882d85cac26dbfe538378045cd769f62f332ab50c` (identical at v6.19 through v7.3); compared with v5.15 (`4e2e2d9a163963821c02021d2a522820a1db5009cf2d83dd228f4a32adc41b0e`, unchanged through v5.17), v5.18 (`e7670dbbc10ebcccf2c5fb65f42a55b27e3efc58f0442f10e75ff7077a8ae570`), v5.19 (`05d1db66f4190868df655cff9f16394bec9eb402f4aae9b75053584b91194dbd`), v6.0 (`6c12df1f620a1c09b33c7ba9988acd4ec3b1ac4c54451b9a8abdad75a70d4782`), v6.1 (`a509e3e60803ff7e4abbdbc97973c94757a4465bed8838985a0145e81c616483`), v6.10 (`2d6006cf78fdfed27aaa90bd5f7593429c7fb6652aa40bf2293df22a110b7d34`), v6.13 (`ebdb0bc4e12ab69430e12fbe6c2bdfefc6c5c4b2a55a59e64fef3b0d38864fa4`, unchanged through v6.18) and `master` as of 2026-11-29 (`54e56f7958df65230ac4aef112b7b21cbac41ed25ce9bbfb49afde844bc4f2c6`). Tag files retrieved from the kernel.org plain-file view. | **lead-only (copyleft)** for the USB match table, the HID driver name, the hwmon name per device, the channel labels and their numbering, the per-version channel additions, the status report ID and its once-per-second cadence, the decode into each input, the not-available sentinel, the two-second staleness window and its error, the absence of any output or feature report at bind or on an input or label read, the feature-report requests behind `pwmN`, `tempN_offset` and `fanN_pulses`, the attribute set (no `pwmN_enable`), the feature-report status query of the legacy devices, the `hidraw` node, and the quirks below. |
| S4 | Linux commits to S3, commit messages: [`120584c728a6`](https://github.com/torvalds/linux/commit/120584c728a69d119f5b970df17052153ad6f438) (Octo flow sensor, first in v6.10), [`998b5a78a9ce`](https://github.com/torvalds/linux/commit/998b5a78a9ce1cc4378e7281e4ea310e37596170) (Octo flow array length, first in v6.13), [`ca2363f8a294`](https://github.com/torvalds/linux/commit/ca2363f8a294de31c78b05f7e97852eacdde02e6) (locking moved to the hwmon core, first in v6.19), [`cf8f81babd2e`](https://github.com/torvalds/linux/commit/cf8f81babd2e96087dedbb74985bc1781c98f73d) (reject short status reports), [`c221091f7e30`](https://github.com/torvalds/linux/commit/c221091f7e30c64280e4cbfa27dc4c05edc96d0c) (High Flow Next power), [`b7743c6dbe3a`](https://github.com/torvalds/linux/commit/b7743c6dbe3ab94045164ef298f65cfb9b0b450a) and [`0fa2474dd3d0`](https://github.com/torvalds/linux/commit/0fa2474dd3d04f31d885c2da82fa5f85ef2bd76c) (Aquaero collection check). The last four are on `master` as of 2026-11-29 and absent from v7.3; each is marked for the stable trees. | **lead-only (copyleft)**: before the short-report fix, a truncated status report is decoded at fixed offsets and can update the cached values with unrelated data; through v7.3 an unavailable High Flow Next power reading is exposed as a large positive number; from v6.10 through v6.12 the Octo flow value is stored one element past the end of the speed array; the Aquaero exposes three HID interfaces under one product ID and its interface selection changed twice after v7.3. |
| S5 | liquidctl at commit [`292dc561361cfbc0cdee155c26662131fe160989`](https://github.com/liquidctl/liquidctl/tree/292dc561361cfbc0cdee155c26662131fe160989), GPL-3.0-or-later: [`docs/aquacomputer-d5next-guide.md`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/docs/aquacomputer-d5next-guide.md) (SHA-256 `125623244a1fed6493a3ee2bb4945601b07e5be6520207e01b7649a9889e4fed`), [`docs/aquacomputer-quadro-guide.md`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/docs/aquacomputer-quadro-guide.md) (`0faf8b5f067c9be52b34cc7712b4183adcd743a74190d9d09df4a16c439cdbb0`), [`docs/aquacomputer-octo-guide.md`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/docs/aquacomputer-octo-guide.md) (`6170bf1bfa39401d1047bf94b8408ddcdadf0b0119a824011948f907db78224f`), [`docs/aquacomputer-farbwerk360-guide.md`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/docs/aquacomputer-farbwerk360-guide.md) (`5098062a1bd336ac8cd3b465acec783472a1488e11d2a157d645cd6dbec701de`) and [`liquidctl/driver/aquacomputer.py`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/liquidctl/driver/aquacomputer.py) (`22b75d123c772dbdb20cf3c80ec6f7c76f20bc0aac98ce038e47e7fde199aec3`), sections Initialization, Monitoring and Interaction with Linux hwmon drivers | **lead-only (copyleft)**: each of these devices sends its status report every second as soon as it is connected, without initialization; the virtual temperature sensors are user assigned; liquidctl reads status through the bound hwmon driver by default and reads the device directly only when forced; to set a fixed fan or pump duty it writes through hwmon only when both `pwmN` and `pwmN_enable` exist, and otherwise, or when forced, sends a feature report to the device itself. |
| S6 | Aqua Computer, [*QUADRO User and installation manual*](https://aquacomputer.de/tl_files/aquacomputer/downloads/manuals/QUADRO_english.pdf), current as of February 2024, file SHA-256 `34c8a92dd941cd101c462d81d79b80eee1f351d8362d05a0451b53fd12a974cc`: §1 p. 5, §5.4 and §5.6 p. 7, §6 p. 8, §9 to §9.3 p. 15, §14.2 p. 22, §16.7 p. 26 | Primary: four temperature sensor inputs and one flow sensor input; USB to an internal header; the flow sensor connector is an optional accessory; aquasuite is Windows software; the device lists thirteen sensors: four temperature inputs, the flow sensor input fifth, and eight software sensors that carry data from the computer over USB; flow calibration values are selected per connected sensor model and the flow can be calibrated by ±10 %; Virtual Software Sensors are computed in aquasuite and may be transmitted to the device; the aquasuite background service configures special USB settings for all connected Aqua Computer devices. |
| S7 | Aqua Computer, [*OCTO User and installation manual*](https://aquacomputer.de/tl_files/aquacomputer/downloads/manuals/OCTO_english.pdf), current as of September 2025, SHA-256 `2180049eb7dbcb788905b7250d5606277b53032303656f09c0f6627f9b0222b6`: §1 p. 4, §9 to §9.2 p. 14, §9.3 p. 15, §17.7 p. 26 | Primary: eight fan outputs with speed monitoring, four temperature sensor inputs, one flow sensor input; twenty-one listed sensors with the flow input fifth and sixteen software sensors carrying data from the computer; the same calibration and background-service statements as S6. |
| S8 | Aqua Computer, [*D5 NEXT User and installation manual*](https://aquacomputer.de/tl_files/aquacomputer/downloads/manuals/D5_NEXT_english.pdf), current as of June 2024, SHA-256 `c9272449f2d920018cd257467dcdf44fbd8876d168912909defc38aa41d7521b`: §1 p. 4, §5.3 p. 7, §6.2 p. 8, §11.6 p. 18, §13.1 p. 19, §13.2 and §13.3 p. 20, §13.4 and §13.5 p. 21, §20.7 p. 30 | Primary: an integrated fan controller for PWM fans and a USB interface; USB is not required for operation; the fan connector is either a fan output with speed signal or a flow sensor input, never both, chosen by the flow sensor type setting; an integrated coolant temperature sensor; flow either computed from pump parameters ("virtual flow sensor", not shown below 80 l/h) or measured by a sensor on the fan connector; software temperature sensors carry readings from the computer over USB. |
| S9 | Aqua Computer, [*high flow NEXT User and installation manual*](https://aquacomputer.de/tl_files/aquacomputer/downloads/manuals/high_flow_NEXT_en.pdf), current as of September 2025, SHA-256 `cd64b03c8b8de352a27970a29dd0272f52dd96b3e39889dfaba6503a2d7d7cb9`: §1 p. 4, §4.1 p. 5, connector for a temperature sensor p. 6, §8.1 and §8.2 p. 13, §8.3 to §8.8 p. 14, §16.7 p. 27 | Primary: a USB flow sensor with an integrated coolant temperature sensor and a conductivity sensor; a connector for one external temperature sensor; flow from predefined calibration curves, manually calibratable; coolant quality derived from conductivity and configured limits; a volume counter; dissipated power computed from the two temperatures and the flow; eight software sensors carrying data from the computer. |
| S10 | Aqua Computer, [*farbwerk 360 User and installation manual*](https://aquacomputer.de/tl_files/aquacomputer/downloads/manuals/farbwerk_360_english.pdf), current as of February 2024, SHA-256 `8390f46765bd1676c26fe0d2644da0e67e5fc7cbc3cbe47e890197c5d3d97702`: §1 p. 4, connector for temperature sensors p. 6, §9.1 to §9.3 p. 14, §15.7 p. 24 | Primary: an RGB controller with four temperature sensor inputs and one flow sensor input; the four inputs are the first four listed sensors; sixteen software sensors carry data from the computer. |
| S11 | Aqua Computer, [*farbwerk User and installation manual*](https://aquacomputer.de/tl_files/aquacomputer/downloads/manuals/farbwerk_english.pdf), Revision 2 / Revision 8, current as of February 2024, SHA-256 `dd1b91e82dca989b1d18e24173df73482190e4ce25dcd34a6793cf77092a049f`: §9.1 and §9.2 p. 14, §9.3 p. 15 | Primary: each sensor input can be set to "Temperature", "Resistance" or "Voltage 0-3.3 V" mode, so a reading is a temperature only in the first mode. Used only for the farbwerk disposition. |
| S12 | Aqua Computer, [*MPS User and installation manual*](https://aquacomputer.de/tl_files/aquacomputer/downloads/manuals/mps_english.pdf), current as of February 2024, SHA-256 `2be43d23b5fb219cc5a896a7c8e45b586710ba434d1dfa5a28ae2ad6773c6ae3`: title page and §1 p. 4 | Primary: the mps family comprises flow sensors (mps flow 100/200/400, high flow USB), pressure and fill-level sensors and a D5 pump motor, all with USB. Used only for the MPS disposition. |
| S13 | Linux Hardware Database observations, `sensors` logs and, where named, `usb-devices` logs: probe [`22c66fcf36`](https://linux-hardware.org/?probe=22c66fcf36) (Bazzite 42, kernel 6.14.6; [sensors](https://linux-hardware.org/?log=sensors&probe=22c66fcf36), [usb-devices](https://linux-hardware.org/?log=usb-devices&probe=22c66fcf36)), [`e5cb79f2cd`](https://linux-hardware.org/?probe=e5cb79f2cd) (CachyOS, kernel 6.13.0; [sensors](https://linux-hardware.org/?log=sensors&probe=e5cb79f2cd), [usb-devices](https://linux-hardware.org/?log=usb-devices&probe=e5cb79f2cd)), [`1fc7de9a08`](https://linux-hardware.org/?log=sensors&probe=1fc7de9a08) (Ubuntu 24.04, kernel 6.11.0), [`f39f2cc1a5`](https://linux-hardware.org/?log=sensors&probe=f39f2cc1a5) (Mint 22, kernel 6.8.12), [`09b21a2c45`](https://linux-hardware.org/?log=sensors&probe=09b21a2c45) (Fedora 40, kernel 6.8.9), [`3103ce546a`](https://linux-hardware.org/?log=sensors&probe=3103ce546a) (EndeavourOS, kernel 6.7.4), [`2b4bc22652`](https://linux-hardware.org/?log=sensors&probe=2b4bc22652) (Mint 21.1, kernel 5.15.0-58), retrieved 2026-11-29 | Independent observations, not maintainer-accepted. Listed per device under Observations. The `usb-devices` logs show each D5 NEXT, QUADRO, OCTO, high flow NEXT and farbwerk 360 with two interfaces: interface 0 vendor-specific (class `ff`, two endpoints, no driver) and interface 1 HID (class `03`, subclass and protocol `00`, one endpoint, `usbhid`). Each device also reports a serial number string (masked by the site). |
| S14 | Linux Hardware Database device pages [`usb:0c70-f001`](https://linux-hardware.org/?id=usb:0c70-f001), [`f00e`](https://linux-hardware.org/?id=usb:0c70-f00e), [`f00d`](https://linux-hardware.org/?id=usb:0c70-f00d), [`f011`](https://linux-hardware.org/?id=usb:0c70-f011), [`f012`](https://linux-hardware.org/?id=usb:0c70-f012), [`f010`](https://linux-hardware.org/?id=usb:0c70-f010), [`f00a`](https://linux-hardware.org/?id=usb:0c70-f00a), [`f00b`](https://linux-hardware.org/?id=usb:0c70-f00b), [`f0b6`](https://linux-hardware.org/?id=usb:0c70-f0b6), [`f003`](https://linux-hardware.org/?id=usb:0c70-f003), [`f014`](https://linux-hardware.org/?id=usb:0c70-f014), retrieved 2026-11-29 | Public opt-in probe database. The number of computers listed per identity is the only installed-base proxy found; it measures Linux probe submissions, not sales. Device names as listed by the site. |
| S15 | [NZXT Kraken X42–X72 specification](cooler-nzxt-kraken-x42-x72.md) revision 2: Sources S4, S11, S12, S14 and S15, sections Quirks (resume rule) and Windows HID transport | Repository specification, Implementation-ready (rev 2). Reused for the Linux HID device name and `HID_ID` uevent formats (lead-only there), the `CLOCK_BOOTTIME` and `CLOCK_MONOTONIC` facts and the resume rule (primary there for the clocks, lead-only for jiffies), and the Microsoft Windows HID and `CreateFileW` facts (primary there). |
| S16 | LibreHardwareMonitor at commit [`2808d80564b6`](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/tree/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Controller/AquaComputer), `Hardware/Controller/AquaComputer` (`AquaComputerGroup.cs` SHA-256 `62a7d30112e7042982fa505486ac44ebcddd7e2ffcb8bfe89358f39d00cf83ca`, `D5Next.cs` `c558e58adb0939a56bed702da9221b5a9ef0febc31fe7b7aeebdcaa64880cdcb`, `MPS.cs` `93424c34f90361984e5b5ba9a901164284f1bce37229870ec07768be5aef0bb3`) | **lead-only (copyleft)**, used only for the Model inventory and the Windows notes: the product IDs it opens on Windows, that it opens the HID device stream and reads input reports for the status-report devices, and that it requests a feature report for the MPS family. |

## Model inventory

Each row is keyed by the USB identity that selects the driver's status
mechanism and channel layout. Product names are evidence; they never
select a path. The computer counts are the S14 proxy, in descending
order.

| Product | USB identity (vendor:product) | Linux `aquacomputer_d5next` binding and mechanism | S14 computers | Disposition in this revision |
| --- | --- | --- | --- | --- |
| aquaero 5 / aquaero 6 | `0C70:F001` (shared) | Bound since v6.3. Status report pushed by the device. The identity carries three HID interfaces and the driver selects one by its collections; that selection changed twice after v7.3 (S3, S4). | 35 | Deferred. Follow-up: specify the interface selection on the bound endpoint across the S4 changes and the physical versus calculated virtual sensor channels. |
| OCTO | `0C70:F011` | Bound since v5.19, flow channel since v6.10. Pushed status report. (S3, S4) | 26 | This specification, Experimental: four temperatures, eight fan RPM, flow. |
| D5 NEXT | `0C70:F00E` | Bound since v5.15 (driver named `aquacomputer-d5next` through v5.17). Pushed status report. (S3) | 22 | This specification, Experimental: coolant temperature, pump RPM, fan RPM. The D5 NEXT's own flow value (S8 §13.2) is not exposed by the driver: Unsupported. |
| high flow NEXT | `0C70:F012` | Bound since v6.1. Pushed status report. (S3) | 22 | This specification, Experimental: coolant and external temperature, flow. Water quality, conductivity, dissipated power and the two 5 V readings: not in this revision (follow-up; see Quirks for the power decode). |
| QUADRO | `0C70:F00D` | Bound since v6.0. Pushed status report. (S3) | 14 | This specification, Experimental: four temperatures, four fan RPM, flow. |
| LEAKSHIELD | `0C70:F014` | Bound since v6.5. Pushed status report on one of two HID interfaces; pressure, reservoir readings and two channels whose values the host provides. (S1, S3) | 10 | Deferred. Not a #2528 target. Follow-up: interface selection and the host-provided channels. |
| aquastream ULTIMATE | `0C70:F00B` | Bound since v6.3. Pushed status report; the pump readings use a device-specific layout (S3). Pressure and flow readings (S1). | 10 | Deferred. Follow-up: a reviewed observation of the bound endpoint and its pump, pressure and flow channels. |
| farbwerk 360 | `0C70:F010` | Bound since v5.18. Pushed status report. (S3) | 9 | This specification, Experimental: four temperatures. Its flow sensor input (S10 §1) is not exposed by the driver: Unsupported. |
| aquastream XT | `0C70:F0B6` | Bound since v6.4. No pushed report: the driver requests a status feature report from the device whenever its cache is older than two seconds (S3). | 7 | Unsupported. Every read would be a live device query; follow-up needs evidence that the query has no side effect. |
| mps flow, high flow USB (MPS family) | `0C70:F003` (shared) | Bound since v6.7 with one flow channel for every device on this identity. Same feature-report query as aquastream XT. (S1, S3) | 7 | Unsupported. Same blocker as aquastream XT. The family also has pressure, fill-level and pump members (S12); whether any of them shares this identity is unknown, so a flow label could be attached to another quantity. |
| farbwerk | `0C70:F00A` | Bound since v5.19. Pushed status report; the four inputs are decoded as temperatures. (S3) | 1 | Unsupported. Each input can be configured as resistance or voltage (S11 §9.1), and the endpoint does not show which mode is active, so a temperature decode can be wrong. |
| poweradjust 3 | `0C70:F0BD` | Bound since v6.3. Same feature-report query as aquastream XT. (S3) | Not recorded | Unsupported. Not a #2528 target. |
| Any other `0C70` identity | Unknown | Not bound by the in-tree driver | n/a | Unsupported. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| A HID device on the Linux HID bus whose sysfs name is `0003:0C70:<PID>.<seq>` and whose `uevent` `HID_ID` is `0003:00000C70:0000<PID>`, with `<PID>` one of `F00E`, `F00D`, `F011`, `F012`, `F010` (four uppercase hex digits in the name, eight in `HID_ID`) | mechanism: selects a pushed-status-report device and its channel layout in the bound driver's match table; excludes a device added through the driver's `new_id` attribute | S3, S15 (lead-only name and uevent formats), S13 |
| That HID device's `subsystem` link resolves to `/sys/bus/hid` and its `driver` link resolves to `/sys/bus/hid/drivers/aquacomputer_d5next`; for `<PID>` `F00E` only, `/sys/bus/hid/drivers/aquacomputer-d5next` also qualifies | mechanism: the bound driver defines the ABI and the cached read; the dash name is the same driver's name in v5.15 through v5.17, which binds only `F00E` | S1, S3 |
| Exactly one hwmon child `<hid device>/hwmon/hwmonN` exists, its `name` is the value in Channel map for that `<PID>`, its `device` link resolves to the same HID device, and `/sys/class/hwmon/hwmonN` resolves to that child | mechanism: hwmon chip kind and ownership | S2, S3, S13 |
| Per channel, the label file exists and equals the label in Channel map | mechanism: the channel metadata detects a changed assignment | S3, S13 |
| Per value, the value passes the decode rules in Read procedure | mechanism (plausibility) | S1, S2, [ADR 0011](../../adr/0011-experimental-sensor-enablement.md) |

The product name, the firmware version, the serial number, the kernel
version, the `hwmonN` number and the HID sequence suffix are evidence
identities or session enumeration details. They never gate a read. The
firmware version and serial number exist only in debugfs and are not
read.

| Scope | Status | Default enablement |
| --- | --- | --- |
| Linux, already bound `aquacomputer_d5next` endpoint on a pushed-status-report identity, scoped temperature, RPM and flow channels of the Channel map: D5 NEXT `F00E`, QUADRO `F00D`, OCTO `F011`, high flow NEXT `F012`, farbwerk 360 `F010` | Public ABI: S1, S2. Lead-only S3: USB match, driver names, hwmon names, labels and numbering, the once-per-second pushed report, decode, `0x7FFF` not-available sentinel, two-second staleness window and its `ENODATA`, no output or feature report at bind or on an input or label read, `hidraw` retention. Lead-only S4: per-version additions and the short-report residual. Lead-only S5: no initialization is needed and reports start at connection. Lead-only S15: HID name and `HID_ID` formats, jiffies tracking `CLOCK_MONOTONIC` (the resume rule depends on it). Primary S6–S10: the physical inputs each channel represents and that software sensors carry host data. Best-evidenced members (S13, not maintainer-accepted): D5 NEXT on kernels 5.15.0, 6.7.4, 6.8.9, 6.8.12, 6.11.0, 6.13.0 and 6.14.6, including two pumps on one host; QUADRO on 6.8.12 and 6.14.6; OCTO on 6.7.4, 6.8.9, 6.11.0 and 6.13.0; high flow NEXT on 6.7.4 and 6.13.0; farbwerk 360 on 6.7.4 and 6.13.0 (labels only, every input not available). No maintainer-accepted hardware dump exists. | Experimental |
| Software (virtual) temperature channels, power, voltage and current channels, high flow NEXT water quality, conductivity and dissipated power | Software sensors carry values the host sends to the device (S6 §9.3, S7 §9.3, S8 §13.5, S9 §8.8, S10 §9.3), not device measurements. The other readings are outside #2528. | Not read. |
| `pwmN`, `tempN_offset`, `fanN_pulses`, debugfs entries | Each `pwmN`, `tempN_offset` or `fanN_pulses` read makes the driver request the device's control feature report (S3); these attributes are also writable (S1). Debugfs needs root. | Not read. |
| Linux `hidraw` access | The bound driver is the evidenced reader; raw access needs permissions this scope does not assume and would add host traffic. | Unsupported |
| Windows direct HID access | See Windows notes and Open questions 1–2. | Unsupported |
| Every other identity in the Model inventory | See the Model inventory. | Unsupported or Deferred as stated there |

## Device status report (facts, lead-only)

The application never reads the report itself. These facts describe
what the bound driver caches, so that the read path can be audited.

| Item | Fact | Source |
| --- | --- | --- |
| Delivery | The device sends an input report with report ID `0x01` once a second, from connection, without any host request or initialization. | S3, S5 |
| Field encoding | Each scoped value is a 16-bit big-endian field at a fixed offset per device. | S3 |
| Temperature | Unsigned hundredths of a degree Celsius. The value `0x7FFF` means not available (sensor not connected). | S3 |
| Pump and fan speed | Unsigned RPM. | S3 |
| Flow | Unsigned deciliters per hour. | S1, S3 |
| Interface | The report arrives on the device's single HID interface (interface 1). Interface 0 is vendor-specific and unused by the driver. | S3, S13 |

## Channel map

Attribute, label (exact) and meaning per identity. Every listed
attribute has mode `0444` (S3). Channels not listed are not read.

| Identity, hwmon `name` | Attribute | Label (exact) | Meaning | Source |
| --- | --- | --- | --- | --- |
| `F00E`, `d5next` | `temp1_input` | `Coolant temp` | Integrated coolant temperature sensor | S1, S8 §13.1; S3 label |
| | `fan1_input` | `Pump speed` | Pump speed, RPM | S1; S3 label |
| | `fan2_input` | `Fan speed` | Speed of the fan on the pump's fan connector, RPM; 0 when no fan is attached | S1, S8 §5.3; S3 label |
| `F00D`, `quadro` | `temp1_input`–`temp4_input` | `Sensor 1`–`Sensor 4` | Temperature sensor inputs 1–4 | S1, S6 §9.1; S3 labels |
| | `fan1_input`–`fan4_input` | `Fan 1 speed`–`Fan 4 speed` | Fan outputs 1–4, RPM | S1; S3 labels |
| | `fan5_input` | `Flow speed [dL/h]` | Flow sensor input, deciliters per hour | S1, S6 §9.2; S3 label |
| `F011`, `octo` | `temp1_input`–`temp4_input` | `Sensor 1`–`Sensor 4` | Temperature sensor inputs 1–4 | S1, S7 §9.1; S3 labels |
| | `fan1_input`–`fan8_input` | `Fan 1 speed`–`Fan 8 speed` | Fan outputs 1–8, RPM | S1; S3 labels |
| | `fan9_input` | `Flow speed [dL/h]` | Flow sensor input, deciliters per hour; the channel exists from v6.10 | S1, S7 §9.2; S3 label, S4 |
| `F012`, `highflownext` | `temp1_input` | `Coolant temp` | Integrated coolant temperature sensor | S1, S9 §8.3; S3 label |
| | `temp2_input` | `External sensor` | External temperature sensor input | S1, S9 §8.4; S3 label |
| | `fan1_input` | `Flow [dL/h]` | Measured flow, deciliters per hour | S1, S9 §8.2; S3 label |
| `F010`, `farbwerk360` | `temp1_input`–`temp4_input` | `Sensor 1`–`Sensor 4` | Temperature sensor inputs 1–4 | S1, S10 §9.1; S3 labels |

- The labels above are constants in the driver and unchanged since each
  channel was added. Reading a label or `name` causes no device
  traffic. (S3)
- The driver exposes flow through a `fan` attribute. Its unit is
  deciliters per hour, not RPM (S1). Generic tools that assume RPM for
  every `fan` attribute print flow with an RPM suffix (S13).
- On every scoped identity the software-sensor channels follow the
  physical temperatures (`temp2`–`temp9` on D5 NEXT, `temp5`–`temp20`
  on the others), with labels beginning `Virtual sensor`. They are not
  read. (S1, S3)

## Read path (lead-only, for the safety review)

| Attribute read | Device transaction caused | Source |
| --- | --- | --- |
| Any scoped `tempN_input` or `fanN_input` | None. The driver returns the value decoded from the most recent cached report. | S3 |
| Any label, `name` | None | S3 |
| `pwmN`, `tempN_offset`, `fanN_pulses` (not read) | A HID Get_Report request for the control feature report | S3 |
| Write to `pwmN`, `tempN_offset`, `fanN_pulses` (never done) | The control feature report sent back with the changed value, followed by a second report that the vendor software sends after every change | S3 |

- At bind the driver starts the HID device with its `hidraw` node,
  opens it so that input reports flow, and registers the hwmon device.
  It sends no output or feature report at bind, at an input or label
  read, or at unbind. (S3)
- When no status report has arrived within two seconds, which includes
  the time between binding and the first report, every input read
  fails with `ENODATA`. The window is measured in jiffies, so time the
  system spends suspended does not count toward it. (S3, S15)
- A physical temperature whose field holds the not-available value
  fails its own read with `ENODATA` while other channels return values.
  From v6.0 this applies to every scoped temperature; before v6.0 the
  D5 NEXT coolant temperature had no such check. (S3)

Why the read path is safe:

1. Every read in scope returns cached data. The application causes no
   USB transfer to or from the device. (S3)
2. The application reads none of the attributes whose read queries the
   device and writes no attribute. (S3)
3. The cached report is the one the device sends anyway, once a second,
   from connection. Reading adds no device load. (S3, S5)
4. The application opens no `hidraw` node, issues no HID ioctl, reads
   no debugfs, and loads, binds or unbinds no driver.

## Read procedure and decode

1. Find HID devices under `/sys/bus/hid/devices` whose name and
   `HID_ID` match the identity gate. If none exists, the scope is
   Unsupported on this machine and nothing else is read.
2. For each matching HID device, apply the driver and hwmon ownership
   gates. A matching HID device without a qualified endpoint is
   Unavailable for this sample. Do not fall back to `hidraw`, do not
   bind, unbind or load a driver, and do not write `new_id`.
3. Read each scoped channel's label once per qualification. A missing
   label file means the endpoint does not offer that channel (for
   example OCTO flow before v6.10): omit it, do not read its input and
   do not report an error. A label that differs from the Channel map
   makes that channel Unavailable.
4. Read the scoped inputs one at a time, between the clock reads of the
   resume rule in Quirks. A failed read, including `ENODATA`, makes
   only that channel Unavailable for the sample. Never substitute zero.
   When every scoped input of an endpoint fails in one sample, the
   endpoint is Unavailable for that sample and stays qualified.
5. Temperature: parse a signed decimal integer and divide by 1000 for
   degrees Celsius. Admit −55 through 125 °C inclusive, the existing
   motherboard plausibility range (product policy under ADR 0011); omit
   values outside it and malformed text.
6. Pump and fan speed: parse an unsigned decimal integer from 0 through
   65535. 0 is an Inactive Fan Reading, a positive value an Active Fan
   Reading, malformed text or a value outside that range an Invalid Fan
   Reading.
7. Flow: parse an unsigned decimal integer from 0 through 65535; the
   value is deciliters per hour, so divide by 10 for liters per hour.
   Malformed text or a value outside that range makes the flow channel
   Unavailable. Flow is never presented as a speed, never with an RPM
   unit, and never merged with a fan or pump channel. A value of 0 is
   a reading of no detected flow, not a failure. On QUADRO and OCTO it
   also occurs when no flow sensor is connected (Open question 3), so
   it is presented as the device's zero reading without an alarm.
8. Apply the resume rule in Quirks before publishing any value of the
   sample.

Cadence and cost: values change at most once a second, and a read is
one sysfs file read without USB traffic (S3, S5). Sample at most once a
second per device, only while the readings are requested; reading
faster returns repeated values.

Attribution and presentation:

- Each qualified HID device is one device. Readings from different
  devices are never merged; two D5 NEXT pumps on one host are two
  devices (S13). The `hwmonN` number and the HID sequence suffix change
  across reboots and re-plugs and identify a device only within a
  session. When an endpoint disappears (unplug, unbind), its channels
  are Unavailable until a qualified endpoint is found again.
- Channel roles are fixed by the Channel map: coolant temperature,
  external temperature, numbered temperature input, pump speed,
  numbered fan speed, flow. A temperature input's number is the
  device's own input number.
- Flow on QUADRO and OCTO depends on the sensor model and calibration
  selected in the device (S6 §9.2, S7 §9.2), and on high flow NEXT on
  its calibration curve (S9 §8.2). The value is presented as the
  device reports it.

## Quirks

- Resume rule. The driver's two-second window is measured in jiffies,
  which do not advance during suspend (S3; S15 lead-only for jiffies).
  After a resume the driver can therefore return the last values
  received before suspend. Apply the resume rule of S15 (Kraken
  specification revision 2, section Quirks) unchanged: the
  `CLOCK_BOOTTIME`, `CLOCK_MONOTONIC`, `CLOCK_BOOTTIME` reads after
  the input reads, the 50 ms trust bound, the 100 ms growth threshold,
  and the 3 s hold after first observation, re-qualification or a
  detected suspend. The 3 s hold still bounds staleness here because it
  exceeds the two-second window plus one tick; the one-second report
  period only decides when fresh values reappear, and until then reads
  fail with `ENODATA`. **lead-only (copyleft)** S3, S15.
- Short reports. Before commit `cf8f81babd2e` (on `master`, absent from
  v7.3), the driver decodes a status report at fixed offsets without
  checking its length, and a truncated report can update the cached
  values with unrelated data. The application cannot detect such a
  value; steps 5–7 omit only out-of-range ones. Residual accepted for
  the Experimental scope; see Open question 5. **lead-only (copyleft)**
  S4.
- Temperatures are unsigned in the report, so the driver never returns
  a negative temperature. A sub-zero field would arrive as a value
  above 125 °C and is omitted by step 5. Before v6.0 a not-available
  D5 NEXT coolant field arrives as 327.67 °C and is omitted the same
  way. **lead-only (copyleft)** S3.
- From v6.10 through v6.12 the OCTO flow value is stored one element
  past the end of the driver's speed array, in the adjacent field,
  which nothing else on OCTO writes; the read returns the stored value.
  v6.13 corrected the array length. (S4) The S13 OCTO observations on
  6.11.0 and 6.13.0 both show a flow channel. **lead-only (copyleft)**
  S3, S4.
- Through v7.3 the high flow NEXT dissipated-power channel reports
  about 4.29 kW instead of failing when no external sensor is
  connected (S4, commit `c221091f7e30`); S13 shows 4.29 kW with
  `External sensor` not available on 6.7.4 and 6.13.0. The channel is
  not read. Recorded for the follow-up. **lead-only (copyleft)** S4.

## Observations

| Device | Probe (kernel) | Labels and values (S13) |
| --- | --- | --- |
| D5 NEXT | `22c66fcf36` (6.14.6) | `Coolant temp` +28.6 °C, `Pump speed` 4806 RPM, `Fan speed` 701 RPM; virtual sensors 1–8 N/A |
| D5 NEXT | `1fc7de9a08` (6.11.0), `f39f2cc1a5` (6.8.12), `3103ce546a` (6.7.4), `e5cb79f2cd` (6.13.0) | Coolant +24.6 to +34.8 °C, pump 3425 to 3696 RPM, `Fan speed` 0 RPM |
| D5 NEXT ×2 | `09b21a2c45` (6.8.9) | Two chips on one host: pump 3644 and 3023 RPM, both coolant +24.9 °C, both fan 0 RPM |
| D5 NEXT | `2b4bc22652` (5.15.0-58) | `Coolant temp` +27.0 °C, `Pump speed` 3219 RPM, `Fan speed` 0 RPM; no virtual sensors and no +12 V reading, as in the v5.15 driver |
| QUADRO | `22c66fcf36` (6.14.6) | `Sensor 1`–`4` +28.7, +26.8, +28.5, +27.4 °C; fans 783, 768, 762, 769 RPM; `Flow speed [dL/h]` 1557 |
| QUADRO | `f39f2cc1a5` (6.8.12) | `Sensor 1` +31.9 °C, `Sensor 2`–`4` N/A; fans 1–4 0 RPM; flow 530 |
| OCTO | `e5cb79f2cd` (6.13.0) | `Sensor 1` +35.7 °C, `Sensor 3` +35.1 °C, `Sensor 2` and `4` N/A; fans 946, 833, 812, 543, 0, 0, 0, 0 RPM; `Flow speed [dL/h]` 1202 |
| OCTO | `1fc7de9a08` (6.11.0) | `Sensor 1` +21.6, `Sensor 2` +24.5 °C, `Sensor 3` and `4` N/A; six fans 1104 to 1190 RPM, two 0 RPM; flow 0 |
| OCTO | `3103ce546a` (6.7.4), `09b21a2c45` (6.8.9) | No flow channel, as before v6.10 |
| high flow NEXT | `e5cb79f2cd` (6.13.0), `3103ce546a` (6.7.4) | `Coolant temp` +34.5 and +32.6 °C, `External sensor` N/A, `Flow [dL/h]` 1279 and 1266; dissipated power 4.29 kW (Quirks) |
| farbwerk 360 | `e5cb79f2cd` (6.13.0), `3103ce546a` (6.7.4) | `Sensor 1`–`4` and all virtual sensors N/A |

## Coexistence

- Linux, liquidctl: by default it reads these devices through the bound
  hwmon endpoint. To set a fixed fan or pump duty it needs both `pwmN`
  and `pwmN_enable`; the in-tree driver offers no `pwmN_enable` (S3),
  so liquidctl then sends its own feature report to the device (S5).
  Those are that tool's writes and change device configuration, not
  this read path. The application's values then reflect the new
  configuration.
- Linux, other hwmon consumers: they read the same cache and do not
  interfere (S3).
- A root user writing `pwmN`, `tempN_offset` or `fanN_pulses` makes
  the driver rewrite the device's control report (S3). The application
  never does.
- aquasuite and its background service run on Windows only (S6 §6).
  A Windows guest with the device passed through owns the USB device,
  and the host then has no bound endpoint, which this scope reports as
  Unavailable.

## Windows notes (scope Unsupported)

No Windows implementation is admitted by this revision. Recorded for
the follow-up, which depends on the external sensor device model
proposed in PR #2827 and on Open questions 1–2:

- Each scoped device presents one HID interface and one vendor-specific
  interface (S13). Windows HID enumeration, identity queries, report
  buffering and sharing behave as recorded in S15.
- The status report arrives without any host request (S3, S5), so a
  Windows reader would only read input reports; it would send nothing.
- The aquasuite background service "configures special USB settings for
  all connected Aqua Computer devices" (S6 §16.7, S7 §17.7, S8 §20.7,
  S9 §16.7, S10 §15.7). What those settings are, and whether aquasuite
  holds the HID interface exclusively, is not documented.
- LibreHardwareMonitor opens the HID stream of the status-report
  devices on Windows and reads input reports, and requests a feature
  report from the MPS family (S16). This shows a reader exists, not
  that it coexists with aquasuite.

## Safety notes

- The Linux path reads sysfs attributes only. It writes no sysfs
  attribute, opens no `hidraw` node, issues no HID ioctl, reads no
  debugfs, loads, binds or unbinds no driver and needs no elevation.
  With the driver's own behavior (S3), no host-to-device transaction
  occurs on this path.
- Never read `pwmN`, `tempN_offset` or `fanN_pulses`: each read makes
  the driver query the device, and the attributes are writable.
- Never read any attribute of an identity outside the identity gate.
  On aquastream XT, the MPS family and poweradjust 3 every input read
  can make the driver query the device (S3).
- Never send an output report, feature report or Get_Report request to
  any Aqua Computer device, on any platform.
- Windows: do not open the device.

## Synthetic fixture acceptance

All fixtures are synthetic. Values are taken from the S13 observations
where noted. Every published case assumes the endpoint is past its
first-observation hold.

| Case | Expected result |
| --- | --- |
| `0003:0C70:F00E.0003`, hwmon `d5next`, labels as in Channel map, inputs `28600`, `4806`, `701` (S13 `22c66fcf36`) | Coolant 28.6 °C; pump 4806 RPM Active; fan 701 RPM Active; one device. Only labels and these three inputs are read; nothing is written. |
| Same with `fan2_input` `0` | Fan Inactive (0 RPM); coolant and pump unchanged. |
| `0003:0C70:F00D.0002`, hwmon `quadro`, inputs `28700`, `26800`, `28500`, `27400`, fans `783`, `768`, `762`, `769`, `fan5_input` `1557` (S13 `22c66fcf36`) | Inputs 1–4 28.7, 26.8, 28.5, 27.4 °C; fans 1–4 Active; flow 155.7 L/h, presented as flow, not RPM. |
| QUADRO with `temp2_input`–`temp4_input` failing with `ENODATA` (S13 `f39f2cc1a5`) | Inputs 2–4 Unavailable, not 0; input 1 and the other channels preserved. |
| `0003:0C70:F011.0008`, hwmon `octo`, `fan9_label` and `fan9_input` absent | Flow omitted without error; four temperatures and eight fans published. |
| OCTO with `fan9_input` `0` | Flow 0 L/h, presented as no detected flow, not as a failure or alarm. |
| `0003:0C70:F012.000E`, hwmon `highflownext`, `temp1_input` `34500`, `temp2_input` fails with `ENODATA`, `fan1_input` `1279`, `power1_input` `4294967235` (S13 `e5cb79f2cd`) | Coolant 34.5 °C; external temperature Unavailable; flow 127.9 L/h; `power1_input` and `fan2`/`fan3` are not read. |
| `0003:0C70:F010.000B`, hwmon `farbwerk360`, all four inputs fail with `ENODATA` (S13 `e5cb79f2cd`) | All four temperatures Unavailable; the endpoint is Unavailable for the sample and stays qualified. |
| Every scoped input of a qualified endpoint fails with `ENODATA` | All channels Unavailable; no zero published; endpoint stays qualified. |
| `temp5_input` (`Virtual sensor 1`) on QUADRO returns `50000` | Not read; no channel published. |
| `temp1_input` `327670` | Temperature omitted. |
| `temp1_input` `125000`, then `125100` | Admit 125.0 °C, then omit. |
| `fan1_input` `65536`, negative or malformed | Invalid Fan Reading; siblings preserved. |
| Flow input malformed or `65536` | Flow Unavailable; siblings preserved. |
| `0003:0C70:F00E.0006` bound to `aquacomputer-d5next`, hwmon `d5next` (v5.15 to v5.17) | Qualified; published as the first case. |
| `0003:0C70:F00D.0005` bound to `aquacomputer-d5next` | Not qualified; Unavailable; no input reads. |
| Two qualified `F00E` HID devices with different sequence suffixes (S13 `09b21a2c45`) | Two independent devices; never merged. |
| `F00D` endpoint whose hwmon `name` is `octo`, or `fan5_label` is `Fan 5 speed` | Endpoint Unavailable, or that channel Unavailable, respectively; no input read for the failing item. |
| HID device `0003:0C70:F011.000F` bound to `hid-generic`, unbound, or without a hwmon child | Unavailable; no `hidraw` access and no driver action. |
| hwmon `device` link not resolving to the matching HID device, class link not resolving to the child, or two hwmon children | Unavailable; no input reads. |
| Qualified endpoint with `pwm1`, `temp1_offset` and `fan5_pulses` present | None of them is read. |
| HID devices `0003:0C70:F00A.0004`, `F0B6`, `F003`, `F0BD`, `F001`, `F00B` or `F014` bound to the driver | Not this scope; no attribute of their hwmon child is read. |
| Endpoint observed for the first time with all inputs returning values | Sample withheld; channels Unavailable until a sample starts 3 s of `CLOCK_MONOTONIC` time after the first observation (S15 rule). |
| A trusted clock pair whose difference grew by 600 s, with inputs returning pre-suspend values | Sample withheld; hold of 3 s as in S15. |
| Endpoint removed, then present again with a new `hwmonN` and HID sequence | Unavailable while absent; readings resume after re-qualification and the hold. |
| No listed `0C70` HID device | Unsupported; no hwmon reads for this scope. |

## Open questions

1. Non-blocking for #2528 Linux hwmon scope: only the Windows scope
   depends on it, and that scope stays Unsupported.
   aquasuite's access mode and its "special USB settings" (S6 §16.7)
   are undocumented. Enabling Windows needs evidence of how aquasuite
   and its service open the HID and vendor interfaces, whether a
   second shared reader of input reports disturbs them, and a
   maintainer decision.
2. Non-blocking for #2528 Linux hwmon scope: Windows only.
   The Windows top-level collection layout (usage page, usage, input
   report length) of each scoped device, and whether a standard user
   can open it for reading, are unrecorded. A report descriptor dump or
   `HidP_GetCaps` capture is needed. Whether Windows readers of one
   collection each receive every input report is the open question
   recorded in S15.
3. Non-blocking for #2528 Linux hwmon scope: a flow of 0 is presented
   as the device's zero reading without an alarm (step 7), so neither
   case is misstated.
   On QUADRO and OCTO the driver gives no sign whether a flow sensor
   is connected; S13 `1fc7de9a08` shows 0 with fans running and no
   record of a sensor. Whether the device marks an absent flow sensor
   in the report is unknown.
4. Non-blocking for #2528 Linux hwmon scope: the channel keeps the
   driver's label and role, and S13 shows only fan use.
   When the D5 NEXT fan connector is configured as a flow sensor input
   (S8 §11.6), what the `Fan speed` channel then carries is unknown.
5. Non-blocking for #2528 Linux hwmon scope: the residual is stated in
   Quirks, out-of-range values are omitted, and the fix is marked for
   stable kernels.
   Whether these devices ever send a status report shorter than the
   driver's fixed offsets is unknown. On kernels without commit
   `cf8f81babd2e` such a report could publish plausible but wrong
   values until the next full report a second later.
6. Non-blocking for #2528 Linux hwmon scope: the software-sensor
   channels are not read.
   S6 §9 lists eight software sensors on QUADRO while the driver
   exposes sixteen virtual channels (S1, S3). The meaning of the extra
   channels is unknown.
7. Non-blocking for #2528 Linux hwmon scope: the scope is live-only
   and stores no history, so session-local identity suffices.
   Each device reports a USB serial number string (S13), and the driver
   exposes a serial number in debugfs (S1). Neither is read. Using one
   as a stable identity needs a product decision (DP-01).
8. Non-blocking for #2528 Linux hwmon scope: readings stay attributed
   to their source, so nothing is merged or counted twice silently.
   A QUADRO, OCTO or D5 NEXT connected to an aquaero by aquabus, or a
   pump speed signal wired to a motherboard header, can make the same
   fan or pump appear under two sources.
9. Non-blocking for #2528 Linux hwmon scope: presentation is an App and
   frontend modelling decision, not a hardware fact; the channel roles,
   units and attribution above are complete.
   The external sensor device model proposed in PR #2827 has no flow
   quantity and no numbered temperature-input or fan roles. This scope
   needs a flow quantity in liters per hour, kept apart from rotational
   speed, a pump role, numbered fan roles, and coolant, external and
   numbered temperature-input roles, decided in that design review.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-29 | Initial version. Propose Implementation-ready for one Experimental Linux `aquacomputer_d5next` hwmon scope keyed by mechanism: the pushed-status-report identities D5 NEXT, QUADRO, OCTO, high flow NEXT and farbwerk 360, read from the driver's cache without device traffic. Record the channel map with flow kept distinct from RPM, the excluded software sensors and device-querying attributes, the reuse of the Kraken resume rule, the short-report and per-version quirks, the Windows blockers, and a ranked disposition for every #2528 target and every other identity of the driver. |
