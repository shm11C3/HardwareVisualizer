# Spec: Gigabyte secondary ITE sensor access

| Field | Value |
| --- | --- |
| Revision | 2 |
| Status | Draft — not implementation-ready |
| Scope | Evidence for a secondary, firmware-open ITE LPC Environment Controller on the Gigabyte X470 AORUS ULTRA GAMING rev. 1.0. Separate ECIO and ISA bridge transport restrictions, and OEM WMI evidence for GA-AB350-Gaming 3 BIOS F54d. |
| Issue phase | [#2519](https://github.com/shm11C3/HardwareVisualizer/issues/2519) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Gigabyte, [X470 AORUS ULTRA GAMING rev. 1.0 specifications](https://www.gigabyte.com/Motherboard/X470-AORUS-ULTRA-GAMING-rev-10/sp) and [User's Manual rev. 1001](https://download.gigabyte.com/FileList/Manual/mb_manual_x470-aorus-ultra-gaming_1001_e_200707.pdf), printed pp. 3–4 and §1-7 | Primary for PCB revision identification and the named fan and temperature connectors. |
| S2 | [X470 AORUS ULTRA GAMING rev. 1.0 owner report](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/issues/251#issuecomment-924449572) | Independent owner observation, with physical revision stated. Reports separate behavior for three fan channels. Includes no register or transport trace and is not maintainer-accepted verification. |
| S3 | [X470 AORUS GAMING 7 WIFI owner report](https://www.reddit.com/r/linuxmint/comments/1vu064e/fan_control_for_x470/), detector log dated 2026-08-20, and Gigabyte [rev. 1.0](https://www.gigabyte.com/Motherboard/X470-AORUS-GAMING-7-WIFI-rev-10/Support/sp) and [rev. 1.1](https://www.gigabyte.com/Motherboard/X470-AORUS-GAMING-7-WIFI-rev-11/sp) pages | Independent raw IDs and bases for a different model. The log's reported board version is a placeholder. |
| S4 | [TRX50 AI TOP owner report](https://github.com/frankcrawford/it87/issues/73) and [later observation on that machine](https://github.com/frankcrawford/it87/issues/73#issuecomment-3592587595) | Independent negative pre-key result and later OS sensor output on a different board. |
| S5 | [Super I/O access rev. 4](superio-access.md#slot-1-ite-exit-withhold-rule) and [PawnIO interface rev. 7](pawnio-interface.md#lpcio) | Existing project scope for the raw diagnostic, exact slot-1 exit withhold, ISA mutex and transport. |
| S6 | Linux [v6.18 IT87 monitor](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c), Git blob `e233aafa8856c94c3fde3502732fcda2bd3bf130`, GPL-2.0-or-later | **lead-only (copyleft)**. Source for the active LDN, base and runtime register locations, fan and temperature formats, primary/secondary port separation, and the firmware-open discovery observation. |
| S7 | lm-sensors [V3-6-2 X470 AORUS ULTRA GAMING profile](https://github.com/lm-sensors/lm-sensors/blob/V3-6-2/configs/Gigabyte/X470-AORUS-ULTRA-GAMING.conf), Git blob `1964faec4aa347715833da50b4188c1c2282a683`, GPL-2.0-or-later project | **lead-only (copyleft)**. Experimental configuration naming two LPC controllers and tentative channel labels. Its comments cite other board profiles and monitor output, so it supplies leads rather than independent board verification. |
| S8 | PawnIO.Modules [`LpcIO`](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/LpcIO.p) and [`IsaBridgeEC`](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/IsaBridgeEC.p), commit `52a7e536dff3e53c96917a28caac5e0fa6510696` | Upstream public PawnIO interface and audit of the transactions required by those calls. |
| S9 | LibreHardwareMonitor [ECIO byte transport](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/IT879xEcioPort.cs), Git blob `cbb52c4b88c8fc606087220be6ad16cfd6315812`, and [secondary LPC discovery](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/LpcIO.cs), Git blob `3bb61abd9d86df2dcf388cc80689d4db61d16537`, MPL-2.0 | **lead-only (copyleft)**. Alternate ID after mode entry, selected ECIO mailbox transactions, and firmware transport selection. |
| S10 | LibreHardwareMonitor [ISA bridge controller](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/IsaBridgeGigabyteController.cs), Git blob `cc9398dc047347d6bfd19129cc00d6ee890e0143`, and [ECIO controller](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/EcioPortGigabyteController.cs), Git blob `e906f6aee3fda7c0b42f3900f3c69f41b6d4376a`, MPL-2.0 | **lead-only (copyleft)**. Confirms what the cited bridge and ECIO examples use the mapped RAM for. |
| S11 | Intel, [82077AA floppy disk controller data sheet](https://www.alldatasheet.com/html-pdf/167793/INTEL/82077AA/1242/8/82077AA.html), §2.1 | Primary for the historical PC/AT or PS/2 port assignment. It establishes the possible address collision, with no claim about a floppy controller being present on the target board. |
| S12 | Gigabyte, [GA-AB350-Gaming 3 rev. 1.x BIOS F54d release](https://www.gigabyte.com/Motherboard/GA-AB350-Gaming-3-rev-1x/Support#support-dl-bios) and [official F54d archive](https://download.gigabyte.com/FileList/BIOS/mb_bios_ab350-gaming-3_8a06bg08_f54d.zip?v=9b00bc778a8a46e655182428ea1bbe5a) | Archive SHA-256 `5ad4baf65297f0a89da0f04da2f00fa0111bc5d634bde30ae9de53956df3f158`. Checksum-valid `GBT / GSWApp` SSDT (28,407 bytes) SHA-256 `0c5707bb1a0fdbe31f22f266695ab38c2b5fd5c2c39b2cc0c0048665ba4311ac`. The table starts at offset `0xA8F114` of the LZMA section decoded from BIOS image offset `0xA246F0`. |
| S13 | Linux, [Gigabyte WMI temperature interface](https://gbmc.googlesource.com/linux/+/1f66f63c7312ee085dc989b3c5fa4b3d09fe9d52/drivers/platform/x86/gigabyte-wmi.c), commit `1f66f63c7312ee085dc989b3c5fa4b3d09fe9d52`, GPL-2.0 | **lead-only (copyleft)**. Source for the WMI GUID and temperature method ID tested against the OEM table. |

## Detection

| Scope | Status | Default enablement |
| --- | --- | --- |
| X470 AORUS ULTRA GAMING rev. 1.0, slot 1 pre-key raw `0x8733`, normal LPC | Candidate Experimental. S1 confirms PCB identity. S2 reports separate fan behavior. S6/S7 lead-only facts identify a possible direct EC at secondary base `0x0A60`, distinct from primary `0x0A40`. | Disabled pending exact-board pre-key identity, successful Windows transport proof and sample evidence. Bases are observations to check, never defaults to substitute. |
| GA-AB350-Gaming 3 rev. 1.x, BIOS F54d, OEM WMI | S12 advertises the GUID but lacks the S13 temperature method. The table's visible monitor pair defaults to the primary controller. | Disabled. No verified secondary read path. |
| Other `0x8733` boards, `0x8790`, `0x8695` | Diagnostic identities only under S5. The shared raw `0x8733` cannot distinguish the IT8792E and IT8795E models. | Disabled until independently scoped. |
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

## Quirks

- A different X470 board reports primary `0x8686` at `0x0A40` and
  secondary `0x8733` at `0x0A60`, but its board revision is missing
  from the log. The data corroborates separation and cannot transfer
  the X470 AORUS ULTRA GAMING candidate to the GAMING 7 WIFI. (S3)
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
