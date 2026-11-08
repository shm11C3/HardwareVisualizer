# Windows IT8728F/EX fan readback on Gigabyte GA-B75M-D3H

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental Windows readback of FAN1 and FAN2 on the OEM-documented factory IT8728F/EX population of Gigabyte GA-B75M-D3H PCB revisions 1.0, 1.1 and 1.2, via the primary LPC Environment Controller |
| Issue phase | Phase 4 (#1635), [#2518](https://github.com/shm11C3/HardwareVisualizer/issues/2518) |

This status is a proposal. The specification can be pinned for clean-room
implementation after the maintainer approves the readiness PR and accepts
the [factory-board discriminator](#factory-board-discriminator) under the
[specification gate](README.md#status-transition-draft--implementation-ready).
The status applies only to the row below. This document owns the board's
IT8728F/EX fan scope. S9 owns the existing IT8728 temperature scope, and
S11 owns the general primary LPC expansion.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Gigabyte [GA-B75M-D3H official product revision selector](https://www.gigabyte.com/Motherboard/GA-B75M-D3H-rev-12), linked variants, and [user manual 12ME-B75MD3H-1102R, rev. 1102](https://download1.gigabyte.com/Files/Manual/mb_manual_ga-b75m-d3h_v1.1_e.pdf), pp. 4–5, §§1-2 and 1-7 | Primary. Gigabyte lists PCB revisions 1.0, 1.1 and 1.2. The manual identifies the LPC-connected ITE controller and the CPU and system fan headers. |
| S2 | Gigabyte GA-B75M-D3H schematic [rev. 1.0](https://www.scribd.com/document/877244866/GA-B75M-D3H-R10), [rev. 1.1](https://studylib.net/doc/28210482/gigabyte-ga-b75m-d3h-rev.-1.1-sch), [rev. 1.2](https://www.scribd.com/document/976494520/GA-B75M-D3H-R12), sheets 17 and 19 of each | Vendor-authored drawings in public mirrors. Sheet 17 labels the controller `IT8728F/EX (GB)/QFP128` and the FAN1/FAN2 tachometer nets. Sheet 19 connects them to the CPU and system fan sense paths. The drawings describe the specified factory population. |
| S3 | [GA-B75M-D3H F12 DMI report](https://marc.info/?l=flashrom&m=136191069930229&w=1), [F14 DMI report](https://marc.info/?l=flashrom&m=136902255812829&w=2), baseboard entries, and [Windows GA-B75M-D3H hardware report](https://www.bleepingcomputer.com/forums/t/634987/bsod-cause-probably-by-ntoskrnlexe-module/), baseboard fields in the first debugger summary | Independent observed board strings. The F12 and F14 reports show `x.x` as the baseboard version. The Windows report shows that system and baseboard identities may differ. These reports provide no raw EC transaction. |
| S4 | Linuxportál [firsthand B75M-D3H observation](https://www.linuxportal.info/leirasok/hardver/hogyan-figyeljuk-szamitogepunk-hardverenek-homersekletet-a-nagy-hoseg-idejen), hardware identification, detection transcript and reported RPM | Independent observation of primary pair `0x2E/0x2F`, IT8728F and base `0x0A30` on the exact board model. The reported FAN1/FAN2 readings were 1674/1157 RPM. The report omits raw CR22, physical package marking, PCB revision and a Windows PawnIO trace. |
| S5 | ITE Tech., Inc., [IT8728F preliminary specification V0.4.2](https://www.rom.by/files/it8728f_datasheet.pdf), E-version cover and SHA-256 `5393224C1E497E0F5DED27CF8609D16D3857003F8235C5128E903A1438680C0B`, as pinned by S9 | Vendor-authored, public mirror. Pins ID/version §8.3.3–8.3.5 (p. 48), routing §8.3.12 (p. 52), LDN 04 §8.8 (p. 61), EC addressing §9.5.1–9.5.2.1 (p. 87), low-index count and enable §§9.5.2.2.14/.16/.19 (pp. 94–96), and tachometer formula/cadence §§9.5.3.5–6 (pp. 109–110). The publicly retrievable file carries a confidentiality watermark. |
| S6 | Microsoft [`Win32_BaseBoard`](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-baseboard), Manufacturer, Product and Version properties | Public Windows interface. Product maps to SMBIOS Type 2 Product. The repository already reads these properties in its Windows WMI motherboard inventory. |
| S7 | Linux v6.18 [IT8728 classification](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L353-L361), [16-bit fan decode](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L650-L654), [counter access order](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L858-L872), Git blob `e233aafa8856c94c3fde3502732fcda2bd3bf130` | lead-only (copyleft). Fixed 16-bit IT8728 counters, divisor 2, low byte followed by high byte, zero count invalid, maximum count no measurable rotation. The source does not distinguish DX from EX. |
| S8 | LibreHardwareMonitor [ITE LPC Environment Controller](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/IT87XX.cs) at `2808d80564b6761e1b01f42fabfd966ed695fcbc`, Git blob `cc3dfd301e46cca5a92801890c512e0af422f8e1`, 16-bit fan reading | lead-only (copyleft). Corroborates low-then-high count order, the fixed divisor and full-count handling. Supports the existing conservative small-count cutoff in S11. The source does not distinguish DX from EX. |
| S9 | [IT8728 Windows temperature specification revision 2](superio-ite-it86xx-it87xx.md), detection, base validity and PawnIO authorization, read procedure, cadence and safety notes | Repository reference for the existing IT8728 path. The authority to use EC I/O follows a successful post-exit proof. |
| S10 | [Super I/O configuration access revision 4](superio-access.md), ITE MB PnP and slot safety, and [PawnIO interface revision 7](pawnio-interface.md#lpcio), signed module and ISA mutex contract | Repository transport references. Existing installed and accepted PawnIO runtime and `LpcIO` module, restricted PIO, slot-specific authorization and caller-held ISA mutex. |
| S11 | [Primary LPC expansion revision 4](superio-ite-primary-lpc-expansion.md#read-procedure-and-decode), shared fan plausibility and repeated count procedure, and [ADR 0011](../../adr/0011-experimental-sensor-enablement.md) | Shared project guards, result contract and Experimental policy. This document supplies the board's fan register fields and qualification. |
| S12 | PawnIO.Modules [`LpcIO` upstream IOCTL interface at `52a7e536dff3e53c96917a28caac5e0fa6510696`](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/LpcIO.p), pinned by S9/S10 | Public interface definition. BAR discovery scans logical devices and may leave a different selection. Slot selection invalidates the previous BAR allow-list. Success from BAR discovery alone does not prove a usable EC window. |

## Detection

| Scope | Status | Default enablement |
| --- | --- | --- |
| Windows GA-B75M-D3H factory revisions 1.0/1.1/1.2 with OEM-documented IT8728F/EX and every runtime predicate below, FAN1 and FAN2 only | **Experimental**. S1–S6 pin the board population, observed identity, configured address and E-version map. Fixed 16-bit count format/divisor, low-then-high sequence and invalid/saturated count semantics depend on S7/S8, and the lower plausibility limit depends on S8/S11. | Best-effort for each eligible input after approval. |
| Other board models, other ITE chip revisions and transports, FAN3–5, Linux hwmon | Outside the new fan scope. | Disabled by this specification. |

### Factory-board discriminator

All of the following are required. The runtime check describes an exact
OEM population scope and can observe only the supplied firmware identity
and controller response.

| Predicate | Condition | Source |
| --- | --- | --- |
| Windows baseboard identity | The complete local `Win32_BaseBoard` query returns exactly one record before any filtering or deduplication. Its Manufacturer is exactly `Gigabyte Technology Co., Ltd.` and its Product is exactly `B75M-D3H`, with only surrounding whitespace removed. A failed query, any other record count, or mismatched fields keep the fan profile unavailable. | S1–S3, S6 |
| PCB revision evidence | Require Version `1.0`, `1.1`, `1.2`, or the observed placeholder `x.x`, with only surrounding whitespace removed. The placeholder carries no revision value. Every OEM-listed revision has the same documented EX part. An absent, unreadable or unrecognized Version keeps the new fan path unavailable. | S1–S3, S6. The Version acceptance rule is a conservative scope guard. |
| Primary slot | PawnIO slot 0, configuration index/data pair `0x2E/0x2F`. | S4–S5, S9–S10 |
| Exact live chip ID | Actual `CR20=0x87` and `CR21=0x28`. | S5, S9 |
| Live chip revision | `CR22[3:0]=0x1`, read during the same primary configuration transaction. An unreadable nibble or any other value excludes the fan profile. Reserved upper bits carry no identity. | S5 §8.3.5, p. 48 |
| Active EC and exact base | LDN `0x04`, `CR30[0]=1`, raw `CR60/CR61=0x0A/0x30`, including the normal S9 width and alignment checks. Use the live bytes for the base. | S4–S5, S9 |
| Authorized I/O | Complete the S9 post-exit index/data proof on the same signed executor and slot. | S9–S10, S12 |

DX and EX both return revision nibble `0x1`. S2 supplies the EX
identification for the documented factory population. A physically
replaced controller, an alternate factory population contradicted by new
OEM evidence, and a machine that reports cloned board strings are outside
the scope. This check cannot distinguish a substituted DX that reports the
same tuple. Acceptance of this exact factory population as the
discriminator is the maintainer decision for this readiness proposal
(S1–S5).

The sampling host resolves the local board before taking the ISA mutex.
Reuse the existing `Win32_BaseBoard` query infrastructure or a local
query that preserves the number of returned records. Check the complete
result for exactly one record before checking Manufacturer, Product and
Version. The current `query_motherboard_info` output retains only the
first baseboard record and cannot be used to establish eligibility.
The host may reuse its own hardware identity snapshot if it preserves
the query's record count. An IPC caller cannot assert eligibility.
The successful profile may be cached with the port-proven executor and
slot per S9. Version is a conservative scope check. BIOS, serial number
and system model are outside the discriminator (S6, S9–S11).

## Register map

All EC addresses below are seven-bit indices at `0x0A35` with byte
reads at `0x0A36` after the S9 proof. Configuration registers are
read while the existing configuration transaction owns the primary slot.

| Register / field | Fact | Access | Source |
| --- | --- | --- | --- |
| Global `CR29[2]` | FAN2 uses the physical FAN_TAC2 pin with bit 2 clear. With bit 2 set the pin is GP52. The register is readable from any LDN. FAN1 has a dedicated tachometer pin. | Config read only, with LDN `0x04` selected during existing discovery | S2, S5 §§4–5 and §8.3.12 |
| EC `0x00[0]`, `0x00[3]` | Require monitoring started and interrupt-clear standby inactive, respectively bit 0 set and bit 3 clear. This is the S9 common EC gate. | EC read only | S5 §9.5.2.2.1 and §9.5.3.6, S9 |
| EC `0x13[4]` / `0x13[5]` | Firmware tachometer enable for FAN1 / FAN2. A set bit authorizes that enabled input after the common and routing checks. | EC read only | S5 §9.5.2.2.16, p. 95 |
| EC `0x0D` / `0x18` | FAN1 count low / high byte. | EC read only | S5 §§9.5.2.2.14/.19, pp. 94, 96 |
| EC `0x0E` / `0x19` | FAN2 count low / high byte. | EC read only | S5 §§9.5.2.2.14/.19, pp. 94, 96 |
| Counter format | Fixed 16-bit counts for these IT8728 inputs and fixed divisor 2. E-version EC `0x0C[2:0]` is reserved. | No mode-register read or write | S5 §§9.5.2.2.13 and 9.5.3.5, S7/S8 (lead-only for the fixed format/divisor) |

## Read procedure and decode

1. Reuse S9 discovery and S10 transport on the selected primary slot.
   While holding the ISA mutex, identify the controller and confirm the
   active LDN and base. Read the additional `CR22` and `CR29` fields
   only for the board-qualified primary IT8728 candidate. Read `CR29`
   after the revision and active-base checks while LDN `0x04` is
   selected and before BAR discovery can alter that selection. An
   unreadable routing byte excludes FAN2 and leaves FAN1 independently
   eligible. A board, revision, or exact fan-base mismatch leaves the
   existing temperature discovery applicable on its own terms
   (S5, S9–S10, S12).
2. Require successful BAR discovery and matching primary ITE configuration
   exit. Under the same ISA mutex, use the normal S9 post-exit EC `0x00`
   index write and data read as the PIO proof. Neither fan eligibility nor
   BAR discovery alone authorizes any EC fan read. A failed exit or proof
   prevents caching the executor/base and requires rediscovery under S9.
   Selecting another slot, replacing the executor, or invalidating its
   state discards the proof and fan qualification (S9–S10, S12).
3. Hold the ISA mutex for each sample burst. Use the existing EC `0x00`
   monitor-state read and the minimum 1.5-second spacing between bursts.
   A failed monitor-state read or unready monitor prevents fan readings
   for that sample. On the qualified path, read `0x13` once and use only
   its two enable bits. A failed enable-byte read prevents both fan
   counts. A disabled or unavailable route prevents its channel's
   count reads (S5, S9–S11).
4. For each eligible channel, read a low byte and then a high byte.
   Immediately repeat the pair under the same mutex. Accept only two
   successful matching pairs. A read failure or disagreement makes
   that channel unavailable. Keep successful siblings and valid
   temperatures. The second pair is a bounded consistency guard and
   does not depend on a hardware latch guarantee (S7–S8 lead-only,
   S9–S11).
5. Combine the bytes as `count = 256 × high + low`. The vendor formula
   uses 1,350,000 divided by count and divisor. With the fixed divisor
   2 the whole-number result is `floor(675000 / count)`, applied only
   to an active count in the table (S5 §9.5.3.5, S7–S8 lead-only).

| Stable count | Output | Basis |
| --- | --- | --- |
| `0x0000` | Invalid, no RPM | S7 (lead-only) |
| `0x0001`–`0x003F` | Invalid, no RPM | Conservative plausibility guard already used by S11, supported by S8 (lead-only) |
| `0x0040`–`0xFFFE` | Active, calculated integer RPM | S5, S7–S8 (fixed divisor is lead-only) |
| `0xFFFF` | Inactive, 0 RPM. The full count indicates no measurable rotation. | S7–S8 (lead-only), S11 |

Use the existing motherboard fan model and preserve the physical numbers in
the generic display names `Fan 1` and `Fan 2`. Do not infer a stopped fan
from a failed I/O operation or a missing route. A valid sibling stays under
its original name. Reuse the existing motherboard provider, collection
and availability contract. A qualified fan profile and an empty sample
are distinct. Existing temperature qualification, decode, availability,
fallback and collection remain governed by S9 and S11.

## Quirks

- The IT8728F E-version prose for the chip-ID low byte disagrees with its
  section heading and register table. Use the exact raw ID in S9 (S5, S9).
- The vendor tachometer description discusses both 8-bit and 16-bit
  counters and gives a default divisor. The exact IT8728 implementation
  sources support a fixed 16-bit path and divisor 2 for these channels.
  These facts are lead-only under ADR 0023 and limit the scope to
  Experimental (S5, S7–S8).
- S4 provides positive OS-reported RPM on the two board channels. It does
  not supply a marked chip, raw counter values, or a Windows transaction.

## Safety notes

- Use only the existing signed PawnIO runtime and module provenance,
  slot restrictions and post-exit port authorization. The caller holds
  `Global\Access_ISABUS.HTP.Method` for discovery and each sample burst
  using the existing bounded acquisition (S9–S10, S12).
- Use only transport writes required for configuration entry, logical
  device selection, exit, and byte index selection. Read firmware
  activation and routing fields without modifying them. The new fan
  scope has no EC data writes or bank selection (S5, S9–S10).
- Do not read EC interrupt-status registers `0x01`–`0x03`. Those
  registers contain read-to-clear alarm status. The fan readout excludes
  FAN3–5, high-index EC addresses, PWM/control, voltage and limit
  registers. Do not invoke another monitoring tool or broaden detection
  to a secondary controller (S5 §§9.5.2.2.2–.4, S9–S11).
- Outside the exact factory-board row, follow the existing profiles and
  diagnostics. A failed board, revision, routing or fan gate cannot
  relax existing EC I/O authorization or promote an otherwise unsupported
  temperature or fan path (S9–S11).

## Fixture cases

All count examples are synthetic. Every case checks ordered I/O
and verifies absence of EC data writes and interrupt-status reads.

| Fixture | Expected observation |
| --- | --- |
| Exact board and `x.x`, slot 0, `0x8728`, `CR22=0x01`, active LDN 04, raw base `0x0A/0x30`, `CR29[2]=0`, BAR/exit/post-exit proof succeeds, EC `0x00=0x01`, EC `0x13=0x30`. FAN1 low/high `0xC2/0x01` twice and FAN2 `0x84/0x03` twice. | Fan 1 at 1500 RPM and Fan 2 at 750 RPM. The normal temperature path can report in the same collection. |
| Otherwise qualified with `CR22=0x81`, or an explicit known PCB revision | The low revision nibble and exact factory tuple still qualify. Upper CR22 bits are never written. |
| No baseboard records, or multiple baseboard records with an exact match first | Reject the new fan profile before its additional discovery reads. This includes duplicate matching records. The existing temperature path remains eligible under S9. |
| Neighboring baseboard `B75M-D3V`, mismatched manufacturer, unresolved or ambiguous board identity, unrecognized version, wrong slot, wrong chip, unreadable/unlisted revision nibble, or a different valid EC base | No new fan-specific EC read. Existing eligible temperatures or unrelated supported fan profiles still follow S9/S11. |
| Configuration exit fails, BAR discovery fails, the post-exit address or data access is denied, or ISA mutex unavailable | No fan-enable or count read. No port-proof cache on failure. If the exit fails, no EC access at all. |
| The executor or slot was replaced after a successful proof | Prior port proof and fan qualification cannot authorize a later sample. |
| EC monitor-state read fails, Start is clear, or INT_Clear is set | No fan-enable or count read in the sample. |
| EC `0x13` read fails | Both fan counts are absent. A valid temperature sibling survives. |
| FAN1 disabled and FAN2 enabled/routed | Read only FAN2 counts and keep the name Fan 2. |
| FAN2 routed to GP52 or CR29 read unavailable, FAN1 enabled | Omit FAN2 before its count reads. Keep a valid FAN1. |
| FAN1 high byte fails or the two FAN1 pairs disagree, with stable valid FAN2 | FAN1 unavailable. FAN2 retains its RPM and name. |
| Valid FAN1 with a failed temperature-mode read | FAN1 remains valid. Temperature processing follows S9. |
| Enabled channel stable at `0x0000`, `0x003F`, `0x0040`, `0xFFFE` or `0xFFFF` | In order, invalid, invalid, active at 10546 RPM, active at 10 RPM, inactive at 0 RPM. |
| FAN3–5 enabled or interrupt-status bytes carry pending alarms | No read of FAN3–5, high-index addresses, or EC `0x01`–`0x03`. |

## Open questions

- Non-blocking for Phase 4 factory-board Experimental readback: the exact scope uses the documented factory population and requires maintainer acceptance of that discriminator.
  **Production and repair coverage.** An accepted same-unit capture with the physical EX marking, PCB
  revision, exact Windows discovery fields and raw counts would establish
  hardware validation beyond the schematic and independent Linux
  observation. A substituted DX that reports the same tuple remains
  outside the scope (S1–S5).
- Non-blocking for Phase 4 factory-board Experimental readback: two immediately repeated matching pairs bound the read without assuming a latch.
  **Counter latch behavior.** The published sources do not specify a hardware latch for these split
  values (S5, S7–S8, S11).
- Non-blocking for Phase 4 factory-board Experimental readback: this row uses only the two documented factory inputs at low indices.
  **Other IT8728 revisions and fan inputs.** A DX-specific map, a revision discriminator beyond this board
  population, and the independent evidence for FAN3–5 remain separate
  investigations (S5, S11).

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-08 | Propose the transition from Draft to Implementation-ready (rev 1) for exact OEM GA-B75M-D3H IT8728F/EX FAN1–2 Experimental readback. The primary-slot authorization applies and the factory discriminator requires maintainer approval. |
