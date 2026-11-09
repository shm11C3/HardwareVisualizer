# Spec: Linux DDR5 DIMM attribution through SPD5118 and SMBIOS Type 17

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Draft — not implementation-ready |
| Scope | Candidate Linux DDR5 DIMM temperatures through already bound SPD5118 I²C clients with a complete SMBIOS inventory and a proven producer-specific Type 17 identity. |
| Issue phase | [#2526](https://github.com/shm11C3/HardwareVisualizer/issues/2526) |

The [approved exact ODROID-H4 scope at rev 2](https://github.com/shm11C3/HardwareVisualizer/blob/6d3ac497aab1b78a38e3334b0ef94467e06fb0fb/docs/specs/sensors/memory-ddr5-odroid-h4-spd5118.md)
remains the current implementation pin in the
[ODROID-H4 specification](memory-ddr5-odroid-h4-spd5118.md).
This draft replaces the general rev 3 proposal for that document.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Hardkernel [ODROID-H4 hardware specification](https://www.hardkernel.com/blog-2/odroid-h4-series/) and Dasharo [H4 hardware validation matrix](https://docs.dasharo.com/variants/hardkernel_odroid_h4/hardware-matrix/) | Primary for the single DDR5 SO-DIMM socket and the exact Samsung M425R1GB4BB0-CWMOD validated in the H4+ configuration. The matrix supplies no temperature claim. |
| S2 | MSI [Cubi service guide v1.0](https://storage-asset.msi.com/global/pdf/Cubi_B0B1v1.0_ServiceGuide_0411-2.pdf#page=51), p. 51, System Info screenshot | Identifies Samsung M425R1GB4BB0-CWMOD as an 8 GB DDR5-5600 module. |
| S3 | Dasharo [ODROID-H4 UEFI v0.9.1 release](https://docs.dasharo.com/variants/hardkernel_odroid_h4/releases/#v091-2025-09-03), September 3, 2025, coreboot `7d10ea62`, [build config](https://github.com/Dasharo/coreboot/blob/7d10ea62/configs/config.hardkernel_odroid_h4), [board Kconfig](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/mainboard/hardkernel/odroid-h4/Kconfig), [memory configuration](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/mainboard/hardkernel/odroid-h4/romstage_fsp_params.c), and [SMBus SPD read](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/soc/intel/common/block/smbus/smbuslib.c) | **lead-only (copyleft)** for exact board strings, socket address and the 1024-byte boot-time DDR5 SPD read. This is the firmware's own source. |
| S4 | Dasharo coreboot at `7d10ea62`, [UEFI BIOS version string](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/vendorcode/dasharo/smbios.c), [Intel DIMM information](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/soc/intel/alderlake/romstage/romstage.c), [saved serial field](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/soc/intel/common/smbios.h#L9-L10), [byte copy](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/soc/intel/common/smbios.c#L41-L46), and [Type 17 output](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/lib/smbios.c#L164-L174) | **lead-only (copyleft)** for the observed firmware's Type 17 derivation from the FSP's SPD information, short serial format and part string. |
| S5 | JEDEC *DDR5 DIMM Label*, JESD401-5D, [public-use document mirror](https://studylib.net/doc/28582427/jesd401-5d), §§2.3–2.5, pp. 11–12, and DMTF [SMBIOS DSP0134 3.9.0](https://www.dmtf.org/sites/default/files/standards/documents/DSP0134_3.9.0.pdf#page=100), §§7.17–7.18 | Primary for SPD manufacturing identity field offsets and byte order, part offsets, and SMBIOS array and memory-device structure semantics. SMBIOS defines Serial Number as a string without specifying an SPD byte mapping. SMBIOS §§5.2.1–5.2.2, 6.1, 7.49 and Annex A define the table bounds, structure termination and End-of-Table record. |
| S6 | Linux [SPD5118 hwmon ABI](https://docs.kernel.org/hwmon/spd5118.html) and [generic hwmon ABI, v6.18](https://www.kernel.org/doc/html/v6.18/hwmon/sysfs-interface.html) | Public ABI for read-only temperature and EEPROM attributes, the temperature enable attribute, millidegrees Celsius and read-to-clear alarm attributes. |
| S7 | Linux v6.18, [SPD5118 driver](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/spd5118.c), November 30, 2025, Git blob `5da44571b6a0ce43b621faba9e67d81491ff0b28`, [thermal reads](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/spd5118.c#L89-L185), [EEPROM read and addressing](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/spd5118.c#L338-L490), [device registration](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/spd5118.c#L534-L568), [chip and bus mode checks](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/spd5118.c#L630-L745), and [Intel I801](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/busses/i2c-i801.c) | **lead-only (copyleft)** for the driver's chip and temperature capability checks, temperature quantization, read-only 1024-byte NVMEM, protocol and locking, and the ODROID controller's SMBus-only capability. |
| S8 | [ODROID-H4 base model PCI inventory](https://forum.level1techs.com/t/mini-pc-home-server/218042/18), October 14, 2024, and Dasharo [board device tree](https://github.com/Dasharo/coreboot/blob/7d10ea62/src/mainboard/hardkernel/odroid-h4/devicetree.cb) | First-hand observation of onboard PCI SMBus `00:1f.4`, PCI `8086:54A3`. The device tree is **lead-only (copyleft)** for the enabled controller. |
| S9 | Linux [regmap, v6.18](https://github.com/torvalds/linux/blob/v6.18/drivers/base/regmap/regmap.c), paged ranges and locked reads | **lead-only (copyleft)** for serialized page selection and read and the lack of post-read page restoration. |
| S10 | Linux v6.18 [I²C bus](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-core-base.c#L699-L705), [adapter registration](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-core-base.c#L1557-L1565), [client registration](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-core-base.c#L999-L1004), [device core subsystem link](https://github.com/torvalds/linux/blob/v6.18/drivers/base/bus.c#L493-L510), [character-device class](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-dev.c#L611-L614), and [I²C sysfs ABI](https://github.com/torvalds/linux/blob/v6.18/Documentation/i2c/i2c-sysfs.rst) | **lead-only (copyleft)** for the adapter and client subsystem targets and the distinct character-device class. The public I²C sysfs ABI describes the kernel-owned adapter/client relationships and logical buses behind muxes. |
| S11 | Linux v6.18 [hwmon device registration](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/hwmon.c#L973-L984) and [NVMEM sysfs attributes](https://github.com/torvalds/linux/blob/v6.18/drivers/nvmem/core.c#L373-L436) | **lead-only (copyleft)** for the hwmon child's physical parent and the legacy client `eeprom` and provider `nvmem` files sharing one provider. |
| S12 | AMD [*Ubuntu Tuning Guide for AMD EPYC 9005 Series Processors*](https://docs.amd.com/api/khub/documents/2tw6thFYsT7~8Hl2Cdo68g/content#page=30), document 58470 rev 1.0, §2.10.1, pp. 26–28 | First-hand Type 17 examples with eighteen hexadecimal digits and repeated Device Locators across distinct Bank Locators and handles. The guide supplies no paired EEPROM dump, producer byte-order rule or temperature. |
| S13 | [Framework Laptop 13 AMD Linux hardware report](https://www.kamprad.net/hardwareinfo/framework13-amd-hardinfo2_report.html), SPD and hardware sensor sections | First-hand report of two bound `spd5118` clients for one part with distinct SPD serials and separate temperatures of 37.00 °C and 34.75 °C. The Type 17 records have distinct handles and banks but repeat Device Locator `DIMM 0`. The report omits Type 17 serials and its SPD and inventory size fields disagree. It corroborates multi-device telemetry and the locator collision only. |
| S14 | Micron [*DDR5 SDRAM RDIMM Core*](https://www.mouser.com/datasheet/2/671/ddr5_rdimm_core-3310292.pdf), Serial Presence Detect section, SPD EEPROM block map | Primary for the base SPD CRC covering bytes 0–509 and manufacturing data starting at byte 512. |
| S15 | dmidecode 3.5, upstream release tag `dmidecode-3-5` in the Git mirror. [Table decoding and diagnostics](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmidecode.c#L5538-L5622), [SMBIOS 3 result](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmidecode.c#L5752-L5776), [exit status](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmidecode.c#L5979-L5983), [structure diagnostic output](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmioutput.c#L127-L137), [Type 127 rendering](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmidecode.c#L5338-L5340), [sysfs acquisition](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmidecode.c#L6063-L6080), and [device fallback option](https://github.com/mirror/dmidecode/blob/dmidecode-3-5/dmiopt.c#L270-L299). Git blobs `54f59c163dafb8bd0902bcaf3a1ba5f1a0efd7f1`, `42f8d3218c433116b50e9856f86f8412c2dabf0e` and `fa84f22cf7426215be7b1043a7920014988f1d0d`. | **lead-only (copyleft)** for the selected-type output and silent SMBIOS 3 clean-prefix truncation, as well as explicit malformed and truncated diagnostics. |
| S16 | Linux v6.18 [SMBIOS table walk](https://github.com/torvalds/linux/blob/v6.18/drivers/firmware/dmi_scan.c#L88-L145), [SMBIOS 3 entry point](https://github.com/torvalds/linux/blob/v6.18/drivers/firmware/dmi_scan.c#L649-L669), [sysfs entry point and table export](https://github.com/torvalds/linux/blob/v6.18/drivers/firmware/dmi_scan.c#L764-L801), and [administrative read mode](https://github.com/torvalds/linux/blob/v6.18/include/linux/sysfs.h#L373-L380). Git blobs `70d39adf50dcaaa7c99b48fe8e9045af5ea24d00` and `9a25a29116528fab77d667390dd3ef5957b86554`. | **lead-only (copyleft)** for kernel trimming, raw table export, and default `0400` permissions. |

## Detection and scoped enablement

| Condition | Gate kind | Source |
| --- | --- | --- |
| An existing I²C client belongs to the kernel `spd5118` driver. Identity requires its direct `eeprom`. Temperature requires a qualified same-client `spd5118` hwmon child. | mechanism | S6, S7/S10/S11 (lead-only) |
| Kernel-owned sysfs ancestry, subsystem links and exact driver binding agree. Each EEPROM or hwmon attribute is opened only through its proven owner. Stop on changed ownership or a known bus ownership conflict. | safety | S6, S7/S9/S10/S11 (lead-only) |
| The active inventory producer's SPD-to-Type 17 mapping is proven and the bound hub uniquely matches one populated DDR5 Type 17 record in a complete current-boot inventory. This gate is unresolved for the general scope. | mechanism | S4 (lead-only exact firmware evidence), S5, S12, S15/S16 (lead-only table behavior) |
| The same child reports `temp1_enable` equal to `1` before the temperature read. | mechanism | S6, S7 (lead-only visibility) |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Linux DDR5, already bound SPD5118 I²C ABI and one-to-one Type 17 identity | Best-evidenced configuration: DMI `HARDKERNEL` / `ODROID-H4`, `Dasharo (coreboot+UEFI) v0.9.1`, Samsung `M425R1GB4BB0-CWMOD`, one DDR5 SO-DIMM, client `0x52` under Intel I801 PCI `0000:00:1f.4` (`8086:54A3`). S1/S2/S8 corroborate the module and platform, with S3/S4/S8 (lead-only) pinning firmware, slot address, controller configuration and short serial format. S5/S6 define identity fields and the ABI. S7/S9/S10/S11 (lead-only) supply device registration, paging/locking and canonical ownership. S12 shows the long syntax without producer provenance. S13 corroborates multiple bound temperature endpoints and duplicate device labels. S15/S16 (lead-only) pin the inventory completeness limit. | Disabled pending a proven inventory producer mapping and completion contract. |
| Unbound hubs, other drivers or bus mechanisms, DDR4 `jc42`, and Windows routes | Separate scopes in [#2526](https://github.com/shm11C3/HardwareVisualizer/issues/2526). | Unsupported by this revision. |

### Kernel ownership

Use the existing bound clients in the kernel's I²C `spd5118` driver
namespace and the kernel hwmon class. A qualified client resolves under
`/sys/devices` with its `driver` link targeting
`/sys/bus/i2c/drivers/spd5118`. Its immediate adapter and the client
each have a kernel-owned `subsystem` link resolving to `/sys/bus/i2c`.
Reject class targets such as `/sys/class/i2c-adapter` and
`/sys/class/i2c-dev`. The latter is the separate character-device
class. The adapter can be a logical bus behind a kernel-owned mux
(S10, lead-only for the subsystem links).

A class entry `/sys/class/hwmon/hwmonN` named `spd5118` resolves to
a child under the client's `hwmon` directory. Its `device` symlink
must resolve to the same canonical client as the direct `eeprom` file's
canonical parent. Require the named attributes to belong to those
canonical nodes. Multiple aliases of the same canonical client and
same hwmon child count once. Distinct clients always count separately.
Include every proven bound client in the identity candidate set,
including clients with a missing, foreign or competing hwmon child.
Such a client cannot supply a temperature. Use its SPD identity if
EEPROM ownership is independently proven and the identity is readable.
Otherwise retain it as an unresolved candidate.
A separate provider `nvmem` attribute exposes the same underlying
NVMEM. Use only the direct client `eeprom` for identity
(S6, S7/S11 lead-only for registration and aliasing).

### Type 17 inventory

Use a complete snapshot of the current boot's system memory inventory.
Identify each Type 17 row by its structure handle and verify its
Type 16 physical memory array handle. Require unique handles and the
declared number of slots for each relevant system-memory array,
including empty Type 17 rows. Empty slots and records known to describe
other memory mechanisms do not compete. A record that could be populated
DDR5 remains a possible candidate when its type, size or identity is
unknown. If parsing or enumeration cannot establish a complete set of
possible competitors, withhold attribution (S5).

For a binary inventory candidate, use the kernel-owned files
`/sys/firmware/dmi/tables/smbios_entry_point` and
`/sys/firmware/dmi/tables/DMI` from the same boot. Validate entry point
signature, length and checksum, with the intermediate anchor and
checksum for SMBIOS 2. Traverse the entire table within the advertised
bounds and a finite read cap. Require in-bounds formatted areas and
terminated string sets. For SMBIOS 2 require the exact declared byte
length and count. For SMBIOS 3 the entry point gives a maximum size,
so require a complete final Type 127 with length 4 and an empty string
set in the actual table. Linux can trim the exported table, which
makes byte count alone insufficient on SMBIOS 3. Reject a missing or
malformed marker, even after a valid prefix. Linux v6.18 exposes both
files with mode `0400`; a direct read is viable only where already
permitted. (S5, S16 lead-only)

A populated record must reach formatted length `0x1B` to supply the
part string index at `0x1A`, serial at `0x18`, Device Locator at
`0x10` and Bank Locator at `0x11`. Validate the required indices
against their record's string set. Apply the uncertainty rule below
to missing values in a structurally valid candidate. (S5)

The existing `dmidecode --type memory` output cannot prove completion.
dmidecode 3.5 may finish at a structure boundary in SMBIOS 3 with a
successful exit and empty stderr, without showing that later arrays
were omitted. An explicit Type 127 filter can expose the terminal
record after decoding it. A fixed `dmidecode --type 16,17,127` capture
is a candidate for the existing user-initiated inventory action. It
would need affirmative sysfs source provenance, success, empty
stderr, structurally complete output and a terminal Type 127 record
with length 4 and the decoded `End Of Table` body. A header alone
can precede a failed structure decode. A fixed `--dev-mem /dev/null`
can prevent physical-memory fallback. The exact text capture still
requires review before use. (S15, lead-only)

## Identity and temperature facts

| Item | Interpretation | Source |
| --- | --- | --- |
| SPD manufacturing serial at offsets 517–520 | JEDEC specifies the four SPD bytes in increasing address order. Their Type 17 representation depends on the active firmware producer. | S5 |
| Type 17 serial of exactly eight hexadecimal characters | The pinned Dasharo producer renders four saved FSP serial bytes in array order. The character count cannot establish that the same rule applies on another firmware or that its input bytes were supplied in JEDEC order. | S4 (lead-only exact firmware), S5 |
| Type 17 serial of exactly eighteen hexadecimal characters | AMD shows this syntax. It is consistent with two manufacturer bytes, a location byte, two BCD date bytes and four serial bytes, but syntax and plausible fields cannot prove the producer's mapping. No general long-form match is qualified. | S5, S12 |
| SPD module part at offsets 521–550 | For a proven equivalent firmware part string, compare complete case-sensitive printable ASCII values after removing trailing ASCII spaces. | S4 (lead-only firmware output), S5 |
| `temp1_enable` | Standard hwmon enable attribute exposed by the driver. Read only. Value `1` is required. | S6, S7 (lead-only visibility) |
| `temp1_input` | One temperature in millidegrees Celsius. The pinned driver reports 250 m°C increments and checks thermal capability before registering the device. | S6, S7 (lead-only quantization and capability check) |

Establish the active producer's mapping independently before comparing
identity bytes. A matching part, hex length, plausible BCD prefix or
unique apparent match cannot select a serial byte order. Do not try
alternative byte orders until a match appears. For a proven hex
format, preserve leading zeros and allow either hex case. Reject
blank, malformed or placeholder identities, including all-zero and
all-ones four-byte serials. Reject empty or non-printable ASCII parts
and placeholder strings such as `Not Specified` or `Unknown`. Do not
truncate, prefix match or case-fold parts. The base SPD CRC excludes the
manufacturing bytes (S14). Treat the match as boot-local attribution
under the kernel and firmware's inventory contract.

### One-to-one attribution

- A complete candidate pair consists of one qualified bound client,
  one populated DDR5 Type 17 record, a serial that matches in full
  under the proven producer mapping and an equal normalized part.
  Treat a record with an unproven mapping as unresolved even when
  its printed serial resembles a supported syntax.
- Require that each member of a complete pair has exactly one possible
  counterpart, counting complete matches and unresolved candidates.
  A client without a qualified hwmon child still competes using its
  known identity, or as an unresolved candidate if its identity cannot
  be established. An in-scope Type 17 row with missing or unrecognized
  identity also remains unresolved. Retain these candidates unless a
  supported, known field
  rules out the pair, such as a different normalized part or a
  different serial in a proven comparable format. A malformed serial
  or an unproven producer format provides no comparable serial bytes. If a client cannot be proven
  to own its EEPROM, do not read it to resolve uncertainty.
- Withhold the affected attribution on competition. A known unrelated
  duplicate or unresolved peer with incompatible known fields does not
  suppress an independent pair. A missing candidate with no
  distinguishing fields may block all pairs. A temperature read
  failure after the join affects only that module. Keep known empty
  slots and confirmed unmatched populated records without temperatures.
- Attach the reading to its matching Type 17 handle in the boot
  snapshot. Use Device Locator as the slot label and include Bank
  Locator when Device Locators repeat. When the combined labels still
  repeat or a label is missing, the rendered row can append a boot-local
  Type 17 handle, such as `(Type 17 0x003B)`, to the available label.
  If neither label is present, the handle can label that row by itself.
  Do not persist the handle or its rendered suffix. Withhold the
  temperature if the consuming path cannot uniquely address the
  matched row. The handle and labels do not infer the SPD connection.
- Do not join by client address, bus or mux index, hwmon number,
  enumeration order, array handle alone or labels. Discard the join
  when its boot inventory or bound endpoint set changes. Revalidate the
  identity and ownership before using a recreated or renumbered node.

## Collection and safety

1. A future qualified scope runs only on a DIMM telemetry request
   using a complete, current-boot inventory. Any privileged inventory
   capture belongs to the existing explicit Linux detailed inventory
   action and requires its own reviewed fixed capture contract. Keep
   ordinary aggregate inventory separate. Its persisted `MemoryInfo`
   cache supplies no per-DIMM identity. No background elevation or
   serial persistence is in scope.
2. Qualify kernel ownership before file reads. For each bound client
   with proven EEPROM ownership, read one bounded 39-byte identity slice at offsets
   512–550 through its direct read-only `eeprom` (S5–S7). A short or
   failed read leaves the client's identity unresolved. No full SPD
   dump, alternative NVMEM file or wider address sweep is needed.
3. The pinned driver exposes a 1024-byte read-only EEPROM and
   serializes EEPROM access with a per-device mutex. In 8-bit
   addressed mode the slice stays on 128-byte page 4 at in-page
   offsets 0–38. The kernel may update the volatile MR11 page bits
   2:0 under the register-map lock while preserving the other bits.
   It does not restore the old page after a read. A subsequent actual
   temperature transfer selects page 0 under the same register-map
   lock. In 16-bit addressed mode the kernel uses a two-byte address
   and a combined I²C transfer without MR11 page selection. The
   driver admits that mode only where native I²C combined transfers
   are available. The application follows the bound ABI in either
   case. These locks cover the kernel's own accesses and do not
   establish arbitration with firmware or independent raw bus
   clients (S7/S9, lead-only).
4. After the join, read only `temp1_enable` and `temp1_input` through
   the matched child. Require enabled state and a complete signed
   decimal temperature greater than 0 and at most 125,000 m°C in
   steps of 250 m°C. Convert m°C to °C. An unavailable, disabled or
   invalid sample remains absent for its matched module. Do not
   access alarm attributes, whose reads can clear state (S6/S7).
5. The application does not open raw I²C, SMBus or I3C devices, probe
   addresses, instantiate or rebind clients, write page selectors,
   change addressing mode, enable sensors, write EEPROM or PMIC data,
   or request firmware changes. Preserve the normal memory inventory
   and utilization independently. Keep serial and part comparisons
   local to the request, with no new setting, history or outbound
   data. No unavailable value is substituted for a temperature.

## Synthetic fixture acceptance

These are acceptance cases for a future qualified general scope. No
expanded collection is enabled by this draft.

| Case | Required result |
| --- | --- |
| ODROID evidence profile above, one populated row, SPD serial bytes `01 23 45 67`, Type 17 `01234567`, matching part, enable `1`, input `40250` | With a complete table and proven producer mapping, one reading at 40.25 °C on the matching Type 17 handle. The test cannot imply that the same syntax is valid on a different firmware. |
| A proven producer and two clients with the same part and different serials, reversed endpoint/record order, reused addresses behind distinct kernel adapters, duplicate Device Locator and distinct banks | Attribute both readings to their corresponding handles. Include bank labels in the display. When the combined labels still repeat or are missing, a boot-local handle can distinguish the rendered rows. No handle or suffix is persisted. |
| Same-part DIMMs have SPD serials `01 23 45 67` and `67 45 23 01`. The unqualified producer renders the serials in reverse byte order, giving two apparently unique forward matches. | Withhold both temperatures. An apparent one-to-one match cannot choose the producer's byte order. |
| Eighteen hex digits match SPD bytes 512–520 including a plausible prefix, but the active producer mapping is unknown | The record remains unresolved. Withhold affected attribution. |
| Duplicate supported identity on either side plus an unrelated unique pair | Withhold the duplicate identity's attribution. Preserve the unrelated reading. A duplicate bound client still competes even if its hwmon child is missing or unqualified. |
| One populated Type 17 candidate has unknown or unrecognized serial but a known different part | It cannot compete for the other part. Preserve the independent pair. If its part also could match, withhold the affected attribution. |
| One bound client has unreadable SPD or a short identity read | Keep it unresolved. Suppress pairs whose uniqueness cannot be established. Leave normal inventory available. |
| Legitimate canonical aliases or a duplicate NVMEM provider attribute | Count one client. Read only its direct `eeprom` and proven hwmon attributes. A distinct client with equal identity remains a competitor. |
| Wrong driver, foreign or competing hwmon child, foreign `eeprom` parent, or adapter/client subsystem link missing, broken or resolving to `/sys/class/i2c-adapter` or `/sys/class/i2c-dev` | Do not read unqualified files or attribute a temperature from that node. A potentially competing bound client remains unresolved unless known facts rule it out. |
| Non-DDR5 or empty Type 17 row, malformed binary table, duplicate handle, missing system-memory array or fewer Type 17 rows than declared slots | Known non-DDR5 or empty rows do not compete. An incomplete or structurally unreliable inventory provides no attributed reading. |
| Text output contains a complete-looking Type 16 array and its matching Type 17 rows, then dmidecode stops at a malformed structure with a fatal stderr diagnostic and exits successfully | Withhold all temperature attribution. Later memory arrays could contain competing rows. Keep ordinary aggregate inventory behavior separate. |
| Text output contains a truncation diagnostic in stdout even when stderr is empty and exit succeeds | Withhold all temperature attribution. |
| SMBIOS 3 contains a complete first array, then ends at a structure boundary before a possible second array and Type 127. Filtered text exits successfully with empty stderr. | Withhold all temperature attribution. Require binary completion or a qualified inventory path that affirmatively proves the terminal record. |
| SMBIOS 3 maximum size exceeds the actual bytes and the in-bounds final Type 127 is valid | The binary completion check can pass. Inventory cardinality and producer mapping still require qualification. |
| Final Type 127 has a malformed length or an unterminated string set, or an explicit text filter shows its header without the decoded `End Of Table` body | Withhold all temperature attribution. |
| Blank, zero, all-ones or malformed serial, prefix-only match, case-changed part or part with non-ASCII/control bytes | No match on that identity. Leading zeros and either hex case are preserved. The only ignored part padding is trailing ASCII space. |
| Missing or disabled sensor, inaccessible sample, or a binding lost during collection | No temperature for the affected module. Preserve still-proven independent results. Requalify ownership and the join before using a new binding. No fallback probing, bus writes or background elevation. |
| `temp1_input` values 0, 250, 125000, 125250 and 40251 m°C | Accept only 250 and 125000 m°C in this set. Keep absent readings distinct from measured zero. |

## Open questions

- Blocking for the general scope. Establish how the active firmware
  maps SPD identity bytes to Type 17 strings, or provide an independent
  one-to-one attribution mechanism. The Dasharo evidence applies to
  its identified producer. The AMD examples and the Framework report
  do not establish a usable mapping. (S4, S5, S12, S13)
- Establish a complete inventory capture contract available to the
  application. Direct binary validation can prove table completion
  where both kernel files are readable. An ordinary unprivileged
  Linux process typically lacks access. The existing filtered text
  output cannot qualify the inventory. (S15/S16 lead-only)

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-09 | Separate the general SPD5118 and Type 17 attribution draft from the approved ODROID-H4 scope. Require independent producer-format evidence and affirmative SMBIOS table completion. |
