# Spec: Linux sole DDR5 module temperature through spd5118 with SPD identity confirmation

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental read-only temperature for a Linux system whose complete SMBIOS inventory reports exactly one populated DDR5 memory device and whose kernel exposes exactly one SPD5118 I²C client. The bound hub's SPD identity must confirm the populated Type 17 record. Systems with two or more modules or two or more SPD5118 clients, LPDDR5, DDR4, Windows and raw bus routes are excluded. |
| Issue phase | [#2701](https://github.com/shm11C3/HardwareVisualizer/issues/2701) |

This document re-scopes the exact ODROID-H4 gates under
[ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md).
Board, firmware version, module part, PCI identity and client address become
evidence identities. The attribution mechanism is the single-candidate SPD
identity confirmation described below. The multi-module join stays in the
[general DDR5 draft](memory-ddr5-spd5118-type17.md).

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | DMTF [SMBIOS DSP0134 3.9.0](https://www.dmtf.org/sites/default/files/standards/documents/DSP0134_3.9.0.pdf), §7.17 Table 71 (pp. 97–98), §7.18 note and Table 75 (pp. 99–100), Table 77 (p. 104), §7.49 Table 141 (p. 152), Annex A §§4.7.7 and 4.8.1 (p. 155) | Primary. Type 16 device count as slots or sockets, one Type 17 per socket whether populated or not, Size 0 for no installed device and `FFFFh` for unknown size, Memory Type `22h` DDR5 and `23h` LPDDR5, Serial Number and Part Number strings, End-of-Table record. |
| S2 | ABLIC [S-34HTS08AB datasheet, Rev.1.1_00](https://www.ablic.com/en/doc/datasheet/dimm_serial_eeprom_spd/S34HTS08AB_E.pdf), Table 4 (p. 5), Table 7 (p. 10), §4.5.1 and Table 8 (p. 11), §4.6 (p. 13), Table 75 MR0/MR1 (p. 51), Table 80 MR5 (p. 53), Table 82 MR11 (p. 54), MR26 (p. 59), temperature format (p. 63), MR49/MR50 (p. 66) | Primary. Public datasheet of a part that states JEDEC SPD5118 compliance. Hub address composition, host-side address strap, paging, device type, capability, sensor disable and temperature format. |
| S3 | JEDEC *DDR5 DIMM Label*, JESD401-5D, [public-use document mirror](https://studylib.net/doc/28582427/jesd401-5d), §§2.3–2.5, pp. 11–12 | Primary for SPD manufacturing identity offsets and byte order: manufacturer ID 512–513, location 514, date 515–516, serial 517–520, module part 521–550, all zero-based. |
| S4 | Micron [*DDR5 SDRAM RDIMM Core*](https://www.mouser.com/datasheet/2/671/ddr5_rdimm_core-3310292.pdf), Serial Presence Detect section, SPD EEPROM block map | Primary. The base SPD CRC covers bytes 0–509. Manufacturing data starts at byte 512 and is not covered by that CRC. |
| S5 | Linux [SPD5118 hwmon ABI](https://docs.kernel.org/hwmon/spd5118.html) and [generic hwmon ABI, v6.18](https://www.kernel.org/doc/html/v6.18/hwmon/sysfs-interface.html) | Public ABI of the kernel interface this project reads. `temp1_input` in millidegrees Celsius, `temp1_enable`, read-only `eeprom`, read-to-clear alarm attributes. |
| S6 | Linux v6.18 [DMI-based SPD instantiation](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-smbus.c#L375-L488), callers in [Intel I801](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/busses/i2c-i801.c#L1180-L1186) ([mux segments](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/busses/i2c-i801.c#L1286-L1289)) and [AMD PIIX4](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/busses/i2c-piix4.c#L966-L974) | **lead-only (copyleft)**. How the kernel counts slots and populated modules from Type 17 and which addresses it tries. |
| S7 | Linux v6.18 [hwmon configuration](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/Kconfig#L2320-L2334) and [SPD5118 driver](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/spd5118.c), Git blob `5da44571b6a0ce43b621faba9e67d81491ff0b28`: [autodetect address list](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/spd5118.c#L30-L31), [temperature unit](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/spd5118.c#L67-L96), [EEPROM read](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/spd5118.c#L336-L400), [chip checks](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/spd5118.c#L590-L745) | **lead-only (copyleft)**. Driver autodetection default, chip and capability checks, 250 m°C quantization, 1024-byte read-only NVMEM and its lock. |
| S8 | Linux [regmap, v6.18](https://github.com/torvalds/linux/blob/v6.18/drivers/base/regmap/regmap.c), paged ranges and locked reads | **lead-only (copyleft)**. Serialized page selection and read, and no restoration of the previous page. |
| S9 | Linux v6.18 [I²C bus](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-core-base.c#L699-L705), [adapter registration](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-core-base.c#L1557-L1565), [client registration](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-core-base.c#L999-L1004), [device core subsystem link](https://github.com/torvalds/linux/blob/v6.18/drivers/base/bus.c#L493-L510), [character-device class](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-dev.c#L611-L614), [hwmon device registration](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/hwmon.c#L973-L984), [NVMEM sysfs attributes](https://github.com/torvalds/linux/blob/v6.18/drivers/nvmem/core.c#L373-L436) and the [I²C sysfs ABI](https://github.com/torvalds/linux/blob/v6.18/Documentation/i2c/i2c-sysfs.rst) | Public sysfs topology ABI. **lead-only (copyleft)** for subsystem link targets, the separate character-device class, the hwmon child's parent and the NVMEM alias. |
| S10 | dmidecode 3.5 [manual](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/man/dmidecode.8), OPTIONS and DMI TYPES, and the [ODROID-H4 rev 3 inventory completion contract](memory-ddr5-odroid-h4-spd5118.md#inventory-completion) with its dmidecode 3.5 sources | Public command interface. Decoder traversal, Type 127 rendering and diagnostic facts are **lead-only (copyleft)** as recorded in that contract. |
| S11 | Hardkernel [ODROID-H4 hardware specification](https://www.hardkernel.com/blog-2/odroid-h4-series/), Dasharo [H4 hardware validation matrix](https://docs.dasharo.com/variants/hardkernel_odroid_h4/hardware-matrix/), and Dasharo coreboot `7d10ea62` [Type 17 serial output](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/lib/smbios.c#L164-L174) | Primary for the single DDR5 SO-DIMM socket. The firmware source is **lead-only (copyleft)** for its eight-digit forward serial rendering. Evidence for the best-evidenced member only. |
| S12 | AMD [*Ubuntu Tuning Guide for AMD EPYC 9005 Series Processors*](https://docs.amd.com/api/khub/documents/2tw6thFYsT7~8Hl2Cdo68g/content#page=30), document 58470 rev 1.0, §2.10.1, pp. 26–28 | First-hand Type 17 examples with eighteen hexadecimal serial digits. No paired EEPROM dump or producer rule. |

## Detection and scoped enablement

| Condition | Gate kind | Source |
| --- | --- | --- |
| The existing inventory completion contract passes: fixed `pkexec dmidecode --type memory --type 127`, successful exit, empty stderr, no decoder diagnostics and exactly one terminal decoded Type 127 record. | safety | S1, S10 (lead-only decoder facts) |
| Type 16 and Type 17 handles are unique. Every Type 17 array handle resolves to a reported Type 16. Each array's declared device count equals its associated Type 17 records. Across all arrays exactly one Type 17 is populated and every other Type 17 has Size 0. The populated record has Memory Type `22h`, a known nonzero size and a usable Device Locator. | mechanism | S1 |
| Exactly one canonical I²C client in the system has the name `spd5118` or a `driver` link to `/sys/bus/i2c/drivers/spd5118`. Bound and unbound clients both count. | mechanism | S5, S7/S9 (lead-only) |
| The client address has bits 6:3 equal to `1010b` (`0x50`–`0x57`). The HID bits 2:0 are never interpreted as a slot. | safety | S2 Table 7 |
| Kernel ownership: the client's `driver` link resolves to `/sys/bus/i2c/drivers/spd5118`. The client and its immediate adapter have `subsystem` links resolving to `/sys/bus/i2c`. The canonical parent of the direct `eeprom` file is the client. Exactly one hwmon child named `spd5118` lies under the client's `hwmon` directory, and its `device` link resolves to the same client. Class targets such as `/sys/class/i2c-adapter` and `/sys/class/i2c-dev` are rejected. | safety | S5, S9 (lead-only) |
| SPD identity confirmation (below) passes against the sole populated Type 17. | safety | S1, S3, S4 |
| `temp1_enable` of that hwmon child reads `1`. | mechanism | S5, S7 (lead-only visibility) |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Sole populated DDR5 module, sole `spd5118` client and confirmed SPD identity | S1 defines the inventory cardinality and DDR5 type. S2 defines the hub address structure and temperature format. S3/S4 define the identity bytes and show they are outside the base CRC. S5 is the public ABI. Depends on lead-only S6 (kernel Type 17 driven instantiation), S7 (driver checks, quantization, NVMEM), S8 (paging and locking), S9 (ownership links) and the S10 decoder facts. Best-evidenced member: HARDKERNEL ODROID-H4, Dasharo (coreboot+UEFI) v0.9.1, Samsung `M425R1GB4BB0-CWMOD`, client `0x52` under Intel I801 `8086:54A3` (S11). No live capture of a non-ODROID member exists. | Experimental. |
| Sole populated DDR5 module whose Type 17 serial uses a syntax other than the two forms below | No confirmation rule. | Unsupported. |
| Two or more populated DDR5 modules, or two or more `spd5118` clients | Producer mapping unresolved; see the [general DDR5 draft](memory-ddr5-spd5118-type17.md). | Unsupported by this document. |
| LPDDR5 (`23h`), DDR4, unbound hubs, Windows and raw bus routes | Not specified here. | Unsupported. |

Precedence. A system that passes every gate of the
[Protectli VP2430 specification](memory-ddr5-protectli-vp2430-spd5118.md)
uses that route, which reads no EEPROM bytes. Every configuration admitted
by the [ODROID-H4 specification](memory-ddr5-odroid-h4-spd5118.md) at rev 3
also passes the gates of this document, so an implementation of this
document covers it. The ODROID-H4 document remains valid as written.

## Attribution mechanism (facts)

| Item | Fact | Source |
| --- | --- | --- |
| Type 17 cardinality | One Type 17 exists per memory-device socket, populated or not. Type 16 Number of Memory Devices counts slots or sockets and equals the associated Type 17 count. | S1 §7.18 note p. 99, Table 71 p. 98, Annex A §§4.7.7, 4.8.1 p. 155 |
| Type 17 Size | `0000h` means no memory device is installed in the socket. `FFFFh` means unknown. `7FFFh` defers to Extended Size. | S1 Table 75 p. 100 |
| Type 17 Memory Type | `22h` DDR5, `23h` LPDDR5. | S1 Table 77 p. 104 |
| Hub 7-bit address | Device type ID `1010b` in bits 6:3, followed by a 3-bit HID in bits 2:0. Up to eight hubs share one bus. | S2 Table 7 p. 10, §4.5.1 p. 11 |
| HID origin | The hub samples a resistor on its HSA pin at power-up. The resistor is on the host side of the connector, so the board design selects each socket's HID. Table 8 numbers example DIMMs 0–7 by HID. It does not relate a HID to a board socket label or an SMBIOS locator. No firmware-independent address-to-socket rule is defined. | S2 Table 4 p. 5, §4.4 p. 10, Table 8 p. 11 |
| Kernel instantiation | On an I801 adapter (the main adapter when not multiplexed, otherwise each mux segment) and on PIIX4 port 0, the kernel counts Type 17 records as slots, capped at 8. It counts records with nonzero size and a defined memory type as populated, and stops if populated types differ. For DDR5 and LPDDR5 it tries addresses `0x50` upward, one per slot, and stops once it has instantiated as many clients as populated records. | S6 (lead-only) |
| Driver autodetection | The SPD5118 driver's own address scan of `0x50`–`0x57` is disabled by default on x86 systems with DMI. | S7 (lead-only) |
| Misattribution hazard | If firmware reports an installed module as empty or omits its record, the kernel can bind the unreported module's hub. The populated count and the client count are then both one. Cardinality alone cannot detect this. | S1, S6 (lead-only) |
| Availability limit | With one Type 17 record the kernel tries only `0x50`. A sole module strapped to another HID has no client unless another mechanism instantiated it. | S2, S6 (lead-only) |

## SPD identity confirmation

Read exactly 39 bytes at EEPROM offsets 512–550 through the client's
direct `eeprom` (S3, S5). A short or failed read fails the gate.

| SPD bytes (zero-based) | Accepted Type 17 counterpart | Source |
| --- | --- | --- |
| 517–520, serial | Type 17 Serial Number of exactly 8 hexadecimal digits equals these four bytes rendered in increasing address order, two digits per byte. | S3; S11 (lead-only) for the observed producer form |
| 512–520, manufacturer ID, location, date and serial | Type 17 Serial Number of exactly 18 hexadecimal digits equals these nine bytes rendered in increasing address order, two digits per byte. | S3, S12 |
| 521–550, module part | ASCII with trailing ASCII spaces removed. When the Type 17 Part Number is present, printable and not a placeholder, it must equal this value exactly. | S1, S3 |

- Accept either hexadecimal letter case. Preserve leading zeros. Do not try
  reversed byte order, other groupings, prefixes or suffixes.
- Reject a Type 17 serial that is absent, blank, of another length, not
  hexadecimal, or a placeholder. Reject an SPD serial of four `00` or four
  `FF` bytes.
- Treat a Type 17 part of `Not Specified`, `Unknown`, all spaces or empty as
  absent. Then the serial alone confirms. Do not case-fold, truncate or
  prefix-match parts.
- The base SPD CRC does not cover bytes 512–550 and cannot replace this
  comparison (S4).

Why this confirms attribution. Both sides have exactly one candidate, so the
comparison never chooses between modules or byte orders. A match shows that
the bound hub's EEPROM carries the identity the firmware reports for its sole
populated record. A producer that renders the serial in another form yields
no match and leaves the temperature unavailable. The comparison needs no
proof of the producer's general encoding.

## Read procedure and decode

1. Run only within the existing explicit Linux detailed inventory request.
   Apply the completion contract and the inventory gates before any sysfs
   attribute read. Use no second privileged action and no background prompt.
2. Qualify the sole client and its ownership links. Count canonical aliases
   of one client once.
3. Read the 39-byte identity slice and apply the confirmation above.
4. Read `temp1_enable` and require `1`. Then read `temp1_input` once.
5. Accept a complete signed decimal integer greater than 0, at most
   125,000 and a multiple of 250. The upper bound and positivity are project
   plausibility gates. The 250 m°C step is the driver's quantization (S7,
   lead-only), consistent with the hub's 0.25 °C format (S2 p. 63).
6. Divide by 1000 to get °C. Attach it to the sole populated Type 17 record
   and label it with its Device Locator, adding Bank Locator when present.
7. Any failed gate leaves the temperature unavailable and keeps the ordinary
   inventory and utilization.

## Quirks

- In legacy one-byte addressing the identity slice lies on 128-byte page 4,
  in-page offsets 0–38. The kernel may write the volatile page pointer
  MR11 bits 2:0 to select it and does not restore the previous page. A later
  temperature read selects page 0 again. Volatile registers, including the
  temperature, need no page selection. These writes are part of the kernel's
  read protocol (S2 §4.6 p. 13 and MR11 p. 54; S7/S8 lead-only).
- The driver checks the hub's device type `0x5118` in MR0/MR1 and the
  temperature support bit MR5[1] before registering the hwmon device
  (S2 pp. 51, 53; S7 lead-only). MR26[0] set disables the sensor (S2 p. 59).
  `temp1_enable` reports this state (S5).

## Safety notes

- The application never opens raw I²C, SMBus or I3C devices, scans
  addresses, instantiates, binds or unbinds clients, selects pages, changes
  addressing mode, enables the sensor, or writes EEPROM, PMIC or limit data.
- Do not read alarm attributes, whose reads can clear state (S5).
- The kernel's NVMEM and register-map locks serialize the kernel's own
  accesses. They do not arbitrate with firmware or with independent raw bus
  clients (S7/S8, lead-only).
- Keep serial and part values local to the request. Do not display, persist
  or send them. Discard the confirmation when the boot inventory, client or
  hwmon child changes, and requalify a recreated or renumbered node.

## Synthetic fixture acceptance

| Case | Required result |
| --- | --- |
| Complete inventory, one array with one populated DDR5 Type 17, sole `spd5118` client at `0x52`, SPD serial `01 23 45 67`, Type 17 serial `01234567`, equal part, enable `1`, input `40250` | 40.25 °C on the sole Type 17 record. |
| SPD serial `0A 1B 2C 3D`, Type 17 serial `0a1b2c3d` or `0A1B2C3D`, otherwise as above | Confirmed. |
| SPD bytes 512–520 `80 CE 01 23 45 01 23 45 67`, Type 17 serial `80CE01234501234567` | Confirmed. |
| Type 17 serial `67452301` against SPD `01 23 45 67` | Unavailable. No reversed match. |
| Serial matches, Type 17 part differs from the SPD part | Unavailable. |
| Type 17 part `Not Specified`, serial matches | Confirmed. |
| SPD serial all `00` or all `FF`, or Type 17 serial blank, 7 or 10 digits, or non-hexadecimal | Unavailable. |
| Two populated Type 17 records, or a populated record with Size `FFFFh` | Unavailable. |
| Two `spd5118` clients, one of them unbound | Unavailable. |
| Firmware reports one populated module, the sole client belongs to an unreported module with a different serial | Unavailable. |
| Populated record is LPDDR5 or DDR4 | Unavailable. |
| Duplicate handle, unresolved array handle or declared count mismatch | Unavailable. |
| Missing Type 127, header-only marker, diagnostics, stderr or failed exit | Unavailable. Ordinary inventory preserved. |
| Client address `0x48` with name `spd5118` | Unavailable. |
| Adapter or client `subsystem` link resolving to `/sys/class/i2c-adapter` or `/sys/class/i2c-dev`, foreign `eeprom` parent, or hwmon child of another client | Unavailable. |
| Board `Protectli` / `VP2430` passing the VP2430 gates | VP2430 route applies. No EEPROM read. |
| ODROID-H4 with another firmware version or another module part, otherwise passing every gate | Confirmed and available. |
| `temp1_enable` `0` | No `temp1_input` read. Unavailable. |
| `temp1_input` 0, 250, 125000, 125250, 40251 | Accept only 250 and 125000. |
| Short EEPROM read | Unavailable. |

## Open questions

- Non-blocking for #2701 Experimental: every runtime gate fails closed and only a confirmed identity yields a reading.
  No same-boot capture of SPD identity, Type 17 identity and temperature on a
  system other than the ODROID-H4 family has been found. Demonstrated
  availability beyond the best-evidenced member is unknown.
- Non-blocking for #2701 Experimental: the residual case needs two modules with equal identity fields, which manufacturers assign to be unique.
  An unreported module whose serial and part equal the reported module's
  values would pass confirmation. The eight-digit form compares only the
  four serial bytes plus the part.
- Non-blocking for #2701 Experimental: the route does not depend on board-specific HID maps.
  Platform design guides that would tie HSA resistors to socket labels are
  not public. Multi-module attribution by address stays unavailable.
- Non-blocking for #2701 Experimental: instantiation facts only limit availability, never attribution.
  The kernel instantiation behavior is pinned to Linux v6.18 (S6, S7).
  Other kernel versions may instantiate more or fewer clients. Any client
  still has to pass every gate here.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-28 | Proposed transition to Implementation-ready for the Experimental sole-module DDR5 route keyed by inventory cardinality and SPD identity confirmation (ADR 0031 re-scope of the ODROID-H4 gates). Maintainer sign-off is pending in the proposing PR. |
