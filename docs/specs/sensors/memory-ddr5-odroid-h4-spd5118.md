# Spec: ODROID-H4 DDR5 DIMM temperature through Linux spd5118

| Field | Value |
| --- | --- |
| Revision | 2 |
| Status | Implementation-ready (rev 2) |
| Scope | Experimental read-only temperature for Samsung M425R1GB4BB0-CWMOD on the single-slot HARDKERNEL ODROID-H4 memory profile with Dasharo UEFI v0.9.1 and an already bound Linux SPD5118 hub on Intel SMBus. |
| Issue phase | [#2526](https://github.com/shm11C3/HardwareVisualizer/issues/2526) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Hardkernel [ODROID-H4 hardware specification](https://www.hardkernel.com/blog-2/odroid-h4-series/) and Dasharo [H4 hardware validation matrix](https://docs.dasharo.com/variants/hardkernel_odroid_h4/hardware-matrix/) | Primary for the single DDR5 SO-DIMM socket and the exact Samsung M425R1GB4BB0-CWMOD validated in the H4+ configuration. No temperature claim is supplied by the matrix. |
| S2 | MSI [Cubi service guide v1.0](https://storage-asset.msi.com/global/pdf/Cubi_B0B1v1.0_ServiceGuide_0411-2.pdf#page=51), p. 51, System Info screenshot | The Memory 1 entry identifies Samsung M425R1GB4BB0-CWMOD as an 8 GB DDR5-5600 module. S1 identifies the ODROID test configuration. |
| S3 | Dasharo [ODROID-H4 UEFI v0.9.1 release](https://docs.dasharo.com/variants/hardkernel_odroid_h4/releases/#v091-2025-09-03), September 3, 2025, coreboot `7d10ea62`, [build config](https://github.com/Dasharo/coreboot/blob/7d10ea62/configs/config.hardkernel_odroid_h4), [board Kconfig](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/mainboard/hardkernel/odroid-h4/Kconfig), [memory configuration](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/mainboard/hardkernel/odroid-h4/romstage_fsp_params.c), and [SMBus SPD read](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/soc/intel/common/block/smbus/smbuslib.c) | **lead-only (copyleft)** for exact board strings, socket address and the 1024-byte boot-time DDR5 SPD read. This is the firmware's own source. |
| S4 | Dasharo coreboot at `7d10ea62`, [UEFI BIOS version string](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/vendorcode/dasharo/smbios.c), [Intel DIMM information](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/soc/intel/alderlake/romstage/romstage.c), [saved serial field](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/soc/intel/common/smbios.h), and [Type 17 output](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/lib/smbios.c) | **lead-only (copyleft)** for deriving Type 17 from the FSP's SPD information, exact serial formatting and part string. Only information describing the single populated slot is used. |
| S5 | JEDEC *DDR5 DIMM Label*, JESD401-5D, [public-use document mirror](https://studylib.net/doc/28582427/jesd401-5d), serial and part fields, and DMTF [SMBIOS DSP0134 3.9.0](https://www.dmtf.org/sites/default/files/standards/documents/DSP0134_3.9.0.pdf), §7.18 | Primary for SPD field offsets and Type 17 memory device locator, serial, part and type semantics. |
| S6 | Linux [SPD5118 hwmon ABI](https://docs.kernel.org/hwmon/spd5118.html) and [generic hwmon ABI, v6.18](https://www.kernel.org/doc/html/v6.18/hwmon/sysfs-interface.html) | Primary for public read-only temperature and EEPROM attributes, temperature enable attribute semantics and units. |
| S7 | Linux [SPD5118 driver, v6.18](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/spd5118.c), November 30, 2025, Git blob `5da44571b6a0ce43b621faba9e67d81491ff0b28`, [Intel I801 driver at the same tag](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/busses/i2c-i801.c), [DMI-based instantiation](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-smbus.c) and [driver configuration](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/Kconfig) | **lead-only (copyleft)** for binding capabilities, the thermal-enable attribute, quarter-degree scaling, locking, EEPROM size and normal kernel enumeration limits. |
| S8 | [ODROID-H4 base model PCI inventory](https://forum.level1techs.com/t/mini-pc-home-server/218042/18), October 14, 2024, and Dasharo [board device tree](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/mainboard/hardkernel/odroid-h4/devicetree.cb) | Independent first-hand observation of the onboard PCI SMBus controller at `00:1f.4`, PCI `8086:54A3`. The device tree is **lead-only (copyleft)** for the enabled controller. |
| S9 | Linux [regmap, v6.18](https://github.com/torvalds/linux/blob/v6.18/drivers/base/regmap/regmap.c), paged ranges and locked reads | **lead-only (copyleft)** for serialized page selection and read, and the lack of post-read page restoration. |
| S10 | Linux v6.18 [I²C bus](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-core-base.c#L699-L705), [adapter registration](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-core-base.c#L1557-L1565), [client registration](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-core-base.c#L999-L1004), [device core subsystem link](https://github.com/torvalds/linux/blob/v6.18/drivers/base/bus.c#L493-L510), [character-device class](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-dev.c#L611-L614), and [I²C sysfs documentation](https://github.com/torvalds/linux/blob/v6.18/Documentation/i2c/i2c-sysfs.rst#L35-L43) | **lead-only (copyleft)** for the canonical subsystem targets and the distinct character-device class. The public sysfs documentation identifies the adapter and client entries in the I²C bus namespace. |

## Detection and scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| DMI board `HARDKERNEL` / `ODROID-H4`, `Dasharo (coreboot+UEFI) v0.9.1`, Samsung `M425R1GB4BB0-CWMOD`, single populated DDR5 SO-DIMM, `spd5118` at `0x52` under `8086:54A3` | S1/S2 pin the DIMM and single-slot platform. Lead-only S3/S4 pin the exact firmware and identity encoding. S5/S6 provide the identity field and telemetry ABI. Lead-only S7/S9 pin the enabled sensor and page-select read details. Lead-only S10 pins the kernel-owned I²C subsystem links. S8 corroborates the PCI identity, with its device tree providing a lead-only binding fact. A one-to-one runtime match and successful temperature are required. | Experimental for the exact already bound endpoint with a unique live identity match. |
| DDR4 `jc42`, other modules, firmware, controllers and Windows routes | Separate scopes in [#2526](https://github.com/shm11C3/HardwareVisualizer/issues/2526). | Unsupported by this revision. |

- Require the exact DMI vendor, board and UEFI version strings in the
  table. The public hardware matrix shows an H4+ system. This scope
  identifies the one-slot memory implementation in the published
  firmware through its DMI strings and fixed memory bus. It does not
  attribute a separately marketed SATA or CPU variant.
- Require one canonical hwmon node named `spd5118` with device and
  `driver` links to the same bound I²C client and the Linux I²C
  `spd5118` driver. Its client address must be `0x52` on the
  non-multiplexed adapter under PCI `0000:00:1f.4` with
  `vendor=0x8086`, `device=0x54a3`, and a binding to
  `/sys/bus/pci/drivers/i801_smbus`. Verify that the kernel-owned
  `subsystem` symlinks of both the native I²C adapter `i2c-N` and its
  SPD client `N-0052` resolve to `/sys/bus/i2c` (S10, lead-only).
  Reject class targets such as `/sys/class/i2c-adapter` and
  `/sys/class/i2c-dev`. The latter is the separate character-device
  class (S10, lead-only). Require the canonical parent of `eeprom` and
  the canonical target of the `hwmonN/device` symlink to be the same
  I²C client. Bus and hwmon enumeration indices are transient.
  A missing, changed, duplicate or ambiguous binding is unavailable.
- Require exactly one populated Type 17 memory record and a usable
  device locator. Require 8 GiB DDR5 SO-DIMM and exact part number
  (S1, S2). For binary SMBIOS, require a formatted Type 17 length
  of at least `0x1B` bytes to contain the part string index at
  `0x1A`, serial at `0x18` and locator at `0x10` (S5). Require
  in-bounds formatted data and a terminated string set with valid
  indices. For inventory text, require a complete Type 17 report
  with those fields and the size, memory type and form factor.
  Empty records are excluded. Duplicate or incomplete populated
  inventory is unavailable. Keep the locator and its
  Type 17 handle scoped to this boot. Do not display the serial or
  use transient sysfs enumeration as identity.

## Identity and temperature facts

| Item | Interpretation | Source |
| --- | --- | --- |
| SPD serial at offsets 517–520, zero-based | Four bytes in increasing address order. Dasharo's Type 17 presents those four SPD-derived bytes as eight lowercase hexadecimal characters, preserving leading zeros and order. | S4 (lead-only), S5 |
| SPD part at offsets 521–550, zero-based | ASCII part field. Strip trailing ASCII space padding only and require exact `M425R1GB4BB0-CWMOD`. The populated Type 17 must have the same exact part. | S1, S2, S4 (lead-only), S5 |
| `temp1_enable` | Standard hwmon enable attribute, visible on the pinned driver. Read only. Value `1` is required. | S6, S7 (lead-only visibility) |
| `temp1_input` | One temperature in millidegrees Celsius. The pinned driver returns 250 m°C increments and independently checks thermal capability before registering the device. | S6, S7 (lead-only scaling and capability) |

Decode only the exact eight-character Type 17 hexadecimal format to
four bytes. Require byte-for-byte equality with SPD. Reject all zero,
all ones, blank, malformed and mismatched identities. Do not swap byte
order, match a prefix, accept an alternate product spelling, or fall
back to address or record order. A complete Type 17 inventory is necessary
to verify uniqueness. The base SPD CRC does not authenticate the
manufacturing identity fields, so it cannot replace this match.

## Collection and safety

1. Scope all work to a request for DIMM telemetry. Use a directly
   readable local SMBIOS table or parse Type 17 in memory from the raw
   result of the existing user-initiated Linux detailed inventory
   action (`pkexec dmidecode --type memory`). The persisted aggregate
   `MemoryInfo` cache has no per-DIMM identity fields. If neither source
   supplies a complete inventory, the temperature is unavailable.
   Do not prompt for background elevation or persist serials.
2. After exact board and bound endpoint checks, read exactly 34 bytes
   starting at offset 517 through the existing I²C client's read-only
   `eeprom` binary attribute. The pinned driver registers a 1024-byte
   read-only NVMEM and serializes its EEPROM reads with a device mutex.
   The exact I801 adapter exposes SMBus operations without native I²C
   transfers. The pinned driver can bind this adapter in legacy 8-bit
   mode. The 34-byte slice stays on 128-byte page 4, at in-page offsets
   5–38. Regmap serializes each page selection and data read under the
   same device lock used for temperature transfers. It may update the
   volatile page selector in MR11 bits 2:0 while preserving the other
   bits. The read callback attempts no restoration of the prior page on
   success or error. A later actual temperature transfer selects page 0
   before reading. These are kernel-owned read protocol operations. The lock
   does not establish arbitration with firmware or independent raw bus
   clients (S7, S9).
3. The application never opens raw SMBus devices, loads or binds a
   driver, changes addressing mode, selects a page itself, enables a
   sensor, writes EEPROM/PMIC data, or requests firmware changes.
   Preserve a verified identity match only while the same boot
   inventory and driver binding remain valid.
4. Read `temp1_enable` and `temp1_input` through the same bound node.
   Require the enabled state and a complete, valid signed decimal
   temperature greater than 0 and at most 125,000 m°C in steps of
   250 m°C. The positive window is a project gate. Convert m°C to °C.
   Read errors and removed devices yield an unavailable reading. Do
   not access alarm attributes, whose reads can clear state (S6).
5. Keep memory inventory and utilization independent from the
   optional temperature. Keep serial and part comparison local and
   ephemeral. No new persisted setting, history or outbound data.

On a kernel with standard x86 DMI registration, I801
scans too few addresses to find a single DIMM at `0x52`. Upstream's
optional driver autodetection can independently bind existing
supported devices. This feature accepts only an endpoint already
bound by the OS and leaves a non-bound configuration unavailable.
Upstream disables optional driver autodetection by default on x86
with DMI support.
It does not change OS probing or override SPD-write protection. (S7)

## Synthetic fixture acceptance

| Case | Required result |
| --- | --- |
| Exact DMI profile and PCI ancestry, one populated Type 17 row, SPD serial bytes `01 23 45 67`, Type 17 `01234567`, exact part, `temp1_enable=1` and `temp1_input=40250` | One temperature at 40.25 °C attributed to the Type 17 locator. |
| Part mismatch, unknown firmware, wrong controller, wrong client address, wrong binding, or a name-only hwmon match | No unsupported read. Leave inventory available. |
| Either native I²C node has a missing, broken or wrong-subsystem link, including a class target such as `/sys/class/i2c-adapter` or `/sys/class/i2c-dev` | No EEPROM or temperature read. Leave inventory available. |
| Second populated Type 17 row, duplicate match, malformed binary table, incomplete inventory text, empty locator, reversed, absent or invalid serial, or a partial EEPROM read | No attributed reading. Do not substitute address or order. |
| Missing or disabled sensor, inaccessible sysfs, aggregate-only or missing DMI inventory, transient bus or binding failure | Unavailable without fallback probing, bus writes or background elevation. |
| `temp1_input` 0, 250, 125000, 125250 or 40251 m°C | Accept only 250 and 125000 m°C in this set. Keep unavailable values distinct from measured zero. |
| Changed driver, I²C node re-created, or hwmon node renumbered | Requalify the endpoint and join before using a new binding. Do not equate an enumeration index with a physical slot. |

## Open questions

- Non-blocking for the Experimental scope: only a unique live match and a usable enabled temperature can produce a reading.
  A published combined SPD, Type 17 and temperature trace is still
  needed to claim independently verified operation. Availability
  depends on the OS already binding the hub.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-08 | Propose Implementation-ready status for the exact Experimental Linux DIMM scope with a fail-closed runtime identity gate. |
| 2 | 2026-11-08 | Pin the existing kernel-owned I²C subsystem identity check to Linux v6.18 and add wrong-subsystem fixture cases for the exact Experimental scope. |
