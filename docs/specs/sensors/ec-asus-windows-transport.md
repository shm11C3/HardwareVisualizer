# Spec: ASUS board EC sensors on Windows — transport feasibility

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Draft — not implementation-ready |
| Scope | Feasibility of a read-only Windows transport for the ASUS motherboard readings that only the board's embedded controller (EC) provides: VRM, chipset, T_SENSOR and water temperatures, and CPU_OPT, chipset and VRM heatsink fan RPM. This document compares raw EC port access through PawnIO with firmware-mediated ACPI-WMI methods. It records what blocks each one. It authorizes no Windows read. The Linux slice is [`ec-asus-crosshair-viii-linux-hwmon.md`](ec-asus-crosshair-viii-linux-hwmon.md). |
| Issue phase | [#2524](https://github.com/shm11C3/HardwareVisualizer/issues/2524) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| W1 | UEFI Forum, [ACPI Specification 6.5](https://uefi.org/specs/ACPI/6.5/12_Embedded_Controller_Interface_Specification.html), Chapter 12: §12 introduction, §12.1, §12.2.1–§12.2.3, §12.3 Table 12.2, §12.3.1, §12.3.2, §12.7, §12.8 | Primary for the EC host interface, command set, ownership and Global Lock rules. |
| W2 | Microsoft, [Windows Instrumentation: WMI and ACPI](https://learn.microsoft.com/en-us/previous-versions/windows/hardware/design/dn614028(v=vs.85)), sections on the mapping driver, `_WDG`, control method naming (`WQxx`, `WMxx`) and MOF data | Primary for how Windows exposes firmware ACPI-WMI blocks and methods. |
| W3 | Microsoft, [Acpi.sys (Windows ACPI driver)](https://learn.microsoft.com/en-us/windows-hardware/drivers/kernel/acpi-driver), sections "ACPI devices" and "ACPI control methods" (page dated 2025-04-23) | Primary. Acpi.sys interprets AML control methods, kernel-mode drivers invoke them through Acpi.sys, and only Acpi.sys modifies operation-region data. |
| W4 | namazso, PawnIO.Modules `LpcACPIEC.p` at tag [`0.2.12`](https://github.com/namazso/PawnIO.Modules/blob/0.2.12/LpcACPIEC.p) (annotated tag object `34be4e1f320b8ae69be6f3172bb62f2ace3f36d3`, commit `40a95f0c812b09f78e05b2d5aa4614cbcef67b0a`). Release asset `release_0_2_12.zip` (83,556 bytes, SHA-256 `09698897ce19b92f1043a2e322cfcf0b1fe823e0b576971b178a65244fb0af9d`) contains `LpcACPIEC.bin` (2,612 bytes). | Upstream-published interface definition of a PawnIO module (LGPL-2.1-or-later). No code was copied. |
| W5 | Linux `asus-ec-sensors` driver and history, as pinned in S5 and S6 of the [Linux slice](ec-asus-crosshair-viii-linux-hwmon.md) | **lead-only (copyleft)** for ASUS EC register maps, bank register `0xFF`, the per-board AML mutex or Global Lock, and the statement that the driver reads the same data as the firmware WMI method under the same mutex. |
| W6 | Linux `asus_wmi_ec_sensors` driver, [v5.19](https://github.com/torvalds/linux/blob/v5.19/drivers/hwmon/asus_wmi_ec_sensors.c), Git blob `a3a2f014dec03d65879d89ec896849dd33fce454`, method constants and argument encoding (L37–L41, L262–L341). Removed after v5.19. [v6.0 documentation](https://docs.kernel.org/6.0/hwmon/asus_wmi_ec_sensors.html). Initial `asus-ec-sensors` commit [`d0ddfd241e57`](https://github.com/torvalds/linux/commit/d0ddfd241e57) | **lead-only (copyleft)** for the ASUS monitoring WMI GUID, the block EC read method ID and its argument and reply encoding, the AMD 500-series board list, and the observation that a WMI read of all sensors took almost a full second. |
| W7 | Linux [`asus_wmi_sensors` documentation, v7.3](https://github.com/torvalds/linux/blob/v7.3/Documentation/hwmon/asus_wmi_sensors.rst), Supported boards and Known Issues, and [driver v7.3](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/asus_wmi_sensors.c) L54–L59 | **lead-only (copyleft)** for the older sensor-method family on X370, X399, X470 and B450 boards and its firmware polling hazard. |
| W8 | `zeule/asus-ec-sensors` [issue #27](https://github.com/zeule/asus-ec-sensors/issues/27) | First-hand statement by the Linux driver author: PRIME X470-PRO support was added to avoid WMI lockups. |
| W9 | Linux Hardware Database, [probe `a599d9b486` sensors log](https://linux-hardware.org/?log=sensors&probe=a599d9b486), ASUS PRIME X570-PRO | Independent observation: EC `CPU` and `Motherboard` equal the same-sample NCT6798D `CPUTIN` and `SYSTIN`. |

## Detection

No Windows runtime condition is authorized by this revision.

| Condition | Gate kind | Source |
| --- | --- | --- |
| None. Every candidate below is Unsupported until a revision states its facts. | — | — |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Raw EC port access through PawnIO `LpcACPIEC` on any ASUS board | Rejected on safety grounds (see Transport findings, candidate A). | Unsupported. |
| ACPI-WMI block EC read method on ASUS AMD 500-series boards | Candidate. The method ID and encoding are lead-only (W6). The Windows class and method names, privilege, cost and polling safety are unknown. | Unsupported until the open questions are resolved. |
| ACPI-WMI sensor methods on X370, X399, X470 and B450 boards | Candidate with a documented firmware polling hazard (W7, W8). | Unsupported. |
| ASUS vendor kernel drivers and utilities | Outside policy: a third-party runtime dependency with write-capable port access. | Not a candidate. |

## Facts

### EC host interface (W1)

| Item | Fact | Source |
| --- | --- | --- |
| Address space | Each EC has a flat byte-addressable space, currently 256 bytes. | W1 §12 |
| Registers | `EC_SC` read is status, `EC_SC` write is command, `EC_DATA` is read/write data. | W1 §12.2.1–§12.2.3 |
| Status bits | Bit 0 OBF, bit 1 IBF, bit 3 CMD, bit 4 BURST, bit 5 SCI_EVT, bit 6 SMI_EVT; bits 2 and 7 ignored. | W1 §12.2.1 Table 12.1 |
| Read side effect | Reading `EC_DATA` clears OBF. | W1 §12.2.1, §12.2.3 |
| Commands | RD_EC `0x80`, WR_EC `0x81`, BE_EC `0x82`, BD_EC `0x83`, QR_EC `0x84`. | W1 §12.3 Table 12.2 |
| Read transaction | A command byte to `EC_SC`, then an address byte to `EC_DATA`. The data is read at `EC_DATA` after OBF is set. | W1 §12.3.1 |
| Ownership | RD_EC and WR_EC are reserved for exclusive use by OSPM. They make the EC generate SCIs, not SMIs, for IBF=0 and OBF=1. | W1 §12.3.1, §12.3.2 |
| Arbitration | OSPM or the system management handler first acquires ownership of the interface, through the Global Lock or as OSPM's default non-shared resource. | W1 §12.7 |
| Shared interface | Sharing with an inter-environment interface such as SMI relies on the Global Lock. Firmware that performs contentious EC accesses must provide `_GLK`. | W1 §12 introduction, §12.1, §12.8 |

### Windows firmware method path (W2, W3)

| Item | Fact | Source |
| --- | --- | --- |
| Mapper | The WMI-ACPI mapping driver (`Wmiacpi.sys`) loads on ACPI devices with PnP ID `PNP0C14`. | W2 |
| Registration | The mapper evaluates `_WDG` to learn the data block, event and method GUIDs and their two-character IDs, then registers them with WMI. | W2 |
| Method call | A WMI method call evaluates `WMxx` with the instance index, the method ID and the input buffer. A GUID registered with the string flag gets its buffer converted between Unicode and ASCIZ. | W2 |
| Class names | WMI class names come from the MOF, supplied by the firmware through the binary MOF GUID or as a resource. | W2 |
| Execution | Acpi.sys interprets AML control methods. Only Acpi.sys modifies operation-region data. | W3 |

### PawnIO `LpcACPIEC` module (W4)

| Function | Input cells | Output cells | Semantics |
| --- | --- | --- | --- |
| `ioctl_pio_read` | `in[0]` = port | `out[0]` = byte | Reads one byte from port `0x62` or `0x66`. Any other port returns `STATUS_ACCESS_DENIED`. |
| `ioctl_pio_write` | `in[0]` = port, `in[1]` = byte | — | Writes one byte to port `0x62` or `0x66`. Any other port returns `STATUS_ACCESS_DENIED`. |

- Both functions document that the caller should hold the
  `\BaseNamedObjects\Access_EC` mutant. The module acquires none.
- Module load does nothing beyond returning success. The module does
  not implement the EC handshake, status polling or any command.
- `LpcACPIEC.bin` is in the signed 0.2.12 release archive. This
  project does not ship or load it.

### ASUS firmware method path (lead-only, W5–W7)

| Item | Fact | Source |
| --- | --- | --- |
| GUID | ASUS boards expose the monitoring methods under WMI GUID `466747A0-70EC-11DE-8A39-0800200C9A66`, instance 0. | W6, W7 (lead-only) |
| Block EC read | Method ID `0x42524543` (ASCII `BREC`) reads a list of EC registers. The driver author describes it as present on ASUS AMD 500-series boards. | W6 (lead-only) |
| Argument | A buffer whose first 16-bit little-endian word is the byte length of what follows. The rest is UTF-16LE text of hexadecimal digits encoding (bank, index) byte pairs. At most 16 registers per call. | W6 (lead-only) |
| Reply | A buffer with the same length-prefixed UTF-16LE hexadecimal text, one byte per register. | W6 (lead-only) |
| Equivalence | The Linux EC driver reads the same data from the same source. It locks the EC with the ACPI mutex in the same way as the WMI code. | W5 (lead-only, commit `d0ddfd241e57`) |
| Cost | On some systems a WMI read of all sensors (under 15 bytes) took almost a full second. | W5/W6 (lead-only, commit `d0ddfd241e57`) |
| Older sensor methods | X370, X399, X470 and B450 boards expose methods for the interface version (`0x50574574`), sensor count (`0x50574572`), sensor information (`0x50574543`), buffer update (`0x51574543`) and sensor value (`0x52574543`) on the same GUID. Frequent polling can stop fans, hold fans at full speed or freeze readings on some BIOS versions. The documentation names method version 2 or later as the fix. | W7 (lead-only), W8 |

## Transport findings

### Candidate A: raw EC ports through PawnIO — rejected

1. An EC register read is a command write of RD_EC to `0x66` and an
   address write to `0x62`, followed by a data read that clears OBF
   (W1 §12.3.1). Bank selection on ASUS boards is a WR_EC of register
   `0xFF` (W5). With `LpcACPIEC` the application would perform all of
   these port writes itself (W4).
2. ACPI reserves RD_EC and WR_EC for OSPM (W1 §12.3.1, §12.3.2). On
   Windows, OSPM is Acpi.sys (W3). Its EC transactions, SCI handling
   and query handling are not visible to user mode.
3. The `Access_EC` mutant serializes only cooperating user-mode tools.
   It does not exclude Acpi.sys, AML methods that hold the firmware's
   AML mutex, or SMM. A user-mode client can acquire neither the ACPI
   Global Lock nor an AML mutex.
4. An interleaved transaction can hand a response byte to the wrong
   party or leave the EC mid-command. An interleaved bank write can
   make firmware AML or the OS read the wrong bank. This is the race
   that the README safety policy forbids.

No evidence can make this path safe without an OS-provided EC
arbitration interface. It is not a candidate for a future revision.

### Candidate B: ASUS ACPI-WMI block EC read — blocked

The method runs as firmware AML inside Acpi.sys. Its EC accesses go
through the OS EC owner, and the firmware's own mutex serializes them
with other firmware EC users (W2, W3, W5). This is the only identified
Windows path that does not race the EC. These facts block it:

| Blocker | Evidence that would unblock |
| --- | --- |
| The Windows WMI class name, method name, parameter names and the string flag for the GUID come from the board's MOF. None is captured for any board. | An opt-in, user-submitted MOF and class listing from `root\WMI` on a ROG CROSSHAIR VIII HERO (for example the class, method and parameter qualifiers for the method mapped to the GUID above), through [#2500](https://github.com/shm11C3/HardwareVisualizer/issues/2500). |
| The argument and reply encoding is lead-only and from a driver removed after v5.19 (W6). It is unknown whether the Windows mapper delivers it unchanged or converts it as a string. | The same capture plus a firmware table capture (DSDT/SSDT) showing the `WMxx` dispatch for the method and its use of the EC mutex. |
| Each call may take close to a second (W6). Sampling must not block the shared runtime ([#1937](https://github.com/shm11C3/HardwareVisualizer/issues/1937)). | A measured call time on the target board and a sampling design reviewed against #1937. |
| The older sensor-method family has a documented firmware polling hazard (W7, W8). Whether the block read method shares it is unknown. | A soak report on the target board at the intended interval, with fan control behavior checked. |
| The required privilege for executing an ACPI-WMI method from the application is not recorded. | A first-hand capture of the call from an unelevated and an elevated process. |
| Calling WMI methods is a new Windows integration in Core. | Design review under `.agents/rules/design.md`. |
| The EC `CPU` and `Motherboard` readings equal Super I/O channels (W9). The Windows NCT6798D path already reads that chip. | Restrict the scope to EC-only readings, as the Linux slice does. |

### Candidate C: older ASUS WMI sensor methods — deferred

These boards predate the block read method. The firmware polling
hazard (W7, W8) needs a same-board method-version rule and soak
evidence before any scope.

## Safety notes

- This revision performs no read. A future candidate B scope evaluates
  only firmware read methods. It must never call a method ID other than
  the documented read, and never write an EC register through PawnIO.
- The PawnIO `LpcACPIEC` module must not be loaded for this purpose.
- Fan control, fan curves and EC writes stay out of scope.

## Open questions

- Is the block EC read method present, with the same encoding, on the
  ROG CROSSHAIR VIII HERO firmware under Windows, and what class and
  method names does its MOF declare? No capture exists.
- Does the firmware's block read method hold the EC mutex for the
  whole multi-register call, including bank selection? The Linux
  driver's statement (W5) is lead-only.
- What is the per-call latency on Windows, and does repeated calling
  at a 1 to 2 second interval affect fan control on the target board?
- Which privilege does an ACPI-WMI method call from user mode need on
  current Windows 11 builds?
- Do AMD 600 and 800-series and Intel 600 and 700-series boards expose
  an equivalent firmware read method? The Linux driver reads them
  directly, so it does not answer this.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-28 | Initial feasibility draft. Reject raw EC port access through PawnIO `LpcACPIEC`. Identify the ASUS ACPI-WMI block EC read as the only non-racing candidate and list its blockers. |
