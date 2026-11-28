# Spec: Gigabyte secondary ITE sensor access

| Field | Value |
| --- | --- |
| Revision | 9 |
| Status | Draft — not implementation-ready |
| Scope | Candidate Experimental Windows read of TMPIN1–3 and FAN1–3 through the direct Environment Controller of a firmware-open secondary ITE part on configuration slot 1, keyed by the pre-key raw chip ID `0x8733` (IT8792E/IT8795E) or `0x8695` (IT87952E). The candidate is blocked by two named safety questions. Separate ECIO and ISA bridge transport restrictions. OEM WMI evidence for GA-AB350-Gaming 3 BIOS F54d and X470 AORUS ULTRA GAMING BIOS F61a. Linux hwmon evidence for the GA-Z87X-UD5H IT8790E. ECIO evidence for X670E AORUS MASTER BIOS F41 and B650E AORUS MASTER rev. 1.0. X470 AORUS GAMING 7 WIFI rev. 1.0 BIOS F64 SMM evidence. X870E AORUS MASTER F12 secondary endpoint observations and static firmware evidence. A disposition for every [#2519](https://github.com/shm11C3/HardwareVisualizer/issues/2519) target. |
| Issue phase | [#2519](https://github.com/shm11C3/HardwareVisualizer/issues/2519) |

Revision 9 re-scopes the earlier exact-board X470 AORUS ULTRA GAMING
candidate under
[ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md).
The new row is keyed by slot, pre-key chip ID and the chip's own
activation, routing and enable registers. Board observations are its
evidence members. The row is the smallest Windows slice with an
independent register dump (S31). It needs no new PawnIO module, reads
no alarm register and writes nothing except the logical-device selector
that the read requires. It stays Disabled until the two blocking Open
questions are resolved:

- B1 covers the selector walk of the only signed port authorization.
- B2 covers a competing configuration-space handler.

A maintainer decision can resolve either question, or the evidence named
in the question can. The Linux hwmon routes for the same parts remain
blocked by the input refresh that reads alarm status. The 2026-11-28
recheck of mainline and the external driver is recorded below.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Gigabyte, [X470 AORUS ULTRA GAMING rev. 1.0 specifications](https://www.gigabyte.com/Motherboard/X470-AORUS-ULTRA-GAMING-rev-10/sp) and [User's Manual rev. 1001](https://download.gigabyte.com/FileList/Manual/mb_manual_x470-aorus-ultra-gaming_1001_e_200707.pdf), printed pp. 3–4 and §1-7 | Primary for PCB revision identification and the named fan and temperature connectors. |
| S2 | [X470 AORUS ULTRA GAMING rev. 1.0 owner report](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/issues/251#issuecomment-924449572) | Independent owner observation, with physical revision stated. Reports separate behavior for three fan channels. Includes no register or transport trace and is not maintainer-accepted verification. |
| S3 | [X470 AORUS GAMING 7 WIFI owner report](https://www.reddit.com/r/linuxmint/comments/1vu064e/fan_control_for_x470/), detector log dated 2026-08-20, and Gigabyte [rev. 1.0](https://www.gigabyte.com/Motherboard/X470-AORUS-GAMING-7-WIFI-rev-10/Support/sp) and [rev. 1.1](https://www.gigabyte.com/Motherboard/X470-AORUS-GAMING-7-WIFI-rev-11/sp) pages | Independent detection of two controllers. The log gives neither the PCB revision nor an identified OEM BIOS release. |
| S4 | [TRX50 AI TOP owner report](https://github.com/frankcrawford/it87/issues/73) and [later observation on that machine](https://github.com/frankcrawford/it87/issues/73#issuecomment-3592587595) | Independent negative pre-key result and later OS sensor output on a different board. |
| S5 | [Super I/O access rev. 4](superio-access.md#slot-1-ite-exit-withhold-rule) and [PawnIO interface rev. 7](pawnio-interface.md#lpcio) | Existing project scope for the raw diagnostic, exact slot-1 exit withhold, ISA mutex and transport. |
| S6 | Linux [v6.18 IT87 monitor](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c), Git blob `e233aafa8856c94c3fde3502732fcda2bd3bf130`, GPL-2.0-or-later, and [IT87 driver documentation v6.18](https://www.kernel.org/doc/html/v6.18/hwmon/it87.html#supported-chips) | **lead-only (copyleft)**. Source for the active LDN, base and runtime register locations, fan and temperature formats, primary/secondary port separation, and the firmware-open discovery observation. The driver maps `0x8733` to the `it8792` prefix documented for IT8792E/IT8795E. |
| S7 | lm-sensors [V3-6-2 X470 AORUS ULTRA GAMING profile](https://github.com/lm-sensors/lm-sensors/blob/V3-6-2/configs/Gigabyte/X470-AORUS-ULTRA-GAMING.conf), Git blob `1964faec4aa347715833da50b4188c1c2282a683`, GPL-2.0-or-later project | **lead-only (copyleft)**. Experimental configuration naming two LPC controllers and tentative channel labels. Its comments cite other board profiles and monitor output, so it supplies leads rather than independent board verification. |
| S8 | PawnIO.Modules [`LpcIO`](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/LpcIO.p) and [`IsaBridgeEC`](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/IsaBridgeEC.p), commit `52a7e536dff3e53c96917a28caac5e0fa6510696` (release 0.2.11) | Upstream public PawnIO interface and audit of the transactions required by those calls. `ioctl_select_slot` clears the recorded BAR list and writes no Super I/O register. `ioctl_find_bars` refuses a `CR20` of `0x00` or `0xFF` and neither enters nor exits configuration mode. It writes every logical-device index `0x00`–`0xFE` to `CR07` and reads back each selection. It records the `CR60/61` and `CR62/63` words that agree across two samples about 1 ms apart. It leaves `CR07` at the last index written. Port I/O is allowed only on the selected configuration pair, ASUS EC ports `0x25C`/`0x25D`, and 8-byte windows at recorded bases. |
| S9 | LibreHardwareMonitor [ECIO byte transport](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/IT879xEcioPort.cs), Git blob `cbb52c4b88c8fc606087220be6ad16cfd6315812`, and [secondary LPC discovery](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/LpcIO.cs#L600-L682), Git blob `3bb61abd9d86df2dcf388cc80689d4db61d16537`, MPL-2.0 | **lead-only (copyleft)**. Alternate ID after mode entry, selected ECIO mailbox transactions, and firmware transport selection. Lines 600–682 show the ecosystem behavior on PawnIO for raw `0x8733`, `0x8695` and `0x8790` on slot 1. The monitor reads the chip ID before any key, calls `ioctl_find_bars`, and selects LDN `0x04` and the GPIO device without restoring the selector. Its exit routine skips port `0x4E`. |
| S10 | LibreHardwareMonitor [ISA bridge controller](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/IsaBridgeGigabyteController.cs), Git blob `cc9398dc047347d6bfd19129cc00d6ee890e0143`, and [ECIO controller](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/EcioPortGigabyteController.cs), Git blob `e906f6aee3fda7c0b42f3900f3c69f41b6d4376a`, MPL-2.0 | **lead-only (copyleft)**. Confirms what the cited bridge and ECIO examples use the mapped RAM for. |
| S11 | Intel, [82077AA floppy disk controller data sheet](https://www.alldatasheet.com/html-pdf/167793/INTEL/82077AA/1242/8/82077AA.html), §2.1 | Primary for the historical PC/AT or PS/2 port assignment. It establishes the possible address collision, with no claim about a floppy controller being present on the target board. |
| S12 | Gigabyte, [GA-AB350-Gaming 3 rev. 1.x BIOS F54d release](https://www.gigabyte.com/Motherboard/GA-AB350-Gaming-3-rev-1x/Support#support-dl-bios) and [official F54d archive](https://download.gigabyte.com/FileList/BIOS/mb_bios_ab350-gaming-3_8a06bg08_f54d.zip?v=9b00bc778a8a46e655182428ea1bbe5a) | Archive SHA-256 `5ad4baf65297f0a89da0f04da2f00fa0111bc5d634bde30ae9de53956df3f158`. Checksum-valid `GBT / GSWApp` SSDT (28,407 bytes) SHA-256 `0c5707bb1a0fdbe31f22f266695ab38c2b5fd5c2c39b2cc0c0048665ba4311ac`. The table starts at offset `0xA8F114` of the LZMA section decoded from BIOS image offset `0xA246F0`. |
| S13 | Linux, [Gigabyte WMI temperature interface](https://gbmc.googlesource.com/linux/+/1f66f63c7312ee085dc989b3c5fa4b3d09fe9d52/drivers/platform/x86/gigabyte-wmi.c), commit `1f66f63c7312ee085dc989b3c5fa4b3d09fe9d52`, GPL-2.0-or-later | **lead-only (copyleft)**. Source for the WMI GUID and temperature method ID tested against the OEM table. |
| S14 | [X470 AORUS ULTRA GAMING rev. 1.0 owner report](https://github.com/t-8ch/linux-gigabyte-wmi-driver/issues/1#issuecomment-814724928), [earlier sensor output](https://github.com/t-8ch/linux-gigabyte-wmi-driver/issues/1#issuecomment-814754053), and [follow-up captures](https://github.com/t-8ch/linux-gigabyte-wmi-driver/issues/2#issuecomment-814796061), April 2021 | Independent report and captures from the same owner. [Boot log](https://github.com/t-8ch/linux-gigabyte-wmi-driver/files/6270793/dmesg.txt) SHA-256 `91c93c2de7001fadd0cbf47e62cbc653303eec4e38dbc8e52060c89a6094da97`. [ACPI capture](https://github.com/t-8ch/linux-gigabyte-wmi-driver/files/6270794/acpidump.txt) SHA-256 `de25cfc02720d7668a240598f7cb54b4a9be553496f52781f0268f7408f16b0e`. |
| S15 | Gigabyte [GA-Z87X-UD5H rev. 1.x specifications](https://www.gigabyte.com/Motherboard/GA-Z87X-UD5H-rev-1x/sp) and [GA-Z87X-UD5H R101 schematic](https://www.scribd.com/document/872779130/GA-Z87X-UD5H-R101) (public mirror) | Vendor board identity and schematic marking `IT8790E/LQFP64`. The schematic does not define register read effects. |
| S16 | GA-Z87X-UD5H [owner boot and sysfs report, March 5, 2017](https://www.linux.org.ru/forum/linux-hardware/13265282) and [same author's sensor output, October 19, 2022](https://www.linux.org.ru/forum/talks/17004990#comment-17005247) | Independent observations of two separate controllers. The first identifies the board and shows an unbound secondary platform node. The later snapshot shows sensor readings at the same bases and omits the full hardware and software identity. |
| S17 | Linux [v7.4-rc1 IT87 driver](https://github.com/torvalds/linux/blob/v7.4-rc1/drivers/hwmon/it87.c#L1118-L1121), Git blob `2757d428a23dba550756cb327c851dbcaa25c078`, and [Frank Crawford DKMS driver](https://github.com/frankcrawford/it87/blob/bd7a566c8a60fe0308d67e26761e16260368191a/it87.c#L3496-L3499), commit `bd7a566c8a60fe0308d67e26761e16260368191a`, Git blob `56148f202fee43436fa73a747a1bf465deb8721e`, both GPL-2.0-or-later. See also [IT87 driver documentation v6.18](https://www.kernel.org/doc/html/v6.18/hwmon/it87.html#hardware-interface). | **lead-only (copyleft)** for the shared cache refresh, its alarm-status reads and their documented general effect. Neither driver revision is identified as the one in the positive S16 capture. |
| S18 | Gigabyte, [X670E AORUS MASTER rev. 1.x BIOS F41 release](https://www.gigabyte.com/Motherboard/X670E-AORUS-MASTER-rev-1x/support#support-dl-bios) and [official F41 archive](https://download.gigabyte.com/FileList/BIOS/mb_bios_x670e-aorus-master_8arpl002_f41.zip?v=5f0e5b9dc371d6536323fbf3445115d1), February 5, 2026, OEM checksum `B0E7` | Static OEM artifact. ZIP SHA-256 `8e44a623a41044e77cd961403e85144394a0fe36b5e4818ff0b5393da11bb8a7`. `X670EAORUSMASTER.F41` image SHA-256 `91df1bca4b0266fffe64bca9a68210f127ac952f6e7001d2566d03abe117d3aa`. |
| S19 | [X670E AORUS MASTER rev. 1.x F41 owner report](https://github.com/frankcrawford/it87/issues/96), March 2026 | Independent DMI, BIOS and H2RAM detection observation with an opt-in driver branch. The secondary headers were unpopulated. |
| S20 | [Separate X670E AORUS MASTER pump report](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/issues/2582), October 2026 | First-hand pump RPM observation from a local modified monitor. The report gives no BIOS version or populated thermistor reading. Its chip and transport attribution is a research lead. |
| S21 | Gigabyte [B650E AORUS MASTER rev. 1.0 Smart Fan 6 documentation](https://www.gigabyte.com/Motherboard/B650E-AORUS-MASTER-rev-10), [BIOS F37 release](https://www.gigabyte.com/Motherboard/B650E-AORUS-MASTER-rev-10/support#support-dl-bios) and [official F37 archive](https://download.gigabyte.com/FileList/BIOS/mb_bios_b650e-aorus-master_8arpl102_f37.zip?v=0431d29bc3abda1187efea552dffc2fe), November 21, 2025, OEM checksum `2211` | Board identity, setup UI and static OEM artifact. ZIP SHA-256 `891581cd7d4c063323ed7f6ac1aee3332c3cdb3dc0d0b940883be5aa6b384257`. `B650EAORUSMASTER.F37` image SHA-256 `391edcc4480e3a413d5d996fc15572a568a222301e3ab88b7d41442db1d1dacd`. |
| S22 | B650E AORUS MASTER [owner's DMI and discovery report](https://github.com/frankcrawford/it87/issues/80), [December 2025 ECIO dump](https://github.com/frankcrawford/it87/issues/80#issuecomment-3626863744), and [August 2026 driver log](https://github.com/frankcrawford/it87/issues/80#issuecomment-5407428995) | Independent observations on the same reported board with opt-in tooling. The posts do not identify BIOS F37 or the EC firmware build. |
| S23 | Frank Crawford DKMS driver, commit `cbfeef4efbc3391624e15b1c9db4f893803eb9f2`, Git blob `56148f202fee43436fa73a747a1bf465deb8721e`, GPL-2.0-or-later. [G3 tachometer fields](https://github.com/frankcrawford/it87/blob/cbfeef4efbc3391624e15b1c9db4f893803eb9f2/it87.c#L450-L470), [RPM units](https://github.com/frankcrawford/it87/blob/cbfeef4efbc3391624e15b1c9db4f893803eb9f2/it87.c#L3919-L3924), [ECIO ports](https://github.com/frankcrawford/it87/blob/cbfeef4efbc3391624e15b1c9db4f893803eb9f2/it87.c#L144-L151), [read transaction](https://github.com/frankcrawford/it87/blob/cbfeef4efbc3391624e15b1c9db4f893803eb9f2/it87.c#L2240-L2348) and [error handling and serialization](https://github.com/frankcrawford/it87/blob/cbfeef4efbc3391624e15b1c9db4f893803eb9f2/it87.c#L2430-L2451) | **lead-only (copyleft)** for the H2RAM tachometer map, the host-side mailbox operations and their limits. The blob matches S17's DKMS blob. |
| S24 | Gigabyte [X470 AORUS GAMING 7 WIFI rev. 1.0 BIOS F64 release](https://www.gigabyte.com/Motherboard/X470-AORUS-GAMING-7-WIFI-rev-10/Support#support-dl-bios) and [official F64 archive](https://download.gigabyte.com/FileList/BIOS/mb_bios_x470-aorus-gaming-7-wifi_8a06bg0_f64.zip?v=56486d580062e1031e8dce5358ac8ae5), March 22, 2024, OEM checksum `C055` | Static OEM artifact. ZIP SHA-256 `2412e1bc94ccff695dfcecdcfae836ec24a8fb720c883f575cea8d7e11451ee0`. `X470AORUSGAMING7WIFI.F64` image SHA-256 `ebd24b8e697d082f55470f47d0f1d58c673599f2b005dac014ab63a6436982ee`. |
| S25 | [X470 AORUS GAMING 7 WIFI board teardown, circuit analysis](https://www.tweaktown.com/reviews/8605/gigabyte-x470-aorus-gaming-7-wifi-motherboard-review/index.html) | Independent photographs show IT8792E and IT8686E markings on a board of unidentified PCB revision. The teardown also reports a partly hidden embedded controller. The photographs have no verified link to the S3 unit or to a board running F64. |
| S26 | UEFI Platform Initialization Specification 1.8, volume IV, [§7.3 MM Sx Dispatch Protocol](https://uefi.org/specs/PI/1.8/V4_MM_Child_Dispatch_Protocols.html#mm-sx-dispatch-protocol) and [§7.8 MM Power Button Dispatch Protocol](https://uefi.org/specs/PI/1.8/V4_MM_Child_Dispatch_Protocols.html#mm-power-button-dispatch-protocol) | Primary definitions of dispatch GUIDs and their hardware event types and phases. |
| S27 | Gigabyte [X870E AORUS MASTER BIOS F12 release](https://www.gigabyte.com/Motherboard/X870E-AORUS-MASTER/support#support-dl-bios) and [official F12 archive](https://download.gigabyte.com/FileList/BIOS/mb_bios_x870e-aorus-master_8arpl302_f12.zip?v=885ef7049c0794810c59e62df7e2b4a9), February 5, 2026, OEM checksum `BCCF` | Static OEM artifact. ZIP SHA-256 `4b2c2ddb7dcf230b6f3b638a6dc8996e22d4af88075f1f86ced9d5fcfd28d493`. `X870EAORUSMASTER.F12` image SHA-256 `8e1f81f404505c47ef470a08a66fed64028f096066e03a86818bc8fe5a4f0be1`. |
| S28 | [X870E AORUS MASTER owner report and readings](https://github.com/frankcrawford/it87/issues/103), BIOS F12 and kernel `6.19.14-1-liquorix-amd64` | Independent live temperature and RPM observations on two reported endpoints. The report identifies its DKMS build and leaves PCB revision and chip marking unconfirmed. |
| S29 | Frank Crawford DKMS driver, commit `20f2f2f4c92c14fcdd26f60d050e693ad2c30bf8`, Git blob `e7cc39fddf72a4ad42d5d190b7a15e7d9c12a512`, GPL-2.0-or-later. [Temperature input](https://github.com/frankcrawford/it87/blob/20f2f2f4c92c14fcdd26f60d050e693ad2c30bf8/it87.c#L2640-L2646), [fan input](https://github.com/frankcrawford/it87/blob/20f2f2f4c92c14fcdd26f60d050e693ad2c30bf8/it87.c#L2932-L2939), and [cold refresh](https://github.com/frankcrawford/it87/blob/20f2f2f4c92c14fcdd26f60d050e693ad2c30bf8/it87.c#L2524-L2527). | **lead-only (copyleft)** for the common cache and alarm register reads in the reported build. |
| S30 | UEFI PI 1.8 [MM protocol notification](https://uefi.org/specs/PI/1.8/V4_Management_Mode_System_Table.html#mmregisterprotocolnotify), and TianoCore [EDK II stable 202511 package protocol declarations](https://github.com/tianocore/edk2/blob/edk2-stable202511/MdeModulePkg/MdeModulePkg.dec#L670) | Public interface definitions for protocol notification and the SMM ExitBootServices GUID. |
| S31 | AX370-Gaming K7 owner report, [LibreHardwareMonitor issue 1030](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/issues/1030), [attached report](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/files/11068646/LibreHardwareMonitor.Report.txt), 2023-03-25, SHA-256 `276b0ff0864c337245602f78df3b25384279aa6838d1d2a8ca92466f7b557ed8`. This is S19 of the [primary LPC expansion specification](superio-ite-primary-lpc-expansion.md#sources). | Independent public register dump, not maintainer-accepted. It records monitor version 0.9.2.0 on Windows 10 build 19045, before PawnIO, so no `ioctl_find_bars` call. Its secondary section is reported as chip ID `0x8733`, chip version 3, EC base `0x0A60` and GPIO base `0x0A50`. The primary is `0x8686` at `0x0A40`. The secondary bank-0 rows are quoted as bytes under [AX370-Gaming K7 secondary dump](#ax370-gaming-k7-secondary-dump). The tool read EC `0x01`–`0x03` for its dump. The candidate never selects them. |
| S32 | Linux mainline [`drivers/hwmon/it87.c`](https://github.com/torvalds/linux/blob/7cd8495c0f16275c4f9abe1cef09a7b5fcd6d046/drivers/hwmon/it87.c) at commit `7cd8495c0f16275c4f9abe1cef09a7b5fcd6d046` (2026-11-28), Git blob `bbb3f9788c2e34c01591b8498c7b32349243042f`, GPL-2.0-or-later. The last change to the file is `4dd10fbbeb9c` (2026-11-06). | **lead-only (copyleft)**. Line-pinned facts: raw IDs `0x8733`, `0x8790` and `0x8695` at lines 154, 161 and 169. The `0x8733` and `0x8695` profiles at lines 499–510 and 578–588 have the same feature set: 16-bit fans, three temperatures, a TMPIN2 extra-source bit, no serial-bus mask. The `0x8790` profile at lines 489–498 has no TMPIN2 extra-source case. Lines 2833–2851 record that these three IDs are left in configuration mode by firmware and that entering or exiting it causes the reported failure. Lines 2975–2993 skip the key for them and read LDN `0x04` `CR30[0]` and `CR60/61`. Lines 3320–3368 select LDN `0x07`, with `CR27[7]` set meaning FAN3 absent and `CR29[2]` set meaning FAN2 absent. Line 210 and lines 213–214 put the index and data ports at base + 5 and + 6. Lines 248 and 250 give count bytes `0x0D`–`0x0F` and `0x18`–`0x1A`, and line 263 temperatures at `0x29`–`0x2B`. Lines 1334–1382 decode the sources from EC `0x51` and `0x55`. Lines 770–771 convert counts, and lines 3562–3563 enable fans from EC `0x13` bits 4–6. Lines 3723–3724 refuse EC `0x00` bit 7 set or EC `0x58` other than `0x90`. Lines 1032–1121 read EC `0x01`–`0x03` in the shared refresh used by input and type attributes. |
| S33 | Out-of-tree IT87 driver, [commit `8fdc406fff148c033d7535a1ac3d905f77edf3cb`](https://github.com/frankcrawford/it87/blob/8fdc406fff148c033d7535a1ac3d905f77edf3cb/it87.c#L3789-L3887) (2026-11-27), Git blob `451bb92a96139d8060bee5e4d93bc9002d6d1bfa`, GPL-2.0-or-later, and the message of [commit `44315f4749831fe4aa47041b199a1df3de5604f3`](https://github.com/frankcrawford/it87/commit/44315f4749831fe4aa47041b199a1df3de5604f3) | **lead-only (copyleft)**. Lines 3789–3887 still read EC `0x01`–`0x03` in the cold or expired refresh. The commit message reports, for X570 AORUS XTREME board revision 1.0, an IT8688E with an IT8792E/IT8795E. It reports an ACPI operation region on the primary EC window `0x0A45`–`0x0A46` with no sensor methods. |
| S34 | LibreHardwareMonitor [pull request 2014](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/pull/2014), November 2025, merged as `f6c2c70a509a882449990e2d91afd0b24a8b9a00` | Independent owner test reports on PawnIO-era test builds, plus contributor statements. Owners report readable secondary sensors on B550 AORUS PRO AX rev. 1.0 and X570 AORUS MASTER rev. 1.0. One owner reports the secondary disappearing after some boots, then appearing again, with the OEM low-level driver file `gdrv3.sys` present. A contributor attributes this, without supporting evidence, to configuration mode being changed by other software. Research lead for the competing handler in B2. |
| S35 | LibreHardwareMonitor [issue 2238](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/issues/2238), 2026 | Independent owner observation on an unidentified Gigabyte board with an IT87952E. The secondary is sometimes not detected and is found if the monitor starts sooner after boot. |
| S36 | LibreHardwareMonitor [pull request 672](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/pull/672), February 2022 | Contributor statement that Z690 AORUS PRO (DDR5) carries a secondary with raw `0x8695` beside an IT8689E. Research lead for the LGA1700 installed base only. No register data. |
| S37 | [Primary LPC expansion specification revision 7](superio-ite-primary-lpc-expansion.md) | Project precedent for the slot-0 banked controllers on the same boards. It reuses the count table, the +1 to +125 °C acceptance, two identical immediate reads, the caller-held ISA mutex and the 1.5-second minimum interval. It supplies no secondary-chip hardware fact. |

## Detection

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| Windows, slot 1 (`0x4E`/`0x4F`), pre-key raw `0x8733` (IT8792E/IT8795E) or `0x8695` (IT87952E), active LDN `0x04` with a base distinct from the slot-0 primary, direct EC TMPIN1–3 and FAN1–3 | **Candidate Experimental.** No vendor document is public for these parts (S5 F4). Lead-only facts and their sources: exact IDs and the shared IT8792E/IT8795E ID (S32, S5 F1, F3); firmware-open state and the entry/exit hazard (S32 lines 2833–2851, S5 F6–F8); LDN `0x04` activation, base and port offsets (S32); EC `0x00` and `0x58` acceptance (S32); FAN1–3 enables, count bytes, fixed 16-bit width, divisor 2 and count dispositions (S32, S6); FAN2 and FAN3 pin routing (S32); temperature bytes, encoding and EC `0x51`/`0x55` source decode (S32, S6); ecosystem use of `ioctl_find_bars` on these IDs (S9). Best-evidenced member: AX370-Gaming K7 BIOS F51i, raw `0x8733` version 3 at `0x0A60`, with an independent register dump (S31). Observed members without a dump: X470 AORUS ULTRA GAMING rev. 1.0 F61a (S14); X470 AORUS GAMING 7 WIFI (S3); B650E AORUS MASTER rev. 1.0 (S22); B550 AORUS PRO AX rev. 1.0 and X570 AORUS MASTER rev. 1.0 (S34); X570 AORUS XTREME rev. 1.0 (S33); Z690 AORUS PRO (DDR5) (S36, contributor statement); X870E AORUS MASTER F12 (S28); TRX50 AI TOP ([TRX50 specification](superio-ite-trx50-secondary-linux-hwmon.md)). | Disabled pending Open questions B1 and B2. The rest of the row is complete. If both are resolved, the row would enable best-effort for eligible inputs. |
| Windows, slot 1, pre-key raw `0x8790` (IT8790E), same transport | S32 gives the same transport and count map. It has no TMPIN2 extra-source case. Observed member GA-Z87X-UD5H (S15, S16) has Linux readings and no Windows dump. | Disabled. Deferred to keep the first slice to one temperature-routing rule. It can join the candidate row by a revision after B1 and B2. |
| X470 AORUS ULTRA GAMING rev. 1.0, BIOS F61a, OEM WMI | S14 advertises the GUID but lacks the S13 temperature method. The observed WMI values are all zero. | Disabled. No verified secondary read path. |
| X470 AORUS GAMING 7 WIFI rev. 1.0, BIOS F64, OEM SMM | S24/S26 identify power-event handlers with hardware writes and no temperature or RPM output. S3 reports raw `0x8733`, which the S6 driver mapping groups under IT8792E/IT8795E (**lead-only (copyleft)**). S25 shows an IT8792E marking on an unidentified PCB revision. No evidence links the photograph and raw ID to the same board. | Disabled. No verified OS-owned secondary read path through firmware. The board's direct path is a member of the candidate row. |
| GA-AB350-Gaming 3 rev. 1.x, BIOS F54d, OEM WMI | S12 advertises the GUID but lacks the S13 temperature method. The table's visible monitor pair defaults to the primary controller. | Disabled. No verified secondary read path. |
| Linux, already-bound `it8790`, `it8792` or `it87952` hwmon endpoint (GA-Z87X-UD5H, X870E AORUS MASTER F12, TRX50 AI TOP) | S15/S16 and S28 record positive secondary endpoints. Lead-only S17, S29, S32 and S33 establish that input and type attributes reach a shared refresh that reads EC `0x01`–`0x03` when the cache is cold or expired. The 2026-11-28 recheck covered mainline `7cd8495` and the external driver at `8fdc406`. | Disabled pending Open question L1. |
| X670E AORUS MASTER rev. 1.x, BIOS F41, secondary ECIO | S18 establishes OEM boot-time mailbox access. S19 reports a separate H2RAM endpoint. S20's pump observation has no F41 identity. | Disabled pending an OS-owned read path, exact channel evidence and mailbox arbitration. |
| X870E AORUS MASTER, BIOS F12, OEM firmware inspection | The inspected firmware paths establish no independent secondary telemetry endpoint (S27, S30). | Disabled. The board's Linux endpoint is in the Linux row. Its direct path is a member of the candidate row. |
| B650E AORUS MASTER rev. 1.0, secondary H2RAM via ECIO and separate BIOS F37 WMI inspection | S22 records a slot-1 `0x8733` observation and a G3 v1.00 window with RPM data under the S23 lead-only map. S21's static F37 tables establish no matching secondary telemetry method. | Disabled pending temperature attribution and a read path with proven mailbox ownership, arbitration and side effects. |
| Alternate `0x8883` report, unreadable, absent or changed pre-key ID | S5/S9 distinguish the alternate report from a usable pre-key identity. S4 records a real absent pre-key result. S34/S35 report a secondary that is not detected on some boots. | Unsupported. The candidate reports the secondary as unavailable and sends no key. |
| ECIO or ISA bridge | Separate access domains. See transport limits below. | Disabled. |

Board names, BIOS versions, chip version nibbles and observed base
addresses in the candidate row are evidence identities. They gate no
read (ADR 0031). Under F5 the Gigabyte placement convention explains
where the members were found. The candidate row does not depend on the
board vendor. A slot-1 part with one of the two IDs and the same chip
registers is in the same mechanism row. (S5 F5, S32)

### Runtime gates (candidate row)

| Condition | Gate kind | Source |
| --- | --- | --- |
| Configuration slot 1 on a dedicated `LpcIO` executor. Its slot is selected once and never changed, because `ioctl_select_slot` clears the recorded BARs. | mechanism | S8 |
| Pre-key `CR20`/`CR21` read with no key written, combined value exactly `0x8733` or `0x8695` | mechanism | S32 lines 154, 169 (lead-only); S5 F1, F3, F7 |
| No configuration-mode key and no `CR02` write on slot 1 at any time on this path | safety | S5 F6–F8; S32 lines 2833–2851 (lead-only) |
| `CR07` read before the first selector write, restored to that value at the end of discovery and read back | safety | S8 (the walk leaves `CR07` changed); project read-only policy |
| LDN `0x04` `CR30[0]` set, and `CR60/61` read twice with equal results, in `0x0100`–`0x0FF0`, with bits 2:0 clear and outside `0x0CF8`–`0x0CFF` | mechanism | S32 lines 2980–2993 (lead-only); S8 |
| Secondary base differs from the base cached by the slot-0 primary path, when that path has one | safety | Attribution rule in [Safety notes](#safety-notes); S14, S22, S31 |
| `ioctl_find_bars` succeeds and the EC port proof below succeeds without any configuration exit | mechanism | S8 |
| EC `0x00` bit 0 set and bit 7 clear, and EC `0x58` equal to `0x90` | mechanism | S32 lines 3723–3724 (lead-only); S31 |
| EC `0x13` bit 4, 5 or 6 set for FAN1, FAN2 or FAN3 | mechanism | S32 lines 3562–3563 (lead-only); S31 |
| `CR29[2]` clear for FAN2 and `CR27[7]` clear for FAN3, read with LDN `0x07` selected | mechanism | S32 lines 3320–3368 (lead-only) |
| EC `0x51` source bits and destination field per input, plus EC `0x55[7]` clear for TMPIN2 | mechanism | S32 lines 499–510, 578–588 and 1334–1382 (lead-only); S31 |
| Two identical immediate reads of each fan count pair and each temperature byte | safety | S37 |
| Caller-held `Global\Access_ISABUS.HTP.Method` with a bounded wait across discovery and each sample, and at least 1.5 seconds between sample bursts | safety | S5, S37 |
| Temperature accepted only from +1 to +125 °C | safety | S37; S31 shows −55 °C on an input the reporting tool did not present |

## Register map (research facts)

Every chip-register row is a lead-only fact for raw `0x8733` and
`0x8695`. These rows back only the candidate Experimental row. S31 shows the listed values on one
`0x8733` part.

| Address | Meaning | Encoding | Source |
| --- | --- | --- | --- |
| Slot 1 global `CR20/21`, `CR22` | Raw identity bytes and chip revision nibble. A firmware-open responder can return identity before any entry key is sent. | High byte first for the ID. Low four bits of `CR22` for the revision. Diagnostic only. | S5, S32, S9 (lead-only) |
| Slot 1 global `CR07` | Logical-device selector. | Writing a device number selects it. `ioctl_find_bars` leaves the last index written. | S8; S32 |
| Slot 1 LDN `0x04` `CR30[0]`, `CR60/61` | Firmware activation and runtime LPC base on the direct path. | An active controller reports bit 0 set. An eight-byte runtime block contains its index and data ports at base + 5 and + 6. | S32 lines 210–214, 2980–2993 (lead-only); S6 |
| Slot 1 LDN `0x07` `CR27[7]`, `CR29[2]` | Tachometer pin routing. | `CR27[7]` set means FAN3 has no tachometer input. `CR29[2]` set means FAN2 has none. FAN1 has no routing condition. | S32 lines 3320–3368 (lead-only) |
| Direct LPC EC `0x00` | Monitor configuration. | Bit 0 set starts monitoring. The leads refuse the block when bit 7 is set. S31 shows `0x19`. | S32 lines 3723–3724 (lead-only); S31 |
| Direct LPC EC `0x58` | Register-block vendor byte. | `0x90`. It does not distinguish two ITE controllers. | S32, S6 (lead-only); S31 |
| Direct LPC EC `0x13` | FAN1–3 tachometer enables. | Bits 4, 5 and 6. S31 shows `0x77`. | S32 lines 3562–3563 (lead-only); S31 |
| Direct LPC EC `0x0D`–`0x0F`, `0x18`–`0x1A` | FAN1–3 count low and high bytes. | 16-bit count, low byte read first. Always 16-bit on these profiles. The divisor is 2. | S32 lines 248, 250, 499–510, 578–588 (lead-only); S6 |
| Direct LPC EC `0x29`–`0x2B` | TMPIN1–3. | Signed 8-bit, 1 °C per step. | S32 line 263 (lead-only); S31 |
| Direct LPC EC `0x51` | Temperature source selection. | For input *n* = 1–3, bit *n* − 1 selects a diode and bit *n* + 2 a thermistor. Bits 7:6 name an input (1–3) that carries an external source, or 0 for none. | S32 lines 1334–1382 (lead-only); S31 shows `0x38` |
| Direct LPC EC `0x55[7]` | Extra external-source case for TMPIN2 only on these two profiles. | Set means TMPIN2 reports an external source. S31 shows `0x40`, bit 7 clear. | S32 lines 499–510, 578–588, 1334–1382 (lead-only); S31 |
| Direct LPC EC `0x01`–`0x03` | Alarm status read by the Linux refresh. | Never selected by the candidate. | S17, S32, S33 (lead-only) |

S6 and S32 treat a zero fan count as invalid and an all-ones 16-bit
count as zero measured RPM. Neither implies a disconnected fan. The
Linux driver can write fan enables and other configuration during
initialization. The candidate reads the firmware's existing monitoring
state and routing and writes none of it. (S6, S32 lead-only)

## Read procedure and decode

This procedure belongs to the Disabled candidate row. It is complete
except for the two blocking Open questions. Steps that depend on a
blocking question name it.

### Discovery

Discovery runs once and its result is cached. Hold the ISA mutex for
the whole sequence. A failed mutex wait ends the attempt without port
I/O. (S5, S37)

1. On the dedicated slot-1 executor, call `ioctl_select_slot` with
   slot 1 once. (S8)
2. Read `CR20` and `CR21` with `ioctl_superio_inb`. Write no key first.
   Combine high:low. Continue only on exactly `0x8733` or `0x8695`.
   Any other value, including `0xFFFF`, `0x0000` and `0x8883`, makes the
   secondary unavailable for the session. Write nothing on slot 1 in
   that case. (S5 F1, F3, F7, F8; S32)
3. Read `CR22` and keep its low nibble for diagnostics only. (S32)
4. Read `CR07` and keep it as the original selector. A failed read ends
   discovery before any write. (S8)
5. Call `ioctl_find_bars`. Open question B1 applies to this step. A
   failure makes the secondary unavailable. Then go to step 10 to
   restore the selector. (S8)
6. Write `CR07 = 0x04` with `ioctl_superio_outb` and read `CR07`
   back. Require `0x04`. The read requires this selection write. A
   failure in steps 6 to 8 makes the secondary unavailable, except as
   step 8 states, and goes to step 10. (S8, S32)
7. Read `CR30` and require bit 0 set. Read `CR60/61` twice and require
   equal values that pass the base gate. Require a base distinct from
   the slot-0 primary's cached base. (S32, S8)
8. Write `CR07 = 0x07` and read it back. Read `CR27` and `CR29` once.
   A failed selection or read makes FAN2 and FAN3 unavailable until
   rediscovery. FAN1 and the temperatures are unaffected. (S32)
9. Apart from the walk inside step 5, steps 6 and 8 are the only
   writes in discovery besides the restore in step 10. No
   configuration-mode exit or `CR02` write occurs. (S5)
10. Write the original `CR07` value and read it back. If the readback
    differs, record a restore failure and mark the secondary
    unavailable for the session. No further slot-1 write follows. (S8)
11. EC port proof: write index `0x00` to base + 5 and read base + 6
    with `ioctl_pio_outb` and `ioctl_pio_inb`. Require bit 0 set and
    bit 7 clear. Write index `0x58` and require `0x90` from the data
    port. An access-denied result means the base was not authorized,
    so the secondary is unavailable. (S8, S32)

Open question B2 applies to steps 4–10, which hold the configuration
selector.

### Sample

Each sample uses the cached proof and one ISA mutex acquisition. Bursts
are at least 1.5 seconds apart. No step waits or retries. The steps
follow the banked-modern order of S37 without its bank checks, which
the leads do not give these single-bank parts. (S32, S37)

1. Read EC `0x00`. Require bit 0 set and bit 7 clear, or the whole
   sample is unavailable.
2. Read EC `0x58` and require `0x90`, or the whole sample is
   unavailable.
3. Read EC `0x13`. A failed read makes FAN1–3 unavailable and leaves
   the temperatures unaffected.
4. Read EC `0x51` and `0x55`. A failed `0x51` read makes TMPIN1–3
   unavailable. A failed `0x55` read makes only TMPIN2 unavailable,
   because its bit 7 gates only TMPIN2 (step 6); TMPIN1 and TMPIN3 are
   still validated from `0x51`. Neither failure affects the fans.
5. For each fan whose enable bit is set and whose routing was clear at
   discovery, read the low byte, high byte, low byte and high byte.
   Accept the count only if both pairs match.
6. For each temperature input *n*, publish only when bits 7:6 of EC
   `0x51` differ from *n* and exactly one of its diode and thermistor
   bits is set. For TMPIN2, EC `0x55` bit 7 must also be clear. Read
   EC `0x28 + n` twice in immediate succession and accept the input
   only if both bytes match.

Decode a count with the table of the
[primary LPC expansion specification](superio-ite-primary-lpc-expansion.md#read-procedure-and-decode):

| Count | Result |
| --- | --- |
| `0x0000`, `0x0001`–`0x003F` | Invalid. No RPM. |
| `0x0040`–`0xFFFE` | Active. RPM is the whole number `floor(675000 / count)`. |
| `0xFFFF` | Inactive at 0 RPM. Saturation means no measurable rotation. |

The `675000` constant restates the documented 16-bit conversion with a
fixed divisor of 2 (S32 lines 770–771, lead-only). Decode a temperature
byte as signed and accept +1 to +125 °C inclusive (S37). A failed
individual read invalidates only its input. Successful siblings keep
the generic secondary names `FAN1`–`FAN3` and `TMPIN1`–`TMPIN3`.
Physical connector labels are unresolved, so the candidate assigns none. The
sample never reads EC `0x01`–`0x03`, `0x0C`, `0x06`, any index at or
above `0x80`, or any configuration register. Primary results stay
independent. A secondary failure never alters them. (S5, S31, S32, S37)

### Synthetic fixture acceptance

These cases are synthetic. The first case copies bytes from the
independent S31 dump. A passing fixture does not prove compatibility
with real hardware.

| Case | Expected result |
| --- | --- |
| Pre-key `0x8733`, version 3, `CR07` original `0x07`, LDN `0x04` active at `0x0A60`, primary cached at `0x0A40`, `CR27[7]` and `CR29[2]` clear, EC `0x00=0x19`, `0x58=0x90`, `0x13=0x77`, `0x51=0x38`, `0x55=0x40`, FAN1 `0xB6`/`0x08` twice, FAN2 and FAN3 `0xFF`/`0xFF` twice, TMPIN1–3 `0x29`, `0xC9`, `0x28` twice. EC bytes are from S31. Configuration-register values are synthetic. | FAN1 Active at 302 RPM, FAN2 and FAN3 Inactive at 0 RPM. TMPIN1 41 °C, TMPIN3 40 °C, TMPIN2 omitted as −55 °C. `CR07` written back to `0x07`. No `CR02`, key or EC `0x01`–`0x03` access. |
| Pre-key `0xFFFF`, `0x0000`, `0x8883`, `0x8790` or any other value | Secondary unavailable. No slot-1 write of any kind. Primary results unchanged. |
| Pre-key `0x8695` with the same register bytes | Same decode as the first case. |
| Secondary base equal to the primary base, unequal double read, base below `0x0100`, misaligned, or in `0x0CF8`–`0x0CFF` | Secondary unavailable. `CR07` restored. |
| `ioctl_find_bars` fails, or the port proof returns access denied | Secondary unavailable. `CR07` restored. |
| `CR07` restore readback differs | Restore failure recorded. Secondary unavailable for the session. No further slot-1 write. |
| EC `0x00` bit 7 set, bit 0 clear, or `0x58` not `0x90` | Whole sample unavailable. |
| `0x51` bits 7:6 equal 1, or `0x55` bit 7 set, or an input with both or neither source bit | Only the affected temperature is omitted. |
| Fan pairs disagree, count `0x0000` or `0x003F`, count `0x0040`, count `0xFFFE` | Invalid, invalid, 10546 RPM, 10 RPM. A sibling survives. |
| Temperature bytes disagree, or `0x00`, `0x7E`, `0x7F` | Omitted, omitted, 126 °C omitted, 127 °C omitted. |
| `CR27` read fails at discovery | FAN2 and FAN3 unavailable until rediscovery. FAN1 and temperatures continue. |
| ISA mutex wait times out | Sample skipped with no port I/O. |
| Secondary absent and primary present, or the reverse | The present controller's results are unchanged. |


## Transport limits

| Transport | Evidence and limitation |
| --- | --- |
| ECIO | S9 describes a firmware command/status and data mailbox, separate from the direct EC register pair. A read still writes the read command and offset to the mailbox. The cited implementation uses `LpcIO` at ports `0x03F4` and `0x03F0`. That window also contains the historical floppy-controller registers, including a write-only data-rate register at `0x03F4`. Authorization for a port range does not prove mailbox ownership. Firmware capability and fan-control state reads leave the telemetry offset map unresolved. Require exact-board port ownership, bounded waits and recovery, and a separately verified telemetry layout. (S8, S11, S9/S10 lead-only) |
| ISA bridge | `IsaBridgeEC` module load checks architecture and CPU vendor. Its discovery IOCTL probes both slots and can issue a slot-1 enter sequence after `0xFFFF` during its own discovery, independently of a caller pre-check. The IOCTL can also restore prior bridge configuration. Mapping the window and programming the bridge are separate operations. The state-changing operation writes PCI bridge decode settings, and the cited consumer may use it even to read a firmware flag. A firmware-preconfigured window would still require safe discovery and validated sensor offsets. (S8, S10 lead-only) |
| Direct LPC on slot 1 | The candidate row's transport. `ioctl_find_bars` is the only signed way to authorize the EC window. On a firmware-open part it walks the selector across every logical-device index and leaves it changed. The candidate saves and restores `CR07` around the walk. It cannot avoid the walk itself (B1). The configuration pair stays open for any other agent while the candidate holds the selector (B2). (S8, S9 lead-only, S34, S35) |

### AX370-Gaming K7 secondary dump

S31 is the only independent register dump of a secondary part found so
far. Its secondary section reports chip ID `0x8733`, chip version 3, EC
base `0x0A60`, GPIO base `0x0A50`, and these bank-0 bytes:

| EC index | Byte | Candidate interpretation (lead-only map, S32) |
| --- | --- | --- |
| `0x00` | `0x19` | Bit 0 set, bit 7 clear: monitoring started, block accepted. |
| `0x01`, `0x02`, `0x03` | `0x10`, `0x40`, `0x00` | Alarm status, with bits set in two of the three bytes. Never read by the candidate. |
| `0x06` | `0x00` | Not read by the candidate. |
| `0x0D`/`0x18` | `0xB6`/`0x08` | FAN1 count `0x08B6`, 302 RPM. |
| `0x0E`/`0x19`, `0x0F`/`0x1A` | `0xFF`/`0xFF`, `0xFF`/`0xFF` | FAN2 and FAN3 saturated, 0 RPM. |
| `0x13` | `0x77` | FAN1–3 tachometers enabled. |
| `0x29`, `0x2A`, `0x2B` | `0x29`, `0xC9`, `0x28` | 41 °C, −55 °C, 40 °C. |
| `0x51` | `0x38` | Thermistor sources on TMPIN1–3, no external destination. |
| `0x55` | `0x40` | Bit 7 clear: no TMPIN2 external source. |
| `0x58` | `0x90` | Vendor byte. |

The same report's sensor tree shows FAN1 between 182 and 363 RPM and
FAN2 and FAN3 at 0 RPM. It shows temperatures #1 and #3 at 40–42 °C.
It omits temperature #2, which matches the −55 °C byte. The tree and
the dump were not taken at the same instant. The report first lists
raw `0x8733` at `0x4E`/`0x4F` under its Winbond/Nuvoton/Fintek probe,
before the ITE listing. Those two probes use different configuration
keys (S5). That order is consistent with F7, but the report does not
record the pre-key state. The primary is reported as `0x8686` at
`0x0A40`, a separate base. The report predates PawnIO and so says
nothing about `ioctl_find_bars`. (S5, S31, S32 lead-only)

### PawnIO-era owner observations

The pinned monitor calls `ioctl_find_bars` on slot 1 for these IDs
(S9, lead-only). In S34, owners of B550 AORUS PRO AX rev. 1.0 and X570
AORUS MASTER rev. 1.0 report secondary sensors with PawnIO-era test
builds. Neither report records a register dump, the original selector
or the BIOS version. A third S34 owner reports a secondary that is
detected on some boots and missing on others while the OEM low-level
driver file `gdrv3.sys` is present. S35 reports a secondary found only
when the monitor starts soon after boot. These reports show that
detection can change after boot. They do not identify the agent
involved or whether it holds the ISA mutex. (S9, S34, S35)

### GA-AB350-Gaming 3 BIOS F54d OEM WMI

The `GSWApp` SSDT advertises WMI GUID
`DEADBEEF-2001-0000-00A0-C90629100000` as method object `BB`. Its
serialized `WMBB` method, at table offset `0x6602`, has no dispatch
branch for the S13 temperature method ID `0x125` (293 decimal). The
table's decimal 125 branch uses method ID `0x7D`. (S12, S13 lead-only)

The table declares primary Super I/O configuration ports
`0x2E`/`0x2F` and a monitor index/data region defaulting to
`0x0A45`/`0x0A46`. This identifies the primary monitor window. The
inspected table does not establish a read-only route to the secondary
controller, its EC firmware identity, or exclusion against concurrent
firmware accesses. Other firmware paths remain unresolved. (S12)

### X470 AORUS ULTRA GAMING BIOS F61a

On April 7, 2021, the owner identified the board as rev. 1.0 and reported
zero values for all six WMI channels. The follow-up boot log identifies
`X470 AORUS ULTRA GAMING-CF` with BIOS `F61a` dated March 29, 2021.
It shows a reservation at `0x0A40`–`0x0A4F`, an unsupported chip
with raw ID `0x8686`, and Linux detection of `IT8733E` at
`0x0A60`, revision 3. The log does not record the secondary response
before mode keys or distinguish IT8792E from IT8795E by physical marking.
The earlier sensor output uses `it8728-isa-0a60`, so its label and
channel values cannot establish a matching sensor profile. (S14)

The captured `ALASKA / A M I` DSDT (26,345 bytes, valid checksum,
SHA-256 `0efdd94d8368dd56bb81124e724e57306ffeebf79fc73713580e232c9817c447`)
declares constants `0x0A60` and `0x0A65`. The declarations do not
show a method accessing either address. (S14)

The captured `GBT / GSWApp` SSDT (27,785 bytes, valid checksum,
SHA-256 `be4fd82b12fb13ccb74726778b29a69a564e47708505709c65d866cc4c9c94ba`)
advertises the S13 WMI GUID. Its `WMBB` dispatcher at offset
`0x65B6` lacks temperature method ID `0x125` (293 decimal).
Its decimal 125 branch is `0x7D`. The SSDT monitor I/O region at
offset `0x2CEF` defaults to primary ports `0x0A45`/`0x0A46`.
These tables do not establish a read-only secondary WMI path or
serialization with concurrent firmware access. (S14, S13 lead-only)

### X470 AORUS GAMING 7 WIFI BIOS F64 OEM SMM

The teardown shows a chip marked IT8792E and a primary chip marked
IT8686E on a board of unidentified PCB revision. A separate August 20,
2026 owner log identifies `X470 AORUS GAMING 7 WIFI-CF`. It reports
primary ID `0x8686` at configuration index port `0x2E` and EC base
`0x0A40`, and secondary ID `0x8733` at `0x4E` and EC base `0x0A60`.
S6 groups that raw ID under IT8792E/IT8795E (**lead-only (copyleft)**).
The detector lists both controllers as detection-only, with no
temperature, RPM, or established pre-key response. Its board version
is `Default string` and its reported BIOS version is `5.17`. No
evidence ties the log to the photographed unit or either observation
to rev. 1.0 running F64.
(S3, S5, S24, S25)

Static inspection of the F64 image identified SMM firmware file
GUID `CB3C180C-CF14-4165-BC6D-2116FF8D8BEA`, with x86-64 PE
SHA-256 `f44efa6dc049790ada37dcea8ea25c1732ef4a6c16960762940bd531001963cf`.
The inspected power-button and S3, S4 and S5 entry handlers write
hardware and return no temperature or RPM data. This finding covers
only those handlers. (S24, S26)

### X670E AORUS MASTER BIOS F41 OEM ECIO

Static inspection of S18 with UEFIExtract A76 identifies the
`ThermalFanCtrlDxe` FFS file
`2FD10E90-C379-45CE-BD79-EEFB1600DEAC`. Its extracted x86-64 PE image
has SHA-256
`4f1f00449d181011a3cfec5ad0b79cc450cfb77c9c07ee785044fe19cecbb2fc`.
The routine at PE RVAs `0x19C8`–`0x1AA3` establishes that the F41
boot-time firmware uses `0x03F4` and `0x03F0` for a command/status
and data mailbox. Its waits for mailbox status are bounded. The
four direct callers use the read in configuration updates. The
read routine can return after a timeout during a partial operation
without an observed abort or recovery step. The image does not
establish host ownership after boot, arbitration with firmware or
other clients, or a fixed read-only temperature and RPM endpoint.
(S18)

Static inspection of 72 distinct checksum-valid DSDT and SSDT
tables from the F41 image found no `_FST`. The three `_TMP`
definitions use AMD processor temperature sources. The OEM
`GBT/GSWApp` WMI table, SHA-256
`ca4bb4556b52c5353671d52a42d1dc20f28f755a29110eccc3675bb65c47754e`,
fixes its indexed ITE monitor region at primary `0x0A45`/`0x0A46`.
These findings cover the recovered static F41 tables. Tables
generated or patched at boot and other BIOS versions are outside
the inspection. (S18)

S19 independently reports an IT8792E/IT8795E-kind device, revision 3,
at `0x0A60` on F41 using the H2RAM backend of an opt-in DKMS build.
Its `SYS_FAN4` and `SYS_FAN5/6_PUMP` headers were unpopulated. This
is a detection observation. S20 separately reports RPM from
`SYS_FAN7/8_PUMP` through a modified monitor, with no BIOS version
or positive thermistor sample. These observations concern different
headers and do not qualify a firmware-matched temperature and RPM
path. Firmware module names do not establish the pump controller's
physical marking. (S18–S20)

### X870E AORUS MASTER BIOS F12 secondary endpoint evidence

The F12 owner report shows separate `it8696-isa-0a40` and
`it87952-isa-0a60` endpoints. The latter reports 37°C, 3,096 RPM
and 497 RPM. The temperature location labels are provisional.
The stated DKMS build uses a shared refresh for both input types,
including reads of all three alarm-status registers on a cold cache.
(S28, S29 lead-only)

Static F12 inspection recovered four checksum-valid DSDTs and 71
checksum-valid SSDTs from the image and standard LZMA regions. The
recovered tables contain no `_FST`, standard ACPI fan or embedded
controller device identity, or ECDT. Three AMD SSDTs define `_TMP`
for CPU/APU thermal zones. The AMD MPMS SSDT at decoded offset
`0x5BB780` presents temperature and fan records with counter-generated
values. Its SHA-256 is
`e443530f5be170ff7ad9e11a488e408584b6b3b3426b0b16bffccba2ea28ae46`.
(S27)

The `GBT / GSWApp` SSDT is 45,533 bytes with SHA-256
`6d81dda24096c802bd91f4d32655cdb1a0315bc7548866d8f521dc249ab4ef72`.
It begins at decoded offset `0x1322904` in the LZMA section at image
offset `0x1079C58`. It advertises the S13 GUID, and its dispatcher
has no temperature method `0x125`. Its indexed monitor region defaults
to `0x0A45`/`0x0A46`. The recovered DSDTs separately declare `0x0A65`.
No recovered method references those declarations. (S13 lead-only, S27)

The inspected SMM handlers supply no temperature or RPM endpoint.
The power event handlers in FFS `CB3C180C-CF14-4165-BC6D-2116FF8D8BEA` write
hardware and return no temperature or RPM. Its x86-64 PE SHA-256 is
`a38dface1c5177c87fa9d745f56473d8d3e62b067e2861a213ff68bc33cc6248`.
The ExitBootServices notification in FFS
`471D888A-AB93-4663-A24D-24FA32F3FC82` returns status only. Its
x86-64 PE SHA-256 is
`c1230b69a9d8d8a9533db00df7eb5477b995f49ab4fc694933da542b03bbf839`.
The static findings cover the recovered tables and inspected handlers.
(S26, S27, S30)

### B650E AORUS MASTER rev. 1.0 H2RAM and F37 WMI

The owner reports DMI board name `B650E AORUS MASTER` and points to
Gigabyte's rev. 1.0 product. The December 2025 detector reports
slot-1 `0x8733` without entering that slot and reads a versioned
H2RAM window through ECIO with SMFI inactive. The August 2026
out-of-tree driver log identifies distinct runtime bases, primary
IT8689E revision 2 at `0x0A40` and secondary raw-ID family revision 3
at `0x0A60`. It reports H2RAM Smart Fan G3 v1.00 via ECIO and SIV ID
`0x900A0909`. The raw ID and driver label do not distinguish IT8792E
from IT8795E. The posts supply neither a PCB marking nor a BIOS or
EC firmware revision. (S21, S22)

S23 identifies three G3 tachometer pairs at H2RAM `0x916`/`0x917`,
`0x918`/`0x919` and `0x91A`/`0x91B`, in little-endian RPM. The
first pair in S22's December dump decodes to 859 RPM under that
lead-only map. This supports a live RPM mirror without identifying
its physical header. It does not establish a live temperature
field, coherency of the word during sampling, or which controller
owns the separate IT57xx extension. (S22, S23 lead-only)

The S23 host-side read sends a command and offset to the ECIO
mailbox, polls `0x03F4` and receives a byte from `0x03F0`. These
operations do not directly read the HWM alarm registers. The
controller's internal handling of the request and the ownership
and arbitration of the mailbox ports remain unproven. The driver
serializes access only among its own users and reports a failed
byte read as zero. Its hwmon cold refresh still reads HWM alarm
registers before returning individual input values. These routes
therefore do not qualify for collection. (S11, S17/S23 lead-only)

The separate static F37 inspection recovered a checksum-valid
`GBT / GSWApp` SSDT of 43,036 bytes, SHA-256
`c1470503b4123457702f650834809965e159f86800cbcdb0cba92bb6e7851c83`.
The table starts at decoded offset `0xD44A8C` in the LZMA section
at BIOS image offset `0x1076028`. The table advertises the S13 WMI
GUID at offset `0x6B38`. The serialized `WMBB` dispatcher at
`0x9C29` has no temperature method `0x125`. The indexed monitor
region defaults to primary `0x0A45`/`0x0A46`. A bounded scan of
simple named SystemIO declarations found no literal base at the
observed secondary monitor or mailbox ports. The inspection did
not establish an OS-callable secondary temperature or RPM query.
It does not cover
SMM handlers or tables generated or patched at boot. S22's live
observations cannot be tied to F37. (S21, S22, S13 lead-only)

### GA-Z87X-UD5H IT8790E through Linux hwmon

On March 5, 2017, the owner identified a GA-Z87X-UD5H running Debian
Linux 4.9.6. Its boot log reports an IT8728F at `0x0A30`, revision 1,
and an IT8790E at `0x0A40`, revision 3. The primary `it87.2608` had
an `it87` driver link and a hwmon child. The secondary `it87.2624`
platform node had neither, so detection alone did not make the
secondary readable. (S16)

On October 19, 2022, the same author reported `it8790-isa-0a40`
alongside `it8728-isa-0a30` on a Z87 system. The IT8790E output shows
three fan readings of 958, 540 and 309 RPM and three temperatures of
29, 34 and 51 °C, including one PECI input. That capture does not repeat
the board model or record its PCB revision, BIOS, kernel or canonical
hwmon links. The readings do not verify physical connector labels.
`0x0A40` is the observed secondary base in this case and the primary
base in the X470 evidence above. A base alone cannot assign a
controller's role or select a board profile. (S14, S16)

A future Linux scope would require matching board identity and a
canonical `it8790` hwmon child under platform device `it87.2624`,
with the parent bound to `it87`. A detected but unbound node leaves
the secondary unavailable. Restrict collection to an existing
kernel-owned endpoint under ordinary driver settings. No module
loading, rebinding, conflict overrides or raw I/O are part of this
candidate. Preserve independent primary results. (S16, S17 lead-only)

Both pinned drivers use the same refresh for `temp*_input` and
`fan*_input`. An empty or expired cache causes reads of EC alarm
registers `0x01`–`0x03` even if the application requests only a
temperature or RPM value. The driver documentation describes latched
alarms that can be lost after a read, but does not establish the read
effects of these bytes on IT8790E. Neither pinned driver supplies a
separate input path that avoids this refresh on IT8790E. (S17 lead-only)

### Linux input refresh recheck, 2026-11-28

Mainline at `7cd8495` (S32) and the external driver at `8fdc406` (S33)
still read EC `0x01`–`0x03` in the refresh. Both temperature and fan
input attributes reach that refresh. In S32 the temperature type
attribute reaches it too. No module parameter or chip flag in either
source skips the alarm reads. The external driver's changes since
`bd7a566` add board admission entries, an Intel bridge fix and
maintenance changes. The X570 AORUS XTREME entry reports an IT8688E
with an IT8792E/IT8795E. It reports an ACPI region over only the
primary EC window. S31 shows set bits in two
of the three status bytes on a running secondary. A refresh on such a
part reads that pending state. The read effects for these parts are
not published. (S31, S32, S33 lead-only)

## Quirks

- Matching reported bases do not transfer a profile between the X470
  AORUS ULTRA GAMING and X470 AORUS GAMING 7 WIFI. (S3, S14)
- One TRX50 AI TOP owner saw pre-key `0xFFFF` on the secondary and later
  reported a separate secondary sensor block after several reboots.
  The two observations do not establish what changed or a stable
  pre-key identity. (S4)
- Detection of the secondary can change between boots, or within a
  boot after other software starts. The candidate treats any ID other
  than the two accepted values as unavailable and never sends a key
  to recover it. (S34, S35)
- An input that the reporting tool does not present can read `0xC9`
  (−55 °C) with its thermistor source bit set. The +1 °C lower bound omits it. (S31)

## Safety notes

- The slot-1 exit-withhold rule remains the diagnostic boundary. The
  candidate forbids both entry and exit writes on slot 1. Its absence
  path stops without trying mode keys. (S5, S32/S9 lead-only)
- S32 states that entering configuration mode on these IDs is also part
  of the reported failure. The existing Phase 2 diagnostic in S5 sends
  the slot-1 ITE key before reading the ID. The candidate does not
  reuse that sequence and treats the `0x8883` result of a re-sent key
  as unavailable. (S5 F8, S32 lead-only)
- The only writes on the candidate path are `CR07` selections required
  by the read, the restore of the original `CR07`, and EC index-port
  writes. Each selection is read back. A failed restore disables the
  secondary for the session. (S8)
- Use a bounded, caller-held ISA mutex for every multi-step collection.
  Keep primary results independent of a secondary failure. Equal,
  invalid or unknown bases fail secondary attribution. (S5)
- Sensor scope excludes monitor activation, pin routing changes, alarm
  reads and clearing, fan/PWM control, limit changes, PCI decode changes
  and EC firmware configuration. Do not reuse a fan-control transport as
  a telemetry fallback. (S5, S8)

The synthetic fixture table above covers absent, alternate and changed
pre-key IDs, equal bases, selector restoration failure, denied ports,
and invalid or unavailable channels beside a successful sibling.
Compare recorded transactions to confirm that no slot-1 mode key, `CR02`
write, alarm read or hardware configuration write occurs. Synthetic
traces demonstrate the software boundary only.

## Target inventory and disposition

Priority follows the installed base named in #2519. Mainstream AM4,
AM5 and LGA1700 boards with a raw `0x8733` or `0x8695` secondary come
first. Every target below is open under
[#2519](https://github.com/shm11C3/HardwareVisualizer/issues/2519)
unless the row names other existing coverage.

| Priority | Target | Disposition | Evidence that unblocks it |
| --- | --- | --- | --- |
| 1 | IT8792E/IT8795E raw `0x8733` and IT87952E raw `0x8695`, Windows direct LPC slot 1. Members include AM4 AX370, AB350, X470, X570 and B550, AM5 B650E and X870E, LGA1700 Z690, and TRX50 (S3, S5 F5, S14, S22, S28, S31, S33, S34, S36). | Candidate Experimental row defined in this revision. Disabled. Open follow-up #2519. | B1 and B2 below. |
| 2 | Linux hwmon `it8792`, `it87952` and `it8790` endpoints: X870E AORUS MASTER F12, TRX50 AI TOP, GA-Z87X-UD5H and X470 AORUS ULTRA GAMING (S14, S16, S28; [TRX50 specification](superio-ite-trx50-secondary-linux-hwmon.md)) | Disabled. Open follow-up #2519. | L1 below. |
| 3 | IT8790E raw `0x8790`, Windows direct LPC slot 1. Members are LGA1150 Z87 and Z97 boards (S5 F5, S15). | Disabled. Deferred row. Open follow-up #2519. | B1 and B2, then a revision that adds its TMPIN2 routing rule (S32). |
| 4 | Alternate `0x8883`, absent `0xFFFF` or a changed pre-key ID | Unsupported for collection. Existing coverage: the diagnostic exit withhold in S5. | None needed. These are state results, not chips. |
| 5 | ECIO mailbox and H2RAM: X670E AORUS MASTER F41 and B650E AORUS MASTER G3 (S18–S23) | Disabled. Open follow-up #2519. | A fixed OS-owned endpoint, or a published mailbox ownership and arbitration contract with a non-mutating read set. A live temperature tied to the physical controller. |
| 6 | ISA bridge: `IsaBridgeEC`, the opt-in Intel bridge on W480 VISION W, and the Z390 AORUS PRO WIFI routing report (S8, S10; #2519 research) | Unsupported. Reads require PCI decode writes. Open follow-up #2519. | An already-owned fixed decode with a read-only access contract. |
| 7 | IT5701 H2RAM extension on X670E and X870E (#2519 research) | Disabled. Open follow-up #2519. | A versioned native read path with an ownership contract. Independent evidence tying a live temperature, its units and its invalid state to the extension. |
| 8 | USB HID `048D:5702` (#2519 research) | Unsupported. No report contract. Open follow-up #2519. | A public input report contract or exact independent capture with read-only temperature and tachometer values tied to the physical controller. |
| 9 | OEM ACPI, WMI and SMM in the audited images: AB350 F54d, X470 ULTRA GAMING F61a, X470 GAMING 7 WIFI F64, X670E F41, B650E F37, X870E F12, TRX50 F11 and F12h, Z490 AORUS MASTER F24a, Z890 AORUS TACHYON ICE F7, GA-Z87X-UD5H F10e | Unsupported for those images. None exposes a secondary temperature and RPM endpoint. Open follow-up #2519 for other images. | A released image with a dispatch branch or method that returns current secondary values, with attribution and serialization evidence. |
| 10 | Linux `gigabyte-wmi` | Not a secondary path. It publishes primary-window temperatures only. Existing coverage for one board: [B850M WIFI6E ICE specification](superio-ite-b850m-wifi6e-ice-linux-wmi.md). | Not applicable to #2519. |
| 11 | Linux `gigabyte_waterforce` | Outside #2519. It serves a separate USB AIO cooler, not a motherboard controller. | Needs its own cooler issue if wanted. |
| 12 | BSD drivers, and BMC sensors on MC12-LE0 and WRX80-SU8-IPMI (#2519 research) | Unsupported. The BSD tables omit the target IDs. The BMC readings are not tied to a secondary ITE. | A driver or management interface that names the target controller and its read semantics. |
| 13 | Pending PawnIO.Modules changes: bridge transactions, firmware identification, generic EC (#2519 research) | No released path. Open follow-up #2519. | A signed release with a current-value interface for an exact controller. A narrow single-device authorization would also resolve B1. |

## Open questions

- **B1. Blocking for the candidate row: selector walk of the signed
  authorization.** The candidate can reach the secondary EC ports only
  through `ioctl_find_bars`. On a firmware-open part, that call writes
  every logical-device index from `0x00` to `0xFE` to `CR07` and leaves
  the selector changed (S8). S9 (lead-only) shows that the pinned
  monitor makes the same call on these IDs. S34 reports working
  secondary readings with PawnIO-era builds on two AM4 boards. No source
  states what selecting an undefined logical device does on these parts.
  No source excludes another agent relying on the selector during the
  walk. Any one of these resolves B1:
  - a signed `LpcIO` release that authorizes the base of one
    caller-named logical device without walking the others, pinned by a
    [PawnIO interface](pawnio-interface.md) revision;
  - a maintainer decision to accept the walk for this Experimental row,
    with the `CR07` save and restore in this procedure, on the
    ecosystem evidence of S9 and S34;
  - an accepted [#2500](https://github.com/shm11C3/HardwareVisualizer/issues/2500)
    report from a member board. In one boot it must record the original
    `CR07`, a completed walk, the restored `CR07` and a successful
    primary sample afterwards.
- **B2. Blocking for the candidate row: competing configuration-space
  handler.** A firmware-open part accepts selector and configuration
  writes from any agent without a key. Between the candidate's `CR07`
  writes in discovery steps 4–10, another agent that does not hold the
  ISA mutex could find a different device selected and apply its own
  write to the wrong device. S34 and S35 report detection changing
  after boot, with an OEM low-level driver present in one report. No
  source identifies that agent, its lock convention or any runtime
  firmware access to slot 1. The static images in this document show no
  ACPI method using the secondary ports (S14, S21, S27). They do not
  cover runtime SMM in general. Any one of these resolves B2:
  - public evidence that the OEM utility and its driver hold
    `Global\Access_ISABUS.HTP.Method`, or never touch slot 1 at
    runtime;
  - a static audit of a member board's software SMI handlers that finds
    no slot-1 or secondary-EC access;
  - a maintainer decision to accept, for discovery on slot 1, the same
    residual concurrency risk that S37 accepts on slot 0. The
    mitigations are the mutex, readback of each selection, a single
    cached discovery, and no retries.

  Sampling touches only the EC index and data ports. The two identical
  reads cover a concurrent index write there, as S37 does on slot 0.
- **L1. Blocking for every Linux hwmon route: alarm-status refresh.**
  In S32 and S33, reading a temperature, fan or temperature type
  attribute can read EC `0x01`–`0x03` when the cache is cold or
  expired. S31 shows set bits in those bytes on a running secondary.
  The read semantics of these bytes on IT8790E, IT8792E/IT8795E and
  IT87952E are not published. Either of these resolves L1:
  - a released kernel or normally bound driver whose input refresh
    excludes alarm reads, battery-measurement triggers, control writes
    and bridge reprogramming, with a capability the app can detect
    before opening an input. Driver name, ancestry and attribute names
    are the same on both sides of such a change.
  - a maintainer decision on kernel-owned status reads, which also
    blocks the [GA-B75M-D3H Linux specification](superio-ite-b75m-d3h-linux-hwmon.md).
    A decision covering both would need the part's alarm-read
    semantics, or a recorded acceptance that they are unknown.
- Non-blocking for the candidate row: physical connector labels are
  unresolved, and the candidate publishes generic `FAN1`–`FAN3` and
  `TMPIN1`–`TMPIN3` names. A board mapping needs a same-session
  correlation of an enabled secondary fan or temperature with the
  board firmware monitor. Record byte pairs, routing flags, disabled and
  inaccessible cases and repeatability. (S2, S7 lead-only, S31)
- Non-blocking for the candidate row: the shared raw `0x8733` does not
  distinguish IT8792E from IT8795E. The two share one lead-only
  profile, so the candidate needs no distinction (S5 F1, S32). A
  physical marking on a member board would let diagnostics name the
  model.
- Non-blocking for the candidate row: an exact-board pre-key trace on
  X470 AORUS ULTRA GAMING rev. 1.0 was the revision 8 entry condition.
  Under ADR 0031 that board is an evidence member, and the runtime ID
  gate checks the pre-key state on every system. A trace with both raw
  ID and revision bytes, activation and base registers, and the PCB
  revision would raise it to a best-evidenced member. (S1, S2, S14)

- For X470 AORUS GAMING 7 WIFI rev. 1.0, establish the exact physical
  secondary chip and link it to the controller endpoint on the same
  board and firmware. Establish an OS-owned read path for temperature
  and RPM on that controller with proven ownership and arbitration
  and without alarm reads or hardware state changes. (S3, S5, S24–S26)

- For X670E AORUS MASTER F41 ECIO, establish a fixed OS-owned
  secondary temperature and RPM endpoint or a proven arbitration
  and recovery contract with an exact, non-mutating read set.
  Neither the recovered OEM tables nor the independent reports
  provide those facts. (S18–S20)

- For X870E AORUS MASTER F12, establish a safe, OS-owned endpoint for
  temperature and RPM with exact secondary controller and channel
  attribution. The observed DKMS input path includes alarm reads.
  The recovered firmware evidence does not establish an independent
  endpoint or its ownership contract. (S27–S30, S29 lead-only)

- For B650E AORUS MASTER rev. 1.0, establish a temperature field
  with exact attribution and valid samples, and a firmware-matched
  OS-owned route for the temperature and RPM. The complete read
  path needs evidence of mailbox ownership, arbitration, error
  handling and absence of alarm reads or control writes. (S21–S23)

- For GA-Z87X-UD5H Linux hwmon, L1 applies. After L1, obtain
  same-session board, driver and canonical hwmon binding evidence with
  a valid secondary sample before enabling this candidate. (S16, S17
  lead-only)

Use [#2500](https://github.com/shm11C3/HardwareVisualizer/issues/2500) for
manual, opt-in collection of the board, PCB and firmware identity and
existing compatibility evidence. A raw transaction trace requires a
separately reviewed capture procedure on that exact board. Prefer a
passive trace of an already working firmware-open session when one is
available. An absent pre-key response is itself an evidence result.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-08 | Record the exact X470 rev. 1.0 research candidate, independent observations, and separate LPC, ECIO and bridge transport evidence requirements. |
| 2 | 2026-11-08 | Pin the GA-AB350-Gaming 3 F54d OEM WMI evidence and the reason this route is disabled. |
| 3 | 2026-11-08 | Corroborate the X470 rev. 1.0 dual-controller observation under F61a and record why its OEM WMI route is disabled. |
| 4 | 2026-11-08 | Record GA-Z87X-UD5H IT8790E Linux endpoint observations and the alarm-refresh evidence that keeps collection disabled. |
| 5 | 2026-11-08 | Record the exact X670E AORUS MASTER F41 boot-time ECIO mailbox and separate Linux and pump observations. Keep the ECIO candidate disabled. |
| 6 | 2026-11-09 | Record the B650E AORUS MASTER rev. 1.0 RPM mirror and separate F37 WMI inspection. Keep the candidate disabled pending temperature and mailbox safety evidence. |
| 7 | 2026-11-09 | Record the X470 AORUS GAMING 7 WIFI F64 SMM findings and separate board marking and raw-ID observations. Keep collection disabled pending exact controller attribution and a safe OS-owned read path. |
| 8 | 2026-11-09 | Record X870E AORUS MASTER F12 secondary readings, the reported driver read effects, and the static firmware findings. Keep the candidate disabled. |
| 9 | 2026-11-28 | Re-scope the exact-board X470 AORUS ULTRA GAMING candidate under ADR 0031. The new candidate row covers Windows slot 1, pre-key raw `0x8733` or `0x8695`, with mechanism and safety gates. Add its runtime gates, lead-only register map from mainline `7cd8495` (S32), discovery and sample procedure with `CR07` save and restore, and synthetic fixtures. Add the AX370-Gaming K7 independent secondary dump (S31), PawnIO-era owner observations (S34, S35), the LGA1700 `0x8695` contributor statement (S36) and the 2026-11-28 Linux refresh recheck (S32, S33). Name blockers B1, B2 and L1 with the evidence that resolves each. Defer IT8790E to a later row. Add a disposition for every #2519 target. Status remains Draft. A failed EC `0x55` read invalidates only TMPIN2. |
