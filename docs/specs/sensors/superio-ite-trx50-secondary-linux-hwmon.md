# Spec: Gigabyte TRX50 AI TOP secondary ITE telemetry through Linux hwmon

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental, read-only sampling through an already bound Linux `it87` hwmon device for the IT87952E secondary LPC controller on Gigabyte TRX50 AI TOP. |
| Issue phase | [#2519](https://github.com/shm11C3/HardwareVisualizer/issues/2519) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Gigabyte [TRX50 AI TOP specifications](https://www.gigabyte.com/Motherboard/TRX50-AI-TOP/sp?lan=en) and [User's Manual rev. 106](https://download.gigabyte.com/FileList/Manual/mb_manual_trx50-ai-top_106_e.pdf), §1-1 and connector descriptions | Primary for board product, ITE Super I/O and board connectors. The manual does not assign the LPC channels to particular headers. |
| S2 | [Board owner's report](https://github.com/frankcrawford/it87/issues/73) and [same owner's positive readings](https://github.com/frankcrawford/it87/issues/73#issuecomment-3592587595), November 2025 | Independent observations of exact DMI identity, a boot where secondary pre-key identity was absent, and another boot with both primary and secondary sensors. This system used Linux 6.14.0-29-generic and the external DKMS driver's reported build `v1.0-204-g60d9def.20251006`. |
| S3 | [Second owner of the same board, raw ID report](https://github.com/frankcrawford/it87/issues/73#issuecomment-5202206494), August 6, 2026 | Independent observation of primary raw `0x8689` and secondary raw `0x8695` at `0x4E` before a mode-entry key. The later dump operations are outside this scope. |
| S4 | Linux [hwmon sysfs interface](https://www.kernel.org/doc/html/v6.18/hwmon/sysfs-interface.html) and [IT87 documentation, v6.18](https://www.kernel.org/doc/html/v6.18/hwmon/it87.html) | Primary for the kernel hwmon units, sensor types, and IT87952E having up to three fans with 16-bit readback. |
| S5 | Linux [IT87 driver, v6.18](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c), Git blob `e233aafa8856c94c3fde3502732fcda2bd3bf130`, [platform device implementation at the same tag](https://github.com/torvalds/linux/blob/v6.18/drivers/base/platform.c) and [observed DKMS driver at `60d9def`](https://github.com/frankcrawford/it87/blob/60d9def/it87.c), Git blob `f74edf51cbb854cb002fc763291345d7df0e424777` | **lead-only (copyleft)** for mainline device naming, exact raw ID association, I/O resource ownership, forced-ID visibility and unavailable fan values. The mainline driver publishes this same device name and input ABI, and is separate from the DKMS build in S2. |
| S6 | [Super I/O access rev. 4](superio-access.md#slot-1-ite-exit-withhold-rule) and [Gigabyte secondary ITE draft](superio-ite-gigabyte-secondary.md) | Project's slot-1 safety rule and restrictions for the Windows direct LPC, ECIO and ISA bridge routes. |

## Detection

| Scope | Status | Default enablement |
| --- | --- | --- |
| Gigabyte TRX50 AI TOP and already bound platform device `it87.2656` with hwmon name `it87952` | S1/S2 pin the board, S2 observes the live secondary device and channels, and S3 observes its exact raw ID. Lead-only S5 associates the name with raw `0x8695`, defines platform identity and input representation, and documents when an input exists. S4 supplies the public Linux ABI. | Experimental for matching Linux endpoints. |
| Other Gigabyte boards, alternate chip IDs, direct Windows LPC, ECIO or ISA bridge paths | See [#2519](https://github.com/shm11C3/HardwareVisualizer/issues/2519) and S6. | Unsupported by this revision. |

- Require `/sys/class/dmi/id/board_vendor` equal to
  `Gigabyte Technology Co., Ltd.` and `board_name` equal to
  `TRX50 AI TOP`, trimming line endings. S2 reports those fields.
  Its observations omit BIOS and physical PCB revision. Gate on the
  exact observed OS board and controller identity with eligible
  channels, without assigning a firmware or pin-routing layout.
- Require one distinct canonical hwmon node with `name` equal to
  `it87952` and physical platform parent
  `/sys/devices/platform/it87.2656`. Its `subsystem` must resolve
  to `/sys/bus/platform` and `driver` to
  `/sys/bus/platform/drivers/it87`. Resolve the class link and its
  `device` association to that same parent. Linux forms the platform
  ID from EC base `0x0A60` in decimal. Reject a name-only match,
  another parent, an unbound node, duplicate distinct children or a
  changed binding. The `hwmonN` suffix does not identify a controller.
  (S4, S5)
- S2 saw `it8689-isa-0a40` as the separate primary. In the
  mainline platform naming scheme it would be `it87.2624` with
  hwmon name `it8689`. Keep its results and attribution independent.
  A primary with missing channels does not remove a valid secondary
  result. Never use readings from the primary path as secondary.
  (S2, S3, S5)
- Mainline can accept a forced chip ID and can ignore a reported
  ACPI conflict under an explicit override (S5). The candidate also
  requires the default `update_vbat` setting. In both driver sources,
  non-default `update_vbat` can set a voltage-sampling enable bit during
  a cache refresh initiated by reading a temperature or fan attribute.
  The exact IT87952E kind has no SMBus-reconfiguration mask. These
  module parameters are not exposed through sysfs (S5). Reject known
  non-default overrides. Canonical driver binding and DMI establish
  identity within the trusted running kernel and its ordinary driver
  configuration. The ABI does not attest a kernel binary or detect an
  administrator's hidden override.

## Published attributes and sample gates

| Attribute | Meaning | Units and selection | Source |
| --- | --- | --- | --- |
| `temp1_input`, `temp3_input` | Two published secondary temperatures observed in S2 | Signed millidegrees Celsius. Require matching `temp1_type` or `temp3_type` equal to `4` (thermistor). | S2, S4, S5 (lead-only visibility) |
| `fan1_input`, `fan2_input`, `fan3_input` | Three independently published secondary fan speeds | Integral RPM. Use only present attributes and retain their channel numbers. | S2, S4, S5 (lead-only visibility and representation) |

S2 reports 46 and 34 °C for temperature inputs 1 and 3 and 1406,
1486 and 2766 RPM for fan inputs 1–3. It also reports −55 °C
on temperature input 2 with unconfirmed physical meaning. Skip input
2, and reject −55 °C on the scoped inputs as a defensive project gate.
Accept temperatures above −55,000 and at or below 125,000 m°C.
S5 reports a zero 16-bit fan count as `-1` RPM and an all-ones
count as `0` RPM. Accept values from 1 through 675,000 RPM,
the theoretical maximum from the documented 16-bit count conversion
and fixed divisor of 2 (S4, S5). Classify `-1` as invalid and `0`
as unavailable for this profile. Missing, out-of-range, malformed
or denied values stay unavailable. Physical connector names are
unresolved, so use generic secondary channel labels.

## Collection and safety

- Read only the scoped kernel-owned input and source metadata files
  during requested motherboard collection. No raw I/O, port
  configuration keys, module loading, module binding, or conflict
  override is part of this route.
- S5 normally checks ACPI resource conflicts and requests the
  platform I/O region. The bound driver handles indexed reads.
  A driver that did not bind leaves the secondary collection
  unavailable. The application must never initiate probing to
  work around this case, including on a matching board.
- Keep valid siblings when a channel is absent or invalid. Leave
  existing providers' results independent. Recheck binding when the
  endpoint changes. A known different board is unsupported and an
  absent exact binding on an admitted board is unavailable. Reuse
  the existing motherboard collection models.

## Synthetic fixture acceptance

| Case | Expected result |
| --- | --- |
| Exact board and canonical secondary endpoint, thermistor types, input values 46000/34000 and 1406/1486/2766 | Temperatures 46/34 °C and the three fan readings, all attributed to the secondary controller. |
| Primary at `it87.2624` present beside the secondary, primary failed, or only primary present | Secondary readings never use primary values. Preserve successful independent results. A missing secondary is unavailable. |
| Different board, `it8792` with the same numeric base, different base, unbound parent, wrong subsystem/driver, duplicate children or wrong symlink target | No reads from an unqualified endpoint. Distinguish a known unsupported board from an unavailable binding on the target board. |
| Missing or denied attribute, malformed integer, unknown sensor type, or temp2 present | Skip only that channel. Temp2 is always outside this scope. |
| Temperatures −55000, −54000, 125000 and 126000; fans −1, 0, 1, 675000 and 675001 | Enforce the stated units and bounds. Invalid or unavailable fan readings never become zero RPM. |
| `hwmonN` renumbering or node replacement | Preserve identity if the same qualified parent and channel still exist. Discard data from a replaced node until independently qualified. |

## Open questions

- Non-blocking for the Linux hwmon slice: S2 demonstrates the endpoint with DKMS and S4/S5 independently document the same mainline ABI.
  The first candidate reads only compatible already bound endpoints.
  Other driver versions and named physical connectors require
  separate evidence.
- Non-blocking for the Linux hwmon slice: OS ownership leaves raw LPC, ECIO and ISA bridge access outside the scope.
  Separate firmware profiles and a safe Windows transport remain
  tracked by [#2519](https://github.com/shm11C3/HardwareVisualizer/issues/2519).

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-08 | Propose exact Experimental Linux sysfs coverage for the secondary controller evidenced on TRX50 AI TOP. |
