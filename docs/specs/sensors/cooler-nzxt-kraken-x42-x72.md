# Spec: NZXT Kraken X42/X52/X62/X72 coolant temperature, fan and pump RPM

| Field | Value |
| --- | --- |
| Revision | 2 |
| Status | Implementation-ready (rev 2) |
| Scope | Experimental, read-only coolant temperature, fan RPM and pump RPM for the NZXT Kraken X42, X52, X62 and X72 (USB vendor `0x1E71`, product `0x170E`) through an already bound Linux `nzxt-kraken2` hwmon endpoint. Records the device's unsolicited status report and the Windows HID transport facts. Windows direct HID access, Linux `hidraw` access and every other NZXT generation are Unsupported by this revision; the Generation inventory gives each one a disposition. No fan, pump, lighting or firmware command is in scope. |
| Issue phase | [#2527](https://github.com/shm11C3/HardwareVisualizer/issues/2527) (parent [#2510](https://github.com/shm11C3/HardwareVisualizer/issues/2510)) |

NZXT publishes no protocol documentation for these coolers. Every
device-protocol and driver-behavior fact below rests on copyleft leads
and is tagged lead-only under
[ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md).
The only enabled scope is Experimental. The Linux ABI documentation,
the Microsoft Windows API documentation and NZXT's own coexistence
statement are the primary sources.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Linux kernel documentation [`Documentation/hwmon/nzxt-kraken2.rst` at v6.18](https://github.com/torvalds/linux/blob/v6.18/Documentation/hwmon/nzxt-kraken2.rst), Git blob `94025de65a818ff196f3de0954b0b168d559cf90`, sections Supported devices, Description, Usage Notes and Sysfs entries | Primary for the public driver ABI: supported products, the three input attributes and their units, no exposed control, automatic loading and hot swap. |
| S2 | Linux kernel documentation [`Documentation/hwmon/sysfs-interface.rst` at v6.18](https://github.com/torvalds/linux/blob/v6.18/Documentation/hwmon/sysfs-interface.rst), Git blob `f76e9f8cc1adae19cdfbd4a7e8761c4feb479a0b`, sections Naming and data format standards, Fans and Temperatures | Primary for the generic hwmon ABI: one fixed-point value per file, `*_label` as the suggested channel label, millidegree temperatures. |
| S3 | Linux [`drivers/hwmon/nzxt-kraken2.c` at v6.18](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/nzxt-kraken2.c), Git blob `034698232758615447c5ed5e469bb448b6597fb0` (file SHA-256 `63def3e5f1563dd1119a74d0991b441587ad65a9c661ad8b9a3581fdc307c982`); first added by [commit `82e3430dfa8c32f35ce24a5c628e3e221f168769`](https://github.com/torvalds/linux/commit/82e3430dfa8c32f35ce24a5c628e3e221f168769), contained in v5.13; [v5.13 file](https://github.com/torvalds/linux/blob/v5.13/drivers/hwmon/nzxt-kraken2.c) | **lead-only (copyleft)** for the USB match, hwmon name, channel labels, status report ID, minimum length and decode, the staleness window and its error, the absence of any output report, `hidraw` retention and the hwmon parent. The v5.13 and v6.18 files differ only in a header path, the visibility declaration style and one error path; the read contract is identical. The commit message states that the four models are indistinguishable at the software level. |
| S4 | Linux [`drivers/hid/hid-core.c` at v6.18](https://github.com/torvalds/linux/blob/v6.18/drivers/hid/hid-core.c), HID device naming, uevent and dynamic-ID attribute, and [`include/uapi/linux/input.h` at v6.18](https://github.com/torvalds/linux/blob/v6.18/include/uapi/linux/input.h), bus type constants | **lead-only (copyleft)** for the HID device sysfs name format (bus, vendor, product, sequence; four uppercase hex digits each), the `HID_ID` uevent format (bus four hex digits, vendor and product eight hex digits each), the HID driver `new_id` attribute, and bus type USB = `0x03`. |
| S5 | Jonas Malaco, [reply in the v1 patch review](https://lkml.iu.edu/2103.2/04481.html), linux-kernel, March 18, 2021 | **lead-only (copyleft)**: statements by the driver author during review of S3. The device sends the status report autonomously twice a second and does not answer HID Get_Report for it. |
| S6 | liquidctl at commit [`292dc561361cfbc0cdee155c26662131fe160989`](https://github.com/liquidctl/liquidctl/tree/292dc561361cfbc0cdee155c26662131fe160989): [`liquidctl/driver/kraken2.py`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/liquidctl/driver/kraken2.py) (file SHA-256 `c6e5ff65c84a4cb5b37fe2e8bbd2d46d11c289fe31b36c40376766a58fac5382`) and [`docs/kraken-x2-m2-guide.md`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/docs/kraken-x2-m2-guide.md) (SHA-256 `6505039001663f7b3bc2441b5779fdcb5109941b579889f297274ae0b561c4b0`), GPL-3.0-or-later | **lead-only (copyleft)** for the 64-byte read length, the firmware-version bytes, the Kraken M22 identity and its missing cooling sensors, persistence of configuration while powered, the pump-speed sense pin and the statement that monitoring fails while CAM runs. |
| S7 | liquidctl at the same commit: [`liquidctl/driver/kraken3.py`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/liquidctl/driver/kraken3.py), [`docs/kraken-x3-z3-guide.md`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/docs/kraken-x3-z3-guide.md) (Initialization) and [`docs/developer/protocol/nzxt_x3-z3-2023.md`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/docs/developer/protocol/nzxt_x3-z3-2023.md) (Cooling Operations) | **lead-only (copyleft)**, used only for the Generation inventory: product IDs of the later generations, the requirement to initialize before monitoring and the host status request on Z-series and 2023 or later devices. |
| S8 | liquidctl at the same commit: [`liquidctl/driver/smart_device.py`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/liquidctl/driver/smart_device.py) and [`liquidctl/driver/asetek.py`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/liquidctl/driver/asetek.py); liquidtux at commit [`7256666cf748244071c9440cee49299ab46df172`](https://github.com/liquidctl/liquidtux/tree/7256666cf748244071c9440cee49299ab46df172): [`drivers/hwmon/nzxt-grid3.c`](https://github.com/liquidctl/liquidtux/blob/7256666cf748244071c9440cee49299ab46df172/drivers/hwmon/nzxt-grid3.c) (header description) and [`README.md`](https://github.com/liquidctl/liquidtux/blob/7256666cf748244071c9440cee49299ab46df172/README.md) (Device support) | **lead-only (copyleft)**, used only for the Generation inventory: GRID+ V3 and Smart Device (V1) identities and initialization behavior, the out-of-tree status of their Linux driver, and the Asetek-based Kraken identity. |
| S9 | Linux [`drivers/hwmon/nzxt-kraken3.c` at v6.18](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/nzxt-kraken3.c) (file SHA-256 `4013a4e89b396407290bdc9508844c2068b2e4cec3fbf486bb3d5b85da1d616b`) and [the same file on `master`](https://github.com/torvalds/linux/blob/master/drivers/hwmon/nzxt-kraken3.c) as of 2026-11-28, and [`Documentation/hwmon/nzxt-kraken3.rst` at v6.18](https://github.com/torvalds/linux/blob/v6.18/Documentation/hwmon/nzxt-kraken3.rst) | Documentation is primary for that driver's ABI; the driver is **lead-only (copyleft)**. Used only for the Generation inventory and for the contrasting fan channel numbering. |
| S10 | NZXT, [*NZXT CAM Not Detecting Your Kraken or RGB Controller? How to Fix It*](https://nzxt.com/en-intl/blogs/news/nzxt-cam-not-detecting-device), Oct 5, 2026, Step 6: Rule out a software conflict | Primary vendor statement: CAM needs exclusive access to the hardware it controls, and another program that grabs the device, including a monitoring tool reading a fan sensor, makes CAM lose its connection. |
| S11 | Microsoft Learn, Windows HID documentation: [Finding and Opening a HID Collection](https://learn.microsoft.com/en-us/windows-hardware/drivers/hid/finding-and-opening-a-hid-collection) (User-mode application); [Obtaining HID Reports](https://learn.microsoft.com/en-us/windows-hardware/drivers/hid/obtaining-hid-reports) (Obtaining HID Reports by user-mode applications); [Troubleshooting HID Reports](https://learn.microsoft.com/en-us/windows-hardware/drivers/hid/troubleshooting-hid-reports) (Dropped HID reports); [Minidrivers and the HID Class Driver](https://learn.microsoft.com/en-us/windows-hardware/drivers/hid/minidriver-operations) (Operational features of the HID class driver); [`HidD_SetNumInputBuffers`](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/hidsdi/nf-hidsdi-hidd_setnuminputbuffers) (Remarks); [`HIDP_CAPS`](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/hidpi/ns-hidpi-_hidp_caps) (Members); [`HIDD_ATTRIBUTES`](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/hidsdi/ns-hidsdi-_hidd_attributes) (Members) | Primary for Windows HID enumeration, identity, report buffering and `HidD_GetInputReport` limits. |
| S12 | Microsoft Learn, [`CreateFileW`](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew), parameters `dwDesiredAccess` and `dwShareMode` | Primary for zero-access attribute queries and sharing-mode conflicts (`ERROR_SHARING_VIOLATION`). |
| S13 | linux-hardware.org, [hardware probe `78c54897a1`](https://linux-hardware.org/?probe=78c54897a1), device list and [`sensors` log](https://linux-hardware.org/?log=sensors&probe=78c54897a1) | Independent observation, not maintainer-accepted. Nobara 37, kernel `6.5.3-200.fsync.fc37.x86_64`, ASUS ROG Maximus XI HERO. USB device `1e71:170e` with interface class `03-00-00` (HID, no subclass, no protocol) on `usbhid`. A `kraken2-hid-3-b` chip on a HID adapter reports `Fan` 1473 RPM, `Pump` 2196 RPM and `Coolant` +32.3 °C. The cooler model and firmware are not recorded. |
| S14 | Linux kernel documentation [`Documentation/core-api/timekeeping.rst` at v6.18](https://github.com/torvalds/linux/blob/v6.18/Documentation/core-api/timekeeping.rst), Git blob `22ec68f244210681d89ab36445ecf3f67f7a597d`, section Basic ktime_t based interfaces; Linux [`kernel/time/tick-sched.c` at v6.18](https://github.com/torvalds/linux/blob/v6.18/kernel/time/tick-sched.c), jiffies update from the monotonic time | Documentation is primary: `CLOCK_MONOTONIC` stops during suspend, `CLOCK_BOOTTIME` does not. The source is **lead-only (copyleft)** for jiffies advancing from the monotonic time. |
| S15 | Linux man-pages 6.19, [`clock_gettime(2)`](https://man7.org/linux/man-pages/man2/clock_gettime.2.html), 2026-03-07, `CLOCK_MONOTONIC` and `CLOCK_BOOTTIME` | Primary for the user-space clocks: `CLOCK_MONOTONIC` does not count suspended time; `CLOCK_BOOTTIME` is identical except that it includes it. |

## Generation inventory

Each row is keyed by the USB identity that selects a status-report
mechanism. Product names are evidence; they never select a path.

| Generation | USB identity (vendor:product) | Status report mechanism | Disposition in this revision |
| --- | --- | --- | --- |
| Kraken X42, X52, X62, X72 | `1E71:170E` | Unsolicited input report twice a second. No host request starts or refreshes it. (S3, S5) | This specification. Linux bound hwmon: Experimental. Windows direct HID and Linux `hidraw`: Unsupported (see Detection). |
| Kraken M22 | `1E71:1715` | Same product family as `1E71:170E`; the device has no pump or fan control and reports no liquid temperature. (S6) | Nothing to read. Unsupported. The `nzxt-kraken2` driver does not bind it. (S3) |
| Kraken X53, X63, X73 | `1E71:2007`, `1E71:2014` | The device must be initialized by the host before it is monitored; the Linux driver sends initialization output reports at bind. A later hwmon read on this kind sends no output report. (S7, S9) | Unsupported. Follow-up: a separate specification for the already bound Linux `nzxt-kraken3` endpoint of this kind. Windows needs the initialization writes and is also subject to the CAM finding (S10). |
| Kraken Z53, Z63, Z73 | `1E71:3008` | Initialization at bind, and a host status request for each status report; a stale Linux hwmon read makes the driver send that request. (S7, S9) | Unsupported. Follow-up: requires evidence that the status request leaves pump, fan, lighting and LCD state unchanged. |
| Kraken 2023 and Kraken 2023 Elite | `1E71:300E`, `1E71:300C` | Same mechanism as the Z-series row. (S7, S9) | Unsupported. Same follow-up as the Z-series row. |
| Kraken 2024 Elite RGB | `1E71:3012` | Same mechanism as the Z-series row; bound by `nzxt-kraken3` on `master`, not at v6.18. (S7, S9) | Unsupported. Same follow-up as the Z-series row. |
| Kraken 2024 Plus | `1E71:3014` | Listed by liquidctl with the 2023 channel layout; not in the Linux driver. (S7, S9) | Unsupported. Same follow-up as the Z-series row. |
| GRID+ V3 and Smart Device (V1) | `1E71:1711`, `1E71:1714` | No status reports until a host initialization request. That request makes the device detect connected fans and select each channel's DC or PWM control mode. Reports then arrive five times a second. The Linux driver is out of tree. (S8) | Unsupported: the only evidenced way to start reports changes fan control state. Follow-up under #2527 for GRID V3. |
| Asetek-based Kraken X31, X41, X61, X40, X60 | `2433:B200`, shared with other Asetek 690LC products | Vendor USB protocol of a different manufacturer, not the NZXT HID protocol. (S8) | Outside the NZXT HID inventory of #2527. Unsupported; needs its own follow-up. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| A HID device on the Linux HID bus whose sysfs name is `0003:1E71:170E.<seq>` (bus USB, vendor `0x1E71`, product `0x170E`, four uppercase hex digits each) and whose `uevent` `HID_ID` value is `0003:00001E71:0000170E` | mechanism: selects the X42–X72 status-report generation; the four models share one USB identity and one protocol | S1, S3, S4 lead-only for the naming and uevent formats, S13 for the observed identity |
| That HID device's `subsystem` link resolves to `/sys/bus/hid` and its `driver` link resolves to `/sys/bus/hid/drivers/nzxt-kraken2` | mechanism: the bound driver defines the ABI being read | S1, S3, S4 |
| Exactly one hwmon child `<hid device>/hwmon/hwmonN` exists, its `name` is `kraken2`, its `device` link resolves to the same HID device, and `/sys/class/hwmon/hwmonN` resolves to that child | mechanism: hwmon chip kind and ownership | S2, S3, S13 |
| `temp1_label` is `Coolant`, `fan1_label` is `Fan` and `fan2_label` is `Pump` on that child | mechanism: the channel metadata detects a changed assignment | S3, S13; S9 shows a related driver that numbers pump and fan the other way round |
| Each input value is within the admitted range below | mechanism (plausibility) | S1, S2, [ADR 0011](../../adr/0011-experimental-sensor-enablement.md) |

The cooler model (X42, X52, X62 or X72), its firmware version, the
kernel version, the `hwmonN` number and the HID sequence suffix are
evidence identities or session enumeration details. They never gate a
read. The firmware version is not read at all.

The `nzxt-kraken2` driver also binds any device added to it through
the HID driver's `new_id` attribute (S4). The USB identity gate above
excludes such a device from this scope.

| Scope | Status | Default enablement |
| --- | --- | --- |
| Linux, already bound `nzxt-kraken2` hwmon endpoint on HID device `1E71:170E` | Public ABI: S1, S2. Lead-only S3: USB match, hwmon name `kraken2`, channel labels, decode of the unsolicited report into the three inputs, two-second staleness measured in jiffies and its no-data error, no output report at bind or read, `hidraw` retention and the hwmon parent. Lead-only S4: HID device name and `HID_ID` formats. Lead-only S5: twice-a-second cadence and no Get_Report answer. Lead-only S14: jiffies advance from `CLOCK_MONOTONIC`, so the staleness window excludes suspended time; the resume rule in Quirks depends on this fact together with the S3 window (the clock semantics themselves are primary S14 and S15). Independent observation S13 (kernel 6.5.3, ASUS ROG Maximus XI HERO) corroborates the USB identity with a HID-class interface, the chip name, the HID adapter, the three labels and plausible values; it is not maintainer-accepted. No maintainer-accepted hardware dump exists. | Experimental |
| Linux `hidraw` access to `1E71:170E` | Not specified: the bound driver is the evidenced owner of the device, and raw device-node access needs permissions this scope does not assume. | Unsupported |
| Windows direct HID read of `1E71:170E` | The transport facts are recorded below. NZXT states that CAM needs exclusive access and loses its connection when a monitoring tool grabs the device (S10); see Open questions 1–3. | Unsupported |
| Every other NZXT generation | See the Generation inventory. | Unsupported |

## Device status report (facts)

The device sends one input report repeatedly. Offsets count from the
report ID at byte 0, which is also where Windows places the report ID
in a `ReadFile` buffer (S11).

| Byte offset | Meaning | Encoding | Source |
| --- | --- | --- | --- |
| 0 | Report ID | `0x04` | S3 |
| 1 | Coolant temperature, whole degrees Celsius | unsigned 8-bit | S3, S6 |
| 2 | Coolant temperature, tenths of a degree | unsigned 8-bit; observed values 1–9 | S3, S5 |
| 3–4 | Fan speed | unsigned 16-bit, big-endian, RPM | S3, S6 |
| 5–6 | Pump speed | unsigned 16-bit, big-endian, RPM | S3, S6 |
| 11–14 | Firmware version components | four unsigned bytes | S6 |

- The device sends this report on its own twice a second. It does not
  answer a HID Get_Report request for it. No host request starts,
  refreshes or paces it. (S3, S5)
- The Linux driver ignores a report shorter than 7 bytes or with
  another report ID. liquidctl reads it as a 64-byte transfer.
  (S3, S6)
- The device accepts output reports that configure fan speed, pump
  speed and lighting. This specification documents none of them and
  the application sends none. Configuration persists while the device
  has power. (S1, S6)
- The pump speed can also be wired to a motherboard fan header through
  the sense pin of a standard fan connector. (S6)

## Linux hwmon endpoint (facts)

| Attribute | Meaning | Units / encoding | Source |
| --- | --- | --- | --- |
| `name` | `kraken2` | text | S3, S13 |
| `temp1_input` | Coolant temperature | millidegrees Celsius; the driver computes whole degrees × 1000 + tenths × 100 | S1, S2, S3 |
| `temp1_label` | `Coolant` | text | S3, S13 |
| `fan1_input` | Fan speed | RPM, unsigned 16-bit source value | S1, S3 |
| `fan1_label` | `Fan` | text | S3, S13 |
| `fan2_input` | Pump speed | RPM, unsigned 16-bit source value | S1, S3 |
| `fan2_label` | `Pump` | text | S3, S13 |

- Every attribute of the endpoint has mode `0444`. The driver exposes
  no PWM, limit or other writable attribute. (S1, S3)
- An input read returns the value decoded from the most recent status
  report and performs no USB transfer. When no status report has
  arrived within the last two seconds, which includes the time between
  binding and the first report, every input read fails with `ENODATA`.
  The driver measures those two seconds in kernel jiffies, so time the
  system spends suspended does not count toward them; see Quirks.
  Label reads always succeed. (S3, S14)
- The driver sends no output report to the device at bind, at read or
  at unbind. It keeps the HID device open so that input reports are
  received continuously, and it keeps the `hidraw` node available to
  user-space tools. (S3)
- The hwmon device is registered with the HID device as its parent.
  The kernel loads the driver automatically for the matching USB HID
  device and supports hot swap. (S1, S3)
- The driver exists since v5.13 and its read contract is unchanged
  through v6.18. (S3)

## Read procedure and decode

1. Find HID devices under `/sys/bus/hid/devices` whose name and
   `HID_ID` match the identity gate. If none exists, the scope is
   Unsupported on this machine and nothing else is read.
2. For each matching HID device, apply the driver, hwmon ownership and
   label gates. A matching HID device without a qualified endpoint is
   Unavailable for this sample. Do not fall back to `hidraw`, do not
   bind, unbind or load a driver, and do not write `new_id`.
3. Read `temp1_input`, `fan1_input` and `fan2_input` independently,
   between the clock reads of the resume rule in Quirks.
   A failed read, including `ENODATA`, makes only that channel
   unavailable for the sample. Never substitute zero for it.
4. Temperature: parse the text as a signed decimal integer. Divide by
   1000 for degrees Celsius. Admit −55 through 125 °C inclusive, the
   existing motherboard plausibility range (product policy under
   ADR 0011); omit values outside it and malformed text. The decode
   cannot produce a negative value (S3); the lower bound only keeps
   one policy across Linux temperature sources.
5. Fan and pump: parse the text as an unsigned decimal integer from 0
   through 65535. A value of 0 is an Inactive Fan Reading, a positive
   value is an Active Fan Reading. Malformed text or a value outside
   that range is an Invalid Fan Reading.
6. Apply the resume rule in Quirks before publishing any value of the
   sample.
7. Keep the three channel roles (coolant, fan, pump) fixed per device.
   A coolant reading at or above 60 °C is a real value and is never
   filtered; S6 describes that temperature as one the coolant should
   not reach.

Cadence and cost: the driver serves cached values that change at most
twice a second, and a read costs one sysfs file read without USB
traffic (S3, S5). Sampling at the application's existing interval
adds no device traffic; reading faster than twice a second returns
repeated values.

Attribution: each qualifying HID device is one cooler. Readings from
different coolers are never merged. The `hwmonN` number and the HID
sequence suffix change across reboots and re-plugs, so they identify
a device only within a session. When an endpoint disappears (unplug,
driver unbind), its channels are Unavailable until a qualified
endpoint is found again.

## Quirks

- Suspend does not age the cached values. The driver timestamps each
  status report in jiffies and returns `ENODATA` once two seconds of
  jiffies have passed without a new report (S3). The kernel advances
  jiffies from `CLOCK_MONOTONIC`, which stops during suspend (S14
  lead-only for the jiffies update; S14 primary for the clock). After
  a resume, the driver can therefore return the last values received
  before suspend until either a new report arrives or two more
  seconds of non-suspended time pass. **lead-only (copyleft)** for the
  driver and jiffies behavior.
- Resume rule. The difference `CLOCK_BOOTTIME` minus
  `CLOCK_MONOTONIC` grows only by the time the system spent suspended
  (S15). Each endpoint keeps a recorded difference and a hold start.
  For every sample of an endpoint:
  1. Read `CLOCK_MONOTONIC` immediately before the input reads (the
     sample start).
  2. Read the three inputs.
  3. Read `CLOCK_BOOTTIME` and `CLOCK_MONOTONIC` immediately after the
     input reads, and compute their difference.
  4. If the endpoint has no recorded difference, because it is
     observed for the first time (application start, hot plug or
     re-qualification), record the difference, set the hold start to
     the `CLOCK_MONOTONIC` value of step 3 and withhold the sample.
  5. Otherwise, if the difference exceeds the recorded one by more
     than 100 ms, a suspend happened after the previous sample, or
     during this one. Record the new difference, set the hold start to
     the `CLOCK_MONOTONIC` value of step 3 and withhold the sample.
  6. Otherwise record the difference. Withhold the sample if its start
     (step 1) is less than 3 seconds of `CLOCK_MONOTONIC` time after
     the hold start; publish it if not.

  A held endpoint's channels are Unavailable, not zero. A driver that
  is already bound when the endpoint is first observed may hold a
  report from before a suspend that happened before the application
  started; no sourced sysfs fact shows when a driver was bound, so the
  first-observation hold applies to every endpoint. Three seconds
  covers the driver's two-second window, the tick granularity of
  jiffies and the 0.5-second report period. After a hold, any value
  the driver still returns is younger than its window, so it arrived
  after the hold start, and step 5 excludes a suspend since then.
  The 100 ms threshold only absorbs the gap between the clock reads.
  The rule needs no driver internals.

## Windows HID transport (facts, scope Unsupported)

These facts are recorded for the follow-up. No Windows implementation
is admitted by this revision.

- Enumeration: obtain the HID interface class GUID with
  `HidD_GetHidGuid`, list present interfaces with `SetupDiGetClassDevs`
  using `DIGCF_PRESENT` and `DIGCF_DEVICEINTERFACE`, iterate with
  `SetupDiEnumDeviceInterfaces`, and obtain each `DevicePath` with
  `SetupDiGetDeviceInterfaceDetail` for `CreateFile`. (S11)
- Identity: `HidD_GetAttributes` fills `HIDD_ATTRIBUTES` with
  `VendorID`, `ProductID` and `VersionNumber`; the caller sets `Size`
  first. (S11) A handle opened with `dwDesiredAccess` zero can query
  device attributes without accessing the device. (S12)
- Sharing: with `dwShareMode` zero the device cannot be opened again
  until the handle closes. A request cannot ask for access that
  conflicts with the sharing mode of an existing open handle. A
  request whose sharing mode conflicts with the access of an existing
  open handle fails with `ERROR_SHARING_VIOLATION`. Sharing options
  stay in effect until the handle closes, regardless of process.
  (S12)
- Reading: `ReadFile` returns input reports that the HID class driver
  buffers in a ring buffer, 32 reports by default, adjustable with
  `HidD_SetNumInputBuffers` from 2 to 512. Reports are lost when the
  buffer overflows. The class driver manages input queues that support
  more than one open file on the same collection. (S11)
- Buffer size: `HidP_GetCaps` reports `InputReportByteLength`, the
  maximum input report size including the prepended report ID. (S11)
- `HidD_GetInputReport` is meant only for obtaining current state;
  some devices do not support it and become unresponsive when it is
  used (S11). This device does not answer Get_Report for its status
  report (S3, S5). It is never used.
- Coexistence: NZXT CAM needs exclusive access and loses its
  connection when a monitoring tool grabs the device (S10). liquidctl
  reports read errors when it monitors these coolers while CAM runs
  (S6).

## Safety notes

- The Linux path reads sysfs attributes only. It opens no `hidraw`
  node, issues no HID ioctl, loads, binds or unbinds no driver, writes
  no sysfs attribute and needs no elevation. With the driver's own
  behavior (S3), no host-to-device transaction occurs on this path at
  all.
- Never send an output report, feature report or Get_Report request to
  this device, on any platform.
- Linux coexistence: user-space tools keep using the `hidraw` node
  alongside the driver (S3). A tool that detaches the USB HID driver
  removes the endpoint, which this scope reports as Unavailable.
- Windows: do not open the device. S10 makes an open handle a
  possible cause of lost cooling control in CAM.

## Synthetic fixture acceptance

All fixtures are synthetic. The values of the first case are taken from
the S13 observation.

| Case | Expected result |
| --- | --- |
| Qualified endpoint past its first-observation hold, labels `Coolant`/`Fan`/`Pump`, inputs `32300`, `1473`, `2196` | Coolant 32.3 °C, fan 1473 RPM Active, pump 2196 RPM Active, attributed to one cooler. No other attribute is read and nothing is written. |
| Same endpoint with `fan1_input` `0` | Fan is an Inactive Fan Reading; coolant and pump unchanged. |
| Every input read fails with `ENODATA` | All three channels Unavailable for this sample; no zero is published; the device stays qualified. |
| Only `temp1_input` fails | Fan and pump preserved. |
| `fan2_input` malformed, negative or `65536` | Pump is an Invalid Fan Reading; siblings preserved. |
| `temp1_input` `125000`, then `125100` | Admit 125.0 °C, then omit the coolant channel. |
| hwmon `kraken2` whose HID parent is `0003:1E71:2007.0004` | Not this scope; no input reads. |
| HID device `0003:1E71:170E.000B` bound to another driver, unbound, or without a hwmon child | Unavailable; no `hidraw` access and no driver action. |
| `fan1_label` `Pump` and `fan2_label` `Fan` | Unavailable for that device; no input reads. |
| hwmon `device` link not resolving to the matching HID device, `/sys/class/hwmon/hwmonN` not resolving to the hwmon child under that HID device, or two hwmon children under it | Unavailable; no input reads. |
| Two qualified HID devices | Two independent sets of three channels; never merged. |
| Endpoint removed, then present again with a new `hwmonN` and HID sequence | Unavailable while absent; readings resume after re-qualification and the first-observation hold. |
| Endpoint observed for the first time (application start or hot plug) with all three inputs returning values | Sample withheld; channels Unavailable until a sample starts 3 s of `CLOCK_MONOTONIC` time after the first observation, then published. |
| `CLOCK_BOOTTIME` minus `CLOCK_MONOTONIC` grew by 600 s between the previous sample and this one, and the inputs still return the pre-suspend values | Sample withheld; channels Unavailable until a sample starts 3 s of `CLOCK_MONOTONIC` time after detection, then published again. |
| Endpoint published normally on the previous sample; the suspend happens between this sample's `CLOCK_MONOTONIC` start read and its input reads, so the difference read after the input reads grew by 600 s | That sample withheld although every input returned a value; the 3 s hold starts at the post-read clock values. |
| The difference grew by 20 ms | Treated as clock-read jitter; values published normally. |
| No `1E71:170E` HID device | Unsupported; no hwmon input reads for this scope. |

## Open questions

1. Non-blocking for #2527 Linux hwmon scope: only the Windows scope
   depends on it, and that scope stays Unsupported.
   CAM's open mode and its failure behavior are unknown. S10 says CAM
   needs exclusive access and loses its connection when another
   program grabs the device. Unknown: the sharing mode CAM requests,
   whether a read-only handle with full sharing held by another
   process causes the loss, and what pump and fan duty the cooler
   keeps after CAM loses its connection. Enabling Windows needs
   evidence that answers these, such as a handle listing of a running
   CAM and a controlled test, and a maintainer decision.
2. Non-blocking for #2527 Linux hwmon scope: Windows only.
   The Windows top-level collection layout is unknown: usage page and
   usage, the number of HID interfaces or collections, input report
   lengths, and whether a standard user can open the collection for
   reading. A report descriptor dump or `HidP_GetCaps` capture is
   needed.
3. Non-blocking for #2527 Linux hwmon scope: Windows only.
   S11 states that the HID class driver's input queues support more
   than one open file on a collection, but it does not state that each
   open handle receives every input report. If readers shared one
   queue, a second reader would take reports from CAM.
4. Non-blocking for #2527 Linux hwmon scope: the driver already
   presents the value in millidegrees and the application displays it
   as given.
   The coolant tenths byte has been observed only in 1–9, with some
   steps skipped for some whole degrees (S3). Whether its resolution is
   really 0.1 °C is unverified.
5. Non-blocking for #2527 Linux hwmon scope: the plausibility range
   omits an implausible temperature, and an empty endpoint is
   Unavailable.
   No fault encoding is known for this generation. The later
   generation reports a fault as `0xFF` in both temperature bytes
   (S7, S9); nothing shows whether this generation does the same.
6. Non-blocking for #2527 Linux hwmon scope: readings stay attributed
   to their source, so nothing is merged or double counted silently.
   The pump speed may also reach a motherboard fan header through the
   sense pin (S6). The same pump can then appear once here and once as
   a motherboard fan.
7. Non-blocking for #2527 Linux hwmon scope: presentation is an App
   and frontend modelling decision, not a hardware fact; the channel
   roles, units and attribution above are complete.
   The readings belong to a USB cooling device, not to the
   motherboard. Whether they reuse the motherboard temperature and
   fan-speed lists with a source label or introduce a cooling-device
   category, and how the pump is named, is decided in the
   implementation design review.
8. Non-blocking for #2527 Linux hwmon scope: the scope is live-only
   and stores no history, so session-local identity suffices.
   No stable per-device identity across reboots is established. No
   evidence shows that the device reports a USB serial number.
9. Non-blocking for #2527 Linux hwmon scope: the resume rule in Quirks
   withholds pre-suspend values, and a device that stops reporting
   produces `ENODATA`, which is handled as Unavailable.
   Whether the cooler resumes sending reports after suspend, or is
   reset and re-enumerated by the USB core, is not recorded. The
   driver defines no resume handling of its own (S3).

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-28 | Initial version. Propose Implementation-ready for the Experimental Linux `nzxt-kraken2` hwmon scope on USB `1E71:170E` only. Record the status report framing and Windows HID transport facts, keep Windows and `hidraw` access Unsupported because of the CAM exclusive-access finding, and give every NZXT generation named in #2527 a disposition. |
| 2 | 2026-11-28 | Record that the driver's two-second staleness window excludes suspended time (S3, S14) and add the resume rule based on `CLOCK_BOOTTIME` and `CLOCK_MONOTONIC` (S15), with fixtures. The rule reads the clocks after the input reads so a suspend during a sample withholds that sample, and holds every newly observed endpoint for 3 s because no sourced fact shows a fresh driver binding. The Linux scope row names the lead-only jiffies fact (S14). Make the class-link fixture match the ownership gate: the hwmon `device` link resolves to the HID device and the class link to the hwmon child. |
