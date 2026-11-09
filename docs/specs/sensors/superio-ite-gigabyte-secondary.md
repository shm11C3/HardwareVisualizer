# Spec: Gigabyte secondary ITE sensor access

| Field | Value |
| --- | --- |
| Revision | 7 |
| Status | Draft — not implementation-ready |
| Scope | Evidence for a secondary, firmware-open ITE LPC Environment Controller on the Gigabyte X470 AORUS ULTRA GAMING rev. 1.0. Separate ECIO and ISA bridge transport restrictions, and OEM WMI evidence for GA-AB350-Gaming 3 BIOS F54d and X470 AORUS ULTRA GAMING BIOS F61a. Linux hwmon evidence for the GA-Z87X-UD5H IT8790E and ECIO evidence for X670E AORUS MASTER BIOS F41 and B650E AORUS MASTER rev. 1.0. Separate X470 AORUS GAMING 7 WIFI rev. 1.0 BIOS F64 SMM evidence and unresolved secondary controller attribution. |
| Issue phase | [#2519](https://github.com/shm11C3/HardwareVisualizer/issues/2519) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Gigabyte, [X470 AORUS ULTRA GAMING rev. 1.0 specifications](https://www.gigabyte.com/Motherboard/X470-AORUS-ULTRA-GAMING-rev-10/sp) and [User's Manual rev. 1001](https://download.gigabyte.com/FileList/Manual/mb_manual_x470-aorus-ultra-gaming_1001_e_200707.pdf), printed pp. 3–4 and §1-7 | Primary for PCB revision identification and the named fan and temperature connectors. |
| S2 | [X470 AORUS ULTRA GAMING rev. 1.0 owner report](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/issues/251#issuecomment-924449572) | Independent owner observation, with physical revision stated. Reports separate behavior for three fan channels. Includes no register or transport trace and is not maintainer-accepted verification. |
| S3 | [X470 AORUS GAMING 7 WIFI owner report](https://www.reddit.com/r/linuxmint/comments/1vu064e/fan_control_for_x470/), detector log dated 2026-08-20, and Gigabyte [rev. 1.0](https://www.gigabyte.com/Motherboard/X470-AORUS-GAMING-7-WIFI-rev-10/Support/sp) and [rev. 1.1](https://www.gigabyte.com/Motherboard/X470-AORUS-GAMING-7-WIFI-rev-11/sp) pages | Independent detection of two controllers. The log gives neither the PCB revision nor an identified OEM BIOS release. |
| S4 | [TRX50 AI TOP owner report](https://github.com/frankcrawford/it87/issues/73) and [later observation on that machine](https://github.com/frankcrawford/it87/issues/73#issuecomment-3592587595) | Independent negative pre-key result and later OS sensor output on a different board. |
| S5 | [Super I/O access rev. 4](superio-access.md#slot-1-ite-exit-withhold-rule) and [PawnIO interface rev. 7](pawnio-interface.md#lpcio) | Existing project scope for the raw diagnostic, exact slot-1 exit withhold, ISA mutex and transport. |
| S6 | Linux [v6.18 IT87 monitor](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c), Git blob `e233aafa8856c94c3fde3502732fcda2bd3bf130`, GPL-2.0-or-later | **lead-only (copyleft)**. Source for the active LDN, base and runtime register locations, fan and temperature formats, primary/secondary port separation, and the firmware-open discovery observation. |
| S7 | lm-sensors [V3-6-2 X470 AORUS ULTRA GAMING profile](https://github.com/lm-sensors/lm-sensors/blob/V3-6-2/configs/Gigabyte/X470-AORUS-ULTRA-GAMING.conf), Git blob `1964faec4aa347715833da50b4188c1c2282a683`, GPL-2.0-or-later project | **lead-only (copyleft)**. Experimental configuration naming two LPC controllers and tentative channel labels. Its comments cite other board profiles and monitor output, so it supplies leads rather than independent board verification. |
| S8 | PawnIO.Modules [`LpcIO`](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/LpcIO.p) and [`IsaBridgeEC`](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/IsaBridgeEC.p), commit `52a7e536dff3e53c96917a28caac5e0fa6510696` | Upstream public PawnIO interface and audit of the transactions required by those calls. |
| S9 | LibreHardwareMonitor [ECIO byte transport](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/IT879xEcioPort.cs), Git blob `cbb52c4b88c8fc606087220be6ad16cfd6315812`, and [secondary LPC discovery](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/LpcIO.cs), Git blob `3bb61abd9d86df2dcf388cc80689d4db61d16537`, MPL-2.0 | **lead-only (copyleft)**. Alternate ID after mode entry, selected ECIO mailbox transactions, and firmware transport selection. |
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

## Detection

| Scope | Status | Default enablement |
| --- | --- | --- |
| X470 AORUS ULTRA GAMING rev. 1.0, slot 1 pre-key raw `0x8733`, normal LPC | Candidate Experimental. S1 confirms PCB identity. S2 reports separate fan behavior. S6/S7 lead-only facts identify a possible direct EC at secondary base `0x0A60`, distinct from primary `0x0A40`. S14 corroborates the distinct bases in Linux without recording the pre-key state. | Disabled pending exact-board pre-key identity, successful Windows transport proof and sample evidence. Bases are observations to check, never defaults to substitute. |
| X470 AORUS ULTRA GAMING rev. 1.0, BIOS F61a, OEM WMI | S14 advertises the GUID but lacks the S13 temperature method. The observed WMI values are all zero. | Disabled. No verified secondary read path. |
| X470 AORUS GAMING 7 WIFI rev. 1.0, BIOS F64, OEM SMM | S24/S26 identify power-event handlers with hardware writes and no temperature or RPM output. S3 reports raw `0x8733`, which is shared by IT8792E and IT8795E. S25 shows an IT8792E marking on an unidentified PCB revision. No evidence links the photograph and raw ID to the same board. | Disabled. No verified OS-owned secondary read path. |
| GA-AB350-Gaming 3 rev. 1.x, BIOS F54d, OEM WMI | S12 advertises the GUID but lacks the S13 temperature method. The table's visible monitor pair defaults to the primary controller. | Disabled. No verified secondary read path. |
| GA-Z87X-UD5H, already-bound Linux `it8790` at observed base `0x0A40` | Candidate Experimental. S15 identifies the part. S16 records an unbound secondary and a later positive endpoint. Lead-only S17 establishes that the published inputs use a cache refresh that reads alarm status. | Disabled pending exact IT8790E read effects or a separate safe kernel input route, plus matching bound-endpoint evidence. |
| X670E AORUS MASTER rev. 1.x, BIOS F41, secondary ECIO | S18 establishes OEM boot-time mailbox access. S19 reports a separate H2RAM endpoint. S20's pump observation has no F41 identity. | Disabled pending an OS-owned read path, exact channel evidence and mailbox arbitration. |
| B650E AORUS MASTER rev. 1.0, secondary H2RAM via ECIO and separate BIOS F37 WMI inspection | S22 records a slot-1 `0x8733` observation and a G3 v1.00 window with RPM data under the S23 lead-only map. S21's static F37 tables establish no matching secondary telemetry method. | Disabled pending temperature attribution and a read path with proven mailbox ownership, arbitration and side effects. |
| Other board/transport combinations for `0x8733`, `0x8790` and `0x8695` | Diagnostic identities only under S5. The shared raw `0x8733` cannot distinguish the IT8792E and IT8795E models. | Disabled until independently scoped. |
| Alternate `0x8883` report, unreadable, absent or changed pre-key ID | S5/S9 distinguish the alternate report from a usable pre-key identity. S4 records a real absent pre-key result. | Disabled. |
| ECIO or ISA bridge | Separate access domains. See transport limits below. | Disabled. |

The smallest candidate keeps the board revision, firmware revision,
pre-key secondary raw ID and distinct runtime bases in one observation.
The board label alone cannot select an EC layout. S7 names the primary
as IT8686 and the secondary as IT8792 in its profile. Those names,
addresses and channel mappings remain lead-only. A board owner report
and the vendor manual establish the target for evidence collection.
(S1, S2, S6, S7)

## Register map (research facts)

| Address | Meaning | Encoding | Source |
| --- | --- | --- | --- |
| Slot 1 global `CR20/21`, `CR22` | Raw identity bytes and chip revision nibble. A firmware-open responder can return identity before any entry key is sent. | High byte first for the ID. Low four bits of `CR22` for the revision. | S5, S6/S9 (lead-only) |
| Slot 1 LDN `0x04`, `CR30[0]`, `CR60/61` | Firmware activation and runtime LPC base on the direct path. | An active controller reports bit 0 set. An eight-byte runtime block contains its index and data ports at base + 5 and + 6. | S6 (lead-only) |
| Direct LPC EC `0x58` | Register-block vendor check used by S6. | `0x90` is the expected vendor byte. It does not distinguish two ITE controllers. | S6 (lead-only) |
| Direct LPC EC `0x13`, `0x0D`–`0x0F`, `0x18`–`0x1A` | FAN1–3 enable bits and low/high count bytes for the `0x8733` profile. | Bits 4–6 of `0x13`. 16-bit little-endian count, low-byte read first. The divisor is 2. | S6 (lead-only) |
| Direct LPC EC `0x29`–`0x2B`, `0x51`, `0x55[7]` | Temperature bytes and source routing. S6 records both normal external-source selection in `0x51` and an extra external-source case for the second channel. | Signed 8-bit degrees C. Physical eligibility needs both route checks on this profile. | S6 (lead-only) |

S6 treats a zero fan count as invalid and an all-ones 16-bit count as
zero measured RPM. Neither implies a disconnected fan. These register
and decode facts describe a Linux driver that can initialize hardware.
A Windows read-only scope needs evidence of the firmware's original
monitoring state and channel routing, with no initialization writes.
(S6, lead-only)

## Read procedure and decode

A candidate access trace must begin with a pre-key secondary identity
read and preserve the firmware's configuration mode. Only the exact
board and pre-key identity above qualify for further research. An absent
or unexpected ID ends the attempt. The trace must establish that the
secondary EC base is active, aligned and distinct from the primary
before reading any secondary runtime port. The original LDN selector
and the final selector must be observed. (S5, S6/S9 lead-only, S8)

The current `LpcIO` BAR query walks logical devices rather than authorizing
only the chosen EC window. It can succeed with no usable EC window and
does not restore the LDN selector. A successful query therefore leaves
two open conditions on a firmware-open controller, safe selector handling
and proof that the EC pair was authorized for the intended secondary
device. No Windows trace meeting those conditions is available. (S8)

## Transport limits

| Transport | Evidence and limitation |
| --- | --- |
| ECIO | S9 describes a firmware command/status and data mailbox, separate from the direct EC register pair. A read still writes the read command and offset to the mailbox. The cited implementation uses `LpcIO` at ports `0x03F4` and `0x03F0`. That window also contains the historical floppy-controller registers, including a write-only data-rate register at `0x03F4`. Authorization for a port range does not prove mailbox ownership. Firmware capability and fan-control state reads leave the telemetry offset map unresolved. Require exact-board port ownership, bounded waits and recovery, and a separately verified telemetry layout. (S8, S11, S9/S10 lead-only) |
| ISA bridge | `IsaBridgeEC` module load checks architecture and CPU vendor. Its discovery IOCTL probes both slots and can issue a slot-1 enter sequence after `0xFFFF` during its own discovery, independently of a caller pre-check. The IOCTL can also restore prior bridge configuration. Mapping the window and programming the bridge are separate operations. The state-changing operation writes PCI bridge decode settings, and the cited consumer may use it even to read a firmware flag. A firmware-preconfigured window would still require safe discovery and validated sensor offsets. (S8, S10 lead-only) |

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
That secondary ID is shared by IT8792E and IT8795E. The detector lists
both controllers as detection-only, with no temperature, RPM, or
established pre-key response. Its board version is `Default string`
and its reported BIOS version is `5.17`. No evidence ties the log to
the photographed unit or either observation to rev. 1.0 running F64.
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

## Quirks

- Matching reported bases do not transfer a profile between the X470
  AORUS ULTRA GAMING and X470 AORUS GAMING 7 WIFI. (S3, S14)
- One TRX50 AI TOP owner saw pre-key `0xFFFF` on the secondary and later
  reported a separate secondary sensor block after several reboots.
  The two observations do not establish what changed or a stable
  pre-key identity. (S4)

## Safety notes

- The slot-1 exit-withhold rule remains the diagnostic boundary. A
  secondary telemetry design must forbid both entry and exit writes on
  the selected firmware-open candidate. Its absence path must stop
  without trying mode keys. (S5, S6/S9 lead-only)
- Use a bounded, caller-held ISA mutex for any approved multi-step
  collection. Keep primary results independent of a secondary failure.
  Equal, invalid or unknown bases fail secondary attribution. (S5)
- Sensor scope excludes monitor activation, pin routing changes, alarm
  clearing, fan/PWM control, limit changes, PCI decode changes and EC
  firmware configuration. Do not reuse a fan-control transport as a
  telemetry fallback. (S5, S8)

For later synthetic transport tests, preserve primary results when the
secondary pre-key ID is absent, alternate, or changes. Exercise equal
bases, selector restoration failure, denied ports, and invalid or
unavailable channels beside a successful sibling. Compare recorded
transactions to confirm that neither slot-1 mode keys nor hardware
configuration writes occur. Synthetic traces demonstrate the software
boundary only.

## Open questions

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

- For B650E AORUS MASTER rev. 1.0, establish a temperature field
  with exact attribution and valid samples, and a firmware-matched
  OS-owned route for the temperature and RPM. The complete read
  path needs evidence of mailbox ownership, arbitration, error
  handling and absence of alarm reads or control writes. (S21–S23)

- For GA-Z87X-UD5H Linux hwmon, establish the exact IT8790E
  alarm-status read effects for `0x01`–`0x03`, or pin a kernel
  input path that avoids those registers. Obtain same-session board,
  driver and canonical hwmon binding evidence with a valid secondary
  sample before enabling this candidate. (S16, S17 lead-only)

- Can the exact X470 AORUS ULTRA GAMING rev. 1.0 board and a named BIOS
  release demonstrate an active slot-1 `0x8733` with no mode keys and an
  LPC base separate from the primary? An accepted trace must give both
  raw ID and revision bytes, activation and base registers, and the
  detected PCB revision. A physical chip marking can resolve the
  shared-ID ambiguity.
- Can a signed PawnIO `LpcIO` release authorize just that secondary
  EC block without affecting the primary or firmware's selector
  state? An accepted trace must show exact slot selection, BAR and
  runtime port results, LDN state before and after, and a successful
  primary sample following the secondary attempt. If the existing scan
  cannot meet this boundary, a separately reviewed narrow authorization
  interface is required.
- Can an accepted trace correlate at least one enabled secondary fan
  or physical temperature with the same-session board firmware monitor?
  Record actual byte pairs, routing flags, disabled and inaccessible
  cases and repeatability. Keep generic channel labels until the board
  mapping is independently supported.

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
