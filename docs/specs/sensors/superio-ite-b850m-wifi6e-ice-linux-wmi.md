# Spec: Gigabyte B850M AORUS ELITE WIFI6E ICE primary ITE temperatures via Linux WMI hwmon

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental motherboard temperature readback on the Gigabyte B850M AORUS ELITE WIFI6E ICE through its already-bound Linux `gigabyte-wmi` hwmon endpoint. The initial profile covers the five independently observed temperature channels. |
| Issue phase | [#2517](https://github.com/shm11C3/HardwareVisualizer/issues/2517) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Gigabyte [board manual](https://download.gigabyte.com/FileList/Manual/mb_manual_b850m-ae-wf6e-ice_1002_e.pdf#page=10) and [OEM BIOS releases](https://www.gigabyte.com/Motherboard/B850M-AORUS-ELITE-WIFI6E-ICE-rev-1x/Support/support). [F4 archive](https://download.gigabyte.com/FileList/BIOS/mb_bios_b850m-aorus-elite-wifi6e-ice_8arpl310_f4.zip?v=1b723198ecab6ae82d8b9f03e3732ebb), May 29, 2025, checksum `D322`. | OEM documentation for ITE monitoring, without per-channel labels. F4 ZIP SHA-256 `fa75c7977da238094cb696533dfb5c3ccf120d23fc51c0b35aaf51071af02d31`, `B850MAELTWF6EICE.F4` image `1b286ca47fd95eeb4895a221a91479b3aebfdb0ccff28e790414885ce5c55ec3`. Checksum-valid `GSWApp` SSDT SHA-256 `e7ddbccd345a33617a7d4d95a00f58a467046aa06c85cda13b7ee2cb17e92718`, at offset `0xE41164` within the region decompressed from image offset `0x1075D50`. |
| S2 | Gigabyte [F11 archive for the same board](https://download.gigabyte.com/FileList/BIOS/mb_bios_b850m-aorus-elite-wifi6e-ice_8arpl310_f11.zip?v=60ee69a47362aa9a52c1ab150a68e9fc), May 21, 2026, checksum `F2A8`. | ZIP SHA-256 `de5538fe3a4992a5de3b45862aef7d62ff17614329b4ec85c560f9d090d1b8c9`, `B850MAELTWF6EICE.F11` image `958986c7420f095e40d54b9d042d1d57cd9f2793fa5b7551a82c8d70fcdfa9bf`. Checksum-valid `GSWApp` SSDT SHA-256 `85d714bc21008e6d28d4958c7fb7e2e4369baff402c2bb480c4ad6d67891afdf`, at offset `0x14C66DC` within the region decompressed from image offset `0x1079CF8`. Static corroboration of the F4 temperature mechanism. |
| S3 | Same-owner public observations, July 2025. [Chip marking](https://github.com/frankcrawford/it87/issues/60#issuecomment-3022095499), [fresh boot, DMI and live WMI temperatures](https://github.com/frankcrawford/it87/issues/60#issuecomment-3026134993), [separate overlapping DKMS experiment](https://github.com/frankcrawford/it87/issues/60#issuecomment-3026218002). | Reports IT8696E marking and F4 with kernel `6.15.2-arch1-1`. Fresh boot showed `gigabyte_wmi-virtual-0` with five temperatures and no `it8696` endpoint in that snapshot. The later DKMS sample showed both endpoints after a resource-conflict override. |
| S4 | Arch Linux [temperature provider](https://github.com/archlinux/linux/blob/5dd0d64a9efca31f7f4636bdb3fe0e478b2b9b32/drivers/platform/x86/gigabyte-wmi.c#L66-L151), commit `5dd0d64a9efca31f7f4636bdb3fe0e478b2b9b32` (`v6.15.2-arch1`), and upstream [same contract in v7.4-rc2](https://github.com/torvalds/linux/blob/7475ea8b58616b33c04afb976cf0fbaad9dd63fb/drivers/platform/x86/gigabyte-wmi.c#L66-L151). | **lead-only (copyleft)** for GUID/driver binding, probe visibility, callback behavior, zero rejection, signed millidegree scaling, and hwmon registration directly under the WMI device. |
| S5 | Same Arch Linux revision [DMI attributes](https://github.com/archlinux/linux/blob/5dd0d64a9efca31f7f4636bdb3fe0e478b2b9b32/drivers/firmware/dmi-id.c#L44-L55), [boot DMI format](https://github.com/archlinux/linux/blob/5dd0d64a9efca31f7f4636bdb3fe0e478b2b9b32/drivers/firmware/dmi_scan.c#L552-L569), [WMI passive metadata](https://github.com/archlinux/linux/blob/5dd0d64a9efca31f7f4636bdb3fe0e478b2b9b32/drivers/platform/x86/wmi.c#L660-L684), [WMI device parent and naming](https://github.com/archlinux/linux/blob/5dd0d64a9efca31f7f4636bdb3fe0e478b2b9b32/drivers/platform/x86/wmi.c#L1046-L1067), [mapper parent](https://github.com/archlinux/linux/blob/5dd0d64a9efca31f7f4636bdb3fe0e478b2b9b32/drivers/platform/x86/wmi.c#L1275-L1291), [hwmon parent](https://github.com/archlinux/linux/blob/5dd0d64a9efca31f7f4636bdb3fe0e478b2b9b32/drivers/hwmon/hwmon.c#L936-L950), and [class device link](https://github.com/archlinux/linux/blob/5dd0d64a9efca31f7f4636bdb3fe0e478b2b9b32/drivers/base/core.c#L3422-L3450). | **lead-only (copyleft)** for the native sysfs device relationships and passive metadata. |
| S6 | UEFI [ACPI 6.6 §19.6.64 IndexField](https://uefi.org/sites/default/files/resources/ACPI_Spec_6.6.pdf), Linux [ACPI WMI interface](https://docs.kernel.org/wmi/acpi-interface.html) and [hwmon input ABI](https://docs.kernel.org/hwmon/sysfs-interface.html#temperatures). | Public specifications for locked indexed access, the firmware WMI descriptor, and read-only temperature input attributes. |
| S7 | Frank Crawford IT87 DKMS [platform-device resource and identity](https://github.com/frankcrawford/it87/blob/bd7a566c8a60fe0308d67e26761e16260368191a/it87.c#L7249-L7333), commit `bd7a566c8a60fe0308d67e26761e16260368191a`, GPL-2.0-or-later. | **lead-only (copyleft)** for passive identification of the competing bound `it87` device at the base reported in S3. This pin explains the naming contract, with the S3 experiment on a separate build. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Linux DMI `sys_vendor` is `Gigabyte Technology Co., Ltd.` and `board_name` is `B850M AORUS ELITE WIFI6E ICE`. These strings select this board's firmware-defined WMI channel mapping to the primary monitor. | mechanism | S1–S3, S5 lead-only for the sysfs attributes |
| Exactly one WMI device advertises GUID `DEADBEEF-2001-0000-00A0-C90629100000` with one instance. It is already bound to the native WMI-bus driver `gigabyte-wmi` and has exactly one owned hwmon child whose `name` is `gigabyte_wmi`. | mechanism | S1, S2, S4/S5 lead-only, S6 |
| Linux metadata can exclude the known overlapping raw IT87 binding for the primary monitor. A bound platform device `it87.2624` owned by the platform-bus `it87` driver is a conflict. | safety | S1–S3, S7 lead-only for the binding name |

The board and the native endpoint together identify the qualified mechanism.
The F4 and F11 releases corroborate it and are evidence identities.
PCB version, BIOS version, ACPI instance suffix and hwmon number are not
runtime admission conditions.

For the WMI gate, resolve the WMI bus device and driver symlinks under the
kernel sysfs tree. Include any suffixed duplicate GUID in the uniqueness
check. The device has a kernel WMI-bus mapper parent for a `PNP0C14`
ACPI device. Its passive `guid` and `instance_count` attributes describe
the enumerated object. The hwmon class link and the unique WMI-owned hwmon
child must resolve to the same hwmon directory under that device, and the
hwmon directory's `device` link must resolve to the same WMI device.
A matching display name elsewhere in sysfs cannot establish ownership.
(S4/S5 lead-only, S6)

For the conflict gate, inspect the platform bus and `it87` driver binding
metadata. A loaded module without a bound overlapping device is not a
conflict. An unreadable platform bus or an unresolvable candidate
binding leaves the safety check incomplete. The observed base `0x0A40`
is the reason the `it87.2624` binding conflicts. The app does not use
that base as a sampling address. (S1–S3, S7 lead-only)

| Scope | Status | Default enablement |
| --- | --- | --- |
| Primary ITE motherboard temperatures through the qualified existing WMI hwmon endpoint | Experimental. F4 owner sample corroborates five channels on the IT8696E board. F11 corroborates the same read contract statically. S4 provides lead-only GUID/driver binding, probe visibility, input callback, zero rejection and units. S5 provides lead-only passive DMI/WMI metadata and canonical ownership links. S7 provides the lead-only conflicting binding identity. | Experimental for the qualifying Linux endpoint |
| Other boards, direct chip access, Windows WMI calls, fan RPM, and other firmware methods | Unsupported by this specification. | Disabled |

## Firmware and collection facts

- The two OEM tables declare one method instance under the matching WMI
  GUID. Temperature method ID `0x125` for selectors 0–5 reads a single
  indexed temperature byte from the primary monitor pair
  `0x0A45`/`0x0A46`. The six hardware fields occupy indexes
  `0x29`–`0x2E`. The indexed field requests ACPI Global Lock
  synchronization. These branches perform the index access and
  measurement read, plus an AML memory counter increment. They do
  not read alarm registers, change banks or configuration, or invoke
  SMI. Selector 10 is a separate firmware value and is excluded.
  (S1, S2, S6)
- The embedded Binary MOF in both tables has SHA-256
  `45ecb518a740d2b551d4fa18f3dc52478c6ab754ab7a1ab97193b16b8a1cb67b`.
  It declares `ROOT\WMI`, class `GSA1_ACPIMethod`, and
  `ZFCGetCurrentTemp` with one unsigned byte input `id` and one unsigned
  byte output `value`. The schema establishes the declared API. It does
  not attest the executable method or source attribution on other
  boards. Windows callable instance, access and ownership have not been
  validated. (S1, S2)
- The Linux driver's existing probe determines which temperature inputs
  to expose. It uses instance 0 and selectors 0–5, converts a nonzero
  firmware byte as signed with 1 °C resolution, and presents millidegrees
  Celsius. A firmware zero, method failure, or wrong output type is an
  error. The per-input read reaches the named temperature method without
  shared alarm or fan refresh. The app never triggers driver activation.
  (S4 lead-only, S6)
- Collect only `temp1_input` through `temp5_input` on the qualified
  child. S3 observed these as 21, 37, 34, 24 and 27 °C and provides no
  channel-to-header or named physical location mapping. Use existing
  generic motherboard temperature names with their original channel
  numbers. The sixth possible hardware channel remains unqualified
  for app collection. (S1–S4, S4 lead-only for numbering)
- Each input is a bounded signed decimal value in millidegrees Celsius.
  Reject malformed, missing, failed or zero reads and values that are
  not integer degrees. Apply the existing Linux motherboard plausibility
  range of −55 through 125 °C, inclusive. The range is product policy,
  and the firmware-specific zero behavior is separate. Preserve
  independent valid channels without renumbering and use the existing
  Linux motherboard sensor collection and display path.
  (S4 lead-only, S6, [ADR 0011](../../adr/0011-experimental-sensor-enablement.md))

## Safety notes

- The app opens existing sysfs inputs read-only. It does not call a WMI or
  ACPI method directly, load or bind a driver, request elevation, bypass
  a reservation, access the EC pair or read fan or alarm attributes.
  The native mapper owns the AML operation. The app does not acquire an
  ISA mutex or assume it would serialize AML. The firmware lock describes
  participating ACPI access and supplies no exclusion against a direct
  I/O client which bypasses it. (S1–S7)
- Check ownership and known conflicts before sampling. Treat a removed
  or changed binding or a newly observed conflict during a sample as
  Unavailable for that sample. An unsupported board is Unsupported.
  On the matching board, missing or ambiguous ownership, incomplete
  metadata, known conflict, and all inputs unavailable are Unavailable.
  Other motherboard sensor providers and their results are independent.

## Fixture acceptance

| Case | Expected result |
| --- | --- |
| Exact DMI and one bound native WMI device with a consistent class child, no overlap, and inputs matching S3 | Five temperatures, preserving channel identity. No fan, alarm, direct WMI or raw I/O access. |
| Change the BIOS-version metadata with the same board and qualified native endpoint | Identical eligibility. The app never uses BIOS version to qualify the route. |
| Same GUID or hwmon name on another board | Unsupported. No input reads. |
| Matching board with missing native owner, wrong driver, spoofed class path, different device-link target, duplicate GUID or duplicate hwmon child | Unavailable. No input reads. |
| Matching board with bound `it87.2624`, unreadable conflict metadata, or ownership lost or changed during a sample | Unavailable. No fallback to raw I/O and no use of samples whose owner changed. |
| Missing individual input, failed read, zero, malformed text, non-integer degrees or out-of-range value | Skip that channel. Preserve independently valid siblings with their original labels. |
| Boundary inputs `-55000` and `125000`, then `-56000` and `126000` millidegrees | Admit the first two and omit the latter two. No extra input is read for the sixth or higher channels. |
| Matching board with no usable temperature input | Temperature unavailable. Independent provider results remain intact. |

## Open questions

- Non-blocking for #2517 Linux temperature scope: RPM requires a
  separately qualified owned endpoint. The audited firmware uses tach
  counts internally, and its named fan WMI methods expose tuning state.
- Non-blocking for #2517 Linux temperature scope: A Windows provider
  would require independent callable instance and ownership evidence.
  Physical temperature labels and live F11 verification remain
  unconfirmed. The five Linux channels retain generic labels and
  Experimental status.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-10 | Propose Implementation-ready for the exact Linux WMI temperature path only. Pin the F4 owner capture and the F4/F11 OEM method and ownership evidence. |
