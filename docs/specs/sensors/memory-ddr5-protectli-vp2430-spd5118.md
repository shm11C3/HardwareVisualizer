# Spec: Protectli VP2430 DDR5 DIMM1 temperature through Linux spd5118

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental read-only temperature for the single physical DIMM1 socket on Protectli VP2430 through an already-bound Linux SPD5118 endpoint and complete one-slot inventory. |
| Issue phase | [#2526](https://github.com/shm11C3/HardwareVisualizer/issues/2526) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Protectli [VP2430 datasheet, 2026-02-27](https://protectli.com/wp-content/uploads/2026/03/VP2430-Datasheet-20260227.pdf#page=13), board item 15 and item 19 on the next page, and [PCI inventory](https://kb.protectli.com/kb/vp2430-hardware-overview/) | OEM identifies the single DDR5 SO-DIMM connector as DIMM1, the auxiliary header as Serial IO I²C0 by its signal names, and SMBus PCI function `00:1f.4` as `8086:54a3`. |
| S2 | Dasharo [VP2430 production release v0.9.0, 2025-05-13](https://docs.dasharo.com/variants/protectli_vp2430/releases/#v090-2025-05-13), source tag `protectli_vp2430_v0.9.0` at `feb2693fea5041d5038c4474b4b601e3e2c77b78`, [build configuration](https://github.com/Dasharo/coreboot/blob/feb2693fea5041d5038c4474b4b601e3e2c77b78/configs/config.protectli_vp2430#L1-L29), [board DMI defaults](https://github.com/Dasharo/coreboot/blob/feb2693fea5041d5038c4474b4b601e3e2c77b78/src/mainboard/protectli/vault_adl_n/Kconfig#L23-L32) and [UEFI version formatting](https://github.com/Dasharo/coreboot/blob/feb2693fea5041d5038c4474b4b601e3e2c77b78/src/vendorcode/dasharo/smbios.c#L15-L20) | Release lists the production image separately from its development-signed image and pins Intel FSP 0C.02.89.30-v0.1. The source facts are **lead-only (copyleft)**. |
| S3 | Same firmware commit, [VP2430 slot SPD configuration](https://github.com/Dasharo/coreboot/blob/feb2693fea5041d5038c4474b4b601e3e2c77b78/src/mainboard/protectli/vault_adl_n/romstage_fsp_params.c#L16-L30), [Alder Lake physical-channel mapping](https://github.com/Dasharo/coreboot/blob/feb2693fea5041d5038c4474b4b601e3e2c77b78/src/soc/intel/alderlake/meminit.c#L62-L67), [SPD address handoff](https://github.com/Dasharo/coreboot/blob/feb2693fea5041d5038c4474b4b601e3e2c77b78/src/soc/intel/alderlake/meminit.c#L218-L231), [board devices](https://github.com/Dasharo/coreboot/blob/feb2693fea5041d5038c4474b4b601e3e2c77b78/src/mainboard/protectli/vault_adl_n/devicetree.cb) and [VP2430 GPIOs](https://github.com/Dasharo/coreboot/blob/feb2693fea5041d5038c4474b4b601e3e2c77b78/src/mainboard/protectli/vault_adl_n/variants/vp2430/gpio.c#L267-L271) | **lead-only (copyleft)** for channel 0/DIMM 0, the sole configured 7-bit SPD address `0x50`, the FSP handoff, and separation of the board's Serial IO I²C0 and PCH SMBus. |
| S4 | Same firmware commit, [present FSP DIMM record selection](https://github.com/Dasharo/coreboot/blob/feb2693fea5041d5038c4474b4b601e3e2c77b78/src/soc/intel/alderlake/romstage/romstage.c#L101-L150), [Type 16 and Type 17 construction](https://github.com/Dasharo/coreboot/blob/feb2693fea5041d5038c4474b4b601e3e2c77b78/src/lib/smbios.c#L697-L771), [Type 127 construction](https://github.com/Dasharo/coreboot/blob/feb2693fea5041d5038c4474b4b601e3e2c77b78/src/lib/smbios.c#L1026-L1034) and [final table emission](https://github.com/Dasharo/coreboot/blob/feb2693fea5041d5038c4474b4b601e3e2c77b78/src/lib/smbios.c#L1251-L1264) | **lead-only (copyleft)** for present DIMM data, array counts and associations, and end-of-table emission. The producer preserves saved FSP serial bytes when formatting Type 17. The physical-slot proposal does not use their order. |
| S5 | [OpenBSD 7.8 report, 2026-02-09](https://dmesgd.nycbug.org/index.cgi?do=view&id=8824) and [OpenBSD 7.7 device and PCI report](https://www.mail-archive.com/bugs@openbsd.org/msg22443.html) | Independent observations of reported system `Protectli VP2430`, firmware vendor `3mdeb`, version `Dasharo (coreboot+UEFI) v0.9.0` and firmware date `04/30/2025`. The PCI report identifies the SMBus controller. The logs report the system string rather than printing separate Type 2 fields. S2 supplies the Type 2 source values. |
| S6 | [DMTF SMBIOS DSP0134 3.9.0](https://www.dmtf.org/sites/default/files/standards/documents/DSP0134_3.9.0.pdf), §§5.2, 6.1, 7.17–7.18 and 7.49 (p. 152, Table 141), and dmidecode 3.5 [manual](https://github.com/mirror/dmidecode/blob/484f8935b0fc768841f43fa388b191196b5e12fd/man/dmidecode.8), [filtered traversal and SMBIOS 3 stop](https://github.com/mirror/dmidecode/blob/484f8935b0fc768841f43fa388b191196b5e12fd/dmidecode.c#L5525-L5622), [SMBIOS 3 entry point](https://github.com/mirror/dmidecode/blob/484f8935b0fc768841f43fa388b191196b5e12fd/dmidecode.c#L5731-L5776), [record header](https://github.com/mirror/dmidecode/blob/484f8935b0fc768841f43fa388b191196b5e12fd/dmioutput.c#L47-L51) and [Type 127 heading](https://github.com/mirror/dmidecode/blob/484f8935b0fc768841f43fa388b191196b5e12fd/dmidecode.c#L5338-L5340) | Primary SMBIOS and CLI contracts. The decoder implementation is **lead-only (copyleft)** for its terminal record rendering and traversal. The [existing ODROID-H4 rev 3 inventory contract](https://github.com/shm11C3/HardwareVisualizer/blob/6a59d721/docs/specs/sensors/memory-ddr5-odroid-h4-spd5118.md#inventory-completion) documents the same command and diagnostics. |
| S7 | Linux [SPD5118 ABI](https://docs.kernel.org/hwmon/spd5118.html), and Linux v6.18 [register and thermal callback](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/spd5118.c#L34-L199), [paged regmap](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/spd5118.c#L438-L483), [thermal registration](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/spd5118.c#L526-L570), [probe](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/spd5118.c#L630-L745) and [regmap](https://github.com/torvalds/linux/blob/v6.18/drivers/base/regmap/regmap.c) | Primary public ABI. Source **lead-only (copyleft)** for the probe checks, per-call register set, page selection and serialization. |
| S8 | Linux v6.18 [I801 driver](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/busses/i2c-i801.c), [I²C sysfs ABI](https://github.com/torvalds/linux/blob/v6.18/Documentation/i2c/i2c-sysfs.rst#L35-L43), [I²C device registration](https://github.com/torvalds/linux/blob/v6.18/drivers/i2c/i2c-core-base.c) and [hwmon registration](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/hwmon.c#L973-L984) | Public I²C sysfs namespace and **lead-only (copyleft)** facts for the I801 adapter, native adapter/client and hwmon parent links, and driver ownership. |

## Detection and scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| Physical DIMM1 on the `VP2430` one-slot board topology with a direct kernel-owned SPD5118 endpoint and complete one-slot inventory | S1 pins the sole connector and identifies the onboard I801 PCI function at `0000:00:1f.4` (`8086:54a3`). The observed firmware is 3mdeb Dasharo UEFI v0.9.0 (S5). Lead-only S2–S4 pin the board identity, separate physical I²C buses, DIMM1's fixed SPD address `0x50`, FSP handoff, and complete inventory producer. S6 defines inventory completion. S7–S8 pin the public thermal and sysfs ABI, with lead-only facts for the thermal callbacks, page selection, and ownership links. | Experimental when the mechanism and safety gates below pass. |
| Different board topology, incomplete or ambiguous inventory, or a nonqualifying endpoint | Unqualified for physical attribution by this specification. | Unsupported. |

| Gate kind | Runtime condition |
| --- | --- |
| mechanism | DMI board vendor `Protectli` and product `VP2430` select the only physical memory connector, DIMM1, and its documented SPD route. The matched board's separate auxiliary I²C0 bus is outside this route (S1–S3). |
| safety | Require complete diagnostic-free same-request SMBIOS inventory with exactly one Type 16 physical memory array declaring one device and exactly one complete associated, populated Type 17 DDR5 SO-DIMM record with a usable locator and nonzero capacity. Corroborate the count, array handle and unique record handles. Reject incomplete or conflicting records and extra arrays or slots (S4, S6). |
| mechanism | Require the board profile's fixed native I801 SMBus function and direct SPD slot address (S1, S3, S8). Match the PCI vendor and device, function location and `i801_smbus` driver. Require one direct bound client under that function's native adapter and one hwmon child of that client (S7–S8). |
| safety | The native I²C adapter and client subsystem links resolve to `/sys/bus/i2c`. The client driver resolves to `/sys/bus/i2c/drivers/spd5118`. The class hwmon link resolves to the canonical child and the child's device association resolves to that same client. Require the hwmon name `spd5118`. Reject missing, changed, duplicate or ambiguous links, sibling buses, mux adapters and the I²C character-device class (S7–S8). |
| safety | Require `temp1_enable` to report 1 before reading `temp1_input` through that node. An absent binding, a write-disabled controller that does not expose the endpoint, a disabled sensor, or an inaccessible attribute leaves temperature unavailable (S7–S8). |

The pinned firmware emits Type 17 from present FSP DIMM records with their
channel and DIMM locator fields (S4, lead-only). In the qualified VP2430
topology, a complete inventory containing only one populated DIMM describes
DIMM1. Attach the temperature to that sole existing inventory row and preserve
its boot-local locator. The runtime association uses physical routing and
inventory cardinality. Firmware vendor, version, serial, part number and
locator spelling are evidence identities. An alternate firmware that reports
different inventory or endpoint structure fails the corresponding mechanism or
safety gate. Its availability has not been tested.

Sysfs shows the current device ownership. It does not attest the initiator of
earlier administrative client instantiation. The pinned stock probe verifies
the hub type and thermal capability (S7, lead-only). Adapter and hwmon numbers
are transient.

## Inventory and sampling safety

1. Reuse the existing explicit Linux detailed inventory request and its fixed
   `pkexec dmidecode --type memory --type 127` result in memory. The production
   command already requests Type 127 and passes stdout and stderr status to
   optional DIMM collection. Apply the existing inventory-completion contract
   before reading sensor attributes. Require successful exit, empty stderr,
   no top-level decoder diagnostics, the linked Type 16/17 records and exactly
   one top-level Type 127 header with length 4 and decoded `End Of Table`
   heading. Allow ordinary decoder version and source notices before the marker
   and only whitespace after it in stdout. Reject a header alone, a substring
   in a firmware field, an invalid formatted length, or a later emitted record.
   The filtered output corroborates the emitted memory inventory through the
   decoded end-of-table marker. The decoder omits unselected types and stops at
   Type 127 for SMBIOS 3. Checking raw bytes beyond the marker or validating
   the binary table requires an independent binary source and is outside this
   text path (S6, lead-only for decoder behavior). Preserve ordinary detailed
   inventory and its cache or fallback on optional temperature failure. Use no
   extra elevation, automatic privilege prompt or new inventory path.
2. Require the positively identified endpoint above and then read `temp1_enable`. Only value `1` permits `temp1_input`. The stock driver checks temperature capability before hwmon registration. In v6.18, enable reads the thermal configuration field and input reads the two current-temperature bytes. The alarm callback that reads status and clears sticky flags is separate. Neither permitted callback writes alarm, thermal configuration, limits, EEPROM or PMIC state.
3. The I801 SMBus route uses the driver's legacy 8-bit access. The permitted thermal fields are on register page zero. Regmap serializes any required page-zero selection and the corresponding read within the kernel. Another kernel consumer can have left a different page selected, so an ordinary read may update the volatile page selector with no restoration of the old page. This is part of the driver's read protocol. Read no SPD EEPROM bytes or alarm attributes. Perform no application-side page selection, raw SMBus access, detection scan or device mutation (S7–S8).
4. Accept a complete signed decimal value in millidegrees Celsius that is positive, at most 125,000 and a multiple of 250. The latter increment is the pinned driver resolution (S7, lead-only). The positive interval is the project's display policy. Convert to Celsius and preserve other inventory data when the temperature is missing, disabled, invalid or unavailable. Revalidate ownership after rebinding before using the endpoint again.

## Acceptance fixtures

| Fixture | Expected result |
| --- | --- |
| Protectli VP2430 with the observed Dasharo v0.9.0 firmware, complete one-device array and terminal marker, one canonical native `0x50` child, enabled sensor, input `40250` | Report 40.25 °C for the sole inventory row corresponding to DIMM1. No EEPROM read or additional inventory invocation. |
| Same exact fixture with absent, all-zero, or byte-reversed firmware serial, or a different valid part string | Physical DIMM1 attribution is unchanged. Serial and part are not attribution inputs and serial is not exposed. |
| VP2430e, VP2440, similar DMI strings or malformed profile identity | No VP2430 temperature reads. |
| Clean SMBIOS truncation before Type 127, partial terminal marker, length 5, marker in record field, later emitted record, decoder diagnostics or failed exit | No temperature reads. Preserve ordinary inventory behavior. |
| Same board with another firmware version and the same qualified physical topology, inventory and binding | Apply the same mechanism and safety gates. The firmware string alone neither enables nor disables sampling. |
| Two populated records, extra arrays, missing locator, conflicting counts or handles, empty or wrong-type slot | No temperature reads. |
| Absent native binding, wrong driver, wrong BDF or PCI ID, wrong address, mux adapter, wrong or changed symlink, duplicate candidates | No temperature reads and no fallback scan, binding or raw I/O. |
| Disabled or inaccessible enable attribute | No temperature input read. Preserve inventory. |
| Input values `0`, `250`, `125000`, `125250`, `40251` and malformed text | Accept only `250` and `125000`. |
| Sensor disappears after qualification or read fails | Temperature unavailable. Do not retry by mutating or re-binding hardware. |

## Open questions

- Non-blocking for #2526 Experimental: No public same-boot VP2430 Linux capture with complete Type 16/17/127 and a positive native sample was found. This limits demonstrated availability. Runtime gates verify the inventory and endpoint before sampling, and skip absent or disabled devices.
- Non-blocking for #2526 Experimental: Exact FSP SPD serial byte order remains unverified. The proposal uses fixed physical routing and complete one-slot inventory, with no serial join.
- Non-blocking for #2526 Experimental: The OEM materials and released firmware establish normal board routing. They do not attest modified hardware, attached nonstandard peripherals, or a custom firmware build spoofing the release strings.

## Revision history

| Rev | Date | Changes |
| --- | --- | --- |
| 1 | 2026-11-13 | Proposed transition to Implementation-ready for the exact physical DIMM1 route, with pinned producer and OS ownership evidence and fail-closed inventory requirements. Maintainer sign-off is pending in the proposing PR. |
