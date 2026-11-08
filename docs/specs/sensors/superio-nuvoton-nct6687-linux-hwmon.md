# Spec: MSI NCT6687D telemetry through Linux hwmon

| Field | Value |
| --- | --- |
| Revision | 3 |
| Status | Implementation-ready (rev 3) |
| Scope | Experimental, read-only sampling through an already bound Linux NCT6683-family hwmon driver on exact MSI PRO B850-P WIFI (temperatures and fans) and MAG B850M MORTAR WIFI (SYS_FAN1 only) profiles. |
| Issue phase | [#2520](https://github.com/shm11C3/HardwareVisualizer/issues/2520) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | MSI, [PRO B850-P WIFI user guide](https://download.msi.com/archive/mnu_exe/mb/PROB850-PWIFI_English.pdf), Specifications, Fan Connectors and Regulatory Notices | Primary for the exact board, model number and Nuvoton NCT6687-R designation. |
| S2 | 3mdeb, [MSI OEM hardware capture](https://paste.dasharo.com/?ba0b5d09afbacfc1#6vRmb9aVC4cG4LVfGVEpD3KV6a5wKVCrui8wveLru5gT) linked in the [pinned article](https://github.com/3mdeb/news-and-ideas/blob/df347e0fd731df6832c19abaeb5dcc594e3b4dc7/blog/content/post/2026-02-09-msi_pro_b850p_part1.md), Super I/O initialization | Independent same-board capture of raw ID, base, firmware identity, assignments and sample values. |
| S3 | [First-hand Linux report](https://lkml.iu.edu/2610.0/07033.html), October 4, 2026 | The tester reports MSI PRO B850-P WIFI (MS-7E56), BIOS 2.A93, EC 0.0 build 2025-04-18, and in-kernel NCT6683-family monitoring on Linux 7.3-rc5 with patches for fan control. The separate fan control behavior is outside this scope. |
| S4 | Linux [hwmon sysfs interface, v7.3-rc5](https://github.com/torvalds/linux/blob/v7.3-rc5/Documentation/hwmon/sysfs-interface.rst) and [NCT6683 driver documentation, v7.3-rc5](https://github.com/torvalds/linux/blob/v7.3-rc5/Documentation/hwmon/nct6683.rst) | Primary for the public Linux ABI and units. The driver guide retains an older statement about Intel-only default support. The later driver source and exact MSI observation establish the applicable binding. |
| S5 | Linux [NCT6683 driver, v7.3-rc5](https://github.com/torvalds/linux/blob/v7.3-rc5/drivers/hwmon/nct6683.c), Git blob `e1c36c95affb201c9edee3e0eeb27a2ddc41f3cd`, [platform device implementation at the same tag](https://github.com/torvalds/linux/blob/v7.3-rc5/drivers/base/platform.c), [hwmon core](https://github.com/torvalds/linux/blob/v7.3-rc5/drivers/hwmon/hwmon.c), and [device core](https://github.com/torvalds/linux/blob/v7.3-rc5/drivers/base/core.c) | **lead-only (copyleft)** for chip-name and base association, sysfs child and parent links, matching firmware customer, assigned-channel indexing, cached readings, I/O region ownership and module override visibility. |
| S6 | [Windows SMBIOS board identity observation](https://xmrig.com/benchmark/5hgiH1) and MSI [B850 GAMING PLUS WIFI user guide](https://download.msi.com/archive/mnu_exe/mb/B850GAMINGPLUSWIFI_German.pdf), Regulatory Notices | Independent manufacturer and full product strings. MSI also uses MS-7E56 for another model. |
| S7 | Dasharo [NCT6687D register map at `713a0b76`](https://github.com/Dasharo/coreboot/blob/713a0b76/src/superio/nuvoton/nct6687d/nct6687d_hwm.h#L20-L42) | **lead-only (copyleft)** corroboration of the exact chip monitor and fan status/error addresses from alternative firmware. |
| S8 | MSI, [MAG B850M MORTAR WIFI user guide](https://download.msi.com/archive/mnu_exe/mb/MAGB850MMORTARWIFI_English.pdf), Specifications p. 20, Fan Connectors p. 48 and Regulatory Notices | Primary for the full board name, NCT6687D-R and the SYS_FAN1 header. |
| S9 | [Owner's system snapshot](https://taegyuhan.github.io/roadmap/hardware-os/mainboard), mainboard identity, and [owner-submitted CPU-Z validation](https://valid.x86.fr/3ayauw), motherboard section | Independent observations of the exact baseboard strings and BIOS 1.A68, dated 2026-09-07. These are separate from the Linux fan test. |
| S10 | Tomasz Bojanowski, [September 21 unforced Linux report](https://lists.openwall.net/linux-kernel/2026/09/21/3030), [November 6 fan test](https://lkml.iu.edu/2611.0/19127.html), and [November 6 customer/build report](https://lore-kernel.gnuweeb.org/linux-hwmon/4394d6d4-ea95-455c-a52e-a1d021afc2d4%40roeck-us.net/T/) | First-hand observations by the same tester of the exact board, BIOS, EC customer/build, unforced binding and the SYS_FAN1 RPM mapping. The exact fan sample was taken on a development driver. |
| S11 | Linux [NCT6683 driver at v7.4-rc1](https://github.com/torvalds/linux/blob/v7.4-rc1/drivers/hwmon/nct6683.c), Git blob `e1c36c95affb201c9edee3e0eeb27a2ddc41f3cd` | **lead-only (copyleft)** for the exact stock refresh, fixed fan channel mapping, chip-kind match, resource ownership and module override visibility. This is the same driver blob as S5. |
| S12 | Hamza Mahfooz, [October 4 Linux fan control patch series](https://lore.landau.one/lkml/af46087b-06d1-44b4-a2a6-f737d5ca247c@roeck-us.net/T/) | **lead-only (copyleft)** for the input-path differences between the stock and development drivers and for the fan-engine status address. No fan control is in scope. |

## Detection

| Scope | Status | Default enablement |
| --- | --- | --- |
| MSI PRO B850-P WIFI (MS-7E56), BIOS 2.A93, already bound `nct6683.2592` with hwmon name `nct6687` | S1/S2/S3/S6 establish the named board and observed OEM firmware/controller tuple. Lead-only S5 establishes kernel device and driver names, the hwmon class link to the canonical child, the child's `device` link to the platform parent, resource ownership, firmware-customer acceptance, channel mapping and the cache refresh read set. Lead-only S7 corroborates the status addresses outside that set. Public ABI is S4. | Experimental for matching Linux endpoints. |
| MSI MAG B850M MORTAR WIFI (MS-7E61), BIOS 1.A68, already bound native `nct6683` with hwmon name `nct6687` | S8/S9 identify the exact board, controller and BIOS. S10 records the EC customer/build, unforced driver and SYS_FAN1 sample. Lead-only S5/S11 establish the canonical sysfs child and parent links, platform instance association, chip-kind check, firmware-customer acceptance, ownership and override limits, fixed fan channel and stock cache refresh. Lead-only S7/S12 identify status addresses outside that refresh. Lead-only S12 also establishes that the development driver retains the stock RPM mapping. Public ABI is S4. | Experimental for SYS_FAN1 on the matching Linux endpoint. |
| Other boards, firmware versions, NCT6686D/NCT6683D and direct Windows access | Require separate exact evidence in [#2520](https://github.com/shm11C3/HardwareVisualizer/issues/2520). See [direct-access draft](superio-nuvoton-nct668x.md). | Unsupported by this revision. |

### MSI PRO B850-P WIFI

- Require DMI baseboard vendor `Micro-Star International Co., Ltd.`,
  board product `PRO B850-P WIFI (MS-7E56)`, and BIOS version `2.A93`.
  Read Linux DMI identity attributes `board_vendor`, `board_name` and
  `bios_version` under `/sys/class/dmi/id`. Compare full values after
  trimming line endings. Treat missing identity as unavailable.
- S2 observes raw chip `0xD592` at `0x4E`, EC Space base `0x0A20` and
  customer `0x020D`, firmware 0.0 build 2025-04-18. S3 ties that EC
  version to BIOS 2.A93. The Linux endpoint reports the `nct6687` kind,
  not the complete raw ID or EC firmware build. The exact BIOS and
  endpoint together select the evidenced profile.
- Require one distinct canonical hwmon node with `name` equal to
  `nct6687` and physical platform parent
  `/sys/devices/platform/nct6683.2592`. Its `subsystem` must resolve
  to `/sys/bus/platform` and `driver` to
  `/sys/bus/platform/drivers/nct6683`. The device identifier uses the
  decimal EC base `2592`. Resolve `/sys/class/hwmon/hwmonN` to the
  canonical child `/sys/devices/platform/nct6683.2592/hwmon/hwmonN`.
  Separately resolve the child's `device` link to the platform parent.
  Reject a name-only match, mismatched ancestry, an unbound node,
  duplicate distinct children or a changed binding. The `hwmonN` number is a session enumeration
  detail, so a new number alone does not change channel identity.
  (S4, S5)
- A driver can accept unknown customer IDs when explicitly forced
  (S5). The upstream parameter is not exposed in sysfs and the endpoint
  does not attest the raw ID or kernel binary. These checks establish
  identity within a trusted Linux sysfs and kernel, with the usual
  kernel configuration. They cannot distinguish a modified kernel or
  an administrator's hidden override. Treat any separately known
  forced or resource-conflict override as outside this profile.

### MSI MAG B850M MORTAR WIFI

- Require the full DMI baseboard vendor `Micro-Star International Co., Ltd.`,
  board product `MAG B850M MORTAR WIFI (MS-7E61)` and BIOS version
  `1.A68`. Use the same identity attributes, comparisons and failure
  handling as on the PRO profile. S8 and S9 establish the board identity.
  S10 identifies the observed firmware as customer `0x020D`, EC version
  0.0 build `03/10/25`. Require the full product string, including WIFI.
- Require one distinct canonical hwmon node with `name` equal to
  `nct6687` and physical platform parent
  `/sys/devices/platform/nct6683.<id>`. The upstream driver uses the
  firmware-discovered EC base, aligned to eight bytes, as the decimal
  platform instance ID (S5, S11). Accept a canonical unsigned decimal
  instance suffix from 8 through 65528 divisible by 8. Use the suffix only to
  validate and identify the sysfs instance. This profile has no fixed base address.
- The class link `/sys/class/hwmon/hwmonN` must resolve to
  `/sys/devices/platform/nct6683.<id>/hwmon/hwmonN`. The child's
  `device` link must resolve to that platform parent. Require the
  parent's `subsystem` and `driver` links to resolve to
  `/sys/bus/platform` and `/sys/bus/platform/drivers/nct6683`
  respectively. The bus device link
  `/sys/bus/platform/devices/nct6683.<id>` must resolve to the same
  parent. Reject multiple distinct matching children, including across
  platform instances, before selecting by attributes. An enumeration
  number change by itself does not change channel identity. (S4, S5, S11)
- The same trusted stock kernel and ordinary ownership policy apply.
  Treat a separately known modified driver, forced binding or
  resource-conflict override as outside this scope. The native
  `nct6687` name represents the chip kind and the sysfs links establish
  its owner. The endpoint supplies no complete raw chip ID, EC build,
  kernel-binary attestation or visible upstream force parameter.
  (S5, S8, S10, S11)

## Published attributes and sample gates

| Attribute | Meaning | Units and selection | Source |
| --- | --- | --- | --- |
| `temp2_input` with `temp2_label` = `Diode 0 (curr)` and `temp2_type` = `3` | First scoped temperature | Signed millidegrees Celsius, from OEM EC slot 1 | S2, S4, S5 (lead-only mapping) |
| `temp3_input` with `temp3_label` = `Thermistor 15` and `temp3_type` = `4` | Second scoped temperature | Signed millidegrees Celsius, from OEM EC slot 2 | S2, S4, S5 (lead-only mapping) |
| `temp4_input` with `temp4_label` = `Thermistor 1` and `temp4_type` = `4` | Third scoped temperature | Signed millidegrees Celsius, from OEM EC slot 3 | S2, S4, S5 (lead-only mapping) |
| `fan1_input` and `fan2_input` | PRO B850-P WIFI fan inputs 1 and 2 | Integral RPM. The driver exposes each only if enabled. | S2, S4, S5 (lead-only visibility and encoding) |
| `fan16_input` | MAG B850M MORTAR WIFI `SYS_FAN1` | Integral RPM. The driver exposes the fixed channel only if firmware enables it. | S8, S10, S4, S11/S12 (lead-only mapping and encoding) |

For PRO B850-P WIFI, S2 shows an AMD TSI source at EC slot 0, followed by
assignment bytes `0x82`, `0x89` and `0x8C`. S5 compacts the assigned temperature sources
in EC slot order and keeps fan indices tied to physical inputs. Check
each temperature channel's label and type on the same bound node and
skip only that channel on a mismatch. Keep generic labels tied to the
scoped OS channel. The source labels identify signal kinds rather
than physical board connectors.

Accept complete decimal temperature values from −55,000 through
125,000 inclusive and convert from millidegrees to degrees Celsius.
Accept fan values from 1 through 65,534 RPM. A zero does not
distinguish a stopped fan from an unpopulated header in S2. The driver
exposes an unsigned direct RPM word (S5), so reject 65,535 as a
defensive project gate. Malformed, overflowing, inaccessible and
missing values stay unavailable. S2 shows 26/22/26 °C for the
temperature tuple and 518/0 RPM for these two fan slots. Those are
examples from the direct OEM capture. The equivalent sysfs values in
fixtures are synthetic.

On MAG B850M MORTAR WIFI, `fan16_input` retains its suffix regardless
of which other channels are enabled. In the pinned driver its visibility
uses configuration byte `0x01CF` bit 7 and it exposes the unsigned
direct RPM word at `0x015E`–`0x015F` (S11). S10 recorded 926 RPM
with the BIOS fan curve active before the control tests. Use the same
positive RPM gate and keep the label `SYS_FAN1` tied to this profile
and fixed channel. No other attributes are scoped on this board.

## Collection and safety

- Read only the scoped input and source metadata files under the
  already bound kernel device during requested motherboard collection.
  Open no raw hardware device and request no module load, binding,
  force option or resource-conflict override. The application performs
  no port access, sensor initialization or fan-control operations.
- S5 requests the Linux I/O region, checks ACPI resource conflicts
  before binding under the ordinary policy, and serializes device
  sample refreshes. The app relies on that ownership and the public
  Linux ABI. A kernel that declines to bind or cannot provide a value
  yields an unavailable result. The Windows selector protocol remains
  outside this Linux path.
- For firmware customer `0x020D`, the pinned [input cache refresh](https://github.com/torvalds/linux/blob/v7.3-rc5/drivers/hwmon/nct6683.c#L603-L655)
  reads sensor values, fan RPM, PWM readback and configured limits
  within `0x0100`–`0x0167` and `0x0330`–`0x03D7`. Its read set
  excludes monitor and fan status/error addresses within
  `0x0170`–`0x017F` (S5, S7). The refresh writes the page and index
  selectors for those reads and leaves EC data and host-event
  registers unwritten (S5).
- A MAG B850M MORTAR WIFI `fan16_input` read uses that entire shared
  stock refresh on a cold or expired cache (S11). It reads assigned
  voltage and temperature values at `0x0100`–`0x013F`, enabled
  fan RPM words at `0x0140`–`0x015F`, configured PWM readback
  at `0x0160`–`0x0167`, monitor limits within
  `0x0330`–`0x03AF`, and enabled fan minima at
  `0x03B8`–`0x03D7`. This is a bounded set whose subset depends on
  configured channels. It excludes the known status/error range
  `0x0170`–`0x017F`, the fan-engine status `0x0CF8` and the
  separate host-event port. It performs only the selector writes
  required for the reads. (S5, S7, S11, S12)
- The S10 development-driver result identifies the channel and its units.
  The S12 patches leave the channel's visibility, RPM addresses and
  decoding unchanged. They add duty readback at `0x0E00`–`0x0E05`
  for enabled system fan channels and separate PWM mode and control
  operations. The exact stock safety audit above applies to S11's
  sampling path. Loading, binding and driver initialization stay under
  the OS, independently of application sampling.
- Preserve successful sibling attributes and provider results. A
  missing or denied exact endpoint on an admitted board is unavailable.
  A known different board or firmware is unsupported. Reuse the
  motherboard collection models and avoid history or new settings.
  Recheck identity when the bound device changes and keep channel
  attribution within the same endpoint.

## Synthetic fixture acceptance

| Case | Expected result |
| --- | --- |
| Exact DMI and bound node, expected temperature metadata, inputs 26000/22000/26000 and 518/0 | Three temperatures in °C, fan input 1 active at 518 RPM, fan input 2 unavailable. |
| Same MS-7E56 on a different product, wrong BIOS, wrong hwmon name/base/driver, wrong subsystem, duplicate canonical children | Unsupported for known out-of-profile identity or unavailable for absent/ambiguous binding. No input reads on a mismatched endpoint. |
| The class link resolves to the platform parent or another device's hwmon child, or the correct child's `device` link resolves elsewhere | Reject the endpoint before reading inputs. |
| Missing node, missing DMI or driver, unreadable channel, missing type, bad label, malformed or overflowing integer | Unavailable at the affected scope. Keep independently valid readings. |
| Temperature −55000, 125000, −55001 and 125001; fan −1, 0, 1, 65534 and 65535 | Accept only the stated inclusive bounds. Negative or invalid fan values never become zero RPM. |
| Transient hwmon enumeration index change or endpoint replacement | Identity uses the validated platform parent and channel. A read from a replaced endpoint is discarded until it independently qualifies. |
| Exact MORTAR DMI and a unique correct bound instance, `fan16_input` = 926, with any other fan or temperature attributes present or absent | Report only `SYS_FAN1` at 926 RPM. Use a synthetic valid instance ID and repeat with another valid ID. Neither a fixed base nor earlier fan files are required. |
| MORTAR without WIFI, wrong BIOS, wrong kind or driver, noncanonical or impossible instance suffix, duplicate matching children even if only one exposes `fan16_input` | No input read for the rejected or ambiguous profile. Check the platform bus link as well as the hwmon class and device links. |
| Exact MORTAR profile with missing, denied, malformed or overflowing `fan16_input`, or value −1, 0 or 65535 | The MORTAR reading stays unavailable and independent provider results remain available. Accept inclusive boundary values 1 and 65534. |
| MORTAR binding changes while collecting, or a forced/override binding is known | Discard the affected reading and require a qualifying binding before sampling again. A known override disqualifies it. |

## Open questions

- Non-blocking for the Linux hwmon slice: the published read-only ABI delegates port and selector handling to the already bound Linux driver.
  Direct Windows access, additional BIOS/EC builds and fan header
  labels require separate evidence.
- Non-blocking for the Linux hwmon slice: only positive RPM words inside the admitted range are exposed.
  Distinguishing a physically stopped fan from an unpopulated header
  requires separate evidence. The Linux ABI does not expose a separate
  per-word freshness or hardware validity bit for these channels.

- Non-blocking for the MAG B850M MORTAR WIFI fan slice: the positive exact-board sample and unchanged RPM path establish SYS_FAN1 for Experimental use.
  A stock-driver sample with full sysfs paths and independently observed
  raw controller and firmware IDs remains useful for verification. The
  published reports do not supply the exact temperature metadata and
  source assignments required for a separate temperature scope.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-08 | Propose exact Experimental Linux sysfs coverage, separate from direct Windows access. |
| 2 | 2026-11-08 | Record the pinned MSI input-cache read set and selector-only writes during refresh. |
| 3 | 2026-11-08 | Propose Implementation-ready Experimental SYS_FAN1 coverage on exact MAG B850M MORTAR WIFI BIOS 1.A68 through its native Linux hwmon endpoint. |
