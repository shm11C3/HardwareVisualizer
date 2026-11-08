# Spec: Nuvoton NCT668x firmware-mediated temperature and fan readings

| Field | Value |
| --- | --- |
| Revision | 3 |
| Status | Draft — not implementation-ready |
| Scope | Existing firmware-enabled EC Space monitoring on MSI PRO B850-P WIFI (MS-7E56), raw chip ID `0xD592`, customer `0x020D`, EC 0.0 build 2025-04-18. Records separate Linux hwmon evidence for two NCT6686D boards. Excludes changing EC or Super I/O configuration, voltage, limits and fan control. |
| Issue phase | [#2520](https://github.com/shm11C3/HardwareVisualizer/issues/2520) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Nuvoton, [NCT6686D Hardware Datasheet v0.5](https://www.nuvoton.com/export/resource-files/NCT6686D_HW_Datasheet_V0_5.pdf), April 21, 2017, §§6.1 and 7.2–7.3, pp. 52–58, and §§8 and 18.11 | Primary for that documented hardware's configuration and EC host interface. The sensor map is firmware-defined. This document does not identify the MSI chip or settle production NCT6686D chip IDs. |
| S2 | MSI, [PRO B850-P WIFI User Guide](https://download.msi.com/archive/mnu_exe/mb/PROB850-PWIFI_English.pdf), Specifications, Fan Connectors and Regulatory Notices | Primary for board name, board number and available physical connectors. MSI lists the I/O controller as NCT6687-R. |
| S3 | Linux [v6.18 NCT6683-series hardware monitor](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/nct6683.c), GPL-2.0-or-later | **lead-only (copyleft)**. Raw-ID family match, EC configuration discovery and application-port transaction, firmware identity, channel configuration, temperature and RPM words. Its mutation paths are outside this draft. |
| S4 | 3mdeb, [Porting MSI PRO B850-P to coreboot, Part 1](https://blog.3mdeb.com/2026/2026-02-09-msi_pro_b850p_part1/#super-io-initialization), February 9, 2026, and its [pinned article source](https://github.com/3mdeb/news-and-ideas/blob/df347e0fd731df6832c19abaeb5dcc594e3b4dc7/blog/content/post/2026-02-09-msi_pro_b850p_part1.md) | Published independent board observations and provenance for the full OEM capture in S8. |
| S5 | [MSI PRO B850-P WIFI hardware report](https://lore.landau.one/lkml/0c8f35ac-3c92-4b1d-a027-7d8912751109%40roeck-us.net/T/), October 4, 2026 | **lead-only (copyleft)** for its report on the exact board, BIOS and EC build, customer ID and fan channels. No control transactions are specified here. |
| S6 | [PawnIO interface](pawnio-interface.md) revision 7, `LpcIO` and mutex sections, and [pinned upstream module interface](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/LpcIO.p) at tag 0.2.11 | Interface and allow-list behavior for discovery and byte I/O. Existing signed module, no additional blob required by this candidate. |
| S7 | Linux [NCT6683-series documentation](https://www.kernel.org/doc/html/v6.18/hwmon/nct6683.html), Tested Boards and Firmware Versions, and [MSI MAG B850M MORTAR WIFI report](https://lists.openwall.net/linux-kernel/2026/09/21/3030) | **lead-only (copyleft)**. Firmware and variant inventory for follow-up, not an enablement list. |
| S8 | 3mdeb, [full OEM Super I/O and EC capture](https://paste.dasharo.com/?ba0b5d09afbacfc1#6vRmb9aVC4cG4LVfGVEpD3KV6a5wKVCrui8wveLru5gT), linked by S4 | First-hand board observation of raw chip ID, enabled EC window, firmware identity bytes and channel readback. The capture gives no system BIOS version. |
| S9 | Dasharo/coreboot at `d08d5eaa98`, [Nuvoton diagnostic](https://github.com/Dasharo/coreboot/blob/d08d5eaa98/util/superiotool/nuvoton.c) and [common configuration operations](https://github.com/Dasharo/coreboot/blob/d08d5eaa98/util/superiotool/superiotool.c), pinned by the tool banner in S8 | **lead-only (copyleft)** for the operations used to obtain the OEM observation. The utility performs other hardware operations outside the project's read-only scope. |
| S10 | Dasharo [MSI PRO B850-P firmware v0.9.0](https://docs.dasharo.com/variants/msi_b850/releases/), coreboot `713a0b76`, [board device tree](https://github.com/Dasharo/coreboot/blob/713a0b76/src/mainboard/msi/ms7e56/devicetree.cb), [chip configuration](https://github.com/Dasharo/coreboot/blob/713a0b76/src/superio/nuvoton/nct6687d/superio.c), [configuration mode](https://github.com/Dasharo/coreboot/blob/713a0b76/src/superio/common/conf_mode.c) and [firmware EC port](https://github.com/Dasharo/coreboot/blob/713a0b76/src/superio/nuvoton/nct6687d/nct6687d_ec.c) | **lead-only (copyleft)** for how separate firmware on the same board exits configuration mode and uses the firmware EC port. |
| S11 | [Windows SMBIOS board identification report](https://xmrig.com/benchmark/5hgiH1) and MSI [B850 GAMING PLUS WIFI User Guide](https://download.msi.com/archive/mnu_exe/mb/B850GAMINGPLUSWIFI_German.pdf), Regulatory Notices | Windows reports the full PRO board product and manufacturer. MSI also uses MS-7E56 for a different product. |
| S12 | Linux v7.0 [NCT6683-family identity and register ranges](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c#L37-L185), [read and refresh](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c#L503-L654), [fan input](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c#L718-L725), [temperature input](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c#L822-L831), [probe](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c#L1100-L1317), [chip match](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c#L1350-L1410) and [resource conflict check](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c#L1450-L1490) | **lead-only (copyleft)** for masked identity, customer admission, driver and attribute behavior, and the full input refresh read set. These sources describe driver accesses, with firmware address semantics still dependent on the EC build. |
| S13 | Linux [BC-250 customer support commit](https://github.com/torvalds/linux/commit/f392611e268f5462b51d90857c72058c31282706), first in v6.15, and [v7.1-rc5 documentation](https://www.kernel.org/doc/html/v7.1-rc5/hwmon/nct6683.html#tested-boards-and-firmware-versions) | **lead-only (copyleft)** for native customer `0x162b` acceptance and reported NCT6686D EC 1.0 build 2021-07-28. |
| S14 | ASRock AMD BC-250 [December 2024 Fedora 41 observation](https://github.com/mothenjoyer69/bc250-documentation/issues/3#issue-2734591172) and [separate Fedora CoreOS boot log](https://github.com/elektricM/amd-bc250-docs/blob/98e48d9a346cb04ba1cabc62e54a0ae5363357a6/docs/linux/fedora-coreos.md#L132-L150) | First-hand raw ID, live values and a separate firmware/version and platform-instance observation. The documented captures use the driver's force option. |
| S15 | Linux [Z590 Taichi customer support commit](https://github.com/torvalds/linux/commit/c0fa7879c9850bd4597740a79d4fac5ebfcf69cc) and [stable submission with the author's two-board report](https://lists.openwall.net/linux-kernel/2026/02/28/1014) | **lead-only (copyleft)** for native customer `0x1621` acceptance and reported instantiation on EC 1.0 build 2021-01-25. |

## Detection

- Require manufacturer `Micro-Star International Co., Ltd.` and full
  board product `PRO B850-P WIFI (MS-7E56)` from Windows board
  identification. The board number `MS-7E56` is shared with another
  product (S2, S11).
- Require raw chip ID `0xD592` in configuration registers `0x20` and
  `0x21` at `0x4E/0x4F`, enabled logical device `0x0B` with base
  `0x0A20`, customer ID `0x020D`, and EC version 0.0 build
  2025-04-18. S8 observes that complete controller tuple under OEM
  firmware. S5 independently reports the same EC build on BIOS 2.A93.
  Firmware identity for this candidate is the EC tuple. Keep other
  raw IDs unsupported even if they match the Linux family mask
  `0xFFF0` against `0xD590` (S3, S5, S8).
- S2, S5 and S8 use different package suffixes for the controller.
  Use the exact board and raw controller tuple for attribution.

| Scope | Status | Default enablement |
| --- | --- | --- |
| MSI PRO B850-P WIFI (MS-7E56), raw `0xD592`, customer `0x020D`, EC 0.0 (2025-04-18) | S8 pins OEM routing, identity bytes and sample readback. Decode relies on lead-only S3. The entry, exit and application-port sequence rely on lead-only S9 and S10 as described below. Safe selector handling and signed-transport confirmation remain open. | Disabled. Experimental candidate after the blocking questions are resolved and the spec is approved. |
| Other MSI NCT6687D/-R deployments | S7 reports other builds, including MAG B850M MORTAR WIFI with EC 0.0 (2025-03-10). A separate profile needs raw ID, firmware identity and channel evidence. | Unsupported by this revision. Follow-up in #2520. |
| Direct-access NCT6686D, including ASRock deployments | S1 pins a host interface but the stated chip IDs conflict with S3. S7 names multiple firmware builds. | Unsupported by this revision. Follow-up in #2520. |
| ASRock AMD BC-250, bound Linux `nct6683` | Lead-only S12/S13 document native admission of customer `0x162b` and the EC 1.0 (2021-07-28) build. Separate S14 observations provide raw `0xD441`, a bound instance and usable channel values under the force option. | Disabled. Firmware read safety and native endpoint qualification are unresolved. |
| ASRock Z590 Taichi, bound Linux `nct6683` | Lead-only S12/S15 document native admission of customer `0x1621` and successful instantiation on two NCT6686D units with EC 1.0 (2021-01-25). | Disabled. Firmware read safety, exact endpoint and channel evidence are unresolved. |
| NCT6683D, including Intel and ASRock deployments | S7 reports firmware-dependent differences and an Intel OEM layout outside the public specification. | Unsupported by this revision. Follow-up in #2520. |

### Linux bound-hwmon research

S14's BC-250 Fedora 41 observation reports `nct6686-isa-0a20` with
fan input 2 at 2,877 RPM and `Thermistor 14` and `Thermistor 15` at
45.5 °C and 45 °C. Those labels identify sensor sources and provide no
physical temperature location. The separate CoreOS log reports
configuration port `0x2e`, EC base `0x0a20`, platform instance
`nct6683.2592` and EC 1.0 build 2021-07-28. The observations do not
provide a same-unit board BIOS, firmware tuple and native unforced
endpoint. The Z590 Taichi two-unit report in S15 gives no complete raw
ID, EC base, BIOS version or individual channel values.

S12 confirms the native kernel matches `0xD440` under mask `0xFFF0`
and accepts both customer IDs without the force option. Normal binding
checks for ACPI conflicts and claims the I/O region. Binding can also start hardware
monitoring if it was disabled. A future Linux path would consume an
already bound hwmon endpoint and request no module actions. The
ordinary input attributes do not expose the raw ID, customer or EC
build. The force option is not exported as a sysfs parameter (S12).

Both input callbacks can refresh a shared cache. For the two customer
IDs, the enabled-channel refresh reads voltages and their high/low
limits, temperatures and their minimum, maximum, hysteresis and critical
limits, RPM and fan minimums, and PWM readback. The reads lie in
`0x0100`–`0x0167` and `0x0330`–`0x03D7`. The driver writes EC
page/index selectors and reads EC data. It makes no EC data or host
event writes during the refresh and does not select the driver's
designated monitor/fan status addresses (S12). These are driver access
facts. S1 §§7.2–7.3 and §8 place the underlying sensor semantics in the
separate EC Space specification. The [MSI NCT6687D Linux evidence](superio-nuvoton-nct6687-linux-hwmon.md)
covers its own OEM firmware profiles.

## Register map (facts)

EC addresses below are page in the high byte and index in the low byte.

| Address | Meaning | Units / encoding | Source |
| --- | --- | --- | --- |
| Super I/O configuration `0x07` | Logical device select. EC Space is logical device `0x0B`. | Byte | S3, corroborated for the documented NCT6686D by S1 §18.11 |
| EC logical device `0x30` bit 0 | EC Space enabled | Flag. S8 observes `0x01` for the full byte. | S3, S8 |
| EC logical device `0x60` and `0x61` | EC Space I/O base | Big-endian word. An eight-port window starting at `0x0A20` for this candidate. | S1, S3, S8 |
| Base + `4`, `5`, `6` | Application EC page selector, index selector and data, respectively | Byte I/O | S1 §7.2.5–7.2.7, S3, S8, S9 |
| EC `0x0602` and `0x0603` | Customer ID | Big-endian word | S3, S8 |
| EC `0x0604`, `0x0605`, `0x0606` | Build year within century, month, day | MSI uses binary byte values. The candidate tuple is decimal 25, 4, 18. | S3, S5, S8 |
| EC `0x0608`, `0x0609` | Major and minor firmware version | Binary byte each | S3, S5, S8 |
| EC `0x0180` bit 7 | Monitoring enabled | Flag. S8 observes `0x80` for the full byte. | S3, S8 |
| EC `0x01A0 + n` bits 6:0 | Assignment of monitoring input `n` (0–31). Bit 7 is separate. | Source code | S3, S4 |
| EC `0x0100 + 2n` and following byte | Temperature value of configured monitoring input `n` | Signed big-endian word. S3 reports half-degree increments after truncating the signed word divided by 128, then multiplying by 0.5 °C. | S3; S4 corroborates whole-degree values for a sample |
| EC `0x01C0 + n` bit 7 | Fan tachometer input `n` (0–15) enabled | Flag | S3 |
| EC `0x0140 + 2n` and following byte | Tachometer value for fan input `n` | Unsigned big-endian direct RPM | S3 |

| Sample input from S8 | Assignment or enable byte | Readback word |
| --- | --- | --- |
| Temperature slot 1 | `0x82` | `0x1A00` (26 °C) |
| Temperature slot 2 | `0x89` | `0x1600` (22 °C) |
| Temperature slot 3 | `0x8C` | `0x1A00` (26 °C) |
| Fan input 0 | `0xA1` | `0x0206` (518 RPM) |
| Fan input 1 | `0xA3` | `0x0000` (physical state unrecorded) |

## Read procedure and decode

1. Scope discovery to the identified MSI board and configuration pair
   at `0x4E/0x4F` (S6, S8). Acquire the ISA mutex before slot selection
   or any configuration operations. The candidate enters with two
   writes of `0x87` to the configuration index port and exits with
   `0xAA` to that port. S10 corroborates this sequence on the same
   board with Dasharo firmware. S9 performs a generic exit that also
   writes an ITE-specific register after `0xAA`. Those additional
   writes are outside the candidate scope.
2. Check the exact raw chip ID before logical-device selection. With
   logical device `0x0B` selected, verify EC Space is already enabled
   and its base is exactly `0x0A20`. `LpcIO` discovers already assigned
   BARs in configuration mode and restricts subsequent byte I/O to
   authorized windows.
   Discovery success alone does not confirm that the EC ports are
   authorized. Slot selection resets this executor's authorization
   state, so recheck access after selecting another slot or loading a
   new executor (S1, S3, S6).
3. Confirm the signed executor permits byte I/O to the discovered
   application EC window after exiting configuration mode. In the OEM
   capture, each EC byte read starts by writing `0xFF` to the application
   page selector, then selects the page and index and reads the data
   port (S8, S9). Use high byte first for each pair. S10 writes `0xFF`
   again at the end on the firmware EC port. The application port's
   post-read release behavior and arbitration require separate evidence.
4. Validate the customer, version and build tuple and check the
   monitoring-enabled bit before accessing channel readings (S3, S8).
   An unexpected tuple or a disabled monitor yields no readings. Do
   not start a disabled monitor.
5. Scope the first candidate temperature inputs to slots 1, 2 and 3.
   S8 observed their assignment bytes as `0x82`, `0x89`, `0x8C`.
   The corresponding seven-bit assignments are temperature diode or
   thermistor sources (S3). Check the low seven bits for the expected
   assignment at each slot. Keep a generic slot label pending evidence
   of its physical location. Skip disabled or different assignments.
   Reject all-ones reads and values outside −55 to +125 °C as
   defensive project gates.
6. Scope the first fan candidate to inputs 0 and 1 (S5, S8). Check
   the enable flag before reading. Emit only verified valid positive
   RPM. Treat zero, all-ones, failed or incomplete reads as unavailable
   under project gates. S8 observes a zero for an enabled input with
   unknown physical state. Keep generic input labels and defer any
   stopped-fan claim, header mapping and other channels.
7. Preserve independently valid channels and keep temperature and
   fan identity tied to the EC input slot. An EC failure does not
   suppress readings from a separate NCT679x controller.

## Quirks

- S4 shows slots configured differently from other MSI boards and
  separate AMD temperature telemetry on slot 0. Slot 0 and virtual
  temperatures are excluded from the initial motherboard profile.
- S1 states that the EC hardware-monitor implementation is provided
  by firmware. EC Space transport agreement does not establish an
  interchangeable sensor map.

## Safety notes

- Require `Global\Access_ISABUS.HTP.Method` with a bounded acquisition
  timeout throughout the configuration, BAR and multi-byte EC read
  sequences. When the lock is unavailable, skip collection (S6). The
  mutex serializes participants that acquire it. Establish protection
  for the application selector against OEM firmware and ACPI access
  separately before enabling this candidate.
- Use application EC ports only. Never write EC data, host events,
  logical-device enable, BARs, monitoring-enable, limits, tachometer
  configuration, fan PWM or power state. Only the documented
  selectors and mode transitions necessary for reads are candidates.

## Fixture acceptance for the candidate

| Case | Required result |
| --- | --- |
| Exact controller tuple, authorized window and synthetic valid samples matching S8 | Three independent temperatures and 518 RPM on fan input 0. Fan input 1 is unavailable. |
| Different board product with the same board number, unknown raw ID within the family, wrong base, or different EC tuple | Reject at the first mismatched gate before any channel reads. |
| Mutex timeout, denied BAR authorization, disabled logical device, or disabled monitoring | Skip channel reads and preserve results from other providers. |
| One temperature assignment mismatch, unavailable byte, all-ones word or out-of-range decode | Skip that input and preserve independently valid results. Cover both limits of the temperature gate. |
| Disabled fan input, zero, all-ones, invalid positive value or one failed byte of a pair | Skip that input and preserve independently valid results, without renumbering any slots. |
| Configuration or selector failure partway through collection | Exercise the evidenced cleanup path. Stop further transactions if it cannot release the mode or selector safely. |

## Open questions

- Blocking transport evidence: confirm configuration exit with only
  `0xAA` and scoped reads via the shipped signed `LpcIO` module on
  this exact OEM tuple with the monitor already enabled. S8 used
  ordinary byte port I/O for its EC reads. It does not establish
  signed executor authorization, timing or error handling (S6, S9).
- Blocking selector safety: verify whether the MSI firmware enables
  a selector protection mechanism like the one documented for
  NCT6686D in S1 §7.2. Establish the exact application page/index
  release behavior on success, access failure and timeout. Determine
  whether OEM firmware or ACPI can use the same window concurrently
  and what prevents interference for the proposed read path.
- Blocking fan validity: establish how a positive word is acquired
  coherently and how to reject invalid or stale positive readback on
  the exact firmware. The S8 snapshot establishes sample words and
  enable flags. It does not establish the firmware's error flag
  semantics or a stable snapshot protocol.
- Non-blocking for a verified positive-only RPM slice: a labeled
  observation or exact firmware documentation is needed to distinguish
  stopped and unpopulated inputs. Until then both are unavailable
  when readback is zero. Any later use of error/status registers needs
  evidence of their meaning and read side effects.
- Non-blocking for the proposed MSI slice: Intel and ASRock variants
  require their own profiles. The Nuvoton NCT6686D document shows
  `0xC7` and differing lower bytes in its global table and detailed
  description. S3 attributes an `0xD440` family to the production
  NCT6686D. Do not resolve this conflict by masking or aliasing IDs.
- **Linux BC-250 evidence remains blocking.** Pin an exact EC 1.0
  (2021-07-28) firmware map or equivalent evidence that the full enabled
  refresh, including limit and PWM readbacks, uses ordinary,
  non-destructive fields. Obtain the board/BIOS and EC identity
  association with a matching unforced kernel-owned hwmon endpoint.
- **Linux Z590 Taichi evidence remains blocking.** Pin the equivalent
  firmware semantics for EC 1.0 (2021-01-25). Associate that build with
  a known board/BIOS, controller identity and native hwmon instance,
  and establish at least one attributable published input.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-07 | Initial draft of the MSI candidate and its firmware-dependent access, with unsupported scopes and unresolved read-only safety conditions. |
| 2 | 2026-11-08 | Pin the exact OEM controller capture, a configuration exit candidate, and sample data. Narrow the fan proposal to verified positive RPM and retain transport, selector and fan validity blockers. |
| 3 | 2026-11-09 | Record separate BC-250 and Z590 Taichi Linux hwmon research, native customer acceptance and unresolved firmware refresh evidence. |
