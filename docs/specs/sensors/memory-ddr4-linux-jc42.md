# Spec: Linux DDR4 DIMM temperature through jc42 and EE1004

| Field | Value |
| --- | --- |
| Revision | 3 |
| Status | Draft — not implementation-ready |
| Scope | Read-only research for DDR4 DIMMs that advertise a TSE2004av sensor and have an already bound Linux `jc42` and `ee1004` pair. Slot attribution remains disabled until the active firmware identity mechanism is established. |
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
| S9 | DMTF [SMBIOS DSP0134 3.9.0](https://www.dmtf.org/sites/default/files/standards/documents/DSP0134_3.9.0.pdf), §§5.2.1–5.2.2, §§6.1.2–6.1.3, §§7.17–7.18, §7.49, Annex A §§2–3 and §§4.7–4.8 | Primary for entry-point anchors and checksums, binary table bounds and terminator, Type 16/17 structure layout and links, occupancy, DDR4 type, identity strings and locators. |
| S10 | Upstream dmidecode tag `dmidecode-3-5`, [string sanitation](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmidecode.c#L116-L159), [Type 127 output](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmidecode.c#L5338-L5340), [table traversal](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmidecode.c#L5525-L5622), [SMBIOS 3 entry point](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmidecode.c#L5721-L5776), [sysfs and fallback](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmidecode.c#L6057-L6126), [record formatting](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmioutput.c#L47-L63), [diagnostic output](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmioutput.c#L127-L137) and [fallback device option](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmiopt.c#L270-L299). Git blobs `54f59c163dafb8bd0902bcaf3a1ba5f1a0efd7f1` and `42f8d3218c433116b50e9856f86f8412c2dabf0e` for the first two files. | **lead-only (copyleft)** for partial success, structural Type 127 output, field sanitation and fallback dispatch. |
| S11 | Linux v6.18 [firmware DMI table ABI](https://github.com/torvalds/linux/blob/v6.18/Documentation/ABI/testing/sysfs-firmware-dmi-tables), [binary registration](https://github.com/torvalds/linux/blob/v6.18/drivers/firmware/dmi_scan.c#L722-L756) and [sysfs binary permissions](https://github.com/torvalds/linux/blob/v6.18/include/linux/sysfs.h#L358-L359) | Public fixed binary source paths. **lead-only (copyleft)** for administrator-read permissions. |
| S12 | Protectli [VP2410 datasheet](https://eu.protectli.com/wp-content/uploads/2024/07/VP2410-Datasheet-20240628.pdf), *Memory Slot* entry, and Dasharo [VP2410 Hardware Configuration Matrix](https://docs.dasharo.com/variants/protectli_vp2410/hardware-matrix/) | Primary for the single `DIMM1` DDR4 SO-DIMM slot and validation module list. No native temperature binding or slot-to-sensor capture. |
| S13 | OSADL [rack 0, slot 5 hardware profile](https://www.osadl.org/Profile-of-system-in-rack-0-slot-5.qa-profile-r0s5.0.html?id=1352), BIOS, DIMMs and DMI sections, and Shuttle [SH370200.106](https://global.shuttle.com/products/productsDownload?c=xpc-cube&pn=SH370R6+V2), build March 25, 2020, [OEM archive](https://download.shuttle.eu/Mirror/MiniPC%26AIO/cube/SH370R6_V2/Bios/SH370200.106.ZIP). Archive SHA-256 `b956d8ee95d93ba8d98c7def38b1a680bf9df6c2f2764bf3436b21d3281e5dfa`, 16 MiB image SHA-256 `664fc58cba98822fc17d586e8c77f7be54d7053910f5d80af993f1465605bca4`. | First-hand OSADL report on Shuttle SH370V2, AMI 1.06. Static vendor artifact for the Type 17 writer in `SiInitDxe`, FFS GUID `acd28235-075b-48b5-98a1-da04fcaf84f3`, PE SHA-256 `df1bc935a127b4132ee025c697896f2f96428f55ce258ba1dad7d554c4a5f560`, RVA `0xD37`–`0xD7D`. Research only. |

## Detection and scoped enablement

| Condition | Gate kind | Source |
| --- | --- | --- |
| An already bound `ee1004` I²C client supplies its own direct read-only `eeprom`. The same canonical I²C adapter owns an already bound `jc42` client with a qualified same-client hwmon child. The two native client addresses encode the same documented strap bits. | mechanism | S2, S3 (lead-only pairing), S4, S5 |
| The kernel ownership links agree and each file belongs to the expected client or child. Stop on a known ownership conflict or forced binding, a changed binding or an ambiguous transport pair. | safety | S3/S4 (lead-only access), S5 |
| The SPD describes conventional DDR4, has a supported format covering the required bytes and affirmatively declares a TSE2004av temperature sensor. | mechanism | S1, S3 (lead-only legacy distinction) |
| The active firmware's Type 17 identity encoding is established independently of the candidate matches, and the qualified module has exactly one possible counterpart in a complete binary boot snapshot. | mechanism | S1, S6 (lead-only example), S9, S11 (lead-only permissions) |

| Scope | Status | Default enablement |
| --- | --- | --- |
| DDR4 TSE2004av with bound Linux `ee1004` + `jc42` and unique Type 17 identity | S1/S2 establish the SPD capability and electrical mechanism. S3/S4 (lead-only) establish Linux addressing, paging and callback behavior. S5 (lead-only) pins sysfs ownership. S6 (lead-only) shows one firmware rendering, and S9 defines Type 16/17 without specifying a serial byte order. S7 corroborates Kingston KSM26ED8/16ME on ASUS Zenith II Extreme. S8 corroborates bound pairs on Gigabyte B550M AORUS ELITE and MSI Thin GF63 12VF. None establishes the active producer's serial encoding or a physical slot mapping on those systems. | Disabled pending a supported attribution mechanism and complete inventory. |
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
| Bytes 325–328, serial | Four serial bytes in SPD address order. For a producer independently established to render these bytes in address order, compare with exactly eight Type 17 hexadecimal digits, preserving leading zeros. Hex letter case is immaterial. Reject all-zero, all-ones, malformed or absent serials. | S1 §8.5.4, S6 (lead-only example), S9 |
| Bytes 329–348, part | Printable ASCII with trailing ASCII spaces removed. Compare the full part to the Type 17 value under the same normalization and exact case. Reject empty or placeholder identities. | S1 §8.5.5, S9 |

#### Complete boot inventory

Use the paired binary SMBIOS entry point and DMI table from the
current boot. Apply an acquisition size limit before reading the table.
Verify the entry point's declared length and byte-sum checksum, with
little-endian length and count fields (S9). For `_SM3_`, require the
`0x18`-byte entry point (length at `0x06`), revision 1 at `0x0A` and
nonzero table maximum size at `0x0C`. The table must fit that maximum.
For `_SM_`, require the `0x1F`-byte entry point (length at `0x05`),
intermediate `_DMI_` anchor at `0x10` and an independent checksum of
its 15 bytes. The binary table length at `0x16` and structure count at
`0x1C` must agree with traversal. Treat physical addresses as metadata.

Traverse all structures with their formatted lengths and double-NUL
string-set terminators. Require a final Type 127 with formatted length
4 and an empty, terminated string set. The SMBIOS 3 maximum size is a
bound only. A complete-looking prefix without Type 127 is insufficient.
Reject a premature end marker with further structures, truncated
header or string data, or an unsupported entry point (S9).

Verify unique Type 16 and Type 17 structure handles and
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

The ordinary `dmidecode --type memory` output cannot prove a complete
inventory. It omits Type 127. On SMBIOS 3, dmidecode 3.5 can reach the
end of a truncated structure-aligned prefix and exit successfully
without a diagnostic, even if every visible Type 16 count is satisfied
(S10, lead-only).

#### One-to-one attribution

Confirm the active producer's serial encoding before evaluating any
serial join. SMBIOS defines Serial Number as a string and does not
specify its relation to SPD byte order (S9). S6 covers one coreboot
writer path. A firmware brand, version string, eight-digit form or
apparently unique match does not establish that the active writer uses
that path. Trying both orders and choosing the one with a complete
match cannot establish a unique physical mapping. Two same-part DIMMs
with reversed serials can swap slots while each order yields a complete
one-to-one join.

For a proven producer, a complete match requires the full serial and
normalized part above and an affirmatively populated DDR4 Type 17 row. Each member must
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

1. On a DIMM telemetry request, read the fixed, already readable
   `/sys/firmware/dmi/tables/smbios_entry_point` and
   `/sys/firmware/dmi/tables/DMI` as a single boot snapshot (S11).
   Both are read-only administrator attributes in the pinned kernel.
   If either file is unavailable, omit temperatures. The ordinary
   memory-detail action and aggregate cache remain separate. Any
   future privileged binary retrieval must use only those fixed inputs
   with bounded process output during the explicit user action and no
   caller-specified output file or second privilege request. This draft
   authorizes no new privileged retrieval.
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
| Hypothetical proven active producer using address order, validated binary entry point and terminal Type 127, bound EEPROM at `0x52` and sensor at `0x1A` on one canonical adapter, DDR4 SPD byte 14 `0x80`, exact Type 17 serial and part, input `44062` | A future qualified scope would report 44.062 °C on the matched Type 17 handle. This fixture supplies a synthetic producer proof only. |
| Same-part modules have SPD serials `12 34 56 78` and `78 56 34 12`, and firmware displays eight-digit serials without independently established order | Withhold both temperatures even if the direct and reversed interpretations each give one-to-one joins. |
| Candidate addresses share strap bits but are on different immediate adapters, including sibling mux channels or adapters on the same controller | No cross-adapter match or temperature. |
| Type, size or encoding unsupported, revision 1.0, thermal bit clear or unreadable, or reserved thermal byte bits set | No temperature for that module. A readable DDR4 identity without an affirmative thermal declaration still counts in duplicate detection. |
| Wrong driver, missing or unrelated hwmon child, path escaping the canonical client, alias-only EEPROM, absent binding or wrong subsystem link | No unqualified file read or inferred pairing. Count an independently known EEPROM identity as a competitor. |
| Missing or short EEPROM data, unsupported serial string, or duplicate serial and part on either side | Withhold ambiguous attribution. Keep independently proven pairs and ordinary inventory. |
| Two valid matches and one temperature error, or one candidate loses its binding during the request | Keep the independent valid reading and omit the affected one. Requalify any recreated binding. |
| SMBIOS 3 ends cleanly at a record boundary after a complete first array, but Type 127 is absent. Filtered `--type memory` exits successfully with no diagnostics. | No temperature attribution. An apparent Type 127 header followed by truncation or a marker-shaped string within a field is also insufficient. |
| Bad entry-point checksum, invalid Type 17 array reference, or a Type 16 count mismatch including missing empty slots | No temperature attribution. Retain ordinary aggregate inventory behavior. |
| Input values `0`, `-62`, `62`, `125`, `44062`, `44063`, `125000` and `125062` m°C | Accept `62`, `125`, `44062` and `125000` only. Read one input attribute and no alarm or control file. |
| No compatible endpoints or a known competing bus owner | Unavailable. No probing, force, module loading, binds, page commands issued by the application, or background elevation. |

### Existing privileged text path research

The pinned dmidecode 3.5 traversal supports a bounded completion
candidate for the existing user-initiated elevation action. Use the
distribution's trusted dmidecode executable with fixed
`--dev-mem /dev/null --type 16,17,127` arguments, with normal sysfs
selection and non-quiet output. The fixed fallback device supplies
no physical memory. The command has no output-file option. Capture
bounded stdout, stderr and exit status. Require success, empty stderr,
the pinned version, the `Getting SMBIOS data from sysfs.` and
`SMBIOS 3.x.y present.` banners (numeric minor and document revisions),
and no fallback or decode diagnostics (S10).

Require one terminal, unindented Type 127 header
`Handle 0xHHHH, DMI type 127, 4 bytes`, where `HHHH` is its four
hexadecimal handle digits, followed by its own unindented
`End Of Table` record body. The decoder emits the header before
checking the string extent. It emits the body only after the record
fits, then stops at Type 127 for SMBIOS 3. Parse the complete output
as records. Field strings in Type 17 have control and non-ASCII bytes
replaced by dots in this mode and cannot add an unindented record
line. A header alone, embedded match, trailing records or diagnostic
leaves the result unusable (S10).

This candidate can distinguish the clean-truncation case in the
pinned decoder. Its output does not carry the binary entry point,
the Type 127 string bytes or unchanged Type 17 identity bytes. It is
a completion witness for research, separate from the binary
attribution requirements of this draft.


## Open questions

- Blocking: identify the active firmware's Type 17 producer and establish its serial format independently of the match, or prove a physical slot mapping that makes serial order irrelevant.
  A public, same-boot capture showing SPD capability and raw identity,
  Type 17 identity and live temperature from the same qualified pair
  has not been established. S7/S8 substantiate the hardware mechanism
  and native binding in separate evidence records. The ABI does not
  attest how an administrator instantiated a client. This scope
  relies on the kernel-owned binding and the affirmative module
  declaration. A known forced binding or bus ownership conflict
  excludes the affected pair.
- The SH370V2 report (S13) shows two DDR4 SPD 1.1 modules of part
  `BL16G32C16U4RL.M16FE` with the TSE2004 declaration. The serials
  displayed for `ee1004` clients `7-0051` and `7-0053`, `E33ECD9A`
  and `E33ED020`, agree with the serials and part in Type 17 handles
  `0x003C` and `0x003E`, respectively. The pinned firmware's writer
  renders four successive saved module serial bytes as hexadecimal
  byte pairs in ascending address order. This evidence is confined to
  that writer and the observed inventory. Physical SPD strap mapping,
  a native bound `jc42` endpoint and a passive way to attest the
  active writer remain open.
- The Protectli VP2410 is a single-slot lead (S12). Its documented
  `DIMM1` connector does not supply a same-platform native `ee1004` and
  `jc42` binding or prove that an exact supported module supplies the
  thermal endpoint. No single-slot profile is qualified.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-09 | Initial Linux DDR4 TSE2004av proposal with a same-adapter function pair and one-to-one Type 17 attribution. |
| 2 | 2026-11-09 | Return to Draft. Require proof of the active firmware identity mechanism and a complete entry-point-verified binary inventory. Record the bounded dmidecode completion candidate separately. |
| 3 | 2026-11-09 | Record the exact Shuttle firmware writer and matching OSADL inventory as research evidence. |
