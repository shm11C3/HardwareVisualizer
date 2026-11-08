# Spec: MSI PRO B850-P WIFI NCT6687D telemetry through Linux hwmon

| Field | Value |
| --- | --- |
| Revision | 2 |
| Status | Implementation-ready (rev 2) |
| Scope | Experimental, read-only temperature and fan sampling through an already bound Linux NCT6683-family hwmon driver on MSI PRO B850-P WIFI with the exact OEM profile below. |
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

## Detection

| Scope | Status | Default enablement |
| --- | --- | --- |
| MSI PRO B850-P WIFI (MS-7E56), BIOS 2.A93, already bound `nct6683.2592` with hwmon name `nct6687` | S1/S2/S3/S6 establish the named board and observed OEM firmware/controller tuple. Lead-only S5 establishes kernel device and driver names, the hwmon class link to the canonical child, the child's `device` link to the platform parent, resource ownership, firmware-customer acceptance, channel mapping and the cache refresh read set. Lead-only S7 corroborates the status addresses outside that set. Public ABI is S4. | Experimental for matching Linux endpoints. |
| Other boards, firmware versions, NCT6686D/NCT6683D and direct Windows access | Require separate exact evidence in [#2520](https://github.com/shm11C3/HardwareVisualizer/issues/2520). See [direct-access draft](superio-nuvoton-nct668x.md). | Unsupported by this revision. |

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

## Published attributes and sample gates

| Attribute | Meaning | Units and selection | Source |
| --- | --- | --- | --- |
| `temp2_input` with `temp2_label` = `Diode 0 (curr)` and `temp2_type` = `3` | First scoped temperature | Signed millidegrees Celsius, from OEM EC slot 1 | S2, S4, S5 (lead-only mapping) |
| `temp3_input` with `temp3_label` = `Thermistor 15` and `temp3_type` = `4` | Second scoped temperature | Signed millidegrees Celsius, from OEM EC slot 2 | S2, S4, S5 (lead-only mapping) |
| `temp4_input` with `temp4_label` = `Thermistor 1` and `temp4_type` = `4` | Third scoped temperature | Signed millidegrees Celsius, from OEM EC slot 3 | S2, S4, S5 (lead-only mapping) |
| `fan1_input` and `fan2_input` | Published fan inputs 1 and 2 | Integral RPM. The driver exposes each only if enabled. | S2, S4, S5 (lead-only visibility and encoding) |

S2 shows an AMD TSI source at EC slot 0, followed by assignment bytes
`0x82`, `0x89` and `0x8C`. S5 compacts the assigned temperature sources
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

## Open questions

- Non-blocking for the Linux hwmon slice: the published read-only ABI delegates port and selector handling to the already bound Linux driver.
  Direct Windows access, additional BIOS/EC builds and fan header
  labels require separate evidence.
- Non-blocking for the Linux hwmon slice: only positive RPM words inside the admitted range are exposed.
  Distinguishing a physically stopped fan from an unpopulated header
  requires separate evidence. The Linux ABI does not expose a separate
  per-word freshness or hardware validity bit for these channels.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-08 | Propose exact Experimental Linux sysfs coverage, separate from direct Windows access. |
| 2 | 2026-11-08 | Record the pinned MSI input-cache read set and selector-only writes during refresh. |
