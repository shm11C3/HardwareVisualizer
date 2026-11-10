# Spec: Gigabyte 8S661FXMP-RZ IT8705F/AF monitor ownership

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Draft — not implementation-ready |
| Scope | Disabled IT8705F/AF G-family fan candidate. Records the Gigabyte 8S661FXMP-RZ rev. 1.0 F3 ACPI ownership boundary and a separate same-model Linux observation. |
| Issue phase | [#2518](https://github.com/shm11C3/HardwareVisualizer/issues/2518) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Gigabyte [8S661FXMP-RZ rev. 1.0 support](https://www.gigabyte.com/Motherboard/8S661FXMP-RZ-rev-10/support), [F3 archive](https://download.gigabyte.com/FileList/BIOS/motherboard_bios_8s661fxmp-rz_f3.exe?v=963eddf906555a156e751e881f0f3459), August 22, 2005, and [board manual](https://download1.gigabyte.com/Files/Manual/motherboard_manual_8s661fxmprz_e.pdf), pp. 6 and 32. | The manual names IT8705AF and documents CPU and system fan RPM in BIOS Setup. The archive SHA-256 is `72ae44e5968da9db6aedfaa8a257353eb62640500ba72b5ec8090945076a21fa`. The `661xmprz.f3` ROM SHA-256 is `7296f807f48a76679178ca975dedab96223d1a426fabaaf57c9f722076a316c2`. The `ACPITBL.BIN` member at ROM offset `0x1CC7E` has decoded SHA-256 `a0d18c80e9256ac01cb8532e58d5f88948bafa8474d7e9f2fd10f7ed2008469e`. Its checksum-valid `GBT/AWRDACPI` DSDT at member offset `0x148` has SHA-256 `f49e83cc1b992bf22720a83e65e5a1b48a913c13a84a6ae6761045d96e1ef2cf`. |
| S2 | [Same-model Ubuntu owner report](https://forums.ubuntulinux.jp/viewtopic.php?id=15507), August 2013. | The kernel log reports IT8705F revision 3 at monitor base `0x0290` and an ACPI resource conflict on `0x0295`–`0x0296`. The owner reports success after a GRUB change and links to instructions for relaxing ACPI resource enforcement. The report does not identify its PCB or BIOS revision or publish a live RPM sample. |
| S3 | ITE, [IT8705F/IT8705AF Preliminary Specification V0.4.1 catalog record](https://www.datasheetarchive.com/?q=ite+it8705f), G version, and [Linux hwmon maintainer discussion](https://www.spinics.net/lists/lm-sensors/msg23641.html), July 7, 2008. | The catalog identifies the G-version document for both package names. **lead-only (copyleft)** for the cited revision-nibble `0x3` to G-version mapping and optional 16-bit tachometer capability. This does not establish which width F3 leaves active. |

## Detection

| Scope | Status | Default enablement |
| --- | --- | --- |
| 8S661FXMP-RZ IT8705F/AF fan readback | S1 names IT8705AF on the OEM board. S2 separately reports IT8705F revision 3. The Linux name and the G-version mapping in S3 (lead-only) provide no independent package marking for the reported unit. | Disabled. No approved runtime admission or collection path. |

## Firmware and safety facts

- The OEM F3 DSDT declares the `IP__` SystemIO region across the full
  indexed monitor pair at `0x0295`–`0x0296`. Its generic indexed byte
  read is unserialized and its field does not request the ACPI Global Lock.
  A host ISA mutex does not establish exclusion against this AML access.
  S2 observes the same port conflict independently. (S1, S2)
- In that DSDT, the generic byte reader is used for temperature and control.
  Fan methods perform control writes, and the fan-on path also reads monitor
  indexes `0x01`–`0x03`. The audited DSDT exposes no fan speed readback
  through standard ACPI fan objects, ATK0110, or a WMI method. The BIOS Setup
  display establishes firmware-side monitoring only. (S1)
- The F3 findings apply to the extracted DSDT. Runtime table selection,
  other BIOS releases, and SMM handlers were outside this audit. F3 is an
  evidence identity and cannot authorize access to the owner system in S2.

## Open questions

- Establish an OS-owned tachometer endpoint with an independently documented
  read contract and exact channel attribution. A direct primary LPC path would
  separately require proven exclusion of competing firmware access, approved
  transport authorization, and exact active counter width, read order and
  sentinel behavior. The owner log and generic AML reader do not establish
  those conditions.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-10 | Record the exact F3 ACPI port conflict and lack of a qualified fan readback endpoint. |
