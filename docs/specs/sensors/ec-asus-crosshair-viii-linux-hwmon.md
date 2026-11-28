# Spec: ASUS ROG CROSSHAIR VIII EC sensors through Linux hwmon

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental, read-only sampling of EC-only board readings through the already bound in-tree Linux `asus-ec-sensors` hwmon endpoint on ROG CROSSHAIR VIII HERO, ROG CROSSHAIR VIII HERO (WI-FI) and ROG CROSSHAIR VIII FORMULA. The scoped readings are the Chipset and VRM temperatures, the T_SENSOR, W_IN and W_OUT header temperatures, and the CPU_OPT and PCH_FAN fan speeds. Board identity selects the EC register map and firmware lock stated below. Excludes the EC CPU, Motherboard, CPU current and CPU core voltage readings, the W_FLOW flow meter, every other board, Windows access (see [`ec-asus-windows-transport.md`](ec-asus-windows-transport.md)) and fan control. |
| Issue phase | [#2524](https://github.com/shm11C3/HardwareVisualizer/issues/2524) |

This document follows the pattern of
[`superio-nuvoton-nct6687-linux-hwmon.md`](superio-nuvoton-nct6687-linux-hwmon.md).
The application reads only public sysfs attributes of a driver that the
kernel has already bound. It performs no port access and evaluates no
ACPI method. The kernel driver issues the EC transactions through the
kernel's own ACPI EC driver, which is the operating system's EC owner
in ACPI terms. The driver also holds the firmware's own mutex. This is
the arbitration that direct EC access from an application cannot
obtain.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | ASUS, [ROG CROSSHAIR VIII HERO user guide](https://dlcdnets.asus.com/pub/ASUS/mb/SocketAM4/ROG_CROSSHAIR_VIII_HERO/E15379_ROG_CROSSHAIR_VIII_HERO_UM_WEB.pdf), E15379, First Edition, May 2019. Specifications summary p. xiii (internal connectors). §1.1.10 Internal connectors, item 7 fan header table p. 1-21, items 8 and 9 p. 1-22, item 15 p. 1-27. | Primary for the physical header names and their purpose: `CPU_OPT`, `PCH_FAN` ("for connecting the PCH fan on your PCH cover"), `W_FLOW`, `W_IN`, `W_OUT`, `T_SENSOR` (separately purchased thermal sensor). |
| S2 | ASUS, [ROG CROSSHAIR VIII HERO (WI-FI) user guide](https://dlcdnets.asus.com/pub/ASUS/mb/SocketAM4/ROG_CROSSHAIR_VIII_HERO_WI-FI/E15378_ROG_CROSSHAIR_VIII_HERO_WI-FI_UM_WEB.pdf), E15378, First Edition, May 2019, specifications summary p. xiii. ASUS, [ROG CROSSHAIR VIII FORMULA user guide](https://dlcdnets.asus.com/pub/ASUS/mb/SocketAM4/ROG_CROSSHAIR_VIII_FORMULA/E15393_ROG_CROSSHAIR_VIII_FORMULA_UM_WEB.pdf), E15393, First Edition, May 2019, specifications summary p. xiii. | Primary. Both list the same `CPU_OPT`, `PCH_FAN`, `W_FLOW`, `W_IN`, `W_OUT` and `T_SENSOR` connectors as S1. |
| S3 | Linux Hardware Database, [probe `1608711aa0`](https://linux-hardware.org/?probe=1608711aa0), collected 2023-01-06 06:47 UTC: [dmidecode](https://linux-hardware.org/?log=dmidecode&probe=1608711aa0), [dmesg](https://linux-hardware.org/?log=dmesg&probe=1608711aa0), [lsmod](https://linux-hardware.org/?log=lsmod&probe=1608711aa0), [sensors](https://linux-hardware.org/?log=sensors&probe=1608711aa0) | Independent first-hand observation on the exact board. Base board manufacturer `ASUSTeK COMPUTER INC.`, product `ROG CROSSHAIR VIII HERO`, BIOS 4201 (04/26/2022). Debian kernel 6.0.12-1 (`6.0.0-6-amd64`) with the native `asus_ec_sensors` module loaded. Kernel log line from device `asus-ec-sensors asus-ec-sensors` reporting 12 EC sensors spanning 16 registers. ACPI EC at command/status `0x66` and data `0x62`. No concurrent-access or lock-failure warning in the roughly 18-minute log. Readings are listed under Published attributes. |
| S4 | Linux [hwmon sysfs interface, v7.3](https://github.com/torvalds/linux/blob/v7.3/Documentation/hwmon/sysfs-interface.rst) and [sysfs module ABI, v7.3](https://github.com/torvalds/linux/blob/v7.3/Documentation/ABI/testing/sysfs-module), `/sys/module/*/taint` entry | Primary for the public Linux ABI. Temperatures are in millidegrees Celsius, fan inputs are in RPM, and `tempN_label` and `fanN_label` carry channel labels. Taint flag `O` marks an out-of-tree module. |
| S5 | Linux [`asus-ec-sensors` driver, v7.3](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/asus-ec-sensors.c), Git blob `e43645e884fdb6cc86c6f79869b1e396ba20c82f`: [bank register and lock timeout](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/asus-ec-sensors.c#L41-L63), [blank values](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/asus-ec-sensors.c#L188-L200), [AMD 500-series register map](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/asus-ec-sensors.c#L252-L287), [CROSSHAIR VIII HERO sensor set and lock](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/asus-ec-sensors.c#L461-L470), [exact DMI match](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/asus-ec-sensors.c#L872-L920), [lock setup](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/asus-ec-sensors.c#L1173-L1203), [bank switching and block read](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/asus-ec-sensors.c#L1205-L1258), [decode, refresh and cache](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/asus-ec-sensors.c#L1260-L1347), [input, label and visibility callbacks](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/asus-ec-sensors.c#L1349-L1417), [probe and hwmon name](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/asus-ec-sensors.c#L1456-L1589), [platform device and module parameter](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/asus-ec-sensors.c#L1602-L1625). The read path is unchanged at v7.4-rc4 (blob `a3cb595e713b74c6a55d6667a34a2720c443aedf`; the diff only adds two other boards and changes allocation helpers). | **lead-only (copyleft)** for the board-to-map association, register addresses, sizes, signedness and byte order, lock selection and timeout, bank-switch writes, refresh read set, cache interval, blank values, label strings, channel visibility, device, driver and hwmon names, and parameter visibility. |
| S6 | Linux [driver block read at v7.1](https://github.com/torvalds/linux/blob/v7.1/drivers/hwmon/asus-ec-sensors.c#L1126-L1167), before the bank loop fix, and commits to S5: [`e741d13cc2ab`](https://github.com/torvalds/linux/commit/e741d13cc2ab) (bank loop fix, first in v7.2), [`60710b2af13b`](https://github.com/torvalds/linux/commit/60710b2af13b) (refresh interval fix, first in v7.2), [`71a6c1950fa5`](https://github.com/torvalds/linux/commit/71a6c1950fa5) (blank-value detection, first in v7.3), [`584d55be66ef`](https://github.com/torvalds/linux/commit/584d55be66ef) (lock timeout 500 to 800 ms, first in v6.18), [`2f66cb5bf352`](https://github.com/torvalds/linux/commit/2f66cb5bf352) (HERO (WI-FI), same sensors), [`5b4285c57b6f`](https://github.com/torvalds/linux/commit/5b4285c57b6f) (FORMULA joined to the HERO definition), [`d0ddfd241e57`](https://github.com/torvalds/linux/commit/d0ddfd241e57) (initial driver, first in v5.18: same data and source as the firmware WMI method, same mutex as the WMI code) | **lead-only (copyleft)** for version-dependent behavior and the member associations. First-release placement was determined with the GitHub compare API against tags v5.17, v5.18, v6.17, v6.18, v7.1, v7.2 and v7.3. |
| S7 | Linux [ACPI EC driver, v7.3](https://github.com/torvalds/linux/blob/v7.3/drivers/acpi/ec.c), Git blob `a89f10256dbb85768b86a2a970c75a7d8e3814fa`: [transaction serialization](https://github.com/torvalds/linux/blob/v7.3/drivers/acpi/ec.c#L818-L840), [exported byte read and write](https://github.com/torvalds/linux/blob/v7.3/drivers/acpi/ec.c#L910-L935), [AML operation-region handler](https://github.com/torvalds/linux/blob/v7.3/drivers/acpi/ec.c#L1358-L1385), [`_GLK` evaluation](https://github.com/torvalds/linux/blob/v7.3/drivers/acpi/ec.c#L1485-L1486) | **lead-only (copyleft)** for the fact that the driver's byte transactions and AML EC operation-region accesses share one kernel EC lock and take the ACPI Global Lock when firmware declares `_GLK`. |
| S8 | Linux v7.3 [platform device naming](https://github.com/torvalds/linux/blob/v7.3/drivers/base/platform.c#L787-L790) and [bundle registration without instance number](https://github.com/torvalds/linux/blob/v7.3/drivers/base/platform.c#L1117), [hwmon `name` attribute](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/hwmon.c#L74), [attribute read serialization](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/hwmon.c#L431-L470) and [hwmon parent assignment](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/hwmon.c#L985). Commit [`3ad2a7b9b15d`](https://github.com/torvalds/linux/commit/3ad2a7b9b15d), first in v6.18, adds the core attribute lock. | **lead-only (copyleft)** for the kernel-owned sysfs topology and the per-device read serialization. |
| S9 | UEFI Forum, [ACPI Specification 6.5](https://uefi.org/specs/ACPI/6.5/12_Embedded_Controller_Interface_Specification.html), Chapter 12: §12 introduction, §12.1, §12.2.1–§12.2.3, §12.3 Table 12.2, §12.3.1, §12.3.2, §12.7, §12.8 | Primary for the EC interface: 256-byte EC address space, read command `0x80` and write command `0x81` reserved for OSPM, interface ownership by OSPM or through the Global Lock, and the `_GLK` requirement for firmware that accesses the EC. |
| S10 | Eugene Shalygin (driver author), [comment in `zeule/asus-wmi-ec-sensors` issue #1](https://github.com/zeule/asus-wmi-ec-sensors/issues/1), CROSSHAIR VIII FORMULA report | First-hand. The author reports about 60 °C chipset temperature and about 2,000 RPM chipset fan on his own CROSSHAIR VIII HERO. He identifies 216 °C as the blank value in the FORMULA report from the earlier WMI-based reader. As an unsigned byte, 216 is `0xD8`, which is −40 as a signed byte. |
| S11 | `zeule/asus-ec-sensors` [issue #43](https://github.com/zeule/asus-ec-sensors/issues/43) and [issue #60](https://github.com/zeule/asus-ec-sensors/issues/60) (concurrent-access warnings on ROG CROSSHAIR X670E HERO and ProArt X670E-CREATOR WIFI), [issue #3](https://github.com/zeule/asus-ec-sensors/issues/3) (VRM reading constant 0 °C on PRIME X570-PRO and stuck at 20 °C on ROG STRIX X570-E GAMING), [issue #27](https://github.com/zeule/asus-ec-sensors/issues/27) (driver author: the driver does not call WMI functions; PRIME X470-PRO was added to avoid WMI lockups), [issue #91](https://github.com/zeule/asus-ec-sensors/issues/91) (open report of swapped VRM heatsink fan channels on Pro WS TRX50-SAGE WIFI) | First-hand reports used for the deferrals and the safety notes. Not normative for the scoped boards. |
| S12 | Linux Hardware Database, [probe `a599d9b486` sensors log](https://linux-hardware.org/?log=sensors&probe=a599d9b486), ASUS PRIME X570-PRO | Independent observation. The EC `CPU` and `Motherboard` values (37 and 48 °C) equal the same-sample NCT6798D `CPUTIN` and `SYSTIN` values. Used only to justify excluding those EC readings. |
| S13 | Linux [`asus_wmi_sensors` documentation, v7.3](https://github.com/torvalds/linux/blob/v7.3/Documentation/hwmon/asus_wmi_sensors.rst), Supported boards and Known Issues | **lead-only (copyleft)** for the firmware WMI polling hazard on older boards. Used only for a deferral. |
| S14 | `zeule/asus-ec-sensors` out-of-tree DKMS repository at commit [`db37d3a1141778aaedf1b4230f736f308c9047b8`](https://github.com/zeule/asus-ec-sensors/tree/db37d3a1141778aaedf1b4230f736f308c9047b8): `Kbuild` module object and the driver's platform driver and hwmon names | **lead-only (copyleft)** for the fact that an out-of-tree build loads as module `asus_ec_sensors` with the same platform driver name `asus-ec-sensors` and hwmon name `asusec` as the in-tree driver. |
| S15 | Linux v7.3 module sysfs: [loadable-module attribute list](https://github.com/torvalds/linux/blob/v7.3/kernel/module/main.c#L1085-L1101) and [its creation at load](https://github.com/torvalds/linux/blob/v7.3/kernel/module/sysfs.c#L364-L392), [built-in module directory creation](https://github.com/torvalds/linux/blob/v7.3/kernel/params.c#L751-L781), [parameter visibility for loadable](https://github.com/torvalds/linux/blob/v7.3/kernel/params.c#L700-L715) and [built-in](https://github.com/torvalds/linux/blob/v7.3/kernel/params.c#L817-L840) modules, [driver `module` link and built-in directory lookup](https://github.com/torvalds/linux/blob/v7.3/drivers/base/module.c#L33-L58), [module name passed by the platform bundle helper](https://github.com/torvalds/linux/blob/v7.3/include/linux/platform_device.h#L379-L380) and [stored on the driver](https://github.com/torvalds/linux/blob/v7.3/drivers/base/platform.c#L988-L993) | **lead-only (copyleft)** for how a loadable and a built-in `asus-ec-sensors` driver differ under `/sys/module`. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| DMI `board_vendor` is exactly `ASUSTeK COMPUTER INC.` and `board_name` is exactly `ROG CROSSHAIR VIII HERO`, `ROG CROSSHAIR VIII HERO (WI-FI)` or `ROG CROSSHAIR VIII FORMULA` | mechanism: the board identity selects the AMD 500-series EC register map, the sensor set and the firmware mutex `\AMW0.ASMX` stated in this document. The kernel driver binds the same map for exactly these strings. | S1, S2, S3, S5, S6 (lead-only association) |
| Exactly one canonical endpoint: platform device `/sys/devices/platform/asus-ec-sensors` with `subsystem` resolving to `/sys/bus/platform` and `driver` resolving to `/sys/bus/platform/drivers/asus-ec-sensors`, the bus link `/sys/bus/platform/devices/asus-ec-sensors` resolving to the same device, and exactly one `hwmon/hwmonN` child whose `name` is `asusec`, whose `device` link resolves to that platform device and which `/sys/class/hwmon/hwmonN` resolves to | mechanism: hwmon chip kind, owning in-tree driver and a unique bound endpoint | S3, S4, S5, S8 (lead-only topology) |
| `/sys/module/asus_ec_sensors` exists and the driver's `module` link resolves to it. Then either it has `initstate` and a readable `taint` without `O` (loadable in-tree module), or it has neither `initstate` nor `taint` (built-in driver) | safety: an out-of-tree build of a driver with the same names carries its own map and lock, which this document does not audit | S4, S14, S15 (lead-only module sysfs) |
| Per scoped channel, exactly one channel of the stated type carries the stated label on the same endpoint | mechanism: the ABI's own channel metadata identifies the register the value came from | S4, S5 (lead-only labels), S3 |
| Per input, the value passes the bounds and blank-value rules below | mechanism (plausibility) | S4, S5, S10, S11 |

The BIOS version, the kernel release and the `hwmonN` number are not
runtime gates. BIOS 4201 and Debian kernel 6.0.12-1 record where the
S3 observation came from.

| Scope | Status | Default enablement |
| --- | --- | --- |
| AMD 500-series ASUS EC map with firmware mutex `\AMW0.ASMX`, selected by DMI board names `ROG CROSSHAIR VIII HERO`, `ROG CROSSHAIR VIII HERO (WI-FI)` and `ROG CROSSHAIR VIII FORMULA`, read through the already bound in-tree `asus-ec-sensors` endpoint | Best-evidenced member: ROG CROSSHAIR VIII HERO. S3 is an independent same-board observation with BIOS 4201, the stock Debian 6.0.12-1 module, the device name, the 12-sensor/16-register map size and positive Chipset (58 °C), VRM (28 °C) and chipset fan (1829 RPM) readings. T_SENSOR, W_IN and W_OUT read −40 °C with nothing connected, matching the lead-only blank value. S10 corroborates the chipset readings on the driver author's own board. HERO (WI-FI) and FORMULA share the map by lead-only S6 commits. S1/S2 are primary for the physical header names. Lead-only facts this row depends on: board-to-map association, register addresses, sizes and encodings, lock path and timeout, bank-switch writes and refresh read set, blank values, labels and names (S5, S6); EC transaction serialization (S7); sysfs topology and read serialization (S8); out-of-tree builds sharing the names (S14); loadable versus built-in module sysfs (S15). No primary EC register documentation exists for this board. Accepted residual: on kernels before v7.2 a failed bank 1 switch can publish bank 0 bytes as `W_IN` and `W_OUT` (Quirks). | Experimental for the scoped readings on matching Linux endpoints. |
| Matching board with a missing, ambiguous, out-of-tree or replaced endpoint | The board matches, but the endpoint conditions cannot be established. | Unavailable. |
| The same `asusec` endpoint on any other DMI board | The kernel selects a different map, sensor set or lock. Deferred boards are listed below. | Unsupported by this revision. |
| The EC readings on Windows | See [`ec-asus-windows-transport.md`](ec-asus-windows-transport.md). | Unsupported by this revision. |

### Board identity

- Read `/sys/class/dmi/id/board_vendor` and
  `/sys/class/dmi/id/board_name`. Compare the full values after
  trimming one trailing line ending. Missing or unreadable identity is
  unavailable. `bios_version` is not read for admission.
- The kernel driver uses an exact match on the same two DMI fields,
  so it binds this map only for the three listed board names (S5).
  The application gate keeps the readings tied to the map and lock that
  this document audits. A future kernel could add another board with
  another map to the same endpoint name.

### Endpoint

- The driver registers its platform device without an instance number,
  so the device directory is exactly
  `/sys/devices/platform/asus-ec-sensors`. S3 shows the kernel naming
  the device `asus-ec-sensors` (S5, S8).
- The hwmon child is registered under that device with the chip name
  `asusec`. Resolve `/sys/class/hwmon/hwmonN` to
  `/sys/devices/platform/asus-ec-sensors/hwmon/hwmonN`. Separately
  resolve the child's `device` link to the platform device and the bus
  link to the same device. Reject a name-only match, mismatched
  ancestry, an unbound device, more than one child or a changed binding.
  A new `hwmonN` number alone does not change channel identity (S4, S5,
  S8).
- The `mutex_path` module parameter is registered without sysfs
  permissions, so an administrator's override of the firmware mutex is
  not visible (S5). The checks establish identity within a trusted
  Linux sysfs and kernel. They cannot detect a modified kernel or a
  hidden parameter override. Treat a separately known override as
  outside this profile.
- Out-of-tree builds of this driver are distributed under the same
  module, driver and hwmon names (S14). A loadable out-of-tree module
  sets taint flag `O` (S4). An out-of-tree build cannot be built in.
- Both build forms create `/sys/module/asus_ec_sensors`. A loadable
  module gets the directory and its `initstate`, `taint` and other
  module attributes when it loads. A built-in platform driver gets the
  directory when the driver registers, because the driver passes its
  module name. That directory carries none of the loadable-module
  attributes, so it has no `initstate` and no `taint`. Parameters
  appear there only if they have sysfs permissions, and `mutex_path`
  has none. In both forms the driver directory
  `/sys/bus/platform/drivers/asus-ec-sensors` has a `module` link to
  `/sys/module/asus_ec_sensors` (S15).
- Admit a loadable module when `initstate` exists and `taint` is
  readable and contains no `O`. Admit a built-in driver when neither
  `initstate` nor `taint` exists. Anything else is unavailable: a
  missing module directory or `module` link, `initstate` without a
  readable `taint`, `taint` without `initstate`, or `O` in `taint`.

## EC register map (facts)

All rows are lead-only (S5) for the AMD 500-series map that the kernel
binds for the three board names, restricted to the sensor set of this
board family. Bank 0 is selected when EC register `0xFF` holds 0; bank
1 when it holds 1. The EC address space is 256 bytes (S9 §12).

| Bank | Index | Size | Signal (driver label) | Encoding | Scope | Source |
| --- | --- | --- | --- | --- | --- | --- |
| any | `0xFF` | 1 | Bank select | Written by the driver to select banks 0 and 1, then restored | Selector only | S5 |
| 0 | `0x3A` | 1 | Chipset (PCH) temperature (`Chipset`) | Signed 8-bit, whole °C | Scoped | S5 |
| 0 | `0x3B` | 1 | CPU temperature (`CPU`) | Signed 8-bit, whole °C | Excluded | S5 |
| 0 | `0x3C` | 1 | Motherboard temperature (`Motherboard`) | Signed 8-bit, whole °C | Excluded | S5 |
| 0 | `0x3D` | 1 | T_SENSOR header (`T_Sensor`) | Signed 8-bit, whole °C | Scoped | S5 |
| 0 | `0x3E` | 1 | VRM temperature (`VRM`) | Signed 8-bit, whole °C | Scoped | S5 |
| 0 | `0xA2`–`0xA3` | 2 | CPU core voltage (`CPU Core`) | Unsigned big-endian, mV | Excluded | S5 |
| 0 | `0xB0`–`0xB1` | 2 | CPU_OPT fan (`CPU_Opt`) | Unsigned big-endian, RPM | Scoped | S5 |
| 0 | `0xB4`–`0xB5` | 2 | Chipset fan (`Chipset`) | Unsigned big-endian, RPM | Scoped | S5 |
| 0 | `0xBC`–`0xBD` | 2 | Water flow meter (`Water_Flow`) | Unsigned big-endian, reported as RPM | Excluded | S5 |
| 0 | `0xF4` | 1 | CPU current (`CPU`) | Unsigned 8-bit, whole A | Excluded | S5 |
| 1 | `0x00` | 1 | W_IN header (`Water_In`) | Signed 8-bit, whole °C | Scoped | S5 |
| 1 | `0x01` | 1 | W_OUT header (`Water_Out`) | Signed 8-bit, whole °C | Scoped | S5 |

These 12 sensors and 16 data registers match the count that S3's
kernel logged on the exact board.

## Published attributes

The kernel numbers channels of each type consecutively in a fixed
internal order. Locate each scoped channel by type and label on the
qualified endpoint, not by number (S5).

| Attribute pair | Required label | Physical label | Units in sysfs | Source |
| --- | --- | --- | --- | --- |
| `tempN_input` / `tempN_label` | `Chipset` | Chipset (PCH) | Signed m°C, whole-degree steps | S4, S5 (lead-only), S3 |
| `tempN_input` / `tempN_label` | `VRM` | VRM | Signed m°C, whole-degree steps | S4, S5 (lead-only), S3 |
| `tempN_input` / `tempN_label` | `T_Sensor` | `T_SENSOR` header | Signed m°C, whole-degree steps | S1, S2, S4, S5 (lead-only) |
| `tempN_input` / `tempN_label` | `Water_In` | `W_IN` header | Signed m°C, whole-degree steps | S1, S2, S4, S5 (lead-only) |
| `tempN_input` / `tempN_label` | `Water_Out` | `W_OUT` header | Signed m°C, whole-degree steps | S1, S2, S4, S5 (lead-only) |
| `fanN_input` / `fanN_label` | `CPU_Opt` | `CPU_OPT` header | Integral RPM | S1, S2, S4, S5 (lead-only) |
| `fanN_input` / `fanN_label` | `Chipset` | `PCH_FAN` header | Integral RPM | S1, S2, S4, S5 (lead-only), S3 |

The label `Chipset` occurs once as a temperature and once as a fan.
The type prefix of the attribute separates them. The driver labels the
fan as the chipset fan and the manuals describe `PCH_FAN` as the fan on
the PCH cover. S3 shows 1829 RPM.

S3 reports, in lm-sensors format: CPU Core 1.06 V, CPU_Opt 0 RPM,
Chipset 1829 RPM, Water_Flow 0 RPM, Chipset +58.0 °C, CPU +40.0 °C,
Motherboard +34.0 °C, T_Sensor −40.0 °C, VRM +28.0 °C, Water_In
−40.0 °C, Water_Out −40.0 °C, CPU 0.00 A. The equivalent sysfs values
in fixtures are synthetic.

## Read procedure and decode

1. Qualify board identity, endpoint and module form as above. Read the
   `tempN_label` and `fanN_label` files of the qualified child. The
   label callback returns a constant string and performs no EC access
   (S5). A missing, unreadable, duplicated or different label excludes
   only that channel.
2. Read the scoped `tempN_input` and `fanN_input` files of the same
   child in one pass. Do not read inputs of excluded channels.
3. A read error on an input, including the "no data" error that v7.3
   and later return for a blank environment header, makes that reading
   unavailable. Keep valid siblings (S5, S6).
4. Temperature: parse a complete signed decimal integer. Convert from
   millidegrees to degrees Celsius. Accept −55,000 through 125,000
   inclusive. Reject 0 on every scoped temperature, as a defensive
   project gate. The driver initializes its buffer to zero and keeps
   the previous byte after a failed register read (Quirks). S11 also
   reports a constant 0 °C from an absent sensor on another board.
5. Blank environment headers: on `T_Sensor`, `Water_In` and
   `Water_Out`, treat −40,000, −60,000 and −62,000 as unavailable
   (no probe connected). Kernels before v7.3 report these values instead
   of an error. S3 shows −40 °C on all three headers with nothing
   connected. S10 shows the same byte as 216 when decoded unsigned
   (S5, S6, S10).
6. Fan: parse a complete unsigned decimal integer. Accept 1 through
   65,534 RPM. A zero does not distinguish a stopped fan from an empty
   header; S3 shows 0 for the unused `CPU_OPT` header. Reject 65,535 as
   a defensive project gate. Negative, malformed, overflowing, missing
   and inaccessible values are unavailable.

## Kernel read path (audit)

A read of any input attribute can trigger one refresh of the whole
board map. The facts below are lead-only (S5, S6, S7, S8) unless noted.

- The refresh runs at most once per second per device. Within that
  interval inputs come from the driver's cache. From v6.18 the hwmon
  core serializes attribute reads on the device (S8).
- Before touching the EC, the driver acquires the AML mutex
  `\AMW0.ASMX` with an 800 ms timeout (500 ms before v6.18). The initial
  driver states that the firmware's WMI methods use this mutex when
  they access the EC (S6). On timeout the refresh fails and the read
  returns an I/O error. The driver accesses no EC register in that
  case.
- Each EC byte access goes through the kernel ACPI EC driver. Its
  transactions share one lock with the AML operation-region handler.
  It also acquires the ACPI Global Lock when the EC device declares
  `_GLK` (S7). The transactions are RD_EC `0x80` and WR_EC `0x81`,
  which ACPI reserves for OSPM (S9 §12.3.1, §12.3.2).
- Refresh sequence on v7.2 and later, for this map: read `0xFF`. If it
  is not 0, write 0 and log a concurrent-access warning. Read bank 0
  registers `0x3A`–`0x3E`, `0xA2`–`0xA3`, `0xB0`–`0xB1`, `0xB4`–`0xB5`,
  `0xBC`–`0xBD` and `0xF4`. Write 1 to `0xFF`. Read bank 1 registers
  `0x00` and `0x01`. Write the originally read bank value back to
  `0xFF`. Then release the mutex.
- On kernels before v7.2, the bank 0 pass also reads bank 0 registers
  `0x00` and `0x01`. The bank 1 pass then overwrites those buffer bytes
  with the bank 1 values, so the published values are the same (S6
  `e741d13cc2ab`). S3 ran this variant.
- The writes of `0xFF` are the bank selection the read requires. No
  other EC register is written. The driver issues no burst, query or
  event command. It exposes no write attribute for these channels (S5).

## Quirks

- The driver does not check the result of each register read. A failed
  read leaves the previous buffer byte, which is zero before the first
  successful refresh. A failed switch to bank 1 stops the bank loop.
  If restoring the bank then succeeds, the refresh still reports
  success. In both cases stale or zero bytes can be published without
  an error (S5, lead-only). The zero-rejection rule above covers the
  never-read case; staleness is not detectable through the ABI.
- Misattributed water bytes on kernels before v7.2: the bank 0 pass
  stores bank 0 bytes `0x00` and `0x01` in the buffer slots of
  `Water_In` and `Water_Out`. Normally the bank 1 pass overwrites
  them. If the write that selects bank 1 fails and the bank restore
  then succeeds, the refresh reports success and those bank 0 bytes
  are published as `Water_In` and `Water_Out`. These kernels also
  predate blank-value detection (v7.3). A bank 0 byte that is not 0,
  not a blank value and within −55 to 125 °C passes every rule in
  this document (S5; S6 v7.1 block read and `e741d13cc2ab`; lead-only).
  On v7.2 and later the same failure leaves the previous bank 1 bytes,
  which are stale but come from the right registers. The kernel logs
  the failed bank switch, but sysfs does not expose it.
- Decision for this revision: `W_IN` and `W_OUT` stay in scope with
  this as an accepted residual of the Experimental row. The kernel
  release is not a reliable gate, because stable and distribution
  kernels may carry the fix under an older version number. The
  failure needs a failed EC write followed by a successful one inside
  one refresh, under the firmware mutex. No such failure appears in S3.
  Excluding the two headers would also drop them on fixed kernels, and
  nothing in sysfs could restore them. A maintainer who prefers no
  residual can exclude `Water_In` and `Water_Out` from this row
  without changing any other fact.
- Before v6.18 the hwmon core did not serialize attribute reads, and
  the driver keeps no lock of its own around its cache. Concurrent
  readers could then see a mix of two refreshes. The EC access itself
  remains under the AML mutex (S5, S8, lead-only).
- Before v7.3, blank environment headers report −40, −60 or −62 °C as
  values. From v7.3 they return a "no data" read error (S6, lead-only).
- All channels of the board's sensor set are visible whether or not a
  header is populated (S5, lead-only).

## Collection and safety

- During requested motherboard collection, read only the scoped label
  and input files of the qualified endpoint. Do not open `/dev/port`,
  `/sys/kernel/debug/ec`, ACPI tables or any raw device. Do not load,
  bind or parameterize the module. The application performs no EC
  access and no fan control.
- Sample at intervals of at least one second. The first input read in
  a pass can wait up to the 800 ms lock timeout plus the EC
  transactions, so keep the read off latency-sensitive paths
  ([#1937](https://github.com/shm11C3/HardwareVisualizer/issues/1937)).
- Preserve successful sibling readings and other provider results. A
  missing or denied endpoint on an admitted board is unavailable. A
  known different board is unsupported. Reuse the motherboard
  collection models and add no history or settings.
- Duplicate attribution: the excluded EC `CPU` and `Motherboard`
  readings equal the Super I/O `CPUTIN` and `SYSTIN` readings on
  another X570 board (S12). The scoped readings have no other source in
  the current Linux providers.

## Synthetic fixture acceptance

| Case | Expected result |
| --- | --- |
| HERO DMI, canonical endpoint, loadable module with `initstate` and empty `taint`, labels as listed, inputs Chipset 58000, VRM 28000, T_Sensor −40000, Water_In −40000, Water_Out −40000, CPU_Opt 0, Chipset fan 1829 | Chipset 58 °C, VRM 28 °C, `PCH_FAN` 1829 RPM. `T_SENSOR`, `W_IN`, `W_OUT` and `CPU_OPT` unavailable. Excluded channels are not read. |
| Same with `T_Sensor` 31000, `Water_In` 27000, `Water_Out` 29000, `CPU_Opt` 1200 | All seven scoped readings available. |
| Same board, blank headers return a read error instead of a value (v7.3 behavior) | Those readings unavailable; siblings unchanged. |
| HERO (WI-FI) or FORMULA DMI with the same endpoint | Admitted identically. |
| BIOS version missing, `4201` or any other string; kernel release any string | Admission unchanged. |
| Other ASUS board name with an `asusec` endpoint, or a non-ASUS vendor string | Unsupported. No input read. |
| Hwmon name other than `asusec`, wrong driver or subsystem link, device with an instance suffix, class link to another device, two children | Endpoint rejected before any input read. |
| Module directory with `initstate` and `taint` `O` or `OE`, or with `initstate` and a missing or unreadable `taint` | Unavailable. No input read. |
| Module directory with neither `initstate` nor `taint`, driver `module` link resolving to it (built-in driver) | Admitted. |
| Module directory with `taint` but no `initstate`; module directory absent; driver `module` link missing or resolving elsewhere | Unavailable. No input read. |
| Label missing, unreadable, different, or the same label on two channels of one type | Only that reading unavailable. |
| Any scoped temperature at 0, −55001 or 125001 | Unavailable. Boundary values −55000 and 125000 are accepted. |
| `T_Sensor`, `Water_In` or `Water_Out` at −40000, −60000 or −62000 | Unavailable as a blank header. |
| `Chipset` or `VRM` at −40000 | Reported as −40 °C. The blank-value rule applies only to the three header channels. |
| Fan −1, 0, 65535, malformed or overflowing; fan 1 and 65534 | Boundary values 1 and 65534 accepted; the rest unavailable and never reported as 0 RPM. |
| Read returns an I/O error (lock timeout) | That reading unavailable; other providers unaffected. |
| Endpoint replaced or `hwmonN` renumbered during collection | Discard affected readings until the endpoint qualifies again. A renumbering alone does not change identity. |

## Deferred boards and sensors

| Item | Reason | Evidence that would unblock |
| --- | --- | --- |
| EC `CPU` and `CPU Package` temperatures | Duplicate the CPU temperature path. | Not planned. |
| EC `Motherboard` temperature | Equals the Super I/O `SYSTIN` value on PRIME X570-PRO (S12). A Linux Super I/O path would duplicate it. | Same-board evidence that it is a separate physical sensor. |
| `W_FLOW` flow meter | The driver reports pulse rate as RPM. Presenting it as a fan speed would mislabel a flow meter, and no flow-unit conversion is documented. | A primary or first-hand pulse-to-flow relationship and a product decision on a flow reading. |
| CPU core voltage and CPU current | Voltage readback is tracked in [#2534](https://github.com/shm11C3/HardwareVisualizer/issues/2534). The motherboard collection has no current kind. | Separate scope. |
| ROG CROSSHAIR VIII DARK HERO and IMPACT, ROG STRIX X570-E/-F/-I and B550-E/-I, PRIME X570-PRO, Pro WS X570-ACE, ProArt X570-CREATOR WIFI and B550-CREATOR, ROG STRIX X570-E GAMING WIFI II | Same EC map family with other sensor subsets. Each needs its own exact observation. S11 reports a constant 0 °C VRM on PRIME X570-PRO and a stuck 20 °C VRM on STRIX X570-E (since removed from the driver). | Per-board first-hand sysfs or `sensors` output with DMI identity, ideally with a connected probe. |
| ROG ZENITH II EXTREME and ALPHA | Different firmware mutex (`\_SB_.PCI0.SBRG.SIO1.MUT0`) and extra bank 1 sensors (S5). | Same as above. |
| AMD 400-series boards (PRIME X470-PRO, ROG STRIX X470-F/-I) | Different map. PRIME X470-PRO uses the ACPI Global Lock (S5). | Same as above. |
| AMD 600 and 800-series boards (X670E, B650E, X870E, X870, B850) | Other maps and locks (S5). S11 records concurrent-access warnings on two X670E boards, one fixed by a mutex-path correction and one after a firmware update. Their EC-only readings are VRM, T_SENSOR and CPU_OPT. The 800-series map reads bank 0 only, which avoids the bank-switch write to bank 1. | Per-board observation without concurrent-access warnings in the kernel log. |
| Intel 200 to 700-series boards in the driver | Other maps and locks (S5). | Per-board observation. |
| LGA1851 (Z890, B860) and any board not in the driver | No EC map is stated anywhere. A mechanism gate is never guessed (ADR 0031). | A map from a primary source or a labeled capture. |
| Pro WS TRX50-SAGE WIFI and WRX90E-SAGE SE | An open report says two fan channels are swapped on TRX50 (S11). | Resolution of the label conflict. |
| Older boards through the Linux `asus_wmi_sensors` WMI path (X370, X399, X470, B450) | The in-tree driver documentation warns that frequent polling of the firmware WMI method can stop fans or lock readings on some BIOS versions (S13). | Same-board soak evidence and a method-version rule. |

## Open questions

- Non-blocking for the Experimental Linux slice: the arbitration rests on the firmware mutex and the kernel EC driver, not on application code.
  No primary source or ACPI table capture for these boards shows that
  every firmware path that switches the EC bank holds `\AMW0.ASMX`.
  The driver restores the original bank and logs a warning when it
  finds a non-zero bank at the start of a refresh. S3 shows no such
  warning over about 18 minutes. A DSDT/SSDT capture from one of the
  three boards, showing the WMI EC methods acquire that mutex and no
  other AML bank switch omits it, would graduate this fact.
- Non-blocking for the Experimental Linux slice: the extra pre-v7.2 reads are RD_EC reads under the same lock, and S3 ran them without a reported fault.
  Kernels before v7.2 also read bank 0 registers `0x00` and `0x01`.
  ACPI defines RD_EC as returning the byte at the address (S9
  §12.3.1). The OEM meaning of those two bytes is not documented. The
  kernel release is not a reliable gate, because stable kernels may
  carry the fix. The application never triggers a read that a
  `sensors` invocation on the same kernel would not. The related
  misattribution of those bytes as water temperatures after a failed
  bank switch is an accepted residual (see Quirks).
- Non-blocking for the Experimental Linux slice: the scoped temperature headers share the decode of the positively observed Chipset and VRM channels.
  The published observations show only blank values for `T_SENSOR`,
  `W_IN` and `W_OUT`. A sample with a connected probe on one of the
  three boards would corroborate the addresses. A non-blank,
  implausible value is rejected by the bounds.
- Non-blocking for the Experimental Linux slice: HERO (WI-FI) and FORMULA follow the shared map under ADR 0031.
  Their association to the HERO map is lead-only (S6). S10 includes a
  FORMULA report from the earlier WMI reader with a chipset reading and
  the blank value. A current same-board sysfs capture would make either
  a best-evidenced member.
- Non-blocking for the Experimental Linux slice: graduation to Verified needs primary or maintainer-accepted evidence of the EC map.
  ASUS publishes no EC register documentation. A maintainer-accepted
  labeled EC capture or firmware table capture covering every
  lead-only fact of the row is required. A Hardware Compatibility
  Report ([#2500](https://github.com/shm11C3/HardwareVisualizer/issues/2500))
  corroborates a member but does not graduate it.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-28 | Initial version. Propose Implementation-ready status for Experimental Chipset and VRM temperatures, `T_SENSOR`, `W_IN` and `W_OUT` temperatures, and `CPU_OPT` and `PCH_FAN` RPM on ROG CROSSHAIR VIII HERO, HERO (WI-FI) and FORMULA through the bound in-tree Linux `asus-ec-sensors` endpoint, pending maintainer sign-off. Review amendments before merge (#2792): document that kernels before v7.2 can publish bank 0 bytes as `W_IN` and `W_OUT` after a failed bank switch, and keep the two headers in scope as an accepted Experimental residual; replace the module gate with a sourced loadable-versus-built-in check (S15), so a built-in in-tree driver is admitted and an out-of-tree module is still rejected. |
