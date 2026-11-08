# Spec: Primary LPC expansion for ITE Environment Controller sensors

| Field | Value |
| --- | --- |
| Revision | 4 |
| Status | Implementation-ready (rev 4) |
| Scope | Read-only access to the primary LPC Environment Controller for IT8716F silicon version D FAN1–3 and exact IT8620E and IT8628E TMPIN1–3 and FAN1–3. Only the model and channel rows below qualify. |
| Issue phase | Phase 4 (#1635), [#2517](https://github.com/shm11C3/HardwareVisualizer/issues/2517), [#2518](https://github.com/shm11C3/HardwareVisualizer/issues/2518) |

This revision records which specification owns the exact GA-B75M-D3H
fan scope. Clean-room implementation can pin revision 4 after the
maintainer approves the specification PR under the
[specification gate](README.md#status-transition-draft--implementation-ready).
All expansion rows are Experimental. They have pinned lead-only
facts and lack an accepted independent hardware dump. The existing
IT8728F/EX temperature path is defined in its
[temperature specification revision 2](superio-ite-it86xx-it87xx.md).
FAN1–2 on the exact factory GA-B75M-D3H IT8728F/EX profile are defined
in the [board specification revision 1](superio-ite-it8728f-ex-ga-b75m-d3h-fans.md).
Apply the existing successful-reading presentation and Experimental
failure diagnostics under [ADR 0011](../../adr/0011-experimental-sensor-enablement.md).

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | ITE Tech., Inc., IT8728F Preliminary Specification V0.4.2 (E version), pinned as S1 in [the existing IT8728 temperature specification](superio-ite-it86xx-it87xx.md#sources) | Vendor-authored primary source. The cover limits V0.4.2 to E. Section 8.3.5, printed p. 48, assigns the same revision nibble to DX and EX. Pins the E-specific fan map in §§9.5.2.1–9.5.2.2 and the temperature path in S3. |
| S3 | [Existing IT8728 temperature specification revision 2](superio-ite-it86xx-it87xx.md) | Baseline for access ordering, PawnIO BAR authorization and the explicit post-exit port proof. |
| S5 | [ADR 0011](../../adr/0011-experimental-sensor-enablement.md) and [design principle DP-02](../../design-principles.md#dp-02-represent-partial-capability-honestly) | Project policy source for Experimental enablement, success/failure presentation, plausibility-gated partial readings, manual feedback, and no new telemetry. It supplies no hardware fact. |
| S6 | Linux kernel [v6.18, `drivers/hwmon/it87.c`](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c), Git blob `e233aafa8856c94c3fde3502732fcda2bd3bf130`, GPL-2.0-or-later | **lead-only (copyleft)**. Pins exact modern IDs, I/O discovery and layout, temperature routing, pin selections, fan count format and availability, vendor register, and 16-bit mode differences. The exact IT8620E and IT8628E profile entries and model-scoped discovery are at lines 477–504, 2779–2787 and 2955–2989. The revision-specific branches at lines 3427–3448 concern other models. |
| S7 | LibreHardwareMonitor [`2808d80564b6`, ITE LPC Environment Controller](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/IT87XX.cs), Git blob `cc3dfd301e46cca5a92801890c512e0af422f8e1`, MPL-2.0 | **lead-only (copyleft)**. Corroborates the fixed divisor, low-byte then high-byte read order, maximum count as no measured rotation, and supports a conservative lower count bound for 16-bit tachometer channels. |
| S8 | ITE Tech., Inc., *IT8716F Environment Control - LPC I/O*, Preliminary Specification V0.3, [public third-party mirror](https://www.rom.by/files/IT8716F_V0.3.pdf) | Vendor-authored primary document. The cover applies V0.3 to D and future versions. Section 8.3.5, printed p. 39, explicitly identifies C and D through `CR22[3:0]`. Pins configuration §8.1 and §§8.3, 8.8, dedicated FAN1 and shared FAN2/3 pin functions §§4–5 and §§8.3.10, 8.3.12, EC access §§9.6.1–9.6.2.1, monitor state §9.6.2.2.1, fan read and enable fields §§9.6.2.2.13–16 and .21, printed pp. 89–91, and tachometer format and cadence §§9.6.3.5–6, printed p. 101. |
| S9 | [Super I/O configuration access revision 4](superio-access.md#slot-1-ite-exit-withhold-rule) and [PawnIO interface revision 7](pawnio-interface.md#lpcio) | Current repository transport facts and scope limits. The Experimental slot-1 withhold applies to diagnostics. |
| S10 | Out-of-tree [IT87 driver](https://github.com/frankcrawford/it87/blob/master/it87.c), Git blob `56148f202fee43436fa73a747a1bf465deb8721e`, GPL-2.0-or-later | **lead-only (copyleft)**. Corroborates the separate exact IT8620E and IT8628E IDs and fixed 16-bit fan profile selection, with distinct bank and SMBus access requirements for deferred chips. |
| S11 | PawnIO.Modules [`LpcIO` at `52a7e536dff3`](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/LpcIO.p) | Upstream public interface used by HardwareVisualizer. Slot selection invalidates previous BAR permissions. BAR discovery is a multi-LDN scan and does not guarantee preservation of the selected LDN. |
| S12 | [GA-B75M-D3H IT8728F/EX fan specification revision 1](superio-ite-it8728f-ex-ga-b75m-d3h-fans.md) | Applicability reference for the separate Experimental board profile and its approval gate. The primary expansion rows in this document do not depend on that profile. |

## Detection

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact raw chip ID `0x8728` / IT8728F/EX TMPIN1–3 | Existing **Experimental** scope from S3. | Enabled under S3. |
| Exact `0x8728` / IT8728F FAN1–3 outside the GA-B75M-D3H FAN1–2 scope in S12 | DX and EX both report `CR22[3:0]=0x1` (S1). The exact-chip leads S6/S7 do not distinguish the two versions. | Disabled. Apply S12 only to its exact board, controller, and channel scope after its approval gate. |
| Slot 0, exact `0x8716` / IT8716F D with `CR22[3:0]=0x1`, FAN1–3 | **Experimental**. S8 identifies D and defines discovery, counters, pin routing, and per-channel 16-bit mode bits. Fixed divisor in 16-bit mode, read order, count sentinels, and conservative lower count bound are lead-only (S6/S7). | Enabled best-effort for eligible firmware-enabled 16-bit inputs. Other or unreadable revision nibbles are outside the scope. |
| Slot 0, exact `0x8620` / IT8620E TMPIN1–3 and FAN1–3 | **Experimental**. Exact ID and model-wide fixed 16-bit fan path from S6/S10. Discovery, vendor check, EC read access, physical-temperature selection and encoding, fan enable, pin routing, divisor, read order, count sentinels, and conservative lower count bound are lead-only (S6/S7). | Enabled best-effort for eligible inputs. |
| Slot 0, exact `0x8628` / IT8628E TMPIN1–3 and FAN1–3 | **Experimental**. Exact ID and model-wide fixed 16-bit fan path from S6/S10. Discovery, vendor check, EC read access, physical-temperature selection and encoding, fan enable, pin routing, divisor, read order, count sentinels, and conservative lower count bound are lead-only (S6/S7). | Enabled best-effort for eligible inputs. |
| Additional channels, IT86xx/IT87xx models, and secondary transport | See [Deferred profiles](#deferred-profiles). A raw `0x8721` cannot use the IT8728 profile. | Disabled. |

### Discovery and authorization

Select a new profile only on slot 0 and only after matching both chip-ID
bytes exactly. Replacing the slot or executor invalidates cached port
proof. The discovery sequence in S3 applies, including requiring
`ioctl_find_bars`, a successful primary ITE exit, and the post-exit EC
`0x00` index-write/data-read proof. Apply these per-profile checks within
that sequence. (S3, S11)

| Profile | Discovery and authorization | Sample gate | Source |
| --- | --- | --- | --- |
| IT8716F D | After raw chip ID `0x8716`, read global `CR22` before LDN selection. Require its low nibble to be exactly `0x1`. A failed read, C (`0x0`), or an unlisted nibble ends expansion discovery with the usual primary configuration exit. Use the same primary configuration keys, active LDN `0x04`, 12-bit aligned base format from `CR60/61`, and EC index/data offsets as IT8728. Read the global pin-routing fields with LDN `0x04` selected. | EC `0x00` bit 0 set and bit 3 clear. | S8 §§8.1, 8.3, 8.8, 9.6.1–9.6.2.1, 9.6.2.2.1; S3 |
| IT8620E, IT8628E | S6 describes the primary configuration keys and exact IDs, LDN `0x04` with `CR30[0]` set, and an eight-port EC block from `CR60/61`, with index/data at base + 5/+ 6. Require the raw big-endian base in `0x0100`–`0x0FF0`, bits 2:0 clear, and an address outside the PawnIO-protected PCI window. Read pin-routing fields with LDN `0x07` selected, then return to LDN `0x04` before BAR discovery. After the EC `0x00` port proof, require bit 7 clear and EC `0x58=0x90` before caching this path. | EC `0x00` bit 0 set and bits 7 and 3 clear. The bit 3 check and stricter base checks are conservative project guards. | Discovery, ports, EC vendor byte, monitor start, and EC `0x00[7]` refusal S6 (lead-only); allow-list S3/S9/S11 |

Mark a failed pin-routing read unavailable for the affected fan until
rediscovery. If selection of the routing LDN fails on a modern profile,
both dependent fans are unavailable. Require successful selection of
LDN `0x04` before continuing to BAR discovery. Successful unrelated
inputs remain usable. Require the caller-held ISA mutex across discovery
and each sample, and apply the existing 1.5-second minimum interval
between sample bursts. (S3, S5, S9 and S8 §9.6.3.6)

On IT8620E and IT8628E, read temperatures as signed bytes at EC
`0x29`, `0x2A`, and `0x2B`. In EC `0x51`, bits 0–2 select physical
diodes and bits 3–5 select thermistors for the three inputs in order.
Bits 7:6 select an external source destination (0 for none, 1–3 for
TMPIN1–3). Publish a channel only if exactly one of its physical modes
is selected and it is not the external destination. Use the existing
project bounds of −55 to +125 °C inclusive and retain a physical
channel reading of 0 °C. Higher numbered temperatures and physical
board labels are excluded. A failed EC `0x51` read prevents temperatures
only; a failed individual temperature read prevents that input only.
(Register facts S6, lead-only; bounds and partial results S5)

## Register map

| Fact | IT8716F D | IT8620E and IT8628E |
| --- | --- | --- |
| FAN1/FAN2/FAN3 low bytes | EC `0x0D/0x0E/0x0F`, S8 §9.6.2.2.14 | EC `0x0D/0x0E/0x0F`, S6 (lead-only) |
| FAN1/FAN2/FAN3 high bytes | EC `0x18/0x19/0x1A`, S8 §9.6.2.2.21 | EC `0x18/0x19/0x1A`, S6 (lead-only) |
| Firmware fan enable | EC `0x13` bits 4/5/6 must be set for the corresponding fan, S8 §9.6.2.2.16 | EC `0x13` bits 4/5/6 must be set, S6 (lead-only) |
| Counter mode | EC `0x0C` bits 0/1/2 must be set for the corresponding fan to use 16-bit mode, S8 §9.6.2.2.13. | Fixed 16-bit for these channels, S6/S10 (lead-only). |
| FAN2/FAN3 pin routing | `CR29[2]` / `CR27[7]` must be clear respectively, S8 §§8.3.10, 8.3.12 | `CR29[2]` / `CR27[7]` of LDN `0x07` must be clear respectively, S6 (lead-only) |
| FAN1 pin routing | Dedicated tachometer pin, S8 §§4–5 | Available through the firmware fan-enable bit without an additional routing gate, S6 (lead-only) |

## Read procedure and decode

After reading the gate bytes, read each eligible channel's low byte and
then its high byte. The sources show that read order (S6/S7, lead-only),
but do not establish a hardware latch. As a bounded project consistency
check, read a second low/high pair immediately under the same mutex and
accept the count only if both pairs match. No waiting or retry loop is
needed. If either pair fails or they disagree, the channel is
unavailable for the sample. A failed common fan-enable or mode read
omits the dependent fan inputs while preserving eligible temperatures.

Compute `count = 256 * high + low`. A 16-bit counter uses divisor 2,
so the whole-number speed is `floor(675000 / count)`. The formula base
is pinned for IT8716F D by S8 §9.6.3.5 and for the modern profiles by
S6 (lead-only). The fixed 16-bit divisor is supported by S6/S7
(lead-only). Interpret the results only after the activation,
routing, mode, authorization, and consistency checks:

| Count | Reading | Basis |
| --- | --- | --- |
| `0x0000` | Invalid, no RPM. | S6 (lead-only) |
| `0x0001`–`0x003F` | Invalid, no RPM. The cutoff is a conservative Experimental plausibility guard. | S7 (lead-only), project policy S5 |
| `0x0040`–`0xFFFE` | Active, computed RPM. | S6/S7 (lead-only) |
| `0xFFFF` | Inactive, 0 RPM. Saturation means no measurable rotation. It does not establish physical disconnection or failure. | S6/S7 (lead-only), project policy S5 |

Omit a disabled or GPIO-routed input without reading its count. Keep the
generic `FAN1`–`FAN3` and `TMPIN1`–`TMPIN3` labels tied to their
channel numbers when a sibling is unavailable. The live display and fan
archive use the reading name as the key, so omitting one input must not
shift the names of the others. Use the existing motherboard fan reading
contract for numeric and invalid values. Report the scoped profile's
fan capability independently of this sample's available rows. (S3, S5;
`core/src/models/metrics.rs`, `core/src/persistence/archive.rs`)

### Synthetic cases for implementation review

| Fixture | Expected observation |
| --- | --- |
| Slot 0 exact `0x8716`, `CR22=0x01`, active aligned EC, EC `0x13=0x70`, EC `0x0C=0x07`, routed FAN1–3, and FAN1 bytes `0xC2/0x01` twice | FAN1 active at 1500 RPM. Repeat takes place within one mutex acquisition. |
| Same profile, enabled and routed; count pairs represent `0x0000`, `0xFFFF`, and `0xFFFE` | Invalid without RPM, inactive at 0 RPM, and active at 10 RPM respectively. |
| Same profile, stable `0x003F` and `0x0040` | Invalid without RPM and active at 10546 RPM respectively. |
| FAN2 routed to GPIO, FAN3 disabled in EC `0x13`, FAN1 saturates | FAN2/FAN3 omitted before reading counts. FAN1 reports no measurable rotation. |
| Exact `0x8716`, `CR22=0x01`, EC `0x13=0x70`, EC `0x0C=0x04` | Read and label FAN3 only. FAN1/FAN2 are outside the firmware-enabled 16-bit scope. |
| Exact `0x8716` and `CR22=0x81` | The low nibble still selects version D. Ignore bit 7 for identity and leave the configuration selector untouched. |
| Exact `0x8716` with `CR22=0x00`, an unlisted version such as `0x02`, or a failed revision read | Reject the expansion before LDN discovery and perform the usual primary configuration exit. |
| Exact `0x8728` with `CR22[3:0]=0x0`, or with `CR22[3:0]=0x1` and a different or unavailable baseboard identity | Exercise the existing temperature path under S3. No fan-routing, mode, or count reads are eligible. The matching board case has its own fixtures in S12. |
| Exact `0x8620` and separately exact `0x8628`, vendor byte `0x90`, EC `0x51=0x94`, bytes `0xC9`, `0x19`, `0x7D` | TMPIN1 unavailable (no physical input selected), TMPIN2 unavailable (external route), TMPIN3 +125 °C. If `0x51=0x15`, TMPIN1 −55 °C, TMPIN2 +25 °C, TMPIN3 +125 °C. |
| EC `0x51=0x07` with bytes `0xC8`, `0x00`, and `0x7E` | TMPIN2 at 0 °C. TMPIN1 and TMPIN3 are outside the accepted range. |
| Failed temperature selection read and valid FAN1, or failed FAN1 high byte and valid FAN2 | Preserve the successful fan under its original channel name. Mismatched count pairs leave the affected channel unavailable. |
| Modern profile with `CR60/61=0x0C/0xF8`, an unaligned base, a base above `0x0FF0`, or inactive LDN | Reject before any EC index write. |
| Mutex unavailable, BAR port denied, failed primary exit, or wrong EC vendor byte on a modern profile | Reject without caching a port proof. A failed primary exit prevents every EC read. |
| Slot 0 unknown ID or neighboring `0x8622`, or slot 1 with `0x8733`, `0x8790`, `0x8695`, or `0x8883` | No expansion LDN discovery or sensor reads. The slot-1 exit-withhold rule still applies to the existing diagnostic. |

## Quirks

- IT8716F `CR22` bit 7 can change which chip responds in a two-chip
  configuration. Read only the revision nibble for identification and
  leave `CR22` untouched. (S8 §8.3.5, printed p. 39)
- The two modern rows use individual chip IDs. S6 and S10 distinguish
  these exact models from neighboring IDs and identify the same fixed
  16-bit counter scheme for their scoped channels. The sources describe
  no revision-dependent path for these channels on either exact model.
  This evidence supports Experimental enablement only. The EC vendor
  byte checks the register block after chip selection. (S6/S10, lead-only)

## Deferred profiles

| Target | Disposition and evidence needed |
| --- | --- |
| IT8728F FAN1–3 outside the exact board and two channels in S12 | [#2518](https://github.com/shm11C3/HardwareVisualizer/issues/2518). S1's E-version document maps `CR22[3:0]=0x0` to AX through CX and `0x1` to both DX and EX. Broader FAN1–2 coverage needs a DX-specific read-only register map, accepted evidence tied to the DX marking with routing, enable, count and RPM, or a safe software-readable EX discriminator. FAN3 needs its own qualified scope. (S1 cover and §8.3.5, printed p. 48) |
| IT8728F/EX and IT8716F FAN4/5 | [#2518](https://github.com/shm11C3/HardwareVisualizer/issues/2518). The register maps name `0x80`–`0x83` for FAN4/5, while the same documents describe a seven-bit writable address field. Require a corroborated high-index transaction that cannot alias the monitor or read-to-clear status registers, plus the exact pin and enable gates. (S1 §§9.5.2.1, 9.5.2.2; S8 §§9.6.2.1, 9.6.2.2.47–48) |
| IT8705F, IT8712F | [#2518](https://github.com/shm11C3/HardwareVisualizer/issues/2518). Counter width depends on revision. Require exact `CR22` cutoffs with the relevant errata, active 8-bit divisor fields, and per-channel firmware mode checks. The IT8716F mode gate alone does not select these profiles. (S6, lead-only) |
| IT8716F C or unlisted revisions, 8-bit mode, IT8726F | [#2518](https://github.com/shm11C3/HardwareVisualizer/issues/2518). Pin the exact silicon revision and its compatible read-only fields. An 8-bit path also needs the active divisor. Raw `0x8726` needs its own EC field definitions. (S6, lead-only; S8) |
| IT8718F, IT8720F, IT8721F | [#2518](https://github.com/shm11C3/HardwareVisualizer/issues/2518). Require separately pinned discovery, per-channel routing, width control, and any high-index access. The firmware pin selection differs even across these three models. (S6, lead-only) |
| IT8771E, IT8772E | [#2518](https://github.com/shm11C3/HardwareVisualizer/issues/2518). Confirm revision-specific channel population and width. S6 flags uncertainty for IT8771E. ITE's [IT8772E V0.4 for F version](https://mediatronik.net/wp/wp-content/uploads/2024/01/IT8772E-ITE.pdf), §§9.5.2.2.14 and .19, names FAN2/3 only. Resolve this with any three-channel evidence before enabling other revisions. |
| IT8613E | [#2517](https://github.com/shm11C3/HardwareVisualizer/issues/2517). Confirm exact routing and enable fields. The published [comparison sheet](https://raw.githubusercontent.com/a1wong/it87/master/ITE_Register_map.pdf) leaves FAN1's enable field unspecified for this model. |
| IT8625E, IT8655E, IT8665E, IT8686E | [#2517](https://github.com/shm11C3/HardwareVisualizer/issues/2517). Pin initial bank, safe access while firmware owns SMBus, and the channel gates before selecting a profile. Some layouts require additional bank or transport handling in S10 (lead-only); this proposal leaves those operations disabled. |
| IT8631E, IT8638E, IT8688E, IT8689E, IT8696E | [#2517](https://github.com/shm11C3/HardwareVisualizer/issues/2517). Confirm the exact EC transport, index/bank layout, and per-input enable/routing independently. A matching chip ID or family name does not determine the transport. |
| IT8620E and IT8628E additional inputs | [#2517](https://github.com/shm11C3/HardwareVisualizer/issues/2517). Keep higher temperature and fan channels separate until routing and high-index access are established for each model. (S6, lead-only) |
| IT8790E `0x8790`, IT8792E/IT8795E `0x8733`, IT87952E `0x8695`, alternate report `0x8883` | [#2519](https://github.com/shm11C3/HardwareVisualizer/issues/2519). Keep the existing raw diagnostic and exit-withhold rule. Before extending telemetry, prove safe pre-key identification, active distinct base and usable BAR ownership without an exit, safe logical-device selection after BAR discovery, board/controller identity where IDs overlap, and channel attribution from a two-controller board. The alternate raw ID is exit-withhold evidence only. (S9, including its lead-only safety facts; S11) |
| Gigabyte ECIO and ISA bridge | [#2519](https://github.com/shm11C3/HardwareVisualizer/issues/2519). Pin independently recognized board/controller combinations, transport authorization, firmware-safe read protocol, concurrency, and duplicate suppression against direct LPC. Neither transport is included in the primary LPC access proposal. |

## Safety notes

- Required writes are limited to ITE configuration enter/exit, LDN
  selection through `CR07`, and EC register-index selection at
  `ec_base + 0x05`.
- Keep slot 1 within the existing S3 and S9 scopes, including S9's
  exit-withhold rule. Apply this expansion on slot 0 only.
- Do not write `CR22`, `CR30`, `CR60`, `CR61`, EC `0x00`, any EC data register,
  monitoring-start state, temperature configuration, interrupt/alarm
  state, fan-control/PWM, divisor, threshold/limit, GPIO, or power state.
- Do not read EC interrupt-status registers `0x01`-`0x03`; S1 documents
  read-to-clear behavior for this group. (S1 §9.5.2.2.2-9.5.2.2.4,
  printed p. 92)
- Hold the ISA mutex across the complete multi-step transaction. PawnIO
  requires caller ownership. A mutex timeout, failed transport proof, or
  unreadable global EC gate aborts the transaction. An individual input
  failure leaves successful siblings usable. Keep all attempts within
  the authorized PawnIO I/O window. (S3, S5, S9)
- PawnIO permission or component failures remain normal availability
  failures. Experimental classification alone must not trigger PawnIO
  installation/elevation guidance. (S5)

## Open questions

- Non-blocking for Phase 4 primary LPC expansion: Accept only an immediately repeated, matching count without relying on a hardware latch.
  S6/S7 establish low-byte then high-byte read order. Hardware latching
  and the exact timing of updates are unverified. The repeated read is
  a bounded consistency filter for Experimental values.
- Non-blocking for Phase 4 primary LPC expansion: Saturation reports no measurable rotation.
  An enabled input can saturate with a stopped fan or with no tachometer
  signal. S6/S7 support the observation but leave physical attachment
  unknown.
- Non-blocking for Phase 4 primary LPC expansion: Limit sampling to the named low-index channels on slot 0.
  High-index access, board-specific labels, and the additional models
  and transports in [Deferred profiles](#deferred-profiles) have separate
  evidence requirements.
- Non-blocking for Phase 4 primary LPC expansion: the IT8716F row uses version D and the separate board specification owns the exact IT8728F/EX fan scope.
  The S1 DX/EX alias cannot select a single version. S12 qualifies its
  factory board independently of that alias. The S8 cover
  describes future versions without enumerating their revision codes.

## Readiness review

- The scoped rows name the exact model IDs, applicable revision gate,
  and register dependencies. Every applicable copyleft-only fact is attributed to S6, S7 or S10 and
  confined to Experimental coverage.
- Discovery retains the active-LDN and post-exit port proofs, with
  bounded, mutex-protected transactions. Each sample has an explicit
  register allow-list and rejects unavailable or invalid inputs.
- The open questions identify facts outside the enabled scope or
  uncertainty covered by conservative validity checks. Approval of
  this specification transition remains with the maintainer.

## Manual compatibility evidence

Use the user-initiated compatibility report in
[#2500](https://github.com/shm11C3/HardwareVisualizer/issues/2500) to establish
board, firmware, exact raw ID, and provider outcome. For a later opt-in dump
of an approved profile, preserve the slot, EC base, post-exit port proof,
per-input gate bytes, and bounded raw pairs from that profile. Correlate
changing generic inputs with a same-session firmware monitor capture where
available. Synthetic fixtures exercise the contract and leave hardware
compatibility unverified.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-07 | Drafted exact primary-slot FAN1–3 for IT8728F/EX and IT8716F and TMPIN1–3 and FAN1–3 for IT8620E and IT8628E. Recorded deferred models and transport requirements without expanding secondary slot access. |
| 2 | 2026-11-08 | Propose Implementation-ready status for the four Experimental primary LPC rows. Pin IT8716F pin routing directly to the vendor specification, constrain modern EC base validation and authorization, and add synthetic scope, partial-result, and boundary cases. |
| 3 | 2026-11-08 | Require the vendor-defined IT8716F D revision nibble. Defer IT8728F fan reads because the E-only source shares the same revision value with DX. Retain the exact IT8620E and IT8628E model-scoped Experimental paths and add revision rejection cases. |
| 4 | 2026-11-08 | Keep the primary expansion Implementation-ready and assign the exact GA-B75M-D3H FAN1–2 profile to its separate specification. Limit the IT8728 no-fan fixture and deferred row to cases outside that profile. Retain the revision 3 primary expansion profiles and transport rules. |
