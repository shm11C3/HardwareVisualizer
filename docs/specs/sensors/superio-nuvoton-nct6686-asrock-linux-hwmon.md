# Spec: ASRock NCT6686D telemetry through Linux hwmon

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental, read-only sampling of two thermistor-source temperatures and fan inputs 1–6 through an already bound Linux NCT6683-family hwmon driver on ASRock B650 Steel Legend WiFi. Board identity selects the OEM EC channel assignment and the firmware customer whose input refresh is audited below. BIOS version, EC build and EC base address are evidence identities. Excludes the AMD TSI source, voltages, limits, PWM, intrusion and beep attributes, other ASRock or AMD NCT6686D boards, NCT6683D boards and direct Windows access. |
| Issue phase | [#2696](https://github.com/shm11C3/HardwareVisualizer/issues/2696) |

This profile follows the bound-endpoint pattern of the
[MSI NCT6687D Linux hwmon specification](superio-nuvoton-nct6687-linux-hwmon.md).
The application consumes an endpoint that the kernel already owns and
performs no port I/O. The [NCT668x research draft](superio-nuvoton-nct668x.md)
records the dispositions of the other #2696 targets.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | ASRock, [B650 Steel Legend WiFi user manual](https://download.asrock.com/Manual/B650%20Steel%20Legend%20WiFi.pdf), Version 1.0, December 2023, §1.2 Specifications (Connector) p. 5 and §1.5 Block Diagram p. 10. File SHA-256 `57f578ab30caa17cdaac224f4b6c95b2e06e6ce3eb7b5a4a4acbfc6afc2c7bb1`. | Primary for the board, the NCT6686D Super I/O and the six fan headers `CPU_FAN1`, `CPU_FAN2/WP` and `CHA_FAN1/WP`–`CHA_FAN4/WP`. |
| S2 | Nuvoton, [NCT6686D Hardware Datasheet v0.5](https://www.nuvoton.com/export/resource-files/NCT6686D_HW_Datasheet_V0_5.pdf), April 21, 2017, §7.1–7.3 pp. 56–58, §8 p. 59 and CR 20h/21h | Primary for the EC Space host interface: one port set for BIOS/ACPI and a second for application software, each with page, index and data registers, plus a separate host interface event register. Monitoring is implemented by firmware. The datasheet's chip ID `0xC73A` differs from the `0xD441` observed on this board (S7), so this profile makes no raw-ID claim. |
| S3 | Linux [hwmon class ABI at v7.3](https://github.com/torvalds/linux/blob/v7.3/Documentation/ABI/testing/sysfs-class-hwmon) (`fanY_input`, `tempY_input`, `tempY_type`) | Primary for the public sysfs ABI: RPM, millidegrees Celsius, and type 4 = thermistor, 5 = AMD AMDSI. |
| S4 | Linux [NCT6683-family driver at v7.3](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/nct6683.c), Git blob `e1c36c95affb201c9edee3e0eeb27a2ddc41f3cd` (file SHA-256 `b328bfdec8adff4cba11f9367bf163ce077057d2e7b8b209ac0a8f6c2e231c7d`), with the [platform device](https://github.com/torvalds/linux/blob/v7.3/drivers/base/platform.c), [hwmon core](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/hwmon.c) and [device core](https://github.com/torvalds/linux/blob/v7.3/drivers/base/core.c) at the same tag | **lead-only (copyleft)** for the chip-kind name, platform instance and sysfs links, customer admission, I/O resource ownership, the source label and type mapping, temperature compaction, attribute visibility by firmware branch, the shared input refresh and its read set. The same blob is pinned by the MSI specification. Upstream v7.4-rc4 changes only allocation helpers and a big-endian PWM index fix in this file. |
| S5 | Linux [commit `cf85760f6a0a`](https://github.com/torvalds/linux/commit/cf85760f6a0a656d06d9e2851fc82baf1240262a), first in v6.14 ([driver blob `416ac02e9f7464e6b3b6c9c63a512a979bee96bf`](https://github.com/torvalds/linux/blob/v6.14/drivers/hwmon/nct6683.c)), and its [list reply](https://lkml.rescloud.iu.edu/2412.2/08939.html) | **lead-only (copyleft)** for the board association and default admission of customer `0x163E`, and the tested-board row NCT6686D EC 1.0 build 11/09/23. The v6.13 driver (blob `f71615e06a8fb98c78a4ad690007e79a5619db8a`) lacks the customer. Between v6.13, v6.14 and v7.3 the file changes only its admission list. |
| S6 | Linux [NCT6683 driver documentation at v7.3](https://github.com/torvalds/linux/blob/v7.3/Documentation/hwmon/nct6683.rst), Usage Note and Tested Boards | **lead-only (copyleft)** for the statement that Intel boards use a special firmware whose limit register locations differ from the Nuvoton datasheet, and for the B650 Steel Legend WiFi tested-board row. |
| S7 | [First-hand B650 Steel Legend WiFi report](https://github.com/Fred78290/nct6687d/issues/123), April 29, 2025, first post and the reporter's [order correction](https://github.com/Fred78290/nct6687d/issues/123#issuecomment-2841227359) | Same-unit observation of raw chip `0xD441` (reported through `pwmconfig`), configuration port `0x2E`, EC base `0x0A20`, EC 1.0 build 11/09/23 and a stock `nct6683` capture under `nct6686-isa-0a20`. The port, base and build lines come from the load log of an out-of-tree module in the same post. That module's readings are outside this profile. The report records neither kernel version nor force setting. |
| S8 | linux-hardware.org [probe `2b05ab93d0`](https://linux-hardware.org/?probe=2b05ab93d0), [sensors log](https://linux-hardware.org/?probe=2b05ab93d0&log=sensors) | Independent hardware observation on Debian kernel `6.12.38+deb13-amd64` with BIOS 3.20 (02/21/2025). The [upstream stable v6.12.38 driver](https://github.com/gregkh/linux/blob/v6.12.38/drivers/hwmon/nct6683.c) that this kernel packages is the v6.13 blob without customer `0x163E` (S5), so the binding required an administrator override. The temperature labels and sensor types are the driver's. The fan labels in this log come from a local configuration and carry no header evidence. |
| S9 | linux-hardware.org probes [`afac7d06ba`](https://linux-hardware.org/?probe=afac7d06ba&log=dmesg) (kernel log DMI line, BIOS 3.16, Linux 6.13.2, no NCT6683-family binding), [`ecb9403016`](https://linux-hardware.org/?probe=ecb9403016&log=dmidecode) (Base Board Information, BIOS 3.30) and [`df465ec7f5`](https://linux-hardware.org/?probe=df465ec7f5) (CachyOS 7.2.4, BIOS 4.41, September 12, 2026) | Independent exact DMI baseboard manufacturer `ASRock` and product `B650 Steel Legend WiFi`. Probe `df465ec7f5` runs a kernel that admits the customer, but the module is absent from its module list and kernel log, and no NCT6683-family endpoint appears in its sensors log. |
| S10 | lm-sensors [V3-6-0 feature ordering](https://github.com/lm-sensors/lm-sensors/blob/V3-6-0/lib/sysfs.c#L391-L525) and [text report formatting](https://github.com/lm-sensors/lm-sensors/blob/V3-6-0/prog/sensors/chips.c#L790-L823) | **lead-only (copyleft)** for reading the S7 and S8 text reports in sysfs channel order. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| DMI `board_vendor` is exactly `ASRock` and `board_name` is exactly `B650 Steel Legend WiFi` | mechanism: the board's OEM EC firmware defines the scoped channel assignment (thermistor sources on `temp1` and `temp2` ahead of the AMD TSI source, six enabled tachometer inputs) and carries the admitted non-Intel customer whose refresh is audited under Collection and safety | S1, S4, S5, S7–S9 |
| One distinct canonical hwmon node with `name` equal to `nct6686`, platform parent `/sys/devices/platform/nct6683.<id>` with a canonical decimal instance suffix from 8 through 65528 divisible by 8, `subsystem` resolving to `/sys/bus/platform`, `driver` to `/sys/bus/platform/drivers/nct6683`, the class link resolving to the parent's `hwmon/hwmonN` child, the child's `device` link resolving to the parent, and the bus device link resolving to the same parent | mechanism: hwmon chip kind (raw IDs matching `0xD440` under mask `0xFFF0`), owning driver and a unique bound endpoint | S3, S4 |
| On the same node, at least one of `temp1_min`, `temp1_max_hyst`, `temp2_min`, `temp2_max_hyst` and `fan1_min`–`fan6_min` exists. Check existence only. Never read them. | safety: the driver hides every one of these attributes on the Intel firmware branch, keyed only on the endpoint-wide firmware customer, so any one present attribute identifies the non-Intel refresh audited below. A missing attribute of an absent channel does not reject the endpoint. | S4, S6 |
| Per temperature channel, `tempN_label` and `tempN_type` equal the scoped source metadata on the same node | mechanism: the ABI's own channel metadata detects a changed assignment | S3, S4, S7, S8, S10 |
| Per input, the value is within the admitted bounds below | mechanism (plausibility) | S3, S4 |

BIOS versions 3.16, 3.20, 3.30 and 4.41, EC 1.0 build 11/09/23, raw chip
`0xD441` and EC base `0x0A20` (platform instance `2592`) are evidence
identities. They record where observations came from and do not gate
a read.

| Scope | Status | Default enablement |
| --- | --- | --- |
| ASRock B650 Steel Legend WiFi, already bound native `nct6683.<id>` with hwmon name `nct6686`: `temp1` (`Thermistor 14`), `temp2` (`Thermistor 15`) and fan inputs 1–6 | S1 establishes the board, the NCT6686D and six fan headers. S9 establishes the exact DMI strings on BIOS 3.16, 3.30 and 4.41. S7 records raw `0xD441` at `0x2E`, base `0x0A20`, EC 1.0 build 11/09/23 and stock readings on one unit. S8 records the same temperature labels, types and order on BIOS 3.20. Lead-only S5 admits customer `0x163E` for this board with the same EC build. The sampled units' numeric customer is unrecorded and is inferred from the build match and the visible non-Intel limit attributes. Lead-only S4 establishes the endpoint links, chip kind, label and type mapping, temperature compaction, attribute visibility, resource ownership and the input refresh read set. Lead-only S6 confines the known layout deviation to Intel firmware. Lead-only S10 corroborates channel order. Public ABI is S3. | Experimental for the scoped inputs on the matching Linux endpoint. |
| `temp3` (`AMD TSI Addr 98h`, type 5) on the same board | S7 and S8 show the source after the two thermistors. Type 5 (AMD AMDSI, S3) marks a processor sideband source, not a board sensor. Processor temperature is outside the motherboard profile. | Outside this profile. |
| Voltages, limits, fan minimums, PWM, `fanN_pulses`, intrusion and beep attributes | Not read by the application. | Out of scope. |
| Other ASRock or AMD NCT6686D boards, ASRock and Intel NCT6683D boards, and direct Windows access | Dispositions are recorded in the [NCT668x research draft](superio-nuvoton-nct668x.md) revision 5. | Unsupported by this revision. |

### Board and endpoint qualification

- Read Linux DMI identity attributes `board_vendor` and `board_name`
  under `/sys/class/dmi/id`. Compare full values after trimming line
  endings. Treat missing or unreadable identity as unavailable. The
  sibling `B650E Steel Legend WiFi` is a different product and is
  unsupported. `bios_version` is not read for admission (S9).
- The kernel masks the raw chip ID with `0xFFF0` and names `0xD440`
  matches `nct6686`. It uses the firmware-discovered EC base, aligned
  to eight bytes, as the decimal platform instance ID, so the observed
  base `0x0A20` gives `2592`. Accept any canonical suffix in the stated
  range. Resolve `/sys/class/hwmon/hwmonN` to the canonical child
  `/sys/devices/platform/nct6683.<id>/hwmon/hwmonN`. Separately resolve
  the child's `device` link to the platform parent, and the bus device
  link `/sys/bus/platform/devices/nct6683.<id>` to the same parent.
  Reject a name-only match, mismatched ancestry, an unbound node,
  duplicate distinct children or a changed binding. The `hwmonN` number
  is a session enumeration detail (S4).
- The module is not loaded automatically by chip discovery. S9's
  admitting CachyOS kernel exposes no endpoint without it. The
  application requests no module load, binding or override. A missing
  endpoint is unavailable (S4, S9).
- A driver can accept an unknown customer only when the administrator
  forces it. The force parameter is not exported in sysfs and the
  endpoint does not attest the raw ID, customer or kernel binary (S4).
  On kernels before the admission, such as the S8 Debian kernel, this
  board binds only with that override. The override changes admission
  only. The refresh below is the same for an admitted and a forced
  non-Intel customer (S4, S5). Treat a separately known override that
  admits a different firmware, a modified driver or a resource-conflict
  override as outside this profile.

## Published attributes and sample gates

| Attribute | Meaning | Units and selection | Source |
| --- | --- | --- | --- |
| `temp1_input` with `temp1_label` = `Thermistor 14` and `temp1_type` = `4` | First scoped board temperature | Signed millidegrees Celsius from the first assigned temperature slot | S3, S7, S8, S4/S10 (lead-only mapping and order) |
| `temp2_input` with `temp2_label` = `Thermistor 15` and `temp2_type` = `4` | Second scoped board temperature | Signed millidegrees Celsius from the second assigned temperature slot | S3, S7, S8, S4/S10 (lead-only mapping and order) |
| `fan1_input` through `fan6_input` | Fan inputs 1–6 | Integral RPM. The driver exposes each fixed suffix only if the firmware enables that tachometer input. | S1, S3, S7, S8, S4 (lead-only visibility and encoding) |

- The driver compacts assigned temperature sources in EC slot order
  and numbers them from 1. Source codes for the thermistor range report
  type 4 and the AMD TSI range type 5. Fan suffixes stay tied to the
  physical tachometer input index and do not shift when another input
  is disabled (S4).
- S7 reports `Thermistor 14` 40.5 °C, `Thermistor 15` 46.5 °C and fan
  inputs 1–6 at 759, 556, 537, 529, 448 and 381 RPM. S8 reports
  33.0 °C and 33.5 °C, and 1581, 0, 537, 0, 0 and 443 RPM. Both list
  `AMD TSI Addr 98h` third. These are observations, and fixture values
  derived from them are synthetic.
- Keep generic labels tied to the scoped OS channel, such as
  `Thermistor 14`, `Thermistor 15` and `Fan input N`. The source labels
  identify signal kinds, not physical locations. The manual names six
  headers (S1). The assignment of each header to a fan input is not
  established.
- The driver reports temperature as the signed EC word divided by 128
  with truncation, multiplied by 500, which gives half-degree steps
  (S4). Accept complete decimal values from −55,000 through 125,000
  inclusive and convert to degrees Celsius.
- The driver reports fan input as an unsigned direct RPM word (S4).
  Accept 1 through 65,534 RPM. Reject 65,535 as a defensive project
  gate. A zero does not distinguish a stopped fan from an unconnected
  header, so it is unavailable. Malformed, overflowing, inaccessible
  and missing values stay unavailable.

## Collection and safety

- Read only `temp1_input`, `temp1_label`, `temp1_type`, `temp2_input`,
  `temp2_label`, `temp2_type` and `fan1_input`–`fan6_input` under the
  qualified endpoint during requested motherboard collection. Check the
  existence of the safety-gate attributes without reading them. Open no
  raw hardware device. The application performs no port access, sensor
  initialization or fan-control operations.
- Under the ordinary policy, the kernel checks ACPI resource conflicts
  for the four application-port I/O addresses at EC base + 4 through
  + 7 before binding and requests that region when it binds (S4). S2
  assigns that port set to
  application software and the separate base + 0 through + 3 set to
  BIOS and ACPI. The application relies on the kernel's ownership and
  serialization. A kernel that declines to bind or cannot provide a
  value yields an unavailable result.
- Each scoped input, label and type read can trigger the driver's shared
  input refresh when its cache is unset or older than about one second.
  The refresh runs under the driver's update lock (S4).
- For a non-Intel customer, that refresh reads only these EC fields
  (S4):
  - each assigned voltage value byte with its low and high limit bytes
  - each assigned temperature word with its low, high, hysteresis and
    critical limit bytes
  - each enabled fan input's RPM word and minimum word
  - each enabled PWM output's readback byte
- All of those reads lie in `0x0100`–`0x0167` and `0x0330`–`0x03D7`.
  For each byte, the driver writes the application page selector
  `0xFF`, then the page and index selectors, and reads the data port.
  It writes no EC data and never accesses the host interface event
  register at base + 7 (S2, S4).
- The refresh excludes the EC addresses that the driver defines as
  monitor status, idle, fan status, fan error and fan initialization
  status at `0x0174`–`0x017F`. It also excludes the monitoring-enable
  byte `0x0180`, the channel configuration at `0x01A0`–`0x01D7`, the
  firmware identity at `0x0602`–`0x0609`, and the driver's fan
  configuration and PWM write addresses `0x0A01` and `0x0A28`–`0x0A2F`.
  Configuration and identity are read only during OS-owned driver
  initialization (S4).
- S6 attributes the known register-location difference to Intel
  firmware only. On this board, S7 and S8 show every exposed voltage
  limit, temperature limit and fan minimum as zero. No source assigns
  status, acknowledgement or event semantics to an address in the
  read set. The safety gate above confirms that the endpoint uses the
  non-Intel refresh.
- Driver initialization can enable the EC logical device and start
  monitoring if firmware left them disabled, and suspend and resume
  rewrite the monitoring-enable byte. These are OS-owned actions
  outside application sampling (S4).
- Preserve successful sibling attributes and provider results. A
  missing or denied endpoint on an admitted board is unavailable. A
  known different board is unsupported. Reuse the motherboard
  collection models and avoid history or new settings. Recheck identity
  when the bound device changes and keep channel attribution within the
  same endpoint.

## Synthetic fixture acceptance

| Case | Expected result |
| --- | --- |
| DMI `ASRock` / `B650 Steel Legend WiFi`, one qualifying endpoint on instance `2592`, safety-gate attributes present, `temp1` `Thermistor 14`/`4` = 40500, `temp2` `Thermistor 15`/`4` = 46500, fans 759/556/537/529/448/381 | Two temperatures at 40.5 °C and 46.5 °C and six generic fan inputs at the given RPM. |
| Same endpoint with `temp1` 33000, `temp2` 33500 and fans 1581/0/537/0/0/443 | Two temperatures, and fan inputs 1, 3 and 6 active. Fan inputs 2, 4 and 5 are unavailable, never 0 RPM. |
| Same profile with `bios_version` `3.16`, `4.41`, missing or any other string, or another canonical instance such as `nct6683.2600` | Admitted identically. |
| `B650E Steel Legend WiFi`, another ASRock product, a different vendor string, or missing DMI | Unsupported for a known different board, unavailable for missing identity. No input or metadata reads. |
| Wrong hwmon name (`nct6683` or `nct6687`), wrong driver or subsystem, noncanonical or impossible instance suffix, duplicate canonical children, or class or `device` links resolving elsewhere | Reject the endpoint before reading inputs or metadata. |
| None of `temp1_min`, `temp1_max_hyst`, `temp2_min`, `temp2_max_hyst` or `fan1_min`–`fan6_min` present | Reject the endpoint before reading inputs or metadata. The fixture verifies that none of these files is opened. |
| Only `temp1` channel files and `fan1_input`–`fan6_input` with their `fanN_min` present, `temp2` files all absent | Accept the endpoint. Report `Thermistor 14` and the valid fan inputs. `temp2` is unavailable. No limit file is opened. |
| No `temp1` or `temp2` files, `fan1_input`–`fan6_input` and their `fanN_min` present | Accept the endpoint and report the valid fan inputs. Both temperatures are unavailable. |
| `temp1` or `temp2` label or type different, missing or unreadable, or the thermistor labels moved to `temp2`/`temp3` | That temperature is unavailable. Never substitute another channel. Preserve the other temperature and fan inputs. |
| `temp3_input` present with `AMD TSI Addr 98h`/`5` | Not read and not reported. |
| Temperature −55000, 125000, −55001 and 125001; fan −1, 0, 1, 65534 and 65535; malformed or overflowing values | Accept only the stated inclusive bounds. Negative or invalid fan values never become zero RPM. |
| Some of `fan1_input`–`fan6_input` absent | Report the present valid inputs on their fixed suffixes without renumbering. |
| Binding changes or the endpoint is replaced during collection, or a forced binding of a different firmware is known | Discard affected readings and require a qualifying binding before sampling again. Preserve other providers. |

## Open questions

- Non-blocking for the Experimental B650 Steel Legend WiFi Linux hwmon slice: the full read set is bounded, outside every driver-defined status range, and the same refresh accepted for the MSI profiles.
  No primary or firmware-specific map for EC 1.0 build 11/09/23
  confirms the absence of read side effects in the limit and minimum
  fields. The audit relies on lead-only S4 and S6 and on the zero
  readbacks in S7 and S8. Verified status needs an EC Space map for
  this firmware or a maintainer-accepted capture showing those fields'
  semantics.
- Non-blocking for the Experimental slice: the admission commit and the stock readings establish the endpoint and channel map, and an override does not change the refresh.
  No published capture shows a native, unforced binding on a kernel
  with the customer admission. S7 leaves its kernel and force setting
  unrecorded, and S8 required an override. A
  [Hardware Compatibility Report](https://github.com/shm11C3/HardwareVisualizer/issues/2500)
  from an admitting kernel would corroborate the native path.
- Non-blocking for the Experimental slice: the build match and the visible non-Intel attributes identify the customer class.
  The numeric customer of the S7 and S8 units is unrecorded. A
  same-unit read of EC identity would corroborate it.
- Non-blocking for the Experimental slice: generic fan input labels need no header mapping.
  The header assigned to each fan input and the physical location of
  each thermistor source are not established. S8's fan names come from
  a local configuration file and are not evidence.
- Non-blocking for the Experimental slice: zero RPM is unavailable.
  Distinguishing a stopped fan from an unconnected header requires
  separate evidence.
- Non-blocking for the Experimental slice: a firmware build that changes the temperature assignment is detected by the per-channel label and type gate.
  The observations span BIOS 3.16 through 4.41 for identity, but the
  channel samples come from BIOS 3.20 and an unrecorded BIOS. A report
  of a changed fan-input assignment would move that build's fan inputs
  to Open questions.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-28 | Propose Implementation-ready Experimental coverage of `Thermistor 14`, `Thermistor 15` and fan inputs 1–6 on ASRock B650 Steel Legend WiFi through the already bound native Linux endpoint, pending maintainer sign-off. Audit the non-Intel input refresh and add a non-Intel branch safety gate. Before merge (PR #2794 review), the gate accepts any one present scoped non-Intel-only limit attribute, including `fanN_min`, so an absent temperature channel stays unavailable without rejecting the endpoint. |
