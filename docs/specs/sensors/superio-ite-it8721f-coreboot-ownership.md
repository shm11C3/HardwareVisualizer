# ASUS Crosshair V Formula-Z Coreboot IT8721F port ownership

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Draft — not implementation-ready |
| Scope | Primary Super I/O configuration-port ownership on the ASUS Crosshair V Formula-Z with the audited Coreboot-15h image |
| Issue phase | [#2518](https://github.com/shm11C3/HardwareVisualizer/issues/2518), primary LPC research |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Coreboot-15h [ASUS Crosshair V Formula-Z release](https://15h.org/index.php/Crosshair_V_Formula-Z), [`2026.05.27-v4.11-57e26bdcd95` SeaBIOS archive](https://15h.org/images/1/1c/Coreboot-15h_2026.05.27-v4.11-57e26bdcd95_asus_crosshair-v-formula-z_seabios_ucode_vga-oproms_8mb.tar.gz) | Archive SHA-256 `9de44447af36e56d02597d92bd646b401f328efff3c50c7373ad5c9ea4cee0a0`. The 8,388,608-byte `.rom` member has SHA-256 `41e9b4fd84df26ef08edca379171922cd57b9edb7e4352f19dd85832557ee0c7`. The checksum-valid `fallback/dsdt.aml` payload at ROM offset `0x61888` is 10,203 bytes and has SHA-256 `070012beac3c1b45a74e60c3f4fe7aa0e9be3147002e87f2c590bf3f11a19399`. Static inspection of the image as data. |
| S2 | Coreboot-15h at `57e26bdcd959808c0171164f3c6197d4a347c3b2`, [board configuration](https://git.15h.org/mrothfuss/coreboot-15h/src/commit/57e26bdcd959808c0171164f3c6197d4a347c3b2/src/mainboard/asus/crosshair-v-formula-z/devicetree.cb), [board early initialization](https://git.15h.org/mrothfuss/coreboot-15h/src/commit/57e26bdcd959808c0171164f3c6197d4a347c3b2/src/mainboard/asus/crosshair-v-formula-z/romstage.c), and [IT8721F ACPI definitions](https://git.15h.org/mrothfuss/coreboot-15h/src/commit/57e26bdcd959808c0171164f3c6197d4a347c3b2/src/superio/ite/it8721f/acpi/superio.asl) | Open firmware source, **lead-only (copyleft)**. The board configuration identifies IT8721F at the primary configuration port `0x2E` and assigns the hardware-monitor base `0x0290`. The ACPI definitions corroborate the observed AML port range and locking. |
| S3 | HardwareVisualizer at `475e146683f329b15e05703e75cf8ba0938d4f80`, Windows [sensor discovery](https://github.com/shm11C3/HardwareVisualizer/blob/475e146683f329b15e05703e75cf8ba0938d4f80/core/src/infrastructure/providers/windows/pawn_io/super_io/motherboard.rs#L653-L676) and [chip-ID diagnostics](https://github.com/shm11C3/HardwareVisualizer/blob/475e146683f329b15e05703e75cf8ba0938d4f80/core/src/infrastructure/providers/windows/pawn_io/super_io/diagnostics.rs#L59-L103) | [Super I/O access rev 4](superio-access.md) defines the primary pair. [PawnIO interface rev 7](pawnio-interface.md) defines the host ISA mutex. [Primary ITE LPC expansion rev 5](superio-ite-primary-lpc-expansion.md#deferred-profiles) records the separate deferred chip profile. |

## Configuration-port ownership

The audited DSDT declares a SystemIO operation region for `0x2E`–`0x2F`
at table offset `0x1F26` and lists the pair as a resource in `_CRS`. The
serial-device (`PNP0501`) `_STA` reads the logical device's activation
state through that pair, with configuration-mode entry and exit writes.
Firmware AML uses a private mutex for these transactions. The Windows
provider's named ISA mutex does not coordinate with this AML mutex.
(S1–S3)

Windows ITE sensor discovery and chip-ID diagnostics can enter
configuration mode on the primary pair before reading the chip ID. The
shared-port concern therefore arises at discovery even with IT8721F
fan collection disabled. A subsequent chip-ID or revision check cannot
exclude access that has already occurred. This is a static overlap in
access paths. Evidence of an actual concurrent transaction on Windows
is unavailable. (S1–S3)

## Disposition

IT8721F fan collection remains disabled. Admission would require a
supported pre-entry ownership check or shared arbitration for the
configuration ports. The release identifies the audited artifact only.
Applicability to stock ASUS firmware, other Coreboot images, and
specific PCB revisions remains unverified. (S1–S3)

## Revision history

| Revision | Date | Summary |
| --- | --- | --- |
| 1 | 2026-11-09 | Recorded the primary configuration-port overlap in the published Coreboot-15h image and the pre-ID access boundary. |
