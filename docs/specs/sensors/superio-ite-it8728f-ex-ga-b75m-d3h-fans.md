# Windows IT8728F primary-slot fan readback (FAN1–2)

| Field | Value |
| --- | --- |
| Revision | 2 |
| Status | Implementation-ready (rev 2) |
| Scope | Experimental Windows readback of FAN1 and FAN2 from the primary LPC (slot 0) Environment Controller of a controller that reports raw chip ID `0x8728` with chip-version nibble `0x1` (IT8728F DX or EX). The Gigabyte GA-B75M-D3H factory IT8728F/EX population is the best-evidenced member. FAN3–5, other version nibbles, the secondary slot and Linux hwmon are excluded. |
| Issue phase | Phase 4 (#1635), [#2518](https://github.com/shm11C3/HardwareVisualizer/issues/2518), ADR 0031 re-scope under [#2510](https://github.com/shm11C3/HardwareVisualizer/issues/2510) |

Revision 2 replaces the GA-B75M-D3H baseboard, PCB-version and exact
EC-base predicates of revision 1 with the mechanism and safety gates
required by
[ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md).
On 2026-11-28 the maintainer decided that the IT8728F primary-slot FAN1–2
readback may become a chip-level Experimental row. The observed board
stays in the Status column as an evidence identity. The row covers DX
and EX alike through a lead-only fact
([ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md)),
stated under [DX and EX applicability](#dx-and-ex-applicability). That
fact keeps every member Experimental. This document owns IT8728F FAN1–2.
S9 owns the existing IT8728 temperature scope, and S11 owns the other
primary LPC profiles. Readiness of this revision follows the
[specification gate](README.md#status-transition-draft--implementation-ready).

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Gigabyte [GA-B75M-D3H official product revision selector](https://www.gigabyte.com/Motherboard/GA-B75M-D3H-rev-12), linked variants, and [user manual 12ME-B75MD3H-1102R, rev. 1102](https://download1.gigabyte.com/Files/Manual/mb_manual_ga-b75m-d3h_v1.1_e.pdf), pp. 4–5, §§1-2 and 1-7 | Primary. Gigabyte lists PCB revisions 1.0, 1.1 and 1.2. The manual identifies the LPC-connected ITE controller and the CPU and system fan headers. Evidence for the best-evidenced member only. |
| S2 | Gigabyte GA-B75M-D3H schematic [rev. 1.0](https://www.scribd.com/document/877244866/GA-B75M-D3H-R10), [rev. 1.1](https://studylib.net/doc/28210482/gigabyte-ga-b75m-d3h-rev.-1.1-sch), [rev. 1.2](https://www.scribd.com/document/976494520/GA-B75M-D3H-R12), sheets 17 and 19 of each | Vendor-authored drawings in public mirrors. Sheet 17 labels the controller `IT8728F/EX (GB)/QFP128` and the FAN1/FAN2 tachometer nets. Sheet 19 connects them to the CPU and system fan sense paths. Evidence for the best-evidenced member only. |
| S3 | [GA-B75M-D3H F12 DMI report](https://marc.info/?l=flashrom&m=136191069930229&w=1), [F14 DMI report](https://marc.info/?l=flashrom&m=136902255812829&w=2), baseboard entries, and [Windows GA-B75M-D3H hardware report](https://www.bleepingcomputer.com/forums/t/634987/bsod-cause-probably-by-ntoskrnlexe-module/), baseboard fields in the first debugger summary | Independent observed board strings. The F12 and F14 reports show `x.x` as the baseboard version. These reports provide no raw EC transaction. Evidence identities only. |
| S4 | Linuxportál [firsthand B75M-D3H observation](https://www.linuxportal.info/leirasok/hardver/hogyan-figyeljuk-szamitogepunk-hardverenek-homersekletet-a-nagy-hoseg-idejen), hardware identification, detection transcript and reported RPM | Independent observation of primary pair `0x2E/0x2F`, IT8728F and base `0x0A30` on the exact board model. The reported FAN1/FAN2 readings were 1674/1157 RPM. The report omits raw CR22, physical package marking, PCB revision and a Windows PawnIO trace. |
| S5 | ITE Tech., Inc., [IT8728F preliminary specification V0.4.2](https://www.rom.by/files/it8728f_datasheet.pdf), E-version cover and SHA-256 `5393224C1E497E0F5DED27CF8609D16D3857003F8235C5128E903A1438680C0B`, as pinned by S9 | Vendor-authored, public mirror. Pins ID/version §8.3.3–8.3.5 (p. 48), routing §8.3.12 (p. 52), LDN 04 §8.8 (p. 61), EC addressing §9.5.1–9.5.2.1 (p. 87), low-index count and enable §§9.5.2.2.14/.16/.19 (pp. 94–96), and tachometer formula/cadence §§9.5.3.5–6 (pp. 109–110). The cover limits V0.4.2 to the E version. The front-matter revision history lists only the E-version initial release. §8.3.5 gives the part number as IT8728F/EX. The publicly retrievable file carries a confidentiality watermark. |
| S6 | Microsoft [`Win32_BaseBoard`](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-baseboard), Manufacturer, Product and Version properties | Public Windows interface. In revision 2 these properties name an observed member in a compatibility report or fixture. They are not a runtime condition. |
| S7 | Linux v6.18 `drivers/hwmon/it87.c`, Git blob `e233aafa8856c94c3fde3502732fcda2bd3bf130`: [IT8728 profile](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L376-L384), [16-bit count decode](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L686-L687), [count read order](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L905-L919), [version read](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L2828-L2831), [FAN2 pin route](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L3114-L3119) in the [common pin branch](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L3070), [fan enable](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L3309-L3310), [version-dependent fan format](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c#L3427-L3448) | lead-only (copyleft). Fixed 16-bit IT8728 counters, divisor 2, low byte followed by high byte, zero count invalid, maximum count no measurable rotation. The source reads the version nibble on every recognized chip. It changes the fan-counter format by version only for raw IDs `0x8705` and `0x8712`, and applies one fan profile to raw `0x8728` for every version value. |
| S8 | LibreHardwareMonitor at `2808d80564b6761e1b01f42fabfd966ed695fcbc`: [ITE LPC Environment Controller](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/IT87XX.cs), Git blob `cc3dfd301e46cca5a92801890c512e0af422f8e1`, lines 239 and 476–495, and [LPC chip detection](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/LpcIO.cs), Git blob `3bb61abd9d86df2dcf388cc80689d4db61d16537`, lines 634, 661 and 710 | lead-only (copyleft). Corroborates low-then-high count order, the fixed divisor and full-count handling. Supports the existing conservative small-count cutoff in S11. The source reads the version nibble for every ITE chip and uses it to select the counter path only for IT8705F and IT8712F. Raw `0x8728` uses the 16-bit path for every version value. |
| S9 | [IT8728 Windows temperature specification revision 2](superio-ite-it86xx-it87xx.md), detection, base validity and PawnIO authorization, read procedure, cadence and safety notes | Repository reference for the existing IT8728 path. The authority to use EC I/O follows a successful post-exit proof. |
| S10 | [Super I/O configuration access revision 4](superio-access.md), ITE MB PnP and slot safety, and [PawnIO interface revision 7](pawnio-interface.md#lpcio), signed module and ISA mutex contract | Repository transport references. Existing installed and accepted PawnIO runtime and `LpcIO` module, restricted PIO, slot-specific authorization and caller-held ISA mutex. |
| S11 | [Primary LPC expansion revision 6](superio-ite-primary-lpc-expansion.md#read-procedure-and-decode), shared fan plausibility and repeated count procedure, and [ADR 0011](../../adr/0011-experimental-sensor-enablement.md) | Shared project guards, result contract and Experimental policy. This document supplies the IT8728F fan register fields and qualification. |
| S12 | PawnIO.Modules [`LpcIO` upstream IOCTL interface at `52a7e536dff3e53c96917a28caac5e0fa6510696`](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/LpcIO.p), pinned by S9/S10 | Public interface definition. BAR discovery scans logical devices and may leave a different selection. Slot selection invalidates the previous BAR allow-list. Success from BAR discovery alone does not prove a usable EC window. |
| S13 | Linux v6.18 [`Documentation/hwmon/it87.rst`](https://github.com/torvalds/linux/blob/v6.18/Documentation/hwmon/it87.rst#L76-L82), Git blob `5cef4f26500028a591006aa410d06e99c6c45cc7`, lines 76–82 and 267–268 | lead-only (copyleft). The document lists no publicly available IT8728F datasheet. It states that the IT8728F was considered compatible with the IT8721F until a datasheet becomes available. Cited only in Open questions as the limit of S7's DX evidence. |
| S14 | [GA-H61M-DS2 fan evidence revision 2](superio-ite-ga-h61m-ds2-fan-evidence.md) | Repository evidence record. A board design whose schematic specifies IT8728F/DX has vendor firmware that reads FAN1/FAN2 at the same index pairs, with the same count combination and divisor 2. The same firmware accesses the EC index/data ports from SMM. Its physical DX attribution is open. Supporting evidence only. No fact in this document rests on S14. |

## Detection

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| Windows PawnIO slot 0, exact raw ID `0x8728` with `CR22[3:0]=0x1` (IT8728F DX or EX), active LDN `0x04` EC with a valid base, FAN1 and FAN2 | **Experimental**. S5 pins the EX register map. Its DX applicability rests on lead-only S7/S8 ([DX and EX applicability](#dx-and-ex-applicability)). The same leads supply the fixed 16-bit count format and divisor, the low-then-high sequence, and invalid/saturated count semantics. The lower plausibility limit depends on S8/S11. Best-evidenced member, an evidence identity that does not gate the read: Gigabyte GA-B75M-D3H PCB revisions 1.0, 1.1 and 1.2 with the factory IT8728F/EX population (S1–S2). Its firmware reports `Gigabyte Technology Co., Ltd.`, `B75M-D3H` and version `x.x` (S3). An independent Linux observation shows slot `0x2E/0x2F`, IT8728F at base `0x0A30` and FAN1/FAN2 at 1674/1157 RPM (S4). This member stays Experimental because no maintainer-accepted dump exists. No DX controller has been observed on this path. S14 provides supporting DX design evidence only. | Best-effort for each eligible input. |
| Raw `0x8728` with `CR22[3:0]=0x0` (AX–CX in S5 §8.3.5), any other nibble, or an unreadable `CR22` | Not covered by this revision. See [Open questions](#open-questions). | Unsupported for fans. Temperatures follow S9. |
| FAN3–5, slot 1 or another transport, other ITE chip IDs, Linux hwmon | Outside this document. See S11 and the [Linux GA-B75M-D3H evidence](superio-ite-b75m-d3h-linux-hwmon.md). | Disabled by this specification. |

### Runtime gates

Every runtime condition is a mechanism or safety gate. No baseboard
string, PCB revision, BIOS version or fixed bus address gates the read
(ADR 0031).

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows host with the existing signed PawnIO runtime and `LpcIO` module | mechanism (the only transport this document defines) | S10, S12 |
| PawnIO slot 0, configuration index/data pair `0x2E/0x2F`. The S10 slot-1 ITE exit-withhold rule stays in force. | mechanism (the primary controller that S5 describes) | S5 §8.1, S9, S10 |
| Live `CR20=0x87` and `CR21=0x28` | mechanism | S5 §§8.3.3–8.3.4, S9 |
| Live `CR22[3:0]=0x1`, read during the same primary configuration transaction. An unreadable nibble or any other value excludes the fan profile. Reserved upper bits carry no identity and are never written. | mechanism (the version code that S5 assigns to DX and EX together, distinct from AX–CX `0x0`) | S5 §8.3.5, p. 48; DX: S7/S8 (lead-only) |
| LDN `0x04` selected, `CR30[0]=1`, and live `CR60/CR61` forming a valid base under the S9 width, alignment and range checks. Use the live base. | mechanism (the register's own activation bit and address) | S5 §8.8, p. 61, S9 |
| `CR29[2]` read with LDN `0x04` selected, before BAR discovery. FAN2 only when bit 2 is clear. | mechanism (FAN2 pin routing) | S5 §8.3.12, p. 52; DX: S7 (lead-only) |
| Successful BAR discovery, matching primary ITE configuration exit, then the S9 post-exit EC `0x00` index/data proof on the same signed executor and slot | safety | S9, S10, S12 |
| Caller holds `Global\Access_ISABUS.HTP.Method` for discovery and each sample burst | safety | S10 |
| Per sample, EC `0x00` bit 0 set and bit 3 clear | mechanism (the monitor's own state bits) | S5 §9.5.2.2.1 and §9.5.3.6, S9; DX: open, see the [Register map](#register-map) note |
| Per sample, EC `0x13` bit 4 / bit 5 set for FAN1 / FAN2 | mechanism (firmware channel enable) | S5 §9.5.2.2.16, p. 95; DX: S7 (lead-only) |

The baseboard Manufacturer, Product and Version (S6), PCB revision, BIOS
version, system model, and the observed base value `0x0A30` are evidence
identities. They appear only in the Status column and in fixtures. The
fan profile needs no baseboard query.

### DX and EX applicability

| Fact | Source |
| --- | --- |
| S5 documents the E version only. Its cover limits V0.4.2 to E, and its revision history lists only the E-version initial release. §8.3.5 assigns `0x0` to AX through CX and `0x1` to both DX and EX. S5 describes no DX register map and no difference between DX and EX. | S5 cover, front-matter revision history, §8.3.5, p. 48 |
| S7 reads `CR22[3:0]` on every recognized ITE chip. It uses the value to change the fan-counter format only for raw IDs `0x8705` (from version `0x3`) and `0x8712` (from version `0x8`). For raw `0x8728`, every version value receives the same fan profile. That profile has fixed 16-bit counts with low bytes at EC `0x0D`/`0x0E` and high bytes at `0x18`/`0x19`. FAN1/FAN2 enables are EC `0x13` bits 4/5, and FAN2 is not a tachometer input when `CR29` bit 2 is set. | S7 (lead-only) |
| S8 reads `CR22[3:0]` for every ITE chip. It uses the value only to select the counter path for IT8705F and IT8712F. For raw `0x8728`, every version value reads FAN1/FAN2 as 16-bit counts, low byte at EC `0x0D`/`0x0E`, then high byte at `0x18`/`0x19`. | S8 (lead-only) |
| Every primary controller that reports raw `0x8728` with `CR22[3:0]=0x1` uses the FAN1–2 readback in the [Register map](#register-map). This covers DX and EX alike. No primary source conflicts with this fact. S5 limits the scope of its document but assigns no other map to DX. | S5, S7/S8 (lead-only) |

The rev 1 board predicate never selected a different map at runtime. It
could not distinguish a substituted DX that reports the same tuple.
Revision 2 states the distinguishing fact instead: the DX and EX fan
fields used here are the same (lead-only). `CR22` therefore remains a
mechanism gate only for the boundary between the AX–CX code and the
DX/EX code. Under ADR 0023, a lead-only fact may back this Experimental
row and no Verified member (S5, S7–S8).

## Register map

All EC addresses below are seven-bit indices. Write them to the live
`ec_base + 0x05`, then read the byte at `ec_base + 0x06`, after the S9
proof. Configuration registers are read while the existing configuration
transaction owns the primary slot. Each DX citation below rests on the
lead-only fact in [DX and EX applicability](#dx-and-ex-applicability).

| Register / field | Fact | Access | Source |
| --- | --- | --- | --- |
| Global `CR29[2]` | FAN2 uses the physical FAN_TAC2 pin with bit 2 clear. With bit 2 set the pin is GP52. The register is readable from any LDN. FAN1 has a dedicated tachometer pin. | Config read only, with LDN `0x04` selected during existing discovery | S5 §§4–5 and §8.3.12; DX: S7 (lead-only) |
| EC `0x00[0]`, `0x00[3]` | Require monitoring started and interrupt-clear standby inactive, respectively bit 0 set and bit 3 clear. This is the S9 common EC gate. | EC read only | S5 §9.5.2.2.1 and §9.5.3.6, S9. DX: open. No pinned source states these bits for DX. The gate only withholds reads, so a different DX meaning can make fans unavailable but cannot admit an extra read. S9 already applies the same gate to every raw `0x8728`. |
| EC `0x13[4]` / `0x13[5]` | Firmware tachometer enable for FAN1 / FAN2. A set bit authorizes that enabled input after the common and routing checks. | EC read only | S5 §9.5.2.2.16, p. 95; DX: S7 (lead-only) |
| EC `0x0D` / `0x18` | FAN1 count low / high byte. | EC read only | S5 §§9.5.2.2.14/.19, pp. 94, 96; DX: S7/S8 (lead-only) |
| EC `0x0E` / `0x19` | FAN2 count low / high byte. | EC read only | S5 §§9.5.2.2.14/.19, pp. 94, 96; DX: S7/S8 (lead-only) |
| Counter format | Fixed 16-bit counts for these IT8728 inputs and fixed divisor 2. E-version EC `0x0C[2:0]` is reserved. | No mode-register read or write | S5 §§9.5.2.2.13 and 9.5.3.5, S7/S8 (lead-only for the fixed format/divisor) |

## Read procedure and decode

1. Reuse S9 discovery and S10 transport on the selected primary slot.
   While holding the ISA mutex, identify the controller and confirm the
   active LDN and base. Read the additional `CR22` and `CR29` fields
   only for a slot-0 raw `0x8728` candidate. Read `CR29` after the
   revision and active-base checks while LDN `0x04` is selected and
   before BAR discovery can alter that selection. An unreadable routing
   byte excludes FAN2 and leaves FAN1 independently eligible. A revision
   mismatch or an unreadable `CR22` excludes only the fan profile and
   leaves the existing temperature discovery applicable on its own terms
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
- DX and EX report the same `CR22[3:0]` value. No software-readable
  field in this document separates them. Their equal treatment rests on
  the lead-only fact in [DX and EX applicability](#dx-and-ex-applicability)
  (S5, S7–S8).
- S4 provides positive OS-reported RPM on the two channels of the
  best-evidenced member. It does not supply a marked chip, raw counter
  values, or a Windows transaction.

## Safety notes

- Use only the existing signed PawnIO runtime and module provenance,
  slot restrictions and post-exit port authorization. The caller holds
  `Global\Access_ISABUS.HTP.Method` for discovery and each sample burst
  using the existing bounded acquisition (S9–S10, S12).
- Use only transport writes required for configuration entry, logical
  device selection, exit, and byte index selection. Read firmware
  activation and routing fields without modifying them. The fan scope
  has no EC data writes or bank selection (S5, S9–S10).
- Do not read EC interrupt-status registers `0x01`–`0x03`. Those
  registers contain read-to-clear alarm status. The fan readout excludes
  FAN3–5, high-index EC addresses, PWM/control, voltage and limit
  registers. Do not invoke another monitoring tool or broaden detection
  to a secondary controller (S5 §§9.5.2.2.2–.4, S9–S11).
- Firmware can access the same EC index/data pair from SMM on at least
  one IT8728F board design (S14). The reader writes only EC indices. An
  SMI between an index write and its data read can return another
  register's byte. The repeated-pair check and the count table reject
  such a read unless both pairs agree. The existing S9 temperature
  transaction on the same ports has the same exposure (S9, S14).
- Outside the scoped row, follow the existing profiles and diagnostics.
  A failed revision, routing or fan gate cannot relax existing EC I/O
  authorization or promote an otherwise unsupported temperature or fan
  path (S9–S11).

## Fixture cases

All count examples are synthetic. Every case checks ordered I/O
and verifies absence of EC data writes and interrupt-status reads.
Board strings in a fixture are evidence identities and never change the
expected observation.

| Fixture | Expected observation |
| --- | --- |
| Best-evidenced member state: slot 0, `0x8728`, `CR22=0x01`, active LDN 04, raw base `0x0A/0x30`, `CR29[2]=0`, BAR/exit/post-exit proof succeeds, EC `0x00=0x01`, EC `0x13=0x30`. FAN1 low/high `0xC2/0x01` twice and FAN2 `0x84/0x03` twice. Recorded board strings `Gigabyte Technology Co., Ltd.` / `B75M-D3H` / `x.x`. | Fan 1 at 1500 RPM and Fan 2 at 750 RPM. The normal temperature path can report in the same collection. |
| The same controller state on a non-Gigabyte baseboard with raw base `0x02/0x90`. Separately, no baseboard record, several records, or a failed baseboard query. | The same Fan 1 and Fan 2 results, with EC indices at `0x0295` and data at `0x0296` for the `0x0290` base. No baseboard query participates in eligibility. |
| Otherwise qualified with `CR22=0x81` | The low revision nibble still qualifies. Upper `CR22` bits are never written. |
| `CR22[3:0]=0x0`, an unlisted nibble such as `0x2`, or an unreadable `CR22` | No `CR29`, EC `0x13` or count read. Existing eligible temperatures follow S9. |
| Raw `0x8728` on slot 1, or slot 0 with a different chip ID such as `0x8721` or `0x8716` | No fan read under this document. Other profiles follow S10/S11. |
| Inactive LDN 04, or a base that fails the S9 checks (for example raw `0xFF/0xFF`, `0x00/0x00`, or set reserved bits) | No EC access for fans. No port-proof cache. |
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

- Non-blocking for Phase 4 chip-level Experimental readback: DX applicability rests on a tagged lead-only fact, and that fact keeps every member Experimental.
  **DX evidence.** S13, the documentation of S7, lists no publicly
  available IT8728F datasheet. It says the IT8728F was considered
  compatible with the IT8721F until one becomes available. The uniform
  treatment of every version value in S7 is therefore not tied to a DX
  document. A
  DX-version vendor document, or a maintainer-accepted dump from a
  controller with physical DX marking, would confirm the fact. That dump
  needs `CR22`, the gate bytes, raw counts and paired RPM. If such
  evidence shows a different DX map, the fact moves here. The `0x1` row
  then needs a software-readable EX discriminator, because DX and EX
  share the version code (S5, S7–S8, S13, S14).
- Non-blocking for Phase 4 chip-level Experimental readback: this revision widens only to the version code that S5 shares between DX and EX.
  **AX–CX parts.** S7/S8 apply the same profile to nibble `0x0`, while
  S5 names AX through CX as separate versions. Widening to `0x0` is a
  separate revision decision (S5, S7–S8).
- Non-blocking for Phase 4 chip-level Experimental readback: every member stays Experimental until accepted evidence exists.
  **Member graduation.** An accepted same-unit capture would establish
  hardware validation beyond the schematic and the independent Linux
  observation. That capture needs the physical marking, PCB revision,
  Windows discovery fields and raw counts. Graduation to Verified also
  needs primary or dump evidence for the fixed format and divisor
  (S1–S5, S7–S8).
- Non-blocking for Phase 4 chip-level Experimental readback: two immediately repeated matching pairs bound the read without assuming a latch.
  **Counter latch behavior.** The published sources do not specify a hardware latch for these split
  values (S5, S7–S8, S11).
- Non-blocking for Phase 4 chip-level Experimental readback: the fan transaction adds no transport or write beyond the existing S9 temperature transaction on the same ports.
  **Firmware access to the EC ports.** S14 records SMM accesses to the
  EC index/data pair on one IT8728F board design. Neither a shared
  arbitration contract nor a software-readable indication of SMM use is
  known. An SMI that leaves an interrupt-status index selected before
  the reader's data read could consume a latched alarm. The S9
  temperature path has the same exposure (S9, S14).
- Non-blocking for Phase 4 chip-level Experimental readback: this row uses only the two low-index inputs.
  **Other IT8728 fan inputs.** FAN3–5 and high-index access remain
  separate investigations (S5, S11).

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-08 | Propose the transition from Draft to Implementation-ready (rev 1) for exact OEM GA-B75M-D3H IT8728F/EX FAN1–2 Experimental readback. The primary-slot authorization applies and the factory discriminator requires maintainer approval. |
| 2 | 2026-11-28 | Re-scope under ADR 0031 per the maintainer decision of 2026-11-28 to a chip-level Experimental row. The row covers slot 0, raw `0x8728`, `CR22[3:0]=0x1`, an active LDN `0x04` with any S9-valid base, the S9 post-exit proof, `CR29[2]` routing and the EC `0x13` enables. Remove the `Win32_BaseBoard`, PCB-version and exact base `0x0A30` predicates and the host-side baseboard lookup. Record GA-B75M-D3H as the best-evidenced member. Add the gate-kind table and the DX and EX applicability facts, which are lead-only (S7/S8, ADR 0023). Correct the S7 line anchors to the pinned blob, and add S13 and S14. Replace the baseboard fixtures with a different-board, different-base case and revision rejections. FAN1–2 only. The decode, the repeated-pair guard and the plausibility cutoffs are unchanged. |
