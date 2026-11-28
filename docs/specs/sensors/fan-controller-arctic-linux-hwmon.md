# Spec: ARCTIC Fan Controller fan RPM through Linux hwmon, and #2530 target dispositions

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental, read-only fan RPM of the ten channels of the ARCTIC Fan Controller (USB `3904:F001`) through an already bound Linux `arctic_fan` hwmon endpoint. No host-to-device transaction occurs on this path. The `pwmN` attributes, fan control, Linux `hidraw` access and Windows direct HID access are out of scope or Unsupported. The Target inventory gives every #2530 target (Razer, AeroCool P7-H1, Heatmaster, T-Balancer) and the adjacent USB fan hubs a disposition. |
| Issue phase | [#2530](https://github.com/shm11C3/HardwareVisualizer/issues/2530) (parent [#2510](https://github.com/shm11C3/HardwareVisualizer/issues/2510)) |

ARCTIC wrote the Linux driver and its kernel documentation, and ARCTIC's
own manual documents the Linux interface. The kernel documentation (S1)
and the vendor manual (S2) are the primary sources for the USB identity,
the ten channels, the hwmon name, the report cadence and the RPM
encoding. The bound HID driver name, the absence of labels, the cached
read, the initial zero values and the absence of any output report on
the read path are known only from the driver source. Those facts are
lead-only under
[ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md),
so the only enabled scope is Experimental. This document follows the
bound-endpoint pattern of the
[NZXT Kraken X42–X72 specification](cooler-nzxt-kraken-x42-x72.md) and
reuses its resume rule.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Linux kernel documentation [`Documentation/hwmon/arctic_fan_controller.rst` at v7.3](https://github.com/torvalds/linux/blob/v7.3/Documentation/hwmon/arctic_fan_controller.rst), Git blob `b5be88ae464d7d7c87474e28b06ba610fa4af8d5` (file SHA-256 `6ab125e146aaa20411c2ec7b1355992e1f870189ccbfada800b0ced2f543879c`, the same blob at v7.2), sections Supported devices, Description, Usage notes and Sysfs entries; compared with [v7.4-rc4](https://github.com/torvalds/linux/blob/v7.4-rc4/Documentation/hwmon/arctic_fan_controller.rst), blob `60ac09a9cea3a9fc7515287d3f18dd7edcf25e36` (SHA-256 `09da9325aa0be90ec9368cbe30383152cad2a7f4de333a9c5a79b27751b93006`). Author: an ARCTIC employee. | Primary for the public driver ABI: supported device `3904:F001`, ten fan channels, status reports about once per second with ten unsigned 16-bit little-endian RPM values in bytes 11–30, fan control only on a host output report, a 2-byte acknowledgement report after an output report, no Get_Report support, hot plug and autodetection, possible power loss over suspend, `fan[1-10]_input` read-only RPM updated at about 1 Hz, `pwm[1-10]` a host cache of the last written value. v7.4-rc4 changes only the documented initial `pwm` cache value (0 before, 102 after) and the license line. |
| S2 | ARCTIC, [*Fan Controller* user manual](https://support.arctic.de/en/fan-controller), retrieved 2026-11-29 (HTML SHA-256 `e242a72c65c47a1010f32bf28d0b6e798e466ea8dfd74c98507079648996fab3`), sections Installation, Software (Windows and Linux) and Troubleshooting; [Downloads](https://support.arctic.de/fan-controller/docs): *Fan Controller – 2D Drawing*, P/N ACFAN00351A, revision date 2026-09-02 (PDF SHA-256 `ee365db6fce8a7c54e43bcb4376cc2278d422d3e820fe8fba5ae8b9bcf01c759`) | Primary vendor statements: part number ACFAN00351A; connected by a SATA power cable and an internal USB 2.0 header; fans must not be connected while the device is powered; until it receives commands over USB the controller drives every fan at a default 40 % duty; on Windows, SignalRGB and FanControl list and control the fans; on Linux the hwmon driver is included from Linux 7.2, loads automatically, has the hwmon name `arctic_fan` and serves fan speed in `fan{i}_input` for `i` 1–10; a device visible in `lsusb` with vendor `3904` but without a driver means a kernel older than 7.2 or a kernel with the driver disabled; the driver was also verified on Linux 5.15 with an out-of-tree module. |
| S3 | Linux kernel documentation [`Documentation/ABI/testing/sysfs-class-hwmon` at v7.3](https://github.com/torvalds/linux/blob/v7.3/Documentation/ABI/testing/sysfs-class-hwmon), file SHA-256 `687fdee89a5370852322f35b543f2aab4b753a4505c2970254db5eaf3330579c`, entries `name` and `fanY_input` | Primary for the generic ABI: `name` is the chip name and the only mandatory attribute; `fanY_input` is read-only, in revolutions per minute. |
| S4 | Linux [`drivers/hwmon/arctic_fan_controller.c` at v7.3](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/arctic_fan_controller.c), Git blob `dbe84cd93c0837fc91c191f50b057f15d16f3335` (file SHA-256 `9082e01f02f7c3e8d348123124f7dec9ac956655b4b004407405f48809af167b`, the same blob at v7.2), GPL-2.0-or-later; compared with [v7.4-rc4](https://github.com/torvalds/linux/blob/v7.4-rc4/drivers/hwmon/arctic_fan_controller.c), blob `e2e0bb8f2a7fc1cc24aaf9bbc08ddd4b059018eb` (SHA-256 `dd3b39944995c57c7c43eb9661bf1f0dac9a81327fbdae9208585eb3ab38d723`), GPL-2.0-or-later OR BSD-2-Clause; and the `SENSORS_ARCTIC_FAN_CONTROLLER` entry of [`drivers/hwmon/Kconfig` at v7.3](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/Kconfig) | **lead-only (copyleft)** for the module name `arctic_fan_controller`, the dependency on USB HID, the HID match (USB transport only, `3904:F001`), the HID driver name `arctic_fan`, the hwmon name `arctic_fan`, the attribute set and modes, the absence of label attributes, the cached read without USB traffic, the zero initial RPM values, the accepted status report (report ID `0x01`, exactly 32 bytes) and the ignored reports, the absence of any output report outside a `pwmN` write, the `pwmN` write path and its one-second acknowledgement wait, the HID connection without a `hidraw` node, the resume handling that leaves the RPM values untouched, and the hwmon parent. v7.4-rc4 differs only in the initial `pwm` cache value, an allocation helper and the license; the fan read contract is identical. |
| S5 | Linux commits [`e28d0c73d4d7`](https://github.com/torvalds/linux/commit/e28d0c73d4d7adc9cd3747d81fdc7338217f9a0c) (driver added, May 2026, first in v7.2), [`d17832c1c97b`](https://github.com/torvalds/linux/commit/d17832c1c97b0a7af780ac7d500b5412d9edabd1) (initial `pwm` cache set to the 40 % factory default) and [`a6a362213998`](https://github.com/torvalds/linux/commit/a6a3622139981f84d600d855d26a55794af29ee5) (dual license), commit messages | **lead-only (copyleft)**: the device never changes its duty on its own; the acknowledgement latency measured by the author was at most about 563 ms over 500 writes; the 40 % factory default applies after power-on and after a power-loss resume; ARCTIC is the sole author of the in-tree driver. |
| S6 | Linux [`drivers/hid/hid-core.c` at v7.3](https://github.com/torvalds/linux/blob/v7.3/drivers/hid/hid-core.c) (file SHA-256 `217a2ee1d59d93fd7c5772b9193d4d3c0e3b7d5d24c9bcd985cb59c172751577`) and [`include/linux/hid.h` at v7.3](https://github.com/torvalds/linux/blob/v7.3/include/linux/hid.h) (SHA-256 `6bd34b0fc374a7bf15ad66f5a579c9a576d811d2841f1b1d77a818161121214a`): device naming, `HID_ID` uevent, driver `new_id` attribute, connect options | **lead-only (copyleft)**: HID device sysfs name `<bus>:<vendor>:<product>.<seq>` with four uppercase hex digits each; `HID_ID` uevent `<bus>:<vendor>:<product>` with four, eight and eight hex digits; every HID driver accepts extra IDs through `new_id`; a `hidraw` node is created only when the binding driver requests it. |
| S7 | Linux [`drivers/hwmon/hwmon.c` at v7.3](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/hwmon.c), file SHA-256 `c42d168282e8dbbacb52e3b78fd1fa6f7a43a35a4600b2f619f88a4dcfb31e49`, attribute show and store | **lead-only (copyleft)**: one per-device lock is held across every attribute read and every attribute write, so a read waits while another process's `pwmN` write is in progress. |
| S8 | linux-hardware.org [device `usb:3904-f001`](https://linux-hardware.org/?id=usb:3904-f001) and [probe `041f2c7b2f`](https://linux-hardware.org/?probe=041f2c7b2f) with its [`sensors` log](https://linux-hardware.org/?log=sensors&probe=041f2c7b2f), retrieved 2026-11-29 (log HTML SHA-256 `2fea17430e4658c81890c78a2b5a834f0f7847df3b3e3a3a32a85c81985c9cc2`) | Independent observation, not maintainer-accepted. CachyOS, kernel `7.2.2-1-cachyos`, ASRock X870 Taichi Creator. USB `3904:f001` "ARCTIC (HK) ARCTIC Fan Controller", interface class `03-00-00` on `usbhid`. Chip `arctic_fan-hid-3-6` on a HID adapter with ten unlabelled fan channels: 823, 441, 1000, 1000, 1000, 0, 0, 0, 0, 0 RPM, and `pwm` entries. |
| S9 | Michael Larabel, [*ARCTIC Fan Controller: The Best Fan Controller For Linux Desktops*](https://www.phoronix.com/review/arctic-fan-controller), Phoronix, August 17, 2026, pages 1 and 2 | Independent observation, not maintainer-accepted. The driver is upstream in Linux 7.2; on Ubuntu with a 7.2 kernel built with `CONFIG_SENSORS_ARCTIC_FAN_CONTROLLER`, all ten channels appear and work with hwmon consumers such as CoolerControl. Product announced at the end of July 2026, priced at 8.99 USD. |
| S10 | LibreHardwareMonitor at commit [`2808d80564b6`](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/tree/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Controller), `Hardware/Controller`: `Arctic/README.MD` (SHA-256 `6f13fca3f715496e5a01a931d885ed39ff11f44621326394fb89357b154fd007`), `Arctic/ArcticFanController.cs` (`d199f416e5422ec9ebe72734c5d14edd9a66a2e374ebb2f34619acf28366eb4d`), `Razer/RazerFanController.cs` (`be4f58d20270690070a2e8b5ad420b1c216a1504ec883c5f429d06775501b8eb`), `Razer/RazerGroup.cs` (`afced588f062e45eb3de9183da4f540c1e0f9258b148ee0a321f0399ade03535`), `AeroCool/P7-H1.cs` (`5312c3b860c0d1a05a5bd80ad1500ea904825d546faa37dadf6321abd6e90271`), `AeroCool/AeroCoolGroup.cs` (`63e2fe0120ffeaa77ba9b603ae18bd7d322f118d045707607ff35075fde53c3e`), `Heatmaster/HeatmasterGroup.cs` (`43761a0a0fad37150f24a0697c89c4065b696bf5431c170ceae00fb8219851ee`), `TBalancer/TBalancer.cs` (`184f9f83d1d1ac8876399ebea1b103b6c181dba46c5940e4b785cd0769c0be44`), `TBalancer/TBalancerGroup.cs` (`cee338025f3df7713b085445ea9b9297193ba8d0482770f9c0ecd1cd9e93c815`); `Hardware/Mutexes.cs`, named mutexes | **lead-only (copyleft)**, used only for the Target inventory and the Windows notes: the ARCTIC status report also carries the current duty of each channel in bytes 1–10; LibreHardwareMonitor opens the ARCTIC device on Windows, writes duty values for every channel while it controls, and writes a fixed 30 % to every channel when it closes; the Razer, AeroCool, Heatmaster and T-Balancer identities, transports and request patterns in the Target inventory. |
| S11 | [NZXT Kraken X42–X72 specification](cooler-nzxt-kraken-x42-x72.md) revision 2: section Quirks (resume rule), section Windows HID transport, Sources S11, S12, S14 and S15, Open questions 3, 10 and 11 | Repository specification, Implementation-ready (rev 2). Reused for the resume rule and its clock facts (primary there), and for the Windows HID facts (primary there). |
| S12 | Pull request [#2827](https://github.com/shm11C3/HardwareVisualizer/pull/2827), `docs/design/external-sensor-devices.md` (open, proposed) | Repository design proposal. Device kinds `Cooler` and `PowerSupply`, a later `FanController` kind, a channel index for devices with several fans, Windows and macOS providers out of scope. Not a hardware source. |
| S13 | Linux kernel documentation [`Documentation/hwmon/corsair-cpro.rst` at v7.3](https://github.com/torvalds/linux/blob/v7.3/Documentation/hwmon/corsair-cpro.rst), blob `15077203a2f81d1c5313f0a653fcae59da7c684c`; Linux [`drivers/hwmon/corsair-cpro.c` at v7.3](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/corsair-cpro.c), blob `c09645152613298e62b6eacf3e07967dd0273009`, first present at v5.9 | Documentation is primary for the Commander Pro ABI (six fans, four temperature probe connectors, three SATA voltages). The driver is **lead-only (copyleft)**, used only for the Target inventory: USB identities `1B1C:0C10` and `1B1C:1D00`, hwmon name `corsaircpro`, one device request per input read, and a kept `hidraw` node whose concurrent use can swap replies. |
| S14 | Razer, [*At a Glance: Razer PWM PC Fan Controller*, RZ34-02140](https://mysupport.razer.com/app/answers/detail/a_id/5737) and the [product page](https://www.razer.com/gaming-pc-accessories/razer-pwm-pc-fan-controller) | Primary vendor statements, used only for the Target inventory: eight 4-pin PWM headers, SATA power, internal USB 2.0 header, control through Razer Synapse. |
| S15 | AeroCool, [*P7-H1*](https://aerocool.io/?p=28944) product page | Primary vendor statements, used only for the Target inventory: five fan connectors, monitors fan RPM but does not control fan speed, 9-pin USB 2.0 connector, P7-S1 software for Windows 7 and later. |
| S16 | Linux kernel directory listings of [`Documentation/hwmon`](https://github.com/torvalds/linux/tree/v7.4-rc4/Documentation/hwmon) and [`drivers/hwmon`](https://github.com/torvalds/linux/tree/v7.4-rc4/drivers/hwmon) at v7.4-rc4, retrieved 2026-11-29 (file names only) | Which hwmon drivers are in the kernel tree. USB cooling drivers present: `aquacomputer_d5next`, `arctic_fan_controller`, `asus_rog_ryujin`, `corsair-cpro`, `corsair-psu`, `gigabyte_waterforce`, `nzxt-kraken2`, `nzxt-kraken3`, `nzxt-smart2`. No hwmon driver for Corsair Commander Core, Razer, AeroCool, Heatmaster or T-Balancer devices. **lead-only (copyleft)** from `drivers/hid/hid-razer.c` and `drivers/hid/hid-ids.h` at v7.4-rc4: the HID Razer driver matches Razer keyboard identities only, not `1532:0F3C`. |
| S17 | [`usb.ids`](http://www.linux-usb.org/usb.ids) version 2026.11.25 | Public identifier database: `10C4:EA60` is the Silicon Labs CP210x UART Bridge, `0403:6001` the FTDI FT232 serial IC, vendor `1532` Razer USA; no entry for vendor `3904`. |

## Target inventory

Order is the spec author's installed-base estimate from launch dates,
price and vendor ecosystem. It is not measured demand. Each row is keyed
by the USB identity and transport that select a mechanism; product names
are evidence.

| Rank | Device | Identity and transport | Read mechanism | Disposition in this revision |
| --- | --- | --- | --- | --- |
| 1 | Corsair Commander Pro, Obsidian 1000D controller (outside the LHM comparison inventory) | USB HID `1B1C:0C10`, `1B1C:1D00` (S13) | Linux in-tree `corsair-cpro` since v5.9, hwmon `corsaircpro`: four external temperature probes, six fans, three SATA voltages. Every input read is a device request; the `hidraw` node stays available and a concurrent `hidraw` user can swap replies. (S13) | Not specified here. Follow-up: a separate live-query specification in the pattern of the [Corsair PSU specification](psu-corsair-hxi-rmi-linux-hwmon.md). Highest installed-base candidate. |
| 2 | Corsair Commander Core, Commander Core XT (outside the LHM comparison inventory) | USB, identity not recorded here | No in-tree Linux hwmon driver at v7.4-rc4 (S16). Windows access would share the device with Corsair iCUE, whose arbitration is unknown (see the Corsair PSU specification, Open question 1). | Unsupported. Follow-up. |
| 3 | Razer PWM PC Fan Controller (RZ34-02140) | USB HID `1532:0F3C` (S10), eight PWM headers (S14) | No Linux driver (S16). On Windows every RPM reading is a host feature-report request followed by a feature-report read. LibreHardwareMonitor queries the firmware version when it opens the device, and serializes its requests with the named mutex `Global\RazerReadWriteGuardMutex`. Razer Synapse controls the device. (S10, S14) | Unsupported. Follow-up: needs evidence that the speed query changes no channel mode or duty, and Synapse arbitration evidence. |
| 4 | ARCTIC Fan Controller (ACFAN00351A) | USB HID `3904:F001` (S1, S2, S8) | Unsolicited status report about once per second. Linux in-tree `arctic_fan` since v7.2. (S1, S2, S4) | This specification. Linux bound hwmon: Experimental. Windows direct HID and Linux `hidraw`: Unsupported (see Detection). Windows follow-up shares the passive input-report mechanism with row 5 and the Kraken Windows blocker (S11). |
| 5 | AeroCool P7-H1 hub | USB HID vendor `2E97`, products `0x1001`–`0x1008` (product minus `0x1000` is the hub number), interface 2 (S10); five fan connectors (S15) | No Linux driver (S16). Unsolicited 16-byte input reports with report ID 0 carry five RPM values; no host request. Monitoring only: the hub does not control fan speed (S15). (S10) | Unsupported. Follow-up: Windows passive HID read, together with the ARCTIC Windows follow-up. |
| 6 | Heatmaster (legacy serial controller) | Silicon Labs CP210x USB-to-UART bridge `10C4:EA60`, a generic identity shared by unrelated devices (S10, S17) | Serial port at 38400 baud with text commands. Detection writes a probe byte and a text query to every port with that generic identity. After detection LibreHardwareMonitor also sends a write command that sets the device's update rate to 2 Hz. (S10) | Unsupported. Detection would write to devices that are not known to be a Heatmaster, and the evidenced read path changes device configuration. Follow-up only if a device-specific identity and a write-free read are evidenced. |
| 7 | T-Balancer bigNG and miniNG (legacy) | FTDI FT232BM through the vendor FTD2XX library, a generic FTDI serial identity (S10, S17) | 19200 baud with RTS/CTS. Detection writes a request byte to every FT232BM device and checks a protocol-version field in a 284-byte response; every sample sends request bytes again. The response carries analog and digital temperature probes, flow meters and fans. (S10) | Unsupported for the same reason as row 6, and because it needs a proprietary library. Follow-up only if a device-specific identity is evidenced. |

### Adjacent coverage (USB cooling endpoints already tracked or untracked)

| Device group | Status |
| --- | --- |
| NZXT Kraken, GRID and RGB & Fan Controller (`nzxt-kraken2`, `nzxt-kraken3`, `nzxt-smart2`) | Tracked by [#2527](https://github.com/shm11C3/HardwareVisualizer/issues/2527). Not duplicated here. |
| Aqua Computer controllers and probes (`aquacomputer_d5next`) | Tracked by [#2528](https://github.com/shm11C3/HardwareVisualizer/issues/2528). Not duplicated here. |
| MSI CoreLiquid | Tracked by [#2529](https://github.com/shm11C3/HardwareVisualizer/issues/2529). Not duplicated here. |
| ASUS ROG Ryujin (`asus_rog_ryujin`) and Gigabyte Waterforce (`gigabyte_waterforce`) AIO coolers | In-tree Linux hwmon drivers at v7.4-rc4 (S16). Not tracked by #2527–#2530. Untracked device groups. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| A HID device on the Linux HID bus whose sysfs name is `0003:3904:F001.<seq>` and whose `uevent` `HID_ID` is `0003:00003904:0000F001` | mechanism: selects the ARCTIC status-report mechanism; excludes a device added to the driver through `new_id` | S1, S2, S4, S6 (lead-only name and uevent formats), S8 |
| That HID device's `subsystem` link resolves to `/sys/bus/hid` and its `driver` link resolves to `/sys/bus/hid/drivers/arctic_fan` | mechanism: the bound driver defines the ABI and the absence of device traffic on a read | S4 |
| Exactly one hwmon child `<hid device>/hwmon/hwmonN` exists, its `name` is `arctic_fan`, its `device` link resolves to the same HID device, and `/sys/class/hwmon/hwmonN` resolves to that child | mechanism: hwmon chip kind and ownership | S2, S3, S4, S8 |
| On that child, `fan1_input` through `fan10_input` exist, `fan11_input` does not, and no `fanN_label` file exists | mechanism: the channel set detects a changed driver ABI | S1, S2, S4, S8 |
| Per value, the value is within the admitted range in Read procedure | mechanism (plausibility) | S3, [ADR 0011](../../adr/0011-experimental-sensor-enablement.md) |

The kernel version, the distribution, the motherboard, the `hwmonN`
number, the HID sequence suffix and the libsensors chip name are
evidence identities or session enumeration details. They never gate a
read.

| Scope | Status | Default enablement |
| --- | --- | --- |
| Linux, already bound `arctic_fan` endpoint on HID device `3904:F001`: `fan1_input`–`fan10_input` | Primary: S1 (identity, ten channels, about 1 Hz status reports, RPM encoding, read-only fan inputs, no Get_Report), S2 (hwmon name `arctic_fan`, `fan{i}_input` 1–10, driver from Linux 7.2), S3 (units). Lead-only S4: HID driver name, absence of labels, cached read without USB traffic, zero values before the first report, accepted report ID and length, no output report outside a `pwmN` write, no `hidraw` node, RPM values untouched at resume, hwmon parent. Lead-only S5: first in v7.2. Lead-only S6: HID name and `HID_ID` formats, `new_id`. Lead-only S7: reads wait behind a concurrent `pwmN` write. Independent observations S8 (kernel 7.2.2, ASRock X870 Taichi Creator) and S9 corroborate the identity, the HID-class interface, the chip name, ten unlabelled channels and plausible values; neither is maintainer-accepted. No maintainer-accepted hardware dump exists. | Experimental |
| `pwm1`–`pwm10` | A host cache of the last value written through sysfs, not the device's duty (S1). | Not read. Never presented as fan duty. |
| Duty bytes 1–10 of the status report | Device-reported duty (S10). Not exposed by the driver (S4). | Unsupported by this revision |
| Fan control (`pwmN` writes) | Out of scope (#2530, specs README safety policy). | Never |
| Linux `hidraw` access | No `hidraw` node exists while the driver is bound (S4, S6). With another driver bound, raw access is not specified. | Unsupported |
| Windows direct HID read | See Windows notes and Open questions 1 and 2. | Unsupported |
| Every other target | See the Target inventory. | Unsupported |

## Device status report (facts)

| Byte offset | Meaning | Encoding | Source |
| --- | --- | --- | --- |
| 0 | Report ID | `0x01` | S1, S4 |
| 1–10 | Current duty of channels 1–10 | unsigned 8-bit, percent 0–100 | S10 |
| 11–30 | Fan speed of channels 1–10, two bytes per channel, channel 1 at bytes 11–12 | unsigned 16-bit little-endian, RPM | S1, S4 |
| 31 | Padding | — | S10 |

- The device sends this 32-byte input report on its own about once per
  second. It does not support Get_Report. (S1)
- After the host sends an output report with new duty values, the
  device applies them and answers with a 2-byte acknowledgement report,
  report ID `0x02`, byte 1 `0x00` on success. The device never changes
  its duty on its own. (S1, S5) This specification sends no output
  report.
- Until it receives commands over USB, including after power-on, the
  controller drives every fan at 40 % duty. (S2, S5)

## Linux hwmon endpoint (facts)

| Attribute | Meaning | Units / encoding | Mode | Source |
| --- | --- | --- | --- | --- |
| `name` | `arctic_fan` | text | `0444` | S2, S4, S8 |
| `fan1_input` … `fan10_input` | Fan speed of channel 1 … 10 | RPM, unsigned 16-bit source value | `0444` | S1, S2, S3, S4 |
| `pwm1` … `pwm10` | Host cache of the last written duty, 0–255 | — | `0644` | S1, S4 |

- The driver defines no label, limit, alarm or temperature attribute.
  (S4)
- A `fanN_input` read returns the RPM value of the most recent
  accepted status report and performs no USB transfer. It does not
  fail. Before the first accepted report after bind, every channel
  reads 0. The driver keeps no timestamp, so a value stays in place
  until the next accepted report replaces all ten. (S4)
- The driver accepts a status report only with report ID `0x01` and a
  length of exactly 32 bytes, and ignores every other input report
  except the acknowledgement. (S4)
- The driver sends an output report only when a process writes a
  `pwmN` attribute. It sends none at bind, at a read, at resume or at
  unbind. (S4)
- A `pwmN` write sends all ten cached duty values in one output report
  and waits up to one second for the acknowledgement (S1, S4). The
  author measured at most about 563 ms (S5). The hwmon core holds one
  per-device lock across that wait, so a `fanN_input` read issued
  meanwhile waits for it (S7).
- The driver connects to the HID device without a `hidraw` node and
  without an input device, and keeps the device open so status reports
  arrive continuously. The hwmon device is registered with the HID
  device as its parent. (S4, S6)
- At resume after a USB reset, the driver resets only its `pwm` cache;
  the RPM values received before suspend stay in place until the next
  status report. (S4) The device may lose power over suspend (S1).
- The driver exists since v7.2. v7.2 and v7.3 carry the same file; the
  v7.4-rc4 changes leave the fan read contract unchanged. (S4, S5) The
  kernel option is `CONFIG_SENSORS_ARCTIC_FAN_CONTROLLER` and the
  module is `arctic_fan_controller` (S4, S9); a kernel can have it
  disabled (S2).

## Read procedure and decode

1. Find HID devices under `/sys/bus/hid/devices` whose name and
   `HID_ID` match the identity gate. If none exists, the scope is
   Unsupported on this machine and nothing else is read.
2. For each matching HID device, apply the driver, hwmon ownership and
   channel-set gates. A matching HID device without a qualified
   endpoint is Unavailable with the reason that the driver is not
   bound or the endpoint does not qualify; this is the expected state
   on a kernel older than 7.2 or without the driver (S2). Do not fall
   back to `hidraw`, do not bind, unbind or load a driver, and do not
   write `new_id`.
3. Read `fan1_input` through `fan10_input` in channel order, between
   the clock reads of the resume rule below. Read no other attribute in
   a sample.
4. Time limit (product policy). Before each input read, if more than
   1.5 s of `CLOCK_MONOTONIC` time has passed since the sample start
   of the resume rule, mark that channel and every later channel
   Unavailable for this sample and read nothing more from that
   endpoint until the next sample. A read can wait behind another
   process's `pwmN` write for up to about one second (S4, S7); the
   limit bounds a sample to about one such wait after it is reached.
5. A failed read makes only that channel Unavailable for the sample.
   Never substitute zero.
6. Parse each input as an unsigned decimal integer from 0 through
   65535. A value of 0 is an Inactive Fan Reading, a positive value an
   Active Fan Reading. Malformed text or a value outside that range is
   an Invalid Fan Reading.
7. Resume and first-observation rule: apply the resume rule of S11
   (NZXT Kraken X42–X72 specification revision 2, Quirks, steps 1–7)
   unchanged, with ten inputs in step 2 instead of three, the same
   50 ms, 100 ms and 3 s parameters, and a per-endpoint recorded clock
   difference and hold start. A held endpoint's channels are
   Unavailable, not zero.

   Why it applies here: before the first status report after bind
   every channel reads 0, and after a resume the driver keeps the
   pre-suspend RPM values (S4). Neither is distinguishable from a real
   reading. The device reports about once per second (S1), and every
   accepted report replaces all ten values (S4), so 3 s of
   non-suspended time after the hold start covers about three report
   periods. A device that powers down over suspend and re-enumerates
   appears as a new HID device and receives the first-observation hold.
   The rule does not detect a device that stays bound but stops
   reporting; see Open question 3.
8. Keep channels 1–10 as ten fan channels of one controller. A channel
   reading 0 RPM is normal: nothing connected to the header, a fan
   without a tachometer signal, or a stopped fan all read 0, and the
   device reports no presence information (S1, S4, S8). Present it as
   Inactive, never as a failure.

Cadence and cost. A read costs one sysfs file read and no USB traffic
(S4). The values change at most about once per second (S1). Sample at
most once per second per controller, and only while the readings are
requested (visible, or an explicit background consumer). Run the reads
off any UI thread, because of the wait in step 4.

Attribution. Each qualified HID device is one controller. Readings from
different controllers are never merged. The `hwmonN` number and the HID
sequence suffix change across reboots and re-plugs and identify a
controller only within a session. When an endpoint disappears (unplug,
unbind), its channels are Unavailable until a qualified endpoint is
found again and its first-observation hold ends.

## Coexistence

- Linux fan-control tools (CoolerControl, scripts writing `pwmN`) write
  through the same hwmon endpoint (S9). Their writes do not change any
  value this scope reads, but a read waits while a write awaits its
  acknowledgement (S7); step 4 bounds that.
- While the driver is bound there is no `hidraw` node, so user-space
  tools that open `hidraw` cannot reach the device (S4, S6).
- ARCTIC ships no Linux control software. On Windows, SignalRGB and
  FanControl control the device (S2); they do not run on Linux.

## Windows notes (scope Unsupported)

No Windows implementation is admitted by this revision. Recorded for
the follow-up:

- The device sends its status report without any host request (S1), so
  a Windows read needs no output, feature or Get_Report request. The
  Windows HID enumeration, identity, sharing and report-buffering facts
  of S11 apply.
- The vendor names SignalRGB and FanControl as the Windows software
  (S2). LibreHardwareMonitor, which FanControl can use as a source,
  opens the device, writes duty values for all ten channels while it
  controls, and writes 30 % to every channel when it closes (S10).
  Those are that software's writes. Whether a second reader handle
  takes reports away from it is the unresolved question of S11 (Open
  question 3 there).
- A Windows provider also needs the external sensor device model of
  S12 to admit Windows providers; S12 keeps them out of scope.

## Safety notes

- The Linux path reads sysfs attributes only. It writes no sysfs
  attribute, opens no `hidraw` node, issues no HID ioctl, loads, binds
  or unbinds no driver and needs no elevation. With the driver's own
  behavior (S4), no host-to-device transaction occurs on this path.
- Never write `pwmN`. A write to one channel sends the cached duty of
  all ten channels (S1), so it can change fans the user did not touch.
- Never send an output, feature or Get_Report request to this device,
  on any platform.
- Windows: do not open the device in this revision.

## Synthetic fixture acceptance

All fixtures are synthetic. Values of the first case are taken from the
S8 observation.

| Case | Expected result |
| --- | --- |
| Qualified endpoint `0003:3904:F001.0006` past its first-observation hold, inputs `823`, `441`, `1000`, `1000`, `1000`, `0`, `0`, `0`, `0`, `0` | Fans 1–5 Active at 823, 441, 1000, 1000 and 1000 RPM; fans 6–10 Inactive; one controller. Only `fan1_input`–`fan10_input` are read; nothing is written. |
| Endpoint observed for the first time (application start or hot plug), all inputs `0` | Sample withheld; channels Unavailable until a sample starts 3 s of `CLOCK_MONOTONIC` time after the first observation. |
| Trusted clock pair whose difference grew by 600 s, inputs still the pre-suspend values | Sample withheld; 3 s hold from detection, then published. |
| `fan4_input` fails with `EIO` | Fan 4 Unavailable, not 0; other channels preserved. |
| `fan7_input` malformed, negative or `65536` | Fan 7 Invalid Fan Reading; siblings preserved. |
| The read of `fan3_input` returns after 1.6 s since the sample start | Fans 1–3 published if the resume rule allows; fans 4–10 Unavailable for this sample; no further reads of that endpoint until the next sample. |
| `fan1_label` present | Endpoint Unavailable; no input reads. |
| `fan10_input` absent, or `fan11_input` present | Endpoint Unavailable; no input reads. |
| hwmon `name` `arctic_fan` whose HID parent is `0003:3904:F002.0004` | Not this scope; no input reads. |
| HID device `0003:3904:F001.0002` bound to `hid-generic`, unbound, or without a hwmon child | Unavailable with the driver-not-bound reason; no `hidraw` access and no driver action. |
| hwmon `device` link not resolving to the matching HID device, class link not resolving to the child, or two hwmon children | Unavailable; no input reads. |
| Two qualified HID devices | Two independent controllers of ten channels each; never merged. |
| Endpoint removed, then present again with a new `hwmonN` and HID sequence | Unavailable while absent; readings resume after re-qualification and the first-observation hold. |
| `pwm1`–`pwm10` present with any value | Not read; no duty channel published. |
| No `3904:F001` HID device | Unsupported; no hwmon reads for this scope. |

## Open questions

1. Non-blocking for #2530 Linux hwmon scope: only the Windows scope
   depends on it, and that scope stays Unsupported.
   How SignalRGB and FanControl (through LibreHardwareMonitor) open the
   device, and whether an additional read-only handle with full sharing
   receives every status report without taking reports from them, is
   unknown (S11 Open question 3). A handle listing and a controlled
   test are needed, followed by a maintainer decision.
2. Non-blocking for #2530 Linux hwmon scope: Windows only.
   The Windows top-level collection layout (usage page, usage, input
   and output report lengths, whether the acknowledgement report shares
   the collection) and whether a standard user can open it are not
   recorded. A report descriptor dump or `HidP_GetCaps` capture is
   needed.
3. Non-blocking for #2530 Linux hwmon scope: the device documents a
   one-second report cadence (S1) and a controller that stops
   reporting while staying enumerated is a device fault; the
   Experimental scope accepts that residual.
   The driver keeps no timestamp (S4), so a controller whose firmware
   stops sending status reports while it stays bound leaves its last
   RPM values in sysfs indefinitely, and the application cannot detect
   it. Whether the firmware can stop reporting without a USB reset is
   not recorded.
4. Non-blocking for #2530 Linux hwmon scope: channels are presented by
   the driver's numbering, which is the device's channel numbering
   (S1).
   No primary source states that `fanN` corresponds to the header
   printed N on the controller; the 2D drawing (S2) carries no header
   labels in its text layer.
5. Non-blocking for #2530 Linux hwmon scope: the gates require the
   driver and hwmon names and channel set of the in-tree driver, and
   the vendor manual documents the same names for its Linux interface.
   S2 states that the driver was verified on Linux 5.15 with an
   out-of-tree module. Whether that module registers the same HID
   driver name, hwmon name and attribute set is not recorded. If it
   does, it qualifies; if not, its endpoint is Unavailable.
6. Non-blocking for #2530 Linux hwmon scope: values are presented as
   reported.
   How the controller derives RPM (tachometer pulses per revolution,
   measurement window) is not documented. S8 shows three channels at
   exactly 1000 RPM in one sample, which may indicate quantization.
7. Non-blocking for #2530 Linux hwmon scope: presentation is an App and
   frontend modelling decision, not a hardware fact.
   The external sensor device model (S12) is open and has no
   `FanController` kind yet. This scope needs one controller entity with
   session-local identity and ten indexed fan channels using the
   existing fan status classification, kept out of motherboard fan
   lists and fan history. Whether Inactive channels are shown or
   collapsed is decided in the implementation design review.
8. Non-blocking for #2530 Linux hwmon scope: the scope is live-only
   and stores no history, so session-local identity suffices.
   No stable per-controller identity across reboots is established.
   Whether the controller reports a USB serial number is not recorded.
9. Non-blocking for #2530 Linux hwmon scope: the residuals are those
   already accepted for the Kraken scope.
   Open questions 10 and 11 of S11 (a suspend after a sample's last
   clock read, and short suspends below the detection thresholds) apply
   unchanged to this endpoint.
10. Non-blocking for #2530 Linux hwmon scope: S4 is still treated as a
    lead under the prohibited-source list, so the scope stays
    Experimental either way.
    From v7.4-rc4 the driver is also offered under BSD-2-Clause (S4,
    S5). Whether a permissively licensed, vendor-authored driver may
    count as a primary source for the lead-only facts above is a
    maintainer policy question, not a hardware fact.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-29 | Initial version. Propose Implementation-ready for the Experimental Linux `arctic_fan` hwmon scope on USB `3904:F001`: ten fan RPM channels, no labels, no host-to-device traffic, the reused Kraken resume and first-observation rule, a 1.5 s per-sample time limit for reads that wait behind another process's `pwmN` write, and the 0 RPM Inactive classification. Rank and disposition every #2530 target and the adjacent USB fan hubs. |
