# Spec: Linux DDR4 DIMM temperature through jc42 and EE1004

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental read-only temperatures for DDR4 DIMMs that advertise a TSE2004av sensor and have an already bound Linux `jc42` and `ee1004` pair with a unique populated SMBIOS Type 17 match. |
| Issue phase | [#2526](https://github.com/shm11C3/HardwareVisualizer/issues/2526) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | JEDEC Standard 21-C, [DDR4 SPD Annex L, release 30](https://studylib.net/doc/28564665/4-01-02-annexl-6r30), §§8.1.1–8.1.4 (pp. 11–14), §8.1.15 (p. 21), §8.5.4–8.5.5 (p. 47) | Primary for the SPD layout, memory type, encoding/additions levels, TSE2004av presence bit, serial bytes and ASCII part padding. |
| S2 | Microchip [AT30TSE004A datasheet, Atmel-8868D, October 2015](https://www.microchip.com/content/dam/mchp/documents/OTH/ProductDocuments/DataSheets/Atmel-8868-DTS-AT30TSE004A-Datasheet.pdf), §5 Table 5-1, §6.1.1, §6.2 and §7.1.1. Renesas [TSE2004GB2B0 datasheet, May 9, 2018](https://www.renesas.com/en/document/dst/tse2004gb2b0-datasheet), pp. 11 and 14, *Event Pin Functionality* and *Temperature Data Register*. ST [STTS2004 datasheet, DocID024229 rev 7](https://www.st.com/resource/en/datasheet/stts2004.pdf), §§4.2.2–4.3.1 | Primary for shared physical address pins, the 7-bit addresses, bus-wide volatile page selection and temperature/event semantics on three JEDEC-compliant combined devices. |
| S3 | Linux v6.18 [EE1004 driver](https://github.com/torvalds/linux/blob/v6.18/drivers/misc/eeprom/ee1004.c), Git blob `e13f9fdd9d7b1c79af0ab67d129ca90054bc8e5e`, [read and paging](https://github.com/torvalds/linux/blob/v6.18/drivers/misc/eeprom/ee1004.c#L82-L181), [temperature client selection](https://github.com/torvalds/linux/blob/v6.18/drivers/misc/eeprom/ee1004.c#L184-L215), [page address reservation](https://github.com/torvalds/linux/blob/v6.18/drivers/misc/eeprom/ee1004.c#L244-L272) and [EEPROM registration](https://github.com/torvalds/linux/blob/v6.18/drivers/misc/eeprom/ee1004.c#L279-L329). Linux v7.3 [relocated driver](https://github.com/torvalds/linux/blob/v7.3/drivers/nvmem/ee1004.c), Git blob `923f404a44c03334eaa25d89d95d3b0e8908ed6d`. [Legacy SPD 1.0 fallback](https://lkml.iu.edu/2406.3/08402.html), upstream commit `6150e5e1ae2d8ad72f52217f8f41fe446cae9e27`. | **lead-only (copyleft)** for Linux driver ownership, page locking and retention, the current same-adapter address rule and the excluded legacy fallback. v7.3 retains these behaviors. |
| S4 | Linux v6.18 [jc42 hwmon ABI](https://github.com/torvalds/linux/blob/v6.18/Documentation/hwmon/jc42.rst#L137-L152) and [generic hwmon ABI](https://www.kernel.org/doc/html/v6.18/hwmon/sysfs-interface.html). Linux v6.18 [jc42 driver](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/jc42.c), Git blob `06f0ab2f52fa2b06152230de32801f7792b3ee6c`, [input read and conversion](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/jc42.c#L204-L228) and [configuration changes at probe and lifecycle](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/jc42.c#L503-L584). | Public read-only temperature ABI. Driver facts about one temperature word, scaling and lifecycle writes are **lead-only (copyleft)**. |
| S5 | Linux v6.18 [I²C sysfs ABI](https://github.com/torvalds/linux/blob/v6.18/Documentation/i2c/i2c-sysfs.rst), [client and adapter registration](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-core-base.c#L999-L1004) ([adapter](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-core-base.c#L1557-L1565)), [device subsystem link](https://github.com/torvalds/linux/blob/v6.18/drivers/base/bus.c#L493-L510), [hwmon child registration](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/hwmon.c#L973-L984) and [NVMEM attribute registration](https://github.com/torvalds/linux/blob/v6.18/drivers/nvmem/core.c#L373-L436) | Public sysfs topology ABI. **lead-only (copyleft)** for subsystem link targets, the hwmon child parent and the NVMEM alias. |
| S6 | Coreboot 26.09 [DDR4 SPD and DIMM metadata](https://github.com/coreboot/coreboot/blob/26.09/src/device/dram/ddr4.c#L229-L278), Git blob `6ccef7274f88ebf67a21c4312917f7e12f760a95`, and [SMBIOS serial rendering](https://github.com/coreboot/coreboot/blob/26.09/src/lib/smbios.c#L164-L173), Git blob `5172c4862fd31924c534c1b8e1ea49a7fa2e24f9` | **lead-only (copyleft)** for a firmware path that emits the four DDR4 SPD serial bytes in address order as eight hexadecimal digits. SMBIOS does not require all firmware to use that encoding (S9). |
| S7 | Kingston [KSM26ED8/16ME data sheet, 4808620B](https://www.kingston.com/datasheets/KSM26ED8_16ME.pdf), p. 1, and [first-hand ASUS Zenith II Extreme inventory and Linux sensor report](https://agx.fi/it/hardware.html) | Primary for the exact module's integrated SPD and temperature sensor. The owner lists eight modules of that part, already loaded drivers, and eight `jc42` sensor names on one PIIX4 adapter. No per-module SMBIOS/SPD serial pairing is shown. |
| S8 | First-hand [Arch Linux 6.17.9-arch1-1 capture on Gigabyte B550M AORUS ELITE](https://bbs.archlinux.org/viewtopic.php?id=310455) (search `6-0052: 512 byte` and `6-001a`) and [MSI Thin GF63 12VF capture](https://github.com/lm-sensors/lm-sensors/issues/475) (search `SMBus I801 adapter at efa0` and `jc42-i2c-0-18`) | Corroborates same-adapter native binding. Arch shows EEPROM `6-0052` and `jc42` child at `6-001a`. The MSI report shows already bound drivers at `0x50` and `0x18` with 44.8 °C from the latter. The captures do not show a full Type 17 join or justify any other address pairing. |
| S9 | DMTF [SMBIOS DSP0134 3.9.0](https://www.dmtf.org/sites/default/files/standards/documents/DSP0134_3.9.0.pdf), §5.2, §§6.1.2–6.1.3, §§7.17–7.18, tables 71, 73, 75 and 77, Annex A §§4.7–4.8 | Primary for structure bounds and strings, system-memory array use and count, Type 17 array references, occupancy, DDR4 type, identity strings and locators. |
| S10 | Upstream dmidecode tag `dmidecode-3-5`, [memory output fields](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmidecode.c#L4649-L4736), [table decode](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmidecode.c#L5538-L5622), [SMBIOS 3 result](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmidecode.c#L5752-L5776), [sysfs result](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmidecode.c#L6067-L6086), [exit status](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmidecode.c#L6165-L6176) and [structure diagnostics](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmioutput.c#L127-L137). Source blobs `54f59c163dafb8bd0902bcaf3a1ba5f1a0efd7f1` and `42f8d3218c433116b50e9856f86f8412c2dabf0e`. | **lead-only (copyleft)** for successful exit after partial decode and diagnostic streams. |

## Detection and scoped enablement

| Condition | Gate kind | Source |
| --- | --- | --- |
| An already bound `ee1004` I²C client supplies its own direct read-only `eeprom`. The same canonical I²C adapter owns an already bound `jc42` client with a qualified same-client hwmon child. The two native client addresses encode the same documented strap bits. | mechanism | S2, S3 (lead-only pairing), S4, S5 |
| The kernel ownership links agree and each file belongs to the expected client or child. Stop on a known ownership conflict or forced binding, a changed binding or an ambiguous transport pair. | safety | S3/S4 (lead-only access), S5 |
| The SPD describes conventional DDR4, has a supported format covering the required bytes and affirmatively declares a TSE2004av temperature sensor. | mechanism | S1, S3 (lead-only legacy distinction) |
| The qualified module identity matches one populated DDR4 Type 17 record and has no other possible match on either side within the boot snapshot. | mechanism | S1, S6 (lead-only short format), S9, S10 (lead-only diagnostics) |

| Scope | Status | Default enablement |
| --- | --- | --- |
| DDR4 TSE2004av with bound Linux `ee1004` + `jc42` and unique Type 17 identity | S1/S2 establish the SPD capability and electrical mechanism. S3/S4 (lead-only) establish Linux addressing, paging and callback behavior. S5 (lead-only) pins sysfs ownership. S6 (lead-only) supplies the firmware serial representation, S9 defines Type 16/17 and S10 (lead-only) identifies incomplete text reports. S7 corroborates Kingston KSM26ED8/16ME on ASUS Zenith II Extreme. S8 corroborates bound pairs on Gigabyte B550M AORUS ELITE and MSI Thin GF63 12VF. | Experimental when the mechanism and safety gates pass. |
| Legacy SPD that does not declare the sensor, DDR4E, hybrid/NVDIMM, unmatched or unbound clients, alternative EEPROM drivers, and raw bus or non-Linux routes | No path specified here. | Unsupported by this revision. |

### Kernel ownership and same-module pairing

Resolve the I²C namespace entries to canonical nodes under `/sys/devices`.
The immediate adapter and both clients must resolve their `subsystem`
links to `/sys/bus/i2c`. Require the EEPROM client's `driver` link to
`/sys/bus/i2c/drivers/ee1004` and the sensor client's `driver` link to
`/sys/bus/i2c/drivers/jc42`. The direct `eeprom` file must belong to
the EEPROM client. The class entry `/sys/class/hwmon/hwmonN` named
`jc42` must resolve under the sensor client's `hwmon` directory and its
`device` link must resolve to that same sensor client. Its input file
must belong to that child. Count aliases of the same canonical node
once and distinct nodes separately. An additional NVMEM provider file
is an alias of the same EEPROM and supplies no second identity path
(S3–S5).

For an EEPROM client at `0x50`–`0x57`, let the low three address bits
be the physical strap value. The corresponding temperature client
address is `0x18` plus that value, on the same **immediate canonical
adapter** (S2/S3). A common PCI controller, common mux ancestor or
matching address suffix on another adapter cannot establish this
relationship. Addresses identify the two functions of a declared
module sensor. Use the Type 17 match to identify the slot. Do not
probe addresses or try to establish pairing by a hwmon name alone.

### SPD and Type 17 identity

| SPD item | Accepted encoding or interpretation | Source |
| --- | --- | --- |
| Byte 0, SPD size | Bits 6:4 = `010b` (512 bytes), bit 7 clear, and bits 3:0 = `3` or `4` (384 or 512 bytes used), covering the complete identity slice. | S1 §8.1.1 |
| Byte 1, SPD revision | Encoding level 1 and additions level at least 1. This excludes the initial 1.0 format where the thermal declaration was absent. The standard defines additions as extending the existing encoding. | S1 §8.1.2, S3 (lead-only legacy note) |
| Bytes 2 and 3, memory and module type | Byte 2 = `0x0C` for DDR4, and byte 3 bits 7:4 clear for a DRAM-only module. | S1 §§8.1.3–8.1.4 |
| Byte 14, thermal presence | Require `0x80`. Bit 7 declares an incorporated TSE2004av sensor and bits 6:0 are reserved. A clear or unreadable declaration supplies no temperature capability. | S1 §8.1.15 |
| Bytes 325–328, serial | Compare to exactly eight Type 17 hexadecimal digits decoded as four byte pairs in increasing SPD address order. Hex letter case is immaterial. Reject all-zero, all-ones, malformed or absent serials. | S1 §8.5.4, S6 (lead-only Type 17 rendering) |
| Bytes 329–348, part | Printable ASCII with trailing ASCII spaces removed. Compare the full part to the Type 17 value under the same normalization and exact case. Reject empty or placeholder identities. | S1 §8.5.5, S9 |

#### Complete boot inventory

Use one current-boot snapshot of the system memory arrays and their
memory devices. Verify unique Type 16 and Type 17 structure handles and
check each Type 17 array handle against its Type 16 record. Each relevant
system-memory array must have the declared number of Type 17 records,
including empty slots. Withhold attribution if the set of possible
competitors cannot be established (S9).

For binary SMBIOS, validate formatted bounds, terminated string sets
and referenced string indices. Structure handles are little-endian
16-bit fields at `0x02`. Type 16 must reach `0x0F`. Its Use at `0x05`
is `0x03` for system memory, and its device count at `0x0D` is 16-bit.
Type 17 carries its 16-bit array handle at `0x04` and Size at `0x0C`.
Size zero marks an empty socket, `0xFFFF` is unknown, and `0x7FFF`
uses Extended Size at `0x1C`. Memory Type at `0x12` is `0x1A` for DDR4.
A Type 17 used for a complete match must reach formatted length
`0x1B` to supply the serial and part string indices at `0x18` and
`0x1A`. The locator string indices are at `0x10` and `0x11` (S9).

For text inventory, require equivalent record structure, handles,
array counts (`Number Of Devices`), Type 17 `Array Handle`, occupancy,
type and identity fields. Require success,
empty stderr and stdout without inventory diagnostics. dmidecode 3.5
can exit successfully after a partial decode. An invalid structure can
produce a diagnostic on stderr, and a displayed truncated structure
can produce one on stdout (S10, lead-only). Unknown fields on an
otherwise identifiable record are handled as uncertainty below.

#### One-to-one attribution

A complete match requires the full serial and normalized part above
and an affirmatively populated DDR4 Type 17 row. Each member must
have exactly one possible counterpart. Include every possibly DDR4
`ee1004` client even when its temperature partner or declaration does
not qualify. Include any Type 17 row that could describe populated
DDR4 despite unknown type, size or identity. Keep unreadable or
unrecognized identities unresolved. Known empty slots and records
known to describe other memory do not compete. Known incompatible
fields can rule out an unresolved competitor. An unrecognized serial
supplies no comparable bytes. Withhold affected matches on ambiguity
and keep independently proven pairs (S1, S5, S6, S9).

Attach the reading to the matched boot-local Type 17 handle. Use
Device Locator for the display label and include Bank Locator when
Device Locator repeats. If labels are absent or still collide, the
boot-local handle can disambiguate the display. Withhold the reading
if the consumer cannot uniquely address the matched row. Addresses,
adapter numbers, array handles and labels do not establish a join.
Discard a join when its boot inventory or bound endpoints change (S9
and project attribution policy).

## Collection and safety

1. On a DIMM telemetry request, use a directly readable local SMBIOS
   table or the raw result of the existing user-initiated Linux
   memory-detail action (`pkexec dmidecode --type memory`). Retain exit
   status, stdout and stderr for the complete inventory checks above.
   The aggregate inventory cache supplies no per-DIMM identity.
   Keep ordinary aggregate inventory behavior separate. No background
   elevation or polling.
2. From an already bound client's proven `eeprom`, read the
   15-byte prefix at offsets 0–14 for format and capability. For a
   supported DDR4 layout, read 24 bytes at offsets 325–348 for
   identity, including when byte 14 does not qualify for temperature.
   Both reads are bounded within one 256-byte page each. The kernel
   reserves the bus-wide volatile page-selection addresses `0x36`
   and `0x37`, chooses the page when needed, and serializes
   selection and the ensuing EEPROM reads with its driver mutex.
   It leaves the selected page in place. The page command changes
   the address window without writing EEPROM contents or changing
   write protection (S2/S3). This lock does not synchronize external
   firmware or independent raw bus clients.
3. After a unique match, read only the matched child's
   `temp1_input`. The driver serializes this callback with its
   per-sensor update lock. It reads the 16-bit temperature word at
   `0x05` and discards the embedded trip flags. It does not access
   a separate alarm or configuration register. Reading the word does
   not clear a latched interrupt on the specified mechanism (S2/S4).
   The kernel can change configuration during probe, removal and
   power-management transitions. The application only consumes an
   already bound endpoint and does not initiate those operations
   (S4, lead-only).
4. Interpret the complete signed decimal result as integer
   millidegrees Celsius and divide by 1000. The kernel converts the
   signed temperature in 62.5 m°C units with truncation to integer
   m°C. For positive readings, valid values are congruent to 0 or
   62 modulo 125. Accept only positive values at most 125,000 m°C
   as a project plausibility window. The driver has no separate
   temperature-enable attribute. Missing, invalid or failed samples
   stay absent for the affected module (S4, lead-only conversion).
5. Do not open raw I²C, SMBus or I3C devices, scan addresses, force an
   address, create or rebind clients, select pages directly, touch
   alarm attributes, change sensor configuration, write EEPROM or
   control the bus. Keep inventory, utilization and independently
   proven temperatures available. Serial, part and handle data stay
   local to the request and are not persisted or sent.

## Synthetic fixture acceptance

| Case | Required result |
| --- | --- |
| Already bound EEPROM at `0x52` and sensor at `0x1A` on one canonical adapter, valid DDR4 SPD with byte 14 `0x80`, exact Type 17 serial and part, input `44062` | 44.062 °C on the matched Type 17 handle. Board and firmware names are evidence only. |
| Candidate addresses share strap bits but are on different immediate adapters, including sibling mux channels or adapters on the same controller | No cross-adapter match or temperature. |
| Type, size or encoding unsupported, revision 1.0, thermal bit clear or unreadable, or reserved thermal byte bits set | No temperature for that module. A readable DDR4 identity without an affirmative thermal declaration still counts in duplicate detection. |
| Wrong driver, missing or unrelated hwmon child, path escaping the canonical client, alias-only EEPROM, absent binding or wrong subsystem link | No unqualified file read or inferred pairing. Count an independently known EEPROM identity as a competitor. |
| Missing or short EEPROM data, unsupported serial string, or duplicate serial and part on either side | Withhold ambiguous attribution. Keep independently proven pairs and ordinary inventory. |
| Two valid matches and one temperature error, or one candidate loses its binding during the request | Keep the independent valid reading and omit the affected one. Requalify any recreated binding. |
| Text inventory has a plausible first array followed by diagnostics, an invalid Type 17 array reference, or a Type 16 count mismatch including missing empty slots | No temperature attribution. Retain ordinary aggregate inventory behavior. |
| Input values `0`, `-62`, `62`, `125`, `44062`, `44063`, `125000` and `125062` m°C | Accept `62`, `125`, `44062` and `125000` only. Read one input attribute and no alarm or control file. |
| No compatible endpoints or a known competing bus owner | Unavailable. No probing, force, module loading, binds, page commands issued by the application, or background elevation. |

## Open questions

- Non-blocking for the Experimental DDR4 scope: the same-module and slot joins must both succeed at runtime.
  A public, same-boot capture showing SPD capability and raw identity,
  Type 17 identity and live temperature from the same qualified pair
  has not been established. S7/S8 substantiate the hardware mechanism
  and native binding in separate evidence records. The ABI does not
  attest how an administrator instantiated a client. This scope
  relies on the kernel-owned binding and the affirmative module
  declaration. A known forced binding or bus ownership conflict
  excludes the affected pair.
- Non-blocking for the Experimental DDR4 scope: unrecognized firmware identity formats do not qualify.
  An alternative Type 17 serial encoding needs independent provenance
  before it can supply a match.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-09 | Propose Implementation-ready status for the Experimental, already bound Linux DDR4 TSE2004av mechanism with a same-adapter function pair and one-to-one Type 17 attribution. Pin the standalone kernel ownership and diagnostic-free Type 16/17 inventory sources. |
