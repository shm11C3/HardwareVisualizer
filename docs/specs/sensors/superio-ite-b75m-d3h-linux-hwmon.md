# Linux IT8728 fan input on Gigabyte GA-B75M-D3H

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Draft — not implementation-ready |
| Scope | Linux OS-reported RPM for two observed fan inputs on the exact Gigabyte GA-B75M-D3H and its already bound `it87` hwmon endpoint |
| Issue phase | [#2518](https://github.com/shm11C3/HardwareVisualizer/issues/2518), Linux OS API candidate |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Gigabyte [GA-B75M-D3H product revision selector](https://www.gigabyte.com/Motherboard/GA-B75M-D3H-rev-12) and [GA-B75M-D3H user manual, 12ME-B75MD3H-1102R, rev. 1102](https://download1.gigabyte.com/Files/Manual/mb_manual_ga-b75m-d3h_v1.1_e.pdf), motherboard layout and hardware monitoring specification | Primary. Product selector lists PCB revisions 1.0, 1.1, and 1.2. |
| S2 | Gigabyte GA-B75M-D3H schematics, sheet 17, ITE 8728 LPC I/O, for [rev. 1.0](https://www.scribd.com/document/877244866/GA-B75M-D3H-R10), [rev. 1.1](https://studylib.net/doc/28210482/gigabyte-ga-b75m-d3h-rev.-1.1-sch), and [rev. 1.2](https://www.scribd.com/document/976494520/GA-B75M-D3H-R12) | Vendor-authored drawings, public mirrors. All three label the controller IT8728F/EX. Drawings describe intended board population. |
| S3 | Linuxportál, [firsthand Debian GA-B75M-D3H report](https://www.linuxportal.info/leirasok/hardver/hogyan-figyeljuk-szamitogepunk-hardverenek-homersekletet-a-nagy-hoseg-idejen), board identification, driver-detection transcript and `sensors` readings | Independent public observation, kernel 4.9.0-7-amd64. PCB revision and complete loaded-module parameter state are unrecorded. |
| S4 | Firsthand flashrom mailing list [GA-B75M-D3H F14 DMI dump](https://marc.info/?l=flashrom&m=136902255812829&w=2) and [F12 DMI dump](https://marc.info/?l=flashrom&m=136191069930229&w=1), baseboard entries | Independent public observations of firmware-reported strings. |
| S5 | Linux v6.18 [hwmon sysfs interface](https://docs.kernel.org/6.18/hwmon/sysfs-interface.html), introduction, global attributes, Fans and Alarms, and [hwmon public ABI](https://github.com/torvalds/linux/blob/v6.18/Documentation/ABI/testing/sysfs-class-hwmon#L207-L213), fan input | Upstream published interface. |
| S6 | Linux v6.18 [`it87` driver documentation](https://docs.kernel.org/6.18/hwmon/it87.html), Supported chips, Module Parameters, Hardware Interfaces and Description | Upstream published driver interface. |
| S7 | Linux v6.18 `it87` driver, [register identities](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L200-L204), [IT8728 classification](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L353-L361), [fan output domain](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L650-L654), [shared cache refresh](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L819-L935), [fan input](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L1233-L1250), [hwmon registration and platform identity](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L3320-L3449) and [module parameters](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L3555-L3568) | lead-only (copyleft). Endpoint identity, fan output domain and kernel-side refresh behavior. |
| S8 | Linux v6.18 [platform naming](https://github.com/torvalds/linux/blob/v6.18/drivers/base/platform.c#L618-L641), [hwmon class registration](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/hwmon.c#L843-L912), [class device links](https://github.com/torvalds/linux/blob/v6.18/drivers/base/core.c#L3215-L3242) and [driver link](https://github.com/torvalds/linux/blob/v6.18/drivers/base/dd.c#L406-L420) | lead-only (copyleft). Canonical path and class/parent/driver association. |
| S9 | Linux v6.18 [sysfs access rules](https://github.com/torvalds/linux/blob/v6.18/Documentation/admin-guide/sysfs-rules.rst), Devices are only devices and Classification by subsystem | Upstream published sysfs conventions. |
| S10 | Linux v6.18 [DMI identification](https://github.com/torvalds/linux/blob/v6.18/drivers/firmware/dmi-id.c), baseboard attributes and DMI class, and [module parameter sysfs exposure](https://github.com/torvalds/linux/blob/v6.18/kernel/params.c#L664-L688) and [built-in parameter exposure](https://github.com/torvalds/linux/blob/v6.18/kernel/params.c#L778-L801) | lead-only (copyleft). Public DMI attribute mapping and visibility of effective options. |
| S11 | ITE, [IT8728F preliminary specification V0.4.2](https://www.rom.by/files/it8728f_datasheet.pdf), E-version cover, §8.3.5, and §§9.5.2.2.1–9.5.2.2.4 (printed pp. 91–92), SHA-256 `5393224C1E497E0F5DED27CF8609D16D3857003F8235C5128E903A1438680C0B` as pinned by the existing Windows IT8728 temperature specification | Vendor-authored, public mirror. Shared revision nibble, battery update request and clear-on-read interrupt status. |

## Detection

| Scope | Status | Default enablement |
| --- | --- | --- |
| Linux, Gigabyte GA-B75M-D3H, exact already bound `it87.2608` platform endpoint, hwmon name `it8728`, FAN1–2 | Candidate. Board and observed readings S1–S4, public ABI S5–S6 and S9, kernel endpoint and output limits S7–S8, DMI mapping and override visibility S10. Default refresh clears hardware alarm status, and an optional kernel setting triggers an additional write (S7, S11). If the safety questions are resolved, the endpoint and output facts from S7–S8 and S10 restrict this row to Experimental. | Disabled |
| Other boards, chip names, addresses, driver bindings, channels, and Windows raw LPC/EC counter readback | Outside this candidate. | Disabled |

| Identity element | Exact criterion | Source |
| --- | --- | --- |
| DMI baseboard | `/sys/class/dmi/id/board_vendor` is `Gigabyte Technology Co., Ltd.` and `board_name` is `B75M-D3H`, with the trailing line terminator removed. The observed `board_version` of `x.x` carries no PCB revision identity. | S2–S4, S10 |
| Canonical platform parent | `/sys/bus/platform/devices/it87.2608` resolves to `/sys/devices/platform/it87.2608`. The decimal device instance 2608 is the observed base `0x0A30`. | S3, S7–S9 |
| Bound driver | The platform parent's `driver` link resolves to the platform driver directory `/sys/bus/platform/drivers/it87`. | S6–S9 |
| Canonical class child | Exactly one `/sys/class/hwmon/hwmonN` link, with decimal `N`, resolves to the canonical platform parent's `hwmon/hwmonN` child. The child's `device` link resolves back to that same parent. | S5, S7–S9 |
| Chip name | The class child's `name` is exactly `it8728` with the trailing line terminator removed. This is the chip identity reported by the bound kernel driver. | S3, S5–S7 |

The device instance, the links and the chip name are joint checks. The hwmon
index can change across registration and carries no channel identity.
An unrelated hwmon device with the same name or an unresolved, missing or
ambiguous class relation does not qualify. The independent board report
observes the same base and driver under an earlier kernel. The exact sysfs
topology in this candidate is the v6.18 lead-only evidence (S3, S5–S10).

## Interface map

| File in the canonical hwmon child | Physical index | Access | Units | Source |
| --- | --- | --- | --- | --- |
| `fan1_input` | 1 | Read-only | Integer RPM | S3, S5 |
| `fan2_input` | 2 | Read-only | Integer RPM | S3, S5 |

The endpoint report records FAN1 at 1674 RPM and FAN2 at 1157 RPM. It records
zero for FAN3–5. No FAN3–5 reading, temperature, voltage, alarm, limit or PWM
attribute is in scope (S3).

## Read procedure and decode

1. Check the board and the exact already bound endpoint before accessing any
   fan input. The safety prerequisite below applies before either input
   can be opened. An unknown, mismatched or ambiguous identity yields an
   unavailable sample with no fan-input reads (S3–S10).
2. Read only the two fixed attributes on the identified class child.
   The kernel reports whole-number RPM. No userspace register decode, scaling
   or channel-role mapping is required (S3, S5, S7).
3. Accept a single ASCII decimal integer from 1 through 675000 RPM, inclusive,
   optionally followed by a single newline. The upper endpoint is the v6.18
   IT8728 fan output bound (S7). Zero provides no positive evidence that a
   fan is connected or rotating. Omit a zero, the negative invalid sentinel,
   any other negative value, out-of-range value, malformed text, absent
   attribute, permission or I/O error, or an attribute that disappears. The
   other valid input remains usable (candidate output policy based on S3, S5
   and S7).
4. Keep the physical fan indices. If only FAN2 survives, return the existing
   motherboard fan model as Fan 2. Valid readings are Active and carry the
   kernel RPM. Empty, invalid and unavailable data never synthesize a numeric
   zero or overwrite an available sibling. A sample with no usable channel
   supplies no Linux motherboard fan readings (candidate output policy).

## Quirks

- In the observed report the kernel is configured to load `it87` normally
  after separate IT8728F detection. The transcript shows no forced ID in
  the described steps (S3). The application does not reproduce those steps.
- The `it87` interface allows an administrator to force a chip ID or bypass
  ACPI resource conflict checks. The driver name and chip name do not attest
  that these overrides were absent, and do not establish package revision
  (S6–S7, S10–S11).
- A fan-input read can refresh the driver's shared cache even with default
  options. The v6.18 refresh reads all three EC interrupt-status registers.
  ITE documents clear-on-read status for the fan, voltage and temperature
  limit latches on the EX chip. The case-open latch has a separate
  write-then-read clear sequence. Reading just FAN1 or FAN2 therefore can
  consume unrelated pending status without an application write or alarm-file
  access (S7 and S11).
- The published default for `update_vbat` is off. With this option enabled
  by an administrator, each cache refresh sets the EC configuration register's
  battery-reading update bit. This is a one-shot battery measurement request.
  The bit clears after the update, and the just-returned value may be from the
  previous measurement. There is no persistent battery mode change in this
  path. Avoiding the voltage input alone does not prevent the effect. The
  standard v6.18 driver does not export the effective
  `update_vbat`, `force_id`, `ignore_resource_conflict` or
  `fix_pwm_polarity` settings as readable sysfs attributes (S6–S7 and S10–S11).

## Safety notes

- The application candidate reads DMI metadata, device links, the chip name
  and, only when the safety prerequisite is met, the two RPM attributes.
  It performs no configuration-mode entry, direct LPC/ISA/EC or SMBus I/O,
  sysfs write, driver load, bind, module parameter change, external monitoring
  command, elevation or ACPI override. It leaves arbitration and transaction
  ownership to the existing kernel binding (candidate access policy based on S5–S10).
- The current evidence establishes that a read of the default kernel
  endpoint can consume hardware interrupt status. Use of this endpoint
  remains blocked pending a maintainer decision on kernel-owned read
  effects or evidence for an endpoint that isolates the RPM read. The
  effective `update_vbat` state poses a separate question. Missing parameter
  files, module defaults, boot command-line text, static configuration files,
  and the endpoint name cannot establish the effective state of a loadable
  module (S6–S7, S10–S11).
- Do not use a forced ID or a resource-conflict override as a means of
  satisfying the profile. Driver initialization is outside the access
  contract. Windows access and temperature authorization are independent
  (candidate access policy based on S6–S7 and S11).

## Fixture cases

| Case | Expected result |
| --- | --- |
| Exact board strings and links, approved or isolated hardware read effects, safe effective options, and inputs `1674\n` and `1157\n` | Candidate contract returns Fan 1 at 1674 RPM and Fan 2 at 1157 RPM. Test input only until the safety boundary can be established. |
| Exact stock v6.18 default endpoint with clear-on-read status refresh | Disabled pending disposition of the default kernel hardware read effect. Neither fan attribute is opened. |
| Otherwise exact with effective `update_vbat` on or unknown and stock parameter attributes absent | The option safety prerequisite is unresolved. Neither fan attribute is opened. |
| Board mismatch, absent DMI, wrong platform instance, wrong parent driver, wrong class path or device link, name mismatch, or duplicate coherent class children | Unsupported or unavailable as appropriate. Neither fan attribute is opened. |
| Identity matches and safety prerequisite is met. FAN2 is valid and FAN1 is missing, denied, invalid or zero. | Only Fan 2 with its original physical index. |
| Safety prerequisite met with `1` and `675000\n` | Both numeric endpoints accepted. |
| Safety prerequisite met with values `0`, `-1`, `675001`, integer overflow, empty, multiple lines, or nondecimal text | No reading from the affected input. A valid sibling survives. |
| Safety prerequisite met, FAN3 has positive RPM, and FAN1–2 have no usable data | No fan readings. FAN3 is never opened. |
| Device links change or endpoint disappears during sampling | Do not carry the old identity into a replacement. A failed individual file read does not hide a sibling still on the identified device. |

## Open questions

- **Default kernel alarm status read.** A stock v6.18 fan input read can
  trigger the clear-on-read EC status accesses even with `update_vbat`
  disabled. Resolve the ownership and policy of that hardware effect
  with an explicit maintainer decision, or source an already bound kernel
  interface that supplies RPM without accessing the status latches.
  This blocks readiness.
- **Effective kernel options.** Establish how to exclude the optional
  one-shot battery measurement write and unsafe overrides for an already
  bound device, without loading or reconfiguring the driver. Stock v6.18
  sysfs omits the needed options. This blocks readiness for the proposed
  no-hardware-write scope.
- **Windows raw IT8728 fan counters.**
  Non-blocking for the Linux OS API candidate: Linux receives already converted RPM from the bound driver.
  The ITE E-version document maps DX and EX to the same revision nibble.
  The GA-B75M-D3H drawings support investigation of an exact-board EX
  profile, but the Linux observation does not establish Windows LPC/EC
  authorization or a directly observed raw counter transaction (S2–S4,
  S11). This remains deferred under #2518.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-08 | Record exact GA-B75M-D3H OS endpoint, two observed Linux RPM channels, and the kernel status-read and option safety gates. |
