# Gigabyte GA-H61M-DS2 fan telemetry evidence

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Draft — not implementation-ready |
| Scope | Static FAN1 and FAN2 readback evidence from the GA-H61M-DS2 rev. 2.2 F9 BIOS and the associated rev. 2.21 schematic |
| Issue phase | [#2518](https://github.com/shm11C3/HardwareVisualizer/issues/2518), primary LPC research |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Gigabyte GA-H61M-DS2 schematic, drawing revision 2.21, March 30, 2012, [sheet 16, ITE 8728 LPC I/O](https://manualmachine.com/gigabyte/gah61mds2/24375246-schematic-rev-2/#:~:text=IT8728F%2FDX) and [sheet 18, HWM, FAN CTRL, OV](https://manualmachine.com/gigabyte/gah61mds2/24375246-schematic-rev-2/#:~:text=Disable%20the%208728%20EX) | Vendor-authored drawing, public mirror. Sheet 16 identifies the specified part and its LPC attachment. Sheet 18 shows CPU and system tach routing and includes a separate EX reference. |
| S2 | Gigabyte [GA-H61M-DS2 rev. 2.2 BIOS support](https://www.gigabyte.com/Motherboard/GA-H61M-DS2-rev-22/support#support-dl-bios), F9, April 19, 2013, [vendor archive](https://download.gigabyte.com/FileList/BIOS/mb_bios_ga-h61m-ds2_v2.x_f9.exe?v=327640d339ee3af7df0f703aa10200b4) | Static inspection of the firmware as data. Archive SHA-256 `4c3788b59d424b1c738218c8f42e85d27f8bac6a13108828c66d27d9985e9620`. Member `h61mds22.f9`, 4,194,304 bytes, SHA-256 `df60a122dc4d8a62ec0a2af12b495860b7102db9f6e2ceeb88081c87842d3bee`. The SMM PE image in FFS `199FD111-0785-4132-A9B3-1F66573F0060` has SHA-256 `9915ae83f946ee799f809ddf1a548ad8efa7b2570fbfe8bbeb726dab432eb797`. Relevant PE-relative addresses are the dispatch call at RVA `0x7406`, the tachometer callback at RVA `0x6DB8`, and its index/data helper at RVA `0x0E94`. |
| S3 | ITE IT8728F Preliminary Specification V0.4.2, cover and §8.3.5, printed p. 48, pinned as S1 in the [IT8728 temperature specification](superio-ite-it86xx-it87xx.md#sources) | Vendor-authored, E-version document. The chip revision table assigns the same `CR22[3:0]` value `0x1` to DX and EX. |
| S4 | [Primary ITE LPC expansion](superio-ite-primary-lpc-expansion.md#deferred-profiles) and [GA-B75M-D3H IT8728F/EX fan specification](superio-ite-it8728f-ex-ga-b75m-d3h-fans.md) | Current applicability and transport boundaries for IT8728F fan readback. |

## Detection

| Scope | Status | Default enablement |
| --- | --- | --- |
| Primary LPC FAN1 and FAN2 on the studied GA-H61M-DS2 design | The rev. 2.21 schematic specifies IT8728F/DX on sheet 16, while a note about overvoltage and undervoltage on sheet 18 refers to 8728 EX. The schematic connects CPU and system tach signals to FANIO1 and FANIO2 respectively. The F9 firmware contains a two-byte tach read path at the port pair shown below. No inspected evidence ties the revision nibble and a live reading to the physical chip marking on this board. (S1–S3) | Disabled |

This draft defines no runtime gate. The DX/EX alias remains unresolved
outside the scope of the separate GA-B75M-D3H specification. (S3, S4)

## Register map (static firmware observations)

| Input | Index port | Data port | First index (high byte) | Second index (low byte) | Source |
| --- | --- | --- | --- | --- | --- |
| FAN1 | `0x0A35` | `0x0A36` | `0x18` | `0x0D` | S2 |
| FAN2 | `0x0A35` | `0x0A36` | `0x19` | `0x0E` | S2 |

The two branches perform byte index selects and byte data reads. The
callback combines the two bytes into an unsigned count. It returns zero
if the count is zero or if either byte equals `0xFF`. Otherwise it returns
the integer quotient of 1,350,000 divided by twice the count, the
firmware's RPM value. These facts describe F9 only. (S2)

## Safety notes

The audited FAN1 and FAN2 callback paths read only their two count bytes.
There are no alarm or status reads or EC data writes in those branches.
The callback runs through a firmware SMM dispatch path using the same
index/data pair a direct LPC reader would need. No shared arbitration
contract with the application's ISA mutex or supported OS-facing query
contract was established. The observed firmware read sequence therefore
supplies no authorization for a direct LPC transaction. (S2, S4)

## Open questions

- **Silicon attribution.** The sheet 16 part callout and the sheet 18 EX
  note leave the physical DX attribution open. A DX-specific primary
  register document or a same-board, same-session capture identifying
  the physical chip marking, board revision, active primary controller,
  raw chip ID and revision, gate bytes, raw counts, and positive RPM
  would resolve the remaining attribution gap. (S1–S3)
- **Runtime route, width, and enable.** The schematic identifies physical
  tach routing. The two F9 callback branches perform no mux, monitor
  state, channel-enable, or counter-width checks. The high-byte-first
  order and byte-level sentinels establish firmware behavior only.
  They do not establish latching, coherence, or a common DX/EX
  decode and eligibility contract. (S1–S4)
- **Runtime ownership.** A supported firmware-mediated query or a
  documented arbitration mechanism for direct access is still needed.
  A host mutex alone does not establish serialization with SMM on this
  board. (S2, S4)

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-09 | Record the GA-H61M-DS2 schematic and F9 FAN1/FAN2 readback evidence. Keep the candidate disabled pending silicon attribution, runtime gates, and transport ownership. |
