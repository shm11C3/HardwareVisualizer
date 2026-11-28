# Spec: Primary LPC expansion for ITE Environment Controller sensors

| Field | Value |
| --- | --- |
| Revision | 8 |
| Status | Implementation-ready (rev 8) |
| Scope | Read-only access to the primary LPC Environment Controller for IT8716F silicon version D FAN1–3, firmware-enabled 16-bit FAN1–3 on IT8718F, IT8721F silicon version DX FAN1–3, IT8772E silicon version FX FAN2–3, exact IT8620E and IT8628E TMPIN1–3 and FAN1–3, and bank-guarded TMPIN1–3 and FAN1–3 on exact IT8686E, IT8688E and IT8689E. Only the model and channel rows below qualify. |
| Issue phase | Phase 4 (#1635), [#2517](https://github.com/shm11C3/HardwareVisualizer/issues/2517), [#2518](https://github.com/shm11C3/HardwareVisualizer/issues/2518), [#2695](https://github.com/shm11C3/HardwareVisualizer/issues/2695), [#2698](https://github.com/shm11C3/HardwareVisualizer/issues/2698) |

Revision 8 adds two Experimental fan profiles for
[#2695](https://github.com/shm11C3/HardwareVisualizer/issues/2695). Both
use the IT8716F D mechanism: slot 0, LDN `0x04`, the low-index FAN1–3
count pairs, `CR27[7]`/`CR29[2]` routing and the EC `0x13` enables. The
ITE datasheets S22–S24 define the register fields of both profiles. The
first profile reads FAN1–3 on exact raw ID `0x8718` (IT8718F) where firmware
has set the per-channel 16-bit counter enable. The second reads FAN1–3 on exact
raw ID `0x8721` with `CR22[3:0]=0x3` (IT8721F DX), whose 16-bit counters
are always enabled. Both rows stay Experimental. The fixed divisor, the
read order and the count dispositions are lead-only on both. On IT8718F,
the coverage of `CR22` values other than `0x0` and `0x1` is lead-only too
(see [Applicability of the IT8718F documents](#applicability-of-the-it8718f-documents)).
Revision 8 also gives every #2695 target a disposition in
[Deferred profiles](#deferred-profiles). The revision 7 profiles, gates
and transport rules are unchanged.

Revision 7 adds one Experimental profile for the banked modern
controllers with exact raw IDs `0x8686`, `0x8688` and `0x8689`. S19–S21
observe these as the primary controllers on Gigabyte AM4, LGA1151 and
AM5 boards. The profile reads only bank-0 registers below index
`0x80`. It never writes the bank selector. Each sample is bracketed by
bank checks (see [Banked modern profile](#banked-modern-profile)).
Revision 7 also adds a runtime gate table and gives every
[#2698](https://github.com/shm11C3/HardwareVisualizer/issues/2698)
target a disposition. The revision 6 profiles, gates and transport
rules are unchanged. Clean-room implementation can pin revision 8 after
the maintainer approves the specification PR under the
[specification gate](README.md#status-transition-draft--implementation-ready).
All expansion rows are Experimental. They have pinned lead-only
facts and lack an accepted independent hardware dump. The existing
IT8728F/EX temperature path is defined in its
[temperature specification revision 2](superio-ite-it86xx-it87xx.md).
Primary-slot FAN1–2 on IT8728F DX and EX (raw `0x8728`,
`CR22[3:0]=0x1`) are defined in the
[IT8728F fan specification revision 2](superio-ite-it8728f-ex-ga-b75m-d3h-fans.md).
Apply the existing successful-reading presentation and Experimental
failure diagnostics under [ADR 0011](../../adr/0011-experimental-sensor-enablement.md).

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | ITE Tech., Inc., IT8728F Preliminary Specification V0.4.2 (E version), pinned as S1 in [the existing IT8728 temperature specification](superio-ite-it86xx-it87xx.md#sources) | Vendor-authored primary source. The cover limits V0.4.2 to E. Section 8.3.5, printed p. 48, assigns the same revision nibble to DX and EX. Pins the E-specific fan map in §§9.5.2.1–9.5.2.2 and the temperature path in S3. Context only for the banked modern profile: on IT8728F the Environment Controller accepts both LPC and serial-bus register access (§9.5, printed p. 87), EC `0x06` is SMI# Mask Register 3 with bits 7–3 reserved (Table 9-2, printed p. 88, §9.5.2.2.7, printed p. 92), and LDN `0x04` `CR F3` bits 4–1 are reserved (§8.8.10, printed p. 63). The bank and serial-bus meanings used for IT8686E, IT8688E and IT8689E are therefore model-specific lead-only facts. They are not inherited from S1. |
| S3 | [Existing IT8728 temperature specification revision 2](superio-ite-it86xx-it87xx.md) | Baseline for access ordering, PawnIO BAR authorization and the explicit post-exit port proof. |
| S5 | [ADR 0011](../../adr/0011-experimental-sensor-enablement.md) and [design principle DP-02](../../design-principles.md#dp-02-represent-partial-capability-honestly) | Project policy source for Experimental enablement, success/failure presentation, plausibility-gated partial readings, manual feedback, and no new telemetry. It supplies no hardware fact. |
| S6 | Linux kernel [v6.18, `drivers/hwmon/it87.c`](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/it87.c), Git blob `e233aafa8856c94c3fde3502732fcda2bd3bf130`, GPL-2.0-or-later | **lead-only (copyleft)**. Pins exact modern IDs, I/O discovery and layout, temperature routing, pin selections, fan count format and availability, vendor register, and 16-bit mode differences. The exact IT8620E and IT8628E profile entries and model-scoped discovery are at lines 477–504, 2779–2787 and 2955–2989. For exact IT8772E, lines 409–418 and 2757–2759 identify the model and fixed 16-bit path. Lines 683–687, 910–921, 3245–3255, 3309–3325 and 3424–3448 pin divisor, count interpretation, read order, enable order and model-specific mode handling. The three-fan claim in this source does not apply to the FX scope in S13. For IT8718F and IT8721F, lines 350–357 and 366–375 give both models 16-bit fan counters, lines 2736–2744 map raw `0x8718` and `0x8721`, lines 2828–2831 read the `CR22` low nibble on every chip, and lines 3070–3119 take FAN3 routing from `CR27` bit 7 and FAN2 routing from `CR29` bit 2 for both models. Lines 3427–3448 change the counter format by revision only for raw `0x8705` and `0x8712`. The fixed divisor 2 of the 16-bit format is at lines 685–687. |
| S7 | LibreHardwareMonitor [`2808d80564b6`, ITE LPC Environment Controller](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/IT87XX.cs), Git blob `cc3dfd301e46cca5a92801890c512e0af422f8e1`, MPL-2.0 | **lead-only (copyleft)**. Corroborates the fixed divisor, low-byte then high-byte read order, maximum count as no measured rotation, and supports a conservative lower count bound for 16-bit tachometer channels. For the banked modern profile, lines 84–86 give IT8689E four register banks, lines 420–436 place the bank field in EC `0x06` bits 6:5 and state that a bank change swaps indexes `0x10`–`0xAF`, lines 151 and 194 require a bank-0 write before every IT8665E and IT8655E update, line 470 treats a temperature byte of 0 or less, or 127, as no reading, and lines 561–563 record that IT8688E does not return the written index when its address port is read. The matching [chip-ID map](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/LpcIO.cs#L623-L625), Git blob `3bb61abd9d86df2dcf388cc80689d4db61d16537`, names raw `0x8686`, `0x8688` and `0x8689`. The same map names raw `0x8718` and `0x8721` at lines 630 and 632. In the controller source, line 239 makes the 16-bit counter path depend on the revision only for IT8705F and IT8712F. Lines 476–495 read FAN1–3 of the other models as a low byte followed by a high byte and convert with divisor 2. |
| S8 | ITE Tech., Inc., *IT8716F Environment Control - LPC I/O*, Preliminary Specification V0.3, [public third-party mirror](https://www.rom.by/files/IT8716F_V0.3.pdf) | Vendor-authored primary document. The cover applies V0.3 to D and future versions. Section 8.3.5, printed p. 39, explicitly identifies C and D through `CR22[3:0]`. Pins configuration §8.1 and §§8.3, 8.8, dedicated FAN1 and shared FAN2/3 pin functions §§4–5 and §§8.3.10, 8.3.12, EC access §§9.6.1–9.6.2.1, monitor state §9.6.2.2.1, fan read and enable fields §§9.6.2.2.13–16 and .21, printed pp. 89–91, and tachometer format and cadence §§9.6.3.5–6, printed p. 101. |
| S9 | [Super I/O configuration access revision 4](superio-access.md#slot-1-ite-exit-withhold-rule) and [PawnIO interface revision 7](pawnio-interface.md#lpcio) | Current repository transport facts and scope limits. The Experimental slot-1 withhold applies to diagnostics. |
| S10 | Out-of-tree [IT87 driver](https://github.com/frankcrawford/it87/blob/master/it87.c), Git blob `56148f202fee43436fa73a747a1bf465deb8721e`, GPL-2.0-or-later | **lead-only (copyleft)**. Corroborates the separate exact IT8620E and IT8628E IDs and fixed 16-bit fan profile selection, with distinct bank and SMBus access requirements for deferred chips. |
| S11 | PawnIO.Modules [`LpcIO` at `52a7e536dff3`](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/LpcIO.p) | Upstream public interface used by HardwareVisualizer. Slot selection invalidates previous BAR permissions. BAR discovery is a multi-LDN scan and does not guarantee preservation of the selected LDN. |
| S12 | [IT8728F primary-slot FAN1–2 specification revision 2](superio-ite-it8728f-ex-ga-b75m-d3h-fans.md) | Applicability reference for the separate chip-level Experimental IT8728F FAN1–2 profile. The primary expansion rows in this document do not depend on that profile. |
| S13 | ITE Tech., Inc., *IT8772E Environment Control – LPC I/O*, Preliminary Specification V0.4 (For F Version), ITPM-PN-2012032, 2012-03-16, [public third-party mirror](https://mediatronik.net/wp/wp-content/uploads/2024/01/IT8772E-ITE.pdf), SHA-256 `e7517c2c4228526e95308dd275cfaf2d70c249b9b36e7657ed800d9c377539db` | Vendor-authored primary source. Printed pages are one-based PDF pages minus 14. Pins primary configuration and exact FX identity (§§8.1, 8.3.1–8.3.5, pp. 25–26, 33), tachometer routing (§§8.3.10, 8.3.12, pp. 36–37), active EC base (§§8.5.1–8.5.3, p. 41), LPC index/data ports (Table 9-1, p. 68), runtime gate and alarms (§§9.5.2.2.1–9.5.2.2.4, pp. 73–74), tachometer mode, counts and enable (§§9.5.2.2.13–9.5.2.2.16, .19, pp. 75–77, 79), vendor byte (§9.5.2.2.34, p. 81), and conversion and cadence (§§9.5.3.5–9.5.3.6, p. 95). |
| S14 | coreboot 4.22, [exact IT8772F firmware definitions](https://github.com/coreboot/coreboot/blob/4.22/src/superio/ite/it8772f/it8772f.h#L36-L38), Git blob `38cfe3b20ee841ca9a89333ffa9cc030e826a0a9`, [enable path](https://github.com/coreboot/coreboot/blob/4.22/src/superio/ite/it8772f/superio.c#L130-L134), Git blob `82f30fa6cd36b9bbaf4aa9469d24ad571af9d7be`, GPL-2.0-or-later, and [Google Beltino board configuration](https://github.com/coreboot/coreboot/blob/4.22/src/mainboard/google/beltino/devicetree.cb#L80-L95), Git blob `cdaab35b54e4a8517431e6926c5e4f724b345cb9` | **lead-only (copyleft)**. Corroborates channel enable bit order and a deployment with FAN2 enabled by firmware on primary LDN `0x04` at `0x0700`. Supplies no live RPM evidence and does not authorize any firmware writes. |
| S15 | Out-of-tree IT87 driver, [commit `bd7a566c8a60`](https://github.com/frankcrawford/it87/blob/bd7a566c8a60fe0308d67e26761e16260368191a/it87.c), Git blob `56148f202fee43436fa73a747a1bf465deb8721e` (the S10 blob), GPL-2.0-or-later | **lead-only (copyleft)**. Line-pinned facts for the banked modern profile. Lines 276–278 give exact IDs `0x8686`, `0x8688` and `0x8689`. Lines 914–949 mark all three as banked, 16-bit-fan, new-temperature-map chips with serial-bus enable bits 1 and 2. Line 286 places those enable bits in LDN `0x04` `CR F3`, and lines 6387–6394 read them during discovery. Lines 353 and 2651–2684 place the bank field in EC `0x06` from bit 5 upward and restore the previous bank after each banked access. Lines 2614–2648 clear the serial-bus enables around every update, a configuration write this profile excludes. Lines 379 and 381 give the FAN1–3 count bytes, line 407 the temperature bytes, and line 6739 the FAN1–3 enables in EC `0x13` bits 4–6. Lines 6062–6100 select FAN2 and FAN3 routing through `CR29[2]` and `CR27[7]` with LDN `0x07` selected. Lines 489–490 and 3712–3740 place the temperature-source selection for these models in bank 2. Lines 6984–6985 refuse EC `0x00` bit 7 set or EC `0x58` other than `0x90`. Lines 3460–3464 read each count low byte then high byte. Lines 5819–5823 select an alternate memory-mapped transport for IT8688E revision 2. |
| S16 | Linux kernel [v7.4-rc1, `drivers/hwmon/it87.c`](https://github.com/torvalds/linux/blob/v7.4-rc1/drivers/hwmon/it87.c), Git blob `2757d428a23dba550756cb327c851dbcaa25c078`, GPL-2.0-or-later, and commit [`66b8eaf8def2`](https://github.com/torvalds/linux/commit/66b8eaf8def2d51dab49c4921b93f1bf1c7638dc) | **lead-only (copyleft)**. Independent upstream corroboration for IT8689E only. Lines 567–576 give 16-bit fans and serial-bus enable bits 1 and 2. Lines 3210–3245 give the same FAN2 and FAN3 routing fields as S15. Lines 3429–3435 read `CR F3`, and lines 3723–3724 refuse EC `0x00` bit 7 or a vendor byte other than `0x90`. This revision has no bank handling for IT8689E. |
| S17 | Guenter Roeck, *ITE register differences*, dated 2017-04-11, [`ITE_Register_map.pdf`](https://github.com/a1wong/it87/blob/65ec08b8a3c118e99a8a5da4052d7719caa69cd1/ITE_Register_map.pdf) in an IT87 driver repository, commit `65ec08b8a3c118e99a8a5da4052d7719caa69cd1`, SHA-256 `856302bc32a1a26af1e1cec22c2244a1700fbc7eb7d0db97276a50aa482b67ff` | **lead-only (copyleft)**. Community-compiled comparison table distributed with a GPL driver. Its sheet marks some cells as guesswork. Page 1 gives IT8686E FAN1–3 enables in EC `0x13` bits 4–6, count bytes `0x0D`–`0x0F` and `0x18`–`0x1A`, temperature bytes `0x29`–`0x2B`, and TMPIN1–3 source selection split between EC `0x51` and bank-2 indexes `0x1D`–`0x1E`. Page 2 warns that the embedded controller on at least one multi-bank chip, IT8665E, changes the bank while it accesses the registers. |
| S18 | Frank Crawford, *hwmon: (it87) Add access control for SMBus*, [v2 cover letter](https://lore.kernel.org/linux-hwmon/20230416042510.1929077-1-frank@crawford.emu.id.au/), 2023-04-16, and applied commit [`9989b3c0ff97`](https://github.com/torvalds/linux/commit/9989b3c0ff9771760c103d46e6b5e8645fd3d836) | **lead-only (copyleft)**. Patch-series description for a GPL driver. On some chips, an embedded controller can reach Environment Controller registers over SMBus at any time without synchronization. On banked chips, this can produce access errors with unpredictable results. |
| S19 | AX370-Gaming K7 owner report, [LibreHardwareMonitor issue 1030](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/issues/1030), [attached report](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/files/11068646/LibreHardwareMonitor.Report.txt), 2023-03-25, SHA-256 `276b0ff0864c337245602f78df3b25384279aa6838d1d2a8ca92466f7b557ed8` | Independent public register dump. It is not maintainer-accepted. It records BIOS F51i, slot-0 raw `0x8686` with chip version 2 at base `0x0A40`, and a separate `0x8733` at `0x4E` with base `0x0A60`. The bank-0 dump shows EC `0x00=0x19`, `0x06=0x00`, `0x0C=0x30`, `0x13=0x77`, `0x51=0x18` and `0x58=0x90`. FAN1–3 count pairs are `0x04AB`, `0x04CB` and `0x0892`. TMPIN1–3 bytes are `0x27`, `0x31` and `0x35`, and the tool reported a changing third input. The tool also read EC `0x01`–`0x03`, which this profile never selects. |
| S20 | Z390 AORUS ELITE owner report, [out-of-tree IT87 issue 148](https://github.com/frankcrawford/it87/issues/148), 2026-11-13, with the driver contributor replies of [2026-11-17](https://github.com/frankcrawford/it87/issues/148#issuecomment-6957429357) and [2026-11-19](https://github.com/frankcrawford/it87/issues/148#issuecomment-7018129185) | Independent owner observation with IT8688E revision 1 and BIOS F12c. The OEM SSDT defines a locked indexed field on ports `0x0A45`/`0x0A46`. While the OS also polled the OEM WMI temperature method, direct reads showed brief fan readings of 80–90 RPM instead of about 700 RPM several times per minute. These disappeared when the direct reads were serialized with the ACPI Global Lock. The contributor states, without supporting evidence, that under normal operation only the firmware's fan-stop SMM routines use those ports. The issue body contains patch code, which this specification does not use. |
| S21 | [Gigabyte secondary ITE specification revision 8](superio-ite-gigabyte-secondary.md) | Repository evidence pinned from OEM artifacts and owner logs. X470 AORUS ULTRA GAMING F61a reports primary raw `0x8686` with a reservation at `0x0A40`–`0x0A4F`, and its OEM `GSWApp` SSDT places an indexed monitor field at `0x0A45`/`0x0A46`. X470 AORUS GAMING 7 WIFI reports primary `0x8686` at `0x2E` with base `0x0A40`. B650E AORUS MASTER reports primary IT8689E revision 2 at `0x0A40`. Its F37 `GSWApp` table and the X670E AORUS MASTER F41 table use the same primary pair. The inspected F64 SMM handlers write hardware only on power-button and S3–S5 entry events. |
| S22 | ITE Tech., Inc., *IT8718F Environment Control – Low Pin Count Input / Output (EC - LPC I/O)*, Preliminary Specification V0.3, 2006, [public third-party mirror](https://theretroweb.com/chip/documentation/it8718f-v0-3-for-c-version-6a17d9849ec13686841932.pdf), SHA-256 `838038b0218eabba54deeb57432126e27e72f7d4321ae7e21e3c54e449201a80` | Vendor-authored primary source. Printed pages are one-based PDF pages minus 18. The cover applies V0.3 to the C version and future versions. The front-matter revision history records that §8.3.5 changed its default from `00h` to `01h` and added `0001b` for C. Pins configuration keys and exit §8.1, pp. 29–30, chip ID and `CR22` §§8.3.3–8.3.5, p. 36, FAN3 and FAN2 pin selection §8.3.10, p. 39, and §8.3.12, p. 41, the dedicated FAN1 pin in Table 5-4, pp. 11–12, LDN `0x04` activation and base §§8.8.1–8.8.3, p. 52, EC ports and the seven-bit index §§9.6.1–9.6.2.1, p. 79, monitor state §9.6.2.2.1, p. 83, read-to-clear status §§9.6.2.2.2–9.6.2.2.4, p. 84, EC `0x0B` §9.6.2.2.12, p. 85, the 16-bit counter enables and count low bytes §§9.6.2.2.13–9.6.2.2.14, p. 86, fan enables §9.6.2.2.16, p. 87, count high bytes §9.6.2.2.21, p. 88, FAN4/FAN5 at `0x80`–`0x83` §§9.6.2.2.47–9.6.2.2.48, p. 92, vendor byte §9.6.2.2.35, p. 90, starting conversion §9.6.3.2, p. 96, and tachometer formula and cadence §§9.6.3.5–9.6.3.6, p. 99. |
| S23 | ITE Tech., Inc., *IT8718F Environment Control – Low Pin Count Input / Output (EC - LPC I/O)*, Preliminary Specification V0.2, 2006, [public third-party mirror](https://theretroweb.com/chip/documentation/it8718f-v0-2-6a17d984a3ac7809239393.pdf), SHA-256 `011cb240c34df409aa398d962354f2db4e0401a7b336e7c70a6d25bdd6a0fb04` | Vendor-authored primary source that predates S22. Printed pages are one-based PDF pages minus 20. §8.3.5, p. 34, gives `CR22` the default `00h` and lists no version code. The spec author compared S23 and S22 section by section. They state the same fields for FAN2/FAN3 pin selection (§8.3.10, p. 37, §8.3.12, p. 38), monitor state (§9.6.2.2.1, p. 81), 16-bit counter enables (§9.6.2.2.13, p. 83), count low bytes and fan enables (§§9.6.2.2.14, 9.6.2.2.16, p. 84), count high bytes (§9.6.2.2.21, p. 86) and the tachometer formula (§9.6.3.5, p. 95). Only line breaks differ. |
| S24 | ITE Tech., Inc., *IT8721F Environment Control – Low Pin Count Input / Output (EC - LPC I/O)*, Preliminary Specification V0.5 (For D Version), 2009, [public third-party mirror](https://15h.org/images/a/a0/IT8721.pdf), SHA-256 `d9f2329cb211b72d23f9c2236aee170d8a3179ec909ae4632f92a917d15ca65f` | Vendor-authored primary source. The publicly retrievable file carries a confidentiality watermark. Printed pages are one-based PDF pages minus 16. The cover applies V0.5 only to the D version. Pins configuration keys and exit §8.1, pp. 31–32, chip ID and `CR22` with `0011b` for DX and part number IT8721F/DX §§8.3.3–8.3.5, p. 40, FAN3 and FAN2 pin selection §8.3.10, p. 43, and §8.3.12, p. 44, the dedicated FAN1 pin in Table 5-4, pp. 11–12, LDN `0x04` activation and base §§8.8.1–8.8.3, p. 55, EC ports and the seven-bit index §§9.6.1–9.6.2.1, p. 83, the register table Table 9-3, p. 84, monitor state §9.6.2.2.1, p. 87, read-to-clear status §§9.6.2.2.2–9.6.2.2.4, p. 88, EC `0x0B` and the always-enabled 16-bit counters §§9.6.2.2.12–9.6.2.2.13, p. 90, count low bytes and fan enables §§9.6.2.2.14, 9.6.2.2.16, p. 91, count high bytes §9.6.2.2.19, p. 92, vendor byte §9.6.2.2.34, p. 95, FAN4/FAN5 at `0x80`–`0x83` §§9.6.2.2.47–9.6.2.2.48, p. 97, starting conversion §9.6.3.2, p. 101, the tachometer formula §9.6.3.5, p. 103, and cadence §9.6.3.6, p. 104. |
| S25 | ITE, *IT8705F/IT8705AF Simple Low Pin Count Input / Output*, Preliminary Specification V0.4, 2003, [public mirror](https://bitsavers.mirrorservice.org/components/ite/IT8705F_Simple_LPCIO_200308.pdf), SHA-256 `12baf5a0f144f121a63e4fc43894b11d5635095a3fb97a4979189ef9fc9a74b2` | Vendor-authored primary source, used here only for the IT8705F disposition. The cover applies V0.4 to the G version and future versions. §8.3.5, p. 35, gives `CR22` the default `03h` and lists no version code. EC `0x0B` holds per-channel fan divisors (§9.5.3.2.12, p. 81) and EC `0x0C` bits 0–2 enable 16-bit counters (§9.5.3.2.13, p. 82). The tachometer text (§9.5.4.5, p. 90) does not state which divisor applies in 16-bit mode. The pin list in §4 multiplexes FAN_TAC2 with GP56 and FAN_TAC3 with FA18/GP57, and adds alternate tachometer inputs on game-port pins. |
| S26 | ITE Tech., Inc., *IT8783E/F Environment Control – Low Pin Count Input / Output*, Preliminary Specification V0.5 (For A Version), 2010, [public mirror](https://bitsavers.mirrorservice.org/components/ite/IT8783_A_EC_LPCIO_V0.5_201006.pdf), SHA-256 `2869d0098911cfed8012ee4d80bade86ef0f6678e9bcd0292a3356a3c8808eb6` | Vendor-authored primary source, used here only for the IT8783E/F disposition. The cover applies V0.5 to the A version. §8.3.5, p. 44, gives `CR22` the default `00h`. Its fan pin selection was not audited for this revision. |
| S27 | ITE Tech., Inc., IT8718F Preliminary Specification V0.8.1 (For J Version) and V0.8.2 (For K Version), [J mirror](https://theretroweb.com/chip/documentation/ite8718f-6419f0e3d83a0219023860.pdf), SHA-256 `0a0826ec49997d8f15babf0a7e52d490b5189741098264ab33d4515c94226ffe`, and [K mirror](https://theretroweb.com/chip/documentation/it8718f-v0-8-2-for-k-version-6a17d984a26c0410668976.pdf), SHA-256 `b7ad09cd0b9ef3f9e4c781442fbd3c21ae09a5ca817db05146a3749ffb672fda` | Vendor-authored. The public copies contain only the features, general description, block diagram, package, ordering and top-marking sections. They define no register. Cited only for the applicability open question. |
| S28 | IT8720F Preliminary Specification V0.7 (For J Version), [catalog listing](https://www.alldatasheet.com/datasheet-pdf/pdf/1150593/ITE/IT8720F.html), 18 pages | Catalog record of the only IT8720F document the spec author found. The listing describes a product brief of the same length as S27. No register-level IT8720F document was found. Cited only for the IT8720F disposition. |
| S29 | [Kraftway 945GCMX-S2 Linux hardware probe](https://linux-hardware.org/?log=dmesg&probe=c426290926), dmesg | Independent observation, not maintainer-accepted. The log identifies board `945GCMX-S2` with BIOS F4 and an IT8718F at base `0x0290` with revision 2, which is the `CR22` low nibble (S6 lines 2828–2831). It records no fan count. Evidence identity only. |
| S30 | Gigabyte GA-945GCM-S2L owner report, [lm-sensors list, message 40954](https://www.spinics.net/lists/lm-sensors/msg40954.html) | Independent observation, not maintainer-accepted. The Linux 3.2 log identifies an IT8718F at base `0x0290` with revision 4. The `sensors` output reports CPU and chassis fans at 1721 and 2096 RPM. The S6 driver sets the 16-bit counter enables itself when firmware left them clear (S6 lines 3245–3260), so the report does not show the firmware state of EC `0x0C`. Evidence identity only. |

## Detection

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact raw chip ID `0x8728` / IT8728F/EX TMPIN1–3 | Existing **Experimental** scope from S3. | Enabled under S3. |
| Exact `0x8728` / IT8728F fans | S12 owns slot-0 FAN1–2 for `CR22[3:0]=0x1`, which covers DX and EX through a lead-only fact. FAN3, other version nibbles and higher inputs have no qualified scope (S1, S12). | Disabled in this document. FAN1–2 follow S12. |
| Slot 0, exact `0x8716` / IT8716F D with `CR22[3:0]=0x1`, FAN1–3 | **Experimental**. S8 identifies D and defines discovery, counters, pin routing, and per-channel 16-bit mode bits. Fixed divisor in 16-bit mode, read order, count sentinels, and conservative lower count bound are lead-only (S6/S7). | Enabled best-effort for eligible firmware-enabled 16-bit inputs. Other or unreadable revision nibbles are outside the scope. |
| Slot 0, exact `0x8718` / IT8718F, firmware-enabled 16-bit FAN1–3 | **Experimental**. S22 defines discovery, the EC block and vendor byte, monitor state, the dedicated FAN1 pin, FAN2/FAN3 routing, the per-channel 16-bit counter enables, the fan enables, both count bytes, the read-to-clear status group, and the tachometer formula with default divisor 2. S23, the earlier document with `CR22` default `00h`, states the same routing, monitor-state, counter-enable, fan-enable, count and formula fields. Lead-only facts: coverage of `CR22` low nibbles other than `0x0` and `0x1` (S6, S7; see [Applicability of the IT8718F documents](#applicability-of-the-it8718f-documents)), and the fixed divisor 2 in 16-bit mode, the read order, the count sentinels and the conservative lower count bound (S6, S7). Observed members, which are evidence identities that do not gate the read: IT8718F revision 2 on Kraftway 945GCMX-S2 BIOS F4 (S29) and revision 4 on Gigabyte GA-945GCM-S2L with Linux-reported fan RPM (S30). Neither observation is a maintainer-accepted dump. | Enabled best-effort for eligible firmware-enabled 16-bit inputs. Inputs left in 8-bit mode are omitted. |
| Slot 0, exact `0x8721` / IT8721F DX with `CR22[3:0]=0x3`, FAN1–3 | **Experimental**. S24 establishes DX identity, discovery, the EC block and vendor byte, monitor state, the dedicated FAN1 pin, FAN2/FAN3 routing, always-enabled 16-bit counters, the fan enables, both count bytes, the read-to-clear status group, and the tachometer formula with default divisor 2. Lead-only facts: the fixed divisor 2 in 16-bit mode, the read order, the count sentinels and the conservative lower count bound (S6, S7). No observation is recorded for this row. | Enabled best-effort for eligible inputs. Other or unreadable revision nibbles are outside the scope. |
| Slot 0, exact `0x8772` / IT8772E FX with `CR22[3:0]=0x2`, FAN2–3 | **Experimental**. S13 establishes FX identity, primary discovery, two tachometer inputs, routing, and both count bytes. The ordinal enable bits, fixed 16-bit interpretation and divisor, read order, and count dispositions use S6/S7/S14 (lead-only). | Enabled best-effort for eligible FAN2 and FAN3 inputs after approval of this revision. |
| Slot 0, exact `0x8620` / IT8620E TMPIN1–3 and FAN1–3 | **Experimental**. Exact ID and model-wide fixed 16-bit fan path from S6/S10. Discovery, vendor check, EC read access, physical-temperature selection and encoding, fan enable, pin routing, divisor, read order, count sentinels, and conservative lower count bound are lead-only (S6/S7). | Enabled best-effort for eligible inputs. |
| Slot 0, exact `0x8628` / IT8628E TMPIN1–3 and FAN1–3 | **Experimental**. Exact ID and model-wide fixed 16-bit fan path from S6/S10. Discovery, vendor check, EC read access, physical-temperature selection and encoding, fan enable, pin routing, divisor, read order, count sentinels, and conservative lower count bound are lead-only (S6/S7). | Enabled best-effort for eligible inputs. |
| Slot 0, exact `0x8686` / IT8686E, `0x8688` / IT8688E or `0x8689` / IT8689E, bank-guarded TMPIN1–3 and FAN1–3 | **Experimental**. No vendor document is public for these models. Lead-only facts: exact IDs (S15, S7, and S16 for `0x8689`); discovery, EC block, vendor byte and EC `0x00` state (S15, S16); bank field in EC `0x06` and the bank-swapped index range (S15, S7); serial-bus access that can change the bank (S17, S18); FAN1–3 enables, count bytes, fixed 16-bit width, divisor 2, read order and count dispositions (S15, S16, S17, S7); FAN2/FAN3 routing (S15, S16); temperature byte location and encoding (S15, S17, S7); and the bank-2 source map that this profile does not read (S15, S17). The S19 dump and the S20 and S21 observations corroborate the transport, base and byte patterns without being maintainer-accepted. Best-evidenced members: IT8686E version 2 on AX370-Gaming K7 F51i (S19), `0x8686` on X470 AORUS ULTRA GAMING F61a and X470 AORUS GAMING 7 WIFI (S21), IT8688E revision 1 on Z390 AORUS ELITE F12c (S20), and IT8689E revision 2 on B650E AORUS MASTER (S21). | Enabled best-effort for eligible inputs under the [Banked modern profile](#banked-modern-profile). Higher channels are deferred. |
| Additional channels, IT86xx/IT87xx models, and secondary transport | See [Deferred profiles](#deferred-profiles). A raw `0x8721` can use only the IT8721F DX row above, never the IT8728 profile. | Disabled. |

### Runtime gates

Every runtime condition below is a mechanism or safety gate
([ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md)).
Board names, BIOS versions, chip revisions outside the listed revision
gates, and observed base addresses are evidence identities. They appear
only in Status and fixtures. "All" means every expansion profile in this
document. The IT8728F rows follow the gates of S3 and S12.

| Condition | Profiles | Gate kind | Source |
| --- | --- | --- | --- |
| Configuration slot 0 (`0x2E`/`0x2F`) only. Slot 1 keeps the S9 diagnostic and exit-withhold rule. | All | safety | S9 (lead-only safety facts in S9) |
| Exact raw chip ID `0x8716`, `0x8718`, `0x8721`, `0x8772`, `0x8620`, `0x8628`, `0x8686`, `0x8688` or `0x8689` | Per profile | mechanism | S8, S22, S24, S13, S6, S10, S15, S16 |
| `CR22[3:0]=0x1` | IT8716F D | mechanism | S8 §8.3.5 |
| `CR22[3:0]=0x3` | IT8721F DX | mechanism | S24 §8.3.5 |
| `CR22[3:0]=0x2` | IT8772E FX | mechanism | S13 §8.3.5 |
| LDN `0x04` `CR30[0]=1` and a valid aligned base from `CR60/61` | All | mechanism | S8, S22, S24, S13, S6, S15 |
| Base in `0x0100`–`0x0FF0`, bits 2:0 clear and outside the PawnIO-protected PCI window | IT8620E, IT8628E, banked modern | safety | S3, S9, S11 |
| `ioctl_find_bars` success, successful primary exit and post-exit EC `0x00` port proof | All | mechanism | S3, S11 |
| EC `0x58=0x90` | IT8718F, IT8721F DX, IT8772E FX, IT8620E, IT8628E (discovery); banked modern (every sample) | mechanism | S22 §9.6.2.2.35; S24 §9.6.2.2.34; S13 §9.5.2.2.34; S6, S15, S16 |
| EC `0x00` bit 0 set and bit 3 clear, plus bit 7 clear on IT8718F, IT8721F DX, IT8772E FX, IT8620E and IT8628E | IT8716F D, IT8718F, IT8721F DX, IT8772E FX, IT8620E, IT8628E | mechanism | S8, S22 §§9.6.2.2.1, 9.6.3.2, S24 §§9.6.2.2.1, 9.6.3.2, S13, S6 |
| EC `0x00` bit 0 set and bit 7 clear. Bit 3 is not a gate on this profile. | Banked modern | mechanism | S15, S16, S19 |
| EC `0x06` bits 7:5 all clear immediately before and immediately after the sample's banked reads | Banked modern | safety | S15, S7, S17, S18 |
| FAN routing `CR29[2]=0` (FAN2) and `CR27[7]=0` (FAN3) | IT8716F D, IT8718F, IT8721F DX, IT8772E FX, IT8620E, IT8628E, banked modern | mechanism | S8, S22 §§8.3.10, 8.3.12, S24 §§8.3.10, 8.3.12, S13, S6, S15, S16 |
| EC `0x13` bits 4–6 per fan | All fan profiles | mechanism | S8, S22 §9.6.2.2.16, S24 §9.6.2.2.16, S13, S6, S15, S17 |
| EC `0x0C` bits 0–2 per fan | IT8716F D, IT8718F | mechanism | S8 §9.6.2.2.13, S22 §9.6.2.2.13 |
| EC `0x51` physical-mode and external-destination bits | IT8620E, IT8628E | mechanism | S6 |
| Two identical immediate reads of each value | Fan pairs on every profile; banked-modern temperatures | safety | S5, S20 |
| Caller-held ISA mutex and at least 1.5 seconds between sample bursts | All | safety | S3, S9, S8 §9.6.3.6, S22 §9.6.3.6, S24 §9.6.3.6 |
| Temperature acceptance −55 to +125 °C (IT8620E, IT8628E) or +1 to +125 °C (banked modern) | Temperature profiles | safety | S5, S7, S19 |

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
| IT8718F | After exact raw chip ID `0x8718`, read global `CR22` before LDN selection and keep the full byte for diagnostics only. Neither its value nor a failed read selects or refuses this profile. Never write `CR22`, whose bit 7 is the two-chip configuration select. Use the same primary configuration keys, active LDN `0x04`, 12-bit aligned base format from `CR60/61`, and EC index/data offsets as IT8716F D. Read `CR27` and `CR29` with LDN `0x04` selected, before BAR discovery. Both are readable from any LDN. After the usual EC `0x00` port proof, require EC `0x58=0x90` before caching the path. | EC `0x00` bit 0 set and bits 7 and 3 clear. | S22 §§8.1, 8.3.3–8.3.5, 8.3.10, 8.3.12, 8.8.1–8.8.3, 9.6.1, 9.6.2.2.1, 9.6.2.2.35, 9.6.3.2; S23 §8.3.5; other `CR22` values S6/S7 (lead-only); S3 |
| IT8721F DX | After exact raw chip ID `0x8721`, read global `CR22` before LDN selection. Require its low nibble to be exactly `0x3`. Refuse a failed revision read or any other nibble and perform the usual primary configuration exit. Use the same primary configuration keys, active LDN `0x04`, 12-bit aligned base format from `CR60/61`, and EC index/data offsets as IT8716F D. Read `CR27` and `CR29` with LDN `0x04` selected, before BAR discovery. Both are readable from any LDN. After the usual EC `0x00` port proof, require EC `0x58=0x90` before caching the path. | EC `0x00` bit 0 set and bits 7 and 3 clear. | S24 §§8.1, 8.3.3–8.3.5, 8.3.10, 8.3.12, 8.8.1–8.8.3, 9.6.1, 9.6.2.2.1, 9.6.2.2.34, 9.6.3.2; S3 |
| IT8772E FX | After exact raw chip ID `0x8772`, read global `CR22` before LDN selection. Require its low nibble to be exactly `0x2`. Refuse a failed revision read or any other nibble and perform the usual primary configuration exit. Use the same primary keys, active LDN `0x04`, 12-bit aligned base from `CR60/61`, and EC index/data offsets as IT8716F D. Read the global pin-routing fields with LDN `0x04` selected. After the usual EC `0x00` port proof, require `EC 0x58=0x90` before caching the path. | EC `0x00` bit 0 set and bits 7 and 3 clear. | S13 §§8.1, 8.3, 8.5, 9.5.1, 9.5.2.2.1, .34 and 9.5.3.6, S3 |
| IT8620E, IT8628E | S6 describes the primary configuration keys and exact IDs, LDN `0x04` with `CR30[0]` set, and an eight-port EC block from `CR60/61`, with index/data at base + 5/+ 6. Require the raw big-endian base in `0x0100`–`0x0FF0`, bits 2:0 clear, and an address outside the PawnIO-protected PCI window. Read pin-routing fields with LDN `0x07` selected, then return to LDN `0x04` before BAR discovery. After the EC `0x00` port proof, require bit 7 clear and EC `0x58=0x90` before caching this path. | EC `0x00` bit 0 set and bits 7 and 3 clear. The bit 3 check and stricter base checks are conservative project guards. | Discovery, ports, EC vendor byte, monitor start, and EC `0x00[7]` refusal S6 (lead-only); allow-list S3/S9/S11 |
| IT8686E, IT8688E, IT8689E (banked modern) | Use the IT8620E discovery and base rules above after exact raw ID `0x8686`, `0x8688` or `0x8689`. Read global `CR22` and keep its value for diagnostics only. No revision nibble selects or refuses this profile. Read `CR27` and `CR29` with LDN `0x07` selected, then return to LDN `0x04` before BAR discovery. After the EC `0x00` port proof, require bit 7 clear. Then read EC `0x06` and require bits 7:5 clear before reading EC `0x58`, and require `0x90` before caching this path. Do not read or write LDN `0x04` `CR F3` on the runtime path. | Per sample, see [Banked modern profile](#banked-modern-profile). | Discovery, ports, routing, vendor byte and EC `0x00[7]` refusal S15/S16 (lead-only); bank field S15/S7 (lead-only); allow-list S3/S9/S11 |

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

| IT8772E FX channel | Pin routing | Firmware fan enable | Count bytes, low then high |
| --- | --- | --- | --- |
| FAN2 | `CR29[2]=0` selects tachometer input over GP52 (S13 §8.3.12). | EC `0x13[5]=1` (S13 §9.5.2.2.16, ordinal bit S6/S14, lead-only). | EC `0x0E` then `0x19` (S13 §§9.5.2.2.14, .19, read order S6, lead-only). |
| FAN3 | `CR27[7]=0` selects tachometer input over GP37 (S13 §8.3.10). | EC `0x13[6]=1` (S13 §9.5.2.2.16, ordinal bit S6/S14, lead-only). | EC `0x0F` then `0x1A` (S13 §§9.5.2.2.14, .19, read order S6, lead-only). |

For FX, the model-specific 16-bit path in S6 uses both documented count
bytes without changing or testing a counter-mode field. S13 §9.5.2.2.13
provides no width-selection field in EC `0x0C`. After the runtime gate,
read EC `0x13` and the count pairs for the eligible inputs. Restrict
sampling to these fields. (S13, S6 lead-only)

### IT8718F and IT8721F DX profiles

All EC addresses below are seven-bit indices behind `ec_base + 0x05` and
`ec_base + 0x06` (S22 §§9.6.1–9.6.2.1, p. 79; S24 §§9.6.1–9.6.2.1,
p. 83).

| Fact | IT8718F | IT8721F DX |
| --- | --- | --- |
| FAN1/FAN2/FAN3 count low bytes | EC `0x0D/0x0E/0x0F` (S22 §9.6.2.2.14, p. 86; S23 §9.6.2.2.14, p. 84) | EC `0x0D/0x0E/0x0F` (S24 §9.6.2.2.14, p. 91) |
| FAN1/FAN2/FAN3 count high bytes, bits 15:8 | EC `0x18/0x19/0x1A` (S22 §9.6.2.2.21, p. 88; S23 §9.6.2.2.21, p. 86) | EC `0x18/0x19/0x1A` (S24 §9.6.2.2.19, p. 92) |
| Firmware fan enable | EC `0x13` bits 4/5/6 must be set for FAN1/FAN2/FAN3 (S22 §9.6.2.2.16, p. 87; S23 §9.6.2.2.16, p. 84) | EC `0x13` bits 4/5/6 must be set for FAN1/FAN2/FAN3 (S24 §9.6.2.2.16, p. 91) |
| Counter width | EC `0x0C` bits 0/1/2 must be set for FAN1/FAN2/FAN3 to use the 16-bit counter. A clear bit leaves the 8-bit counter, and EC `0x0B` bits 5–0 are reserved, so S22 documents no 8-bit divisor. Omit such an input without reading its count. (S22 §§9.6.2.2.12–9.6.2.2.13, pp. 85–86; S23 §9.6.2.2.13, p. 83) | Fixed 16-bit. S24 marks the FAN1–3 16-bit counter enables in EC `0x0C` bits 0–2 as always enabled. Do not read or test EC `0x0C`. (S24 §9.6.2.2.13, p. 90) |
| FAN2 pin routing | `CR29[2]=0` selects FAN_TAC2. A set bit selects GP52. (S22 §8.3.12, p. 41; S23 §8.3.12, p. 38) | `CR29[2]=0` selects FAN_TAC2. A set bit selects GP52. (S24 §8.3.12, p. 44) |
| FAN3 pin routing | `CR27[7]=0` selects FAN_TAC3. A set bit selects GP37. (S22 §8.3.10, p. 39; S23 §8.3.10, p. 37) | `CR27[7]=0` selects FAN_TAC3. A set bit selects GP37. (S24 §8.3.10, p. 43) |
| FAN1 pin routing | Pin 7 is a dedicated tachometer input. No routing field. (S22 Table 5-4, pp. 11–12) | Pin 7 is a dedicated tachometer input. No routing field. (S24 Table 5-4, pp. 11–12) |
| Tachometer formula | `RPM = 1,350,000 / (count × divisor)`, default divisor 2, two pulses per revolution, 22.5 kHz count clock (S22 §9.6.3.5, p. 99; S23 §9.6.3.5, p. 95) | Same formula and default divisor (S24 §9.6.3.5, p. 103) |
| Divisor in 16-bit mode | No fan divisor field in EC `0x0B` (S22 §9.6.2.2.12, p. 85). The fixed divisor 2 is lead-only (S6 lines 685–687, S7 lines 491–492). | No fan divisor field in EC `0x0B` (S24 §9.6.2.2.12, p. 90). The fixed divisor 2 is lead-only (S6 lines 685–687, S7 lines 491–492). |

After the runtime gate, read EC `0x13` once. On IT8718F, also read EC
`0x0C` once and use only bits 0–2. A failed read of either byte makes
FAN1–3 unavailable for the sample. Then read the count pairs of the
eligible inputs as described in
[Read procedure and decode](#read-procedure-and-decode). Restrict
sampling to EC `0x00`, `0x13`, `0x0C` (IT8718F only) and the count bytes
above. (S22, S24; read order S6/S7, lead-only)

#### Applicability of the IT8718F documents

| Fact | Source |
| --- | --- |
| S22 applies to the C version and future versions. Its §8.3.5 assigns `0001b` to C. Its revision history records that the `CR22` default changed from `00h` to `01h` when C was added. | S22 cover, front-matter revision history, §8.3.5, p. 36 |
| S23, the earlier document, gives `CR22` the default `00h` and states the same fan fields as S22. | S23 §8.3.5, p. 34, and the sections listed in Sources |
| ITE issued separate documents for the J and K versions. Their public copies define no register and do not list a `CR22` code. | S27 |
| Neither S22 nor S23 lists a `CR22` code for any version after C. Field observations report low nibbles 2 and 4. | S22 §8.3.5; S29, S30 |
| S6 and S7 apply one FAN1–3 profile to raw `0x8718` for every `CR22` value. Each changes the counter path by revision only for raw `0x8705` and `0x8712`. | S6 lines 350–357, 2828–2831 and 3427–3448; S7 line 239 (lead-only) |
| Every primary controller that reports raw `0x8718` uses the FAN1–3 fields in the table above, whatever its `CR22` value. For `0x0` this rests on S23, for `0x1` on S22, and for every other value on the lead-only fact in the previous row together with the S22 cover. No primary source assigns a different fan map to any IT8718F version. | S22, S23; S6, S7 (lead-only) |

`CR22` is therefore not a gate on this profile. Under
[ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md)
the lead-only coverage fact backs this Experimental row and no Verified
member. (S6, S7, S22, S23)

### Banked modern profile

This profile applies to slot 0 with exact raw ID `0x8686`, `0x8688` or
`0x8689`. All facts in this subsection are lead-only and back only the
Experimental row. S19 corroborates the byte patterns on one IT8686E.

| EC index | Fact | Use in this profile | Source |
| --- | --- | --- | --- |
| `0x00` | Bit 0 starts monitoring. Bit 7 set causes the leads to refuse the block. | Require bit 0 set and bit 7 clear. Do not interpret bit 3. S19 shows `0x19`, which has bit 3 set, while its inputs change. | S15, S16; S19 |
| `0x06` | Bank field, from bit 5 upward. A non-zero bank replaces the contents of indexes `0x10`–`0xAF`. S7 masks bits 6:5 and S15 reads bits 7:5. | Read only. Require bits 7:5 clear before and after the banked reads. Never write this register. | S15, S7; S19 shows `0x00` |
| `0x58` | Vendor byte `0x90` in bank 0. | Require `0x90` within each sample after the opening bank check. | S15, S16; S19 |
| `0x13` | Bits 4, 5 and 6 enable the FAN1, FAN2 and FAN3 tachometers. | Require the bit for each fan. | S15, S17; S19 shows `0x77` |
| `0x0D`/`0x0E`/`0x0F` | FAN1/FAN2/FAN3 count low bytes. | Read low byte first. | S15, S17 |
| `0x18`/`0x19`/`0x1A` | FAN1/FAN2/FAN3 count high bytes. | Read after the matching low byte. | S15, S17 |
| `0x0C` | FAN1–FAN3 counts are always 16-bit on these models. | Not read. S19 shows bits 0–2 clear while all three 16-bit pairs hold plausible counts. | S15, S16; S19 |
| `0x29`/`0x2A`/`0x2B` | TMPIN1/TMPIN2/TMPIN3 readings, signed 8-bit, 1 °C per step. | Read each byte twice. | S15, S17, S7; S19 |
| `0x51` and bank-2 `0x1D`–`0x1E` | Together they select the source of TMPIN1–3. The bank-2 part needs a bank write. | Not read. S19 shows `0x51=0x18`, which marks no source for TMPIN3 while that input changes. EC `0x51` alone therefore cannot qualify these inputs. | S15, S17; S19 |

| Global register (LDN `0x07` selected) | Fact | Source |
| --- | --- | --- |
| `CR29[2]` | Must be clear for FAN2 to use its tachometer pin. | S15, S16 |
| `CR27[7]` | Must be clear for FAN3 to use its tachometer pin. | S15, S16 |
| — | FAN1 has no routing condition in the leads. | S15, S16 |

S15 and S16 give these models serial-bus access enables. S18 explains
that an embedded controller can use such access at any time, outside the
ISA mutex. On banked chips, that access can change the bank while
another client reads. S17 reports this for IT8665E. The leads
avoid the hazard by clearing serial-bus enables in LDN `0x04` `CR F3` and by
rewriting the bank. Both are configuration writes that this specification
excludes. This profile instead brackets its reads with bank checks and
discards inconsistent samples. (S15, S16, S17, S18)

Each sample uses the cached port proof and one ISA mutex acquisition.
Use this order:

1. Read EC `0x00`. Require bit 0 set and bit 7 clear. Otherwise the
   whole sample is unavailable.
2. Read EC `0x06`. Require bits 7:5 clear. Otherwise make no further
   banked read, report the sample as unavailable and do not retry.
3. Read EC `0x58` and require `0x90`. Otherwise the sample is
   unavailable.
4. Read EC `0x13`. A failed read makes FAN1–3 unavailable but does
   not affect the temperature inputs.
5. For each enabled fan whose routing field was clear at discovery,
   read low byte, high byte, low byte and high byte. Accept the count
   only if both pairs match.
6. Read EC `0x29`, `0x2A` and `0x2B`, each twice in immediate
   succession. Accept an input only if both bytes match.
7. Read EC `0x06` again and require bits 7:5 clear. If this read fails
   or the bank is not zero, discard every value from steps 4–6 for this
   sample.

No step waits or retries. A failed individual read invalidates only its
input. Successful siblings keep their original `FAN1`–`FAN3` and
`TMPIN1`–`TMPIN3` names. Decode counts with the table in
[Read procedure and decode](#read-procedure-and-decode). Decode a
temperature byte as signed and accept it only from +1 to +125 °C
inclusive. This range is narrower than the IT8620E rule because the
profile cannot read the source selection. S7 also treats zero and below
as no reading. In S19, an input the tool did not present reads `0xC9`
(−55 °C). The sample never reads EC `0x01`–`0x03`, `0x0C`, `0x51`, any
index at or above `0x80`, or any bank-2 register. (S5, S7, S15, S17,
S19)

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
is pinned for IT8716F D by S8 §9.6.3.5, for IT8718F by S22 §9.6.3.5
and S23 §9.6.3.5, for IT8721F DX by S24 §9.6.3.5, for IT8772E FX by S13
§9.5.3.5, for IT8620E and IT8628E by S6, and for the banked modern
profile by S15 (lead-only). The fixed 16-bit divisor is supported by
S6/S7, and by S15/S7 for the banked modern profile (lead-only). Interpret the results only after the activation,
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
| Exact `0x8728` with `CR22[3:0]=0x0`, an unlisted nibble, or an unreadable `CR22` | Exercise the existing temperature path under S3. No fan-routing, mode, or count reads are eligible. Slot-0 `CR22[3:0]=0x1` FAN1–2 cases have their own fixtures in S12. No case reads IT8728 FAN3. |
| Exact `0x8620` and separately exact `0x8628`, vendor byte `0x90`, EC `0x51=0x94`, bytes `0xC9`, `0x19`, `0x7D` | TMPIN1 unavailable (no physical input selected), TMPIN2 unavailable (external route), TMPIN3 +125 °C. If `0x51=0x15`, TMPIN1 −55 °C, TMPIN2 +25 °C, TMPIN3 +125 °C. |
| EC `0x51=0x07` with bytes `0xC8`, `0x00`, and `0x7E` | TMPIN2 at 0 °C. TMPIN1 and TMPIN3 are outside the accepted range. |
| Failed temperature selection read and valid FAN1, or failed FAN1 high byte and valid FAN2 | Preserve the successful fan under its original channel name. Mismatched count pairs leave the affected channel unavailable. |
| Exact primary FX (`0x8772`, `CR22=0x02`), active aligned EC, vendor byte `0x90`, `EC 0x00=0x11`, `EC 0x13=0x60`, both pins routed to tachometers, FAN2 bytes `0xC2/0x01` twice and FAN3 bytes `0x84/0x03` twice | FAN2 1500 RPM and FAN3 750 RPM. Under one ISA mutex, read each low/high pair twice. No FAN1, `EC 0x0C`, alarm, or high-index read. |
| Exact primary FX with `CR29[2]=1` and `EC 0x13=0x40`, or `CR29` unreadable with FAN3 eligible | FAN2 omitted before its counter reads. FAN3 retains its label and its reading. Exercise the inverse with `CR27[7]=1` or unreadable and FAN2 eligible, and each single enable bit with both pins routed. |
| Exact FX with a failed byte or mismatched count pairs on one fan and a valid sibling | Affected fan unavailable. Preserve the successful sibling. A failed `EC 0x13` read leaves both unavailable. |
| Exact FX, separately stable `0x0000`, `0x003F`, `0x0040`, `0xFFFE` and `0xFFFF` on an enabled and routed fan | Invalid without RPM, invalid without RPM, active at 10546 RPM, active at 10 RPM, and inactive at 0 RPM respectively. |
| `0x8772` with an older or unlisted revision, or `CR22` unreadable | Refuse the FX profile before LDN discovery. Perform the usual primary configuration exit. The same ID on slot 1 is outside this profile. |
| Exact FX with inactive LDN, denied BAR, failed exit, wrong vendor byte, or runtime `EC 0x00` unreadable, stopped, or in initialization | No fan count reads. A failed primary exit prevents every EC read. |
| IT8718F, synthetic: slot 0 exact `0x8718`, `CR22=0x05`, `CR27=0x00`, `CR29=0x00`, active LDN `0x04`, raw base `0x02/0x90`, successful BAR, exit and port proof, vendor byte `0x90`, `EC 0x00=0x11`, `EC 0x13=0x70`, `EC 0x0C=0x07`. FAN1 bytes `0xC2/0x01`, FAN2 `0x84/0x03` and FAN3 `0xFF/0xFF`, each pair twice | FAN1 1500 RPM, FAN2 750 RPM, FAN3 inactive at 0 RPM. EC indices at `0x0295`, data at `0x0296`. One mutex acquisition covers the sample. `CR22` is reported for diagnostics only. |
| Same IT8718F transaction with `CR22=0x00`, `0x01`, `0x02`, `0x04`, `0x81`, or a failed `CR22` read | Identical readings. `CR22` is never written and does not gate. |
| IT8718F with `EC 0x0C=0x04` and `EC 0x13=0x70` | Read and label FAN3 only. FAN1 and FAN2 are 8-bit inputs and are omitted before their count reads. |
| IT8718F with `EC 0x0C` bits 4–5 also set (`0x37`) | Same FAN1–3 eligibility as `0x07`. No read at `0x80`–`0x83`. |
| IT8718F with `CR29[2]=1`, `CR27[7]=1`, or an unreadable `CR27` or `CR29` | The affected FAN2 or FAN3 is omitted before its count reads. FAN1 keeps its name and reading. |
| IT8718F with a failed `EC 0x0C` or `EC 0x13` read | FAN1–3 unavailable for the sample. No count read. The next sample starts independently. |
| IT8718F or IT8721F DX with vendor byte other than `0x90` after the port proof | No path cached. No fan read. |
| IT8718F or IT8721F DX with `EC 0x00` equal to `0x18` (stopped, `INT_Clear` set), `0x91` (initialization set) or `0x10` (Start clear) | No fan-enable, mode or count read in the sample. |
| IT8721F DX, synthetic: slot 0 exact `0x8721`, `CR22=0x03`, routing clear, active LDN `0x04`, successful proof, vendor byte `0x90`, `EC 0x00=0x11`, `EC 0x13=0x70`, FAN1–3 pairs `0xC2/0x01`, `0x84/0x03`, `0x40/0x00` twice each | FAN1 1500 RPM, FAN2 750 RPM, FAN3 10546 RPM. No read of EC `0x0C`. |
| Exact `0x8721` with `CR22=0x01`, `0x02`, `0x04`, `0x0F`, or a failed `CR22` read | Refuse the DX profile before LDN discovery. Perform the usual primary configuration exit. No other profile applies to raw `0x8721`. |
| IT8721F DX with `EC 0x13=0x20` and both routes clear | Read and label FAN2 only. |
| IT8718F or IT8721F DX, one fan with a failed high byte or mismatched pairs and a valid sibling | Affected fan unavailable. The sibling keeps its name and reading. |
| IT8718F or IT8721F DX recorded transaction | The only writes are the slot-0 configuration keys, LDN selections through `CR07` and the configuration exit during discovery, plus EC index selections at `ec_base + 0x05`. No read of EC `0x01`–`0x03` or of any index at or above `0x80`. No write to `CR22`, `CR27`, `CR29` or EC `0x0C`. |
| Slot 1 with `0x8718` or `0x8721` | No expansion profile and no sensor reads. |
| Modern profile with `CR60/61=0x0C/0xF8`, an unaligned base, a base above `0x0FF0`, or inactive LDN | Reject before any EC index write. |
| Mutex unavailable, BAR port denied, failed primary exit, or wrong EC vendor byte on a modern profile | Reject without caching a port proof. A failed primary exit prevents every EC read. |
| Slot 0 unknown ID or neighboring `0x8622`, or slot 1 with `0x8733`, `0x8790`, `0x8695`, or `0x8883` | No expansion LDN discovery or sensor reads. The slot-1 exit-withhold rule still applies to the existing diagnostic. |
| Banked modern, synthetic from S19 bytes: slot 0 exact `0x8686`, `CR22=0x02`, base `0x0A40`, routing clear, EC `0x00=0x19`, `0x06=0x00` before and after, `0x58=0x90`, `0x13=0x77`, FAN1–3 pairs `0xAB/0x04`, `0xCB/0x04`, `0x92/0x08` twice each, TMPIN bytes `0x27`, `0x31`, `0x35` twice each | FAN1 564 RPM, FAN2 550 RPM, FAN3 307 RPM, TMPIN1 39 °C, TMPIN2 49 °C and TMPIN3 53 °C. EC `0x00` bit 3 does not block the sample. One mutex acquisition covers the sample. |
| Same transaction with `0x8688` and `CR22=0x01`, then `0x8689` and `CR22=0x02` | Identical readings. The revision is reported and does not gate. |
| Banked modern, EC `0x06` is `0x20`, `0x40`, `0x60` or `0x80` at the opening check | No read of EC `0x58`, `0x13`, counts or temperatures. All six inputs are unavailable for this sample, and no EC `0x06` write occurs. The next sample starts independently. |
| Banked modern, EC `0x06` is `0x00` at the opening check and `0x20` or unreadable at the closing check | Every fan and temperature value of this sample is discarded. |
| Banked modern, EC `0x58=0x00` after a zero bank, or EC `0x00` bit 0 clear or bit 7 set | Sample unavailable. No further banked read. |
| Banked modern, TMPIN bytes stable at `0xC9`, `0x00`, `0x80`, `0x7E`, `0x7D` and `0x01` | Unavailable, unavailable, unavailable, unavailable, +125 °C and +1 °C respectively. |
| Banked modern, TMPIN2 reads `0x31` then `0x32`, siblings stable | TMPIN2 unavailable. TMPIN1 and TMPIN3 keep their names and values. |
| Banked modern with `CR29[2]=1`, EC `0x13=0x37`, FAN1 pairs `0xFF/0xFF` | FAN2 omitted before its count reads, FAN3 omitted because its enable is clear, FAN1 inactive at 0 RPM. |
| Banked modern recorded transaction | The only writes are the slot-0 configuration keys, LDN selections through `CR07` and the configuration exit during discovery, plus EC index selections at `ec_base + 0x05`. No read of EC `0x01`–`0x03`, `0x0C`, `0x51` or any index at or above `0x80`. No access to `CR F3` and no configuration entry during sampling. |
| Slot 0 with `0x8696`, `0x8698`, `0x8665`, `0x8655`, `0x8625` or `0x8613`, or slot 1 with `0x8686` | No banked modern profile and no sensor reads. |

## Quirks

- IT8716F `CR22` bit 7 can change which chip responds in a two-chip
  configuration. Read only the revision nibble for identification and
  leave `CR22` untouched. (S8 §8.3.5, printed p. 39)
- IT8718F `CR22` bit 7 has the same two-chip configuration-select
  meaning. The IT8718F profile reads `CR22` for diagnostics only and
  never writes it. (S22 §8.3.5, p. 36)
- On IT8721F DX, S24 lists EC `0x0C` bits 0–2 as read/write but states
  that the FAN1–3 16-bit counters are always enabled. The profile
  therefore neither reads nor depends on those bits. (S24 §9.6.2.2.13,
  p. 90)
- IT8718F inputs whose EC `0x0C` bit is clear count in 8 bits, and
  S22 documents no divisor for that mode. Those inputs stay omitted
  rather than decoded with an assumed divisor. (S22 §§9.6.2.2.12–9.6.2.2.13,
  pp. 85–86)
- The two modern rows use individual chip IDs. S6 and S10 distinguish
  these exact models from neighboring IDs and identify the same fixed
  16-bit counter scheme for their scoped channels. The sources describe
  no revision-dependent path for these channels on either exact model.
  This evidence supports Experimental enablement only. The EC vendor
  byte checks the register block after chip selection. (S6/S10, lead-only)
- The banked modern leads disagree on the width of the bank field.
  S7 changes bits 6:5 and preserves bit 7. S15 reads bits 7:5. The
  profile requires all of bits 7:5 clear, which satisfies both leads.
  (S7, S15, lead-only)
- On the S19 IT8686E, EC `0x00` reads `0x19` while the inputs change.
  The bit 3 meaning pinned for IT8728F in S1 therefore cannot be
  assumed on this family. (S19)
- S7 records that IT8688E does not return the written index when its
  address port is read. No profile in this document reads the address
  port back. (S7, lead-only)
- S15 selects a memory-mapped transport for IT8688E revision 2. It also
  does so for IT8689E when global `CR24` bit 5 is set (lines 5819–5835).
  S15 still registers the I/O block as its primary window (line 6895).
  This profile uses only the I/O block. The port proof, vendor byte and
  bank checks qualify that block at runtime. (S15, lead-only)

## Deferred profiles

| Target | Disposition and evidence needed |
| --- | --- |
| IT8728F FAN3 | [#2695](https://github.com/shm11C3/HardwareVisualizer/issues/2695). Not in this document, because S12 owns the IT8728F fans. The evidence for a chip-level row exists. S1 gives FAN3 routing in `CR27[7]` (§8.3.10, printed p. 50), the enable in EC `0x13` bit 6 (§9.5.2.2.16, p. 95) and the count bytes EC `0x0F`/`0x1A` (§§9.5.2.2.14, 9.5.2.2.19, pp. 94, 96). The S12 lead-only DX fact covers the same fields (S12 S7/S8). Needed: one S12 revision that adds FAN3 under the existing `CR22[3:0]=0x1` row. No new hardware evidence is required for an Experimental row. (S1, S12) |
| IT8728F FAN1–2 outside the `CR22[3:0]=0x1` row in S12, and the DX/EX distinction | [#2695](https://github.com/shm11C3/HardwareVisualizer/issues/2695). The board limit is resolved: S12 revision 2 is chip-level. DX and EX share one version code and S12 covers both through a lead-only fact. S12 lists the evidence that would confirm DX or require an EX discriminator. S1 maps `CR22[3:0]=0x0` to AX through CX. S7/S8 of S12 give that code the same profile, but widening to it needs its own S12 revision decision. (S1 cover and §8.3.5, printed p. 48; S12) |
| IT8728F/EX, IT8716F, IT8718F and IT8721F FAN4/5 | [#2695](https://github.com/shm11C3/HardwareVisualizer/issues/2695). The register maps name `0x80`–`0x83` for FAN4/5, while the same documents describe a seven-bit writable address field. Require a corroborated high-index transaction that cannot alias the monitor or read-to-clear status registers, plus the exact pin and enable gates. (S1 §§9.5.2.1, 9.5.2.2; S8 §§9.6.2.1, 9.6.2.2.47–48; S22 §§9.6.2.1, 9.6.2.2.47–48; S24 §§9.6.2.1, 9.6.2.2.47–48) |
| IT8705F | [#2695](https://github.com/shm11C3/HardwareVisualizer/issues/2695). Deferred. S25 is a G-version vendor document, but it lists no `CR22` version code. It has per-channel 8-bit divisors in EC `0x0B` and 16-bit enables in EC `0x0C`, and does not state which divisor applies in 16-bit mode. FAN2 and FAN3 share pins with GP56 and GP57 and have alternate game-port inputs, so the IT8716F routing gate does not apply. [#2670](https://github.com/shm11C3/HardwareVisualizer/issues/2670) records unserialized OEM AML on the EC index/data pair of one IT8705F/AF board ([`superio-ite-8s661fxmp-rz-f3-ownership.md`](superio-ite-8s661fxmp-rz-f3-ownership.md)). Needed: a vendor statement or accepted dump of the G version code and the 16-bit divisor, the routing fields for both pin sets, and evidence that excludes the AML access or a decision that accepts it. (S25; S6 lines 3427–3448, lead-only) |
| IT8712F | [#2695](https://github.com/shm11C3/HardwareVisualizer/issues/2695). Deferred. No register-level IT8712F document was retrieved. S6 cites an IT8712F V0.9.1 datasheet that places 16-bit counters from version code `0x8` and reads EC `0x0B` divisors below it (lines 230–233 and 3424–3448). Needed: that vendor document, or an accepted dump, pinning the version cutoff, the 16-bit enable and count bytes, the FAN2/FAN3 routing and the divisor in each mode. (S6, lead-only) |
| IT8716F C or unlisted revisions, 8-bit mode, IT8726F | [#2695](https://github.com/shm11C3/HardwareVisualizer/issues/2695). Deferred. S8 applies only to D and later versions and does not list later version codes. Needed: a vendor document for C or the later versions, or an accepted dump, and for 8-bit inputs the active divisor. Raw `0x8726` needs its own EC field definitions. No IT8726F document was found. (S6, lead-only; S8) |
| IT8718F | [#2695](https://github.com/shm11C3/HardwareVisualizer/issues/2695). Firmware-enabled 16-bit FAN1–3 are an Experimental row in this revision. Inputs left in 8-bit mode stay omitted because S22 documents no 8-bit divisor. FAN4/5 follow the high-index row above. |
| IT8720F | [#2695](https://github.com/shm11C3/HardwareVisualizer/issues/2695). Deferred. The only IT8720F document found is an 18-page J-version brief without a register section (S28). S6 gives IT8720F the same FAN1–3 feature set and FAN2/FAN3 routing as IT8718F (lines 358–364, 3070–3119), but its FAN4/FAN5 routing sense is inverted relative to IT8718F (lines 3080–3093), so the models are not interchangeable by family. Needed: a register-level IT8720F vendor document pinning `CR22`, EC `0x0C`, EC `0x13`, the count bytes and `CR27[7]`/`CR29[2]`, or a maintainer decision to admit a lead-only row on S6/S7, preferably with a #2500 dump. (S28; S6, lead-only) |
| IT8721F other than DX | [#2695](https://github.com/shm11C3/HardwareVisualizer/issues/2695). DX FAN1–3 are an Experimental row in this revision. S24 applies only to the D version. Needed for other `CR22` values: a vendor document for that version or an accepted dump. (S24) |
| IT8771E and IT8772E outside FX FAN2–3 | [#2695](https://github.com/shm11C3/HardwareVisualizer/issues/2695). Deferred. No IT8771E document was found. IT8771E needs model-specific population and width evidence. IT8772E BX/CX, DX/EX, or unlisted versions need their own map and width evidence. FX FAN1 and additional fan inputs remain outside the vendor's two tachometer inputs. (S6 lead-only, S13 §§5, 8.3.5, 9.5) |
| IT8781F, IT8782F, IT8783E/F | [#2695](https://github.com/shm11C3/HardwareVisualizer/issues/2695). Deferred. S26 is an A-version IT8783E/F vendor document. S6 takes IT8783E/F FAN2 and FAN3 routing from different global registers than the common path (lines 2862–2885), so the routing gate of this document does not apply. No IT8781F or IT8782F document was found. Needed: the S26 routing, version and fan fields pinned in their own row, and a vendor document or accepted dump for IT8781F and IT8782F. (S26; S6, lead-only) |
| IT8613E | [#2698](https://github.com/shm11C3/HardwareVisualizer/issues/2698). Lead-only routing is now pinned: S16's later IT8613E entry (lines 521–531, 3123–3157) has no FAN1. FAN2 and FAN3 use `CR29[2]` and `CR27[7]` as on IT8620E. Its temperature map is new and it knows only three inputs. Still needed: evidence that excludes the competing firmware EC access recorded for the known platforms in [#2606](https://github.com/shm11C3/HardwareVisualizer/issues/2606) and [`superio-ite-lattepanda-sigma-linux-ec.md`](superio-ite-lattepanda-sigma-linux-ec.md), and a qualified temperature source rule for the new map. (S16, lead-only) |
| IT8625E | [#2698](https://github.com/shm11C3/HardwareVisualizer/issues/2698). Banked, with serial-bus enable bits and an IT8665E-style FAN6 location in S15 (lines 865–876). No independent dump or reported board population supports a bank-guarded read. Needed: a dump showing the bank and the FAN1–3 and TMPIN1–3 bytes on a named board, plus the routing fields in S15 lines 6212–6245. (S15, lead-only) |
| IT8655E, IT8665E | [#2698](https://github.com/shm11C3/HardwareVisualizer/issues/2698). Not admitted to the banked modern profile. S17 reports that the embedded controller on IT8665E changes the bank during its accesses. S7 writes bank 0 before every update on both models. A read-only bracket would race that controller and often find a non-zero bank. FAN2 routing uses `CR2D[4]` instead of `CR29[2]` (S15 lines 6188–6245). The `EC 0x00` bit-4 expectation also differs (S7 line 74). Needed: evidence for the competing controller's bank use, or a read path that needs no bank write. (S7, S15, S17, lead-only) |
| IT8686E, IT8688E, IT8689E FAN4–FAN6 and TMPIN4–TMPIN6 | [#2698](https://github.com/shm11C3/HardwareVisualizer/issues/2698). FAN4/FAN5 counts sit at `0x80`–`0x83`. A chip that decodes only seven index bits would alias those reads to `0x00`–`0x03`, which include the read-to-clear status group. S19 shows plausible 8-bit-index data on one IT8686E. That evidence gives no runtime proof, and IT8688E cannot confirm the index through readback (S7). FAN6 is a low-index candidate (`0x4C`/`0x4D`, enable `0x0C[2]`, routing `CR29[7]` set, S15 lines 6096–6100 and 6750–6762) that needs a dump with an enabled FAN6. TMPIN4–6 depend on EC `0x77` (S15 lines 7040–7051). Physical sources and labels need the bank-2 map. (S1 §9.5.2.1; S7, S15, lead-only; S19) |
| IT8631E, IT8638E | [#2698](https://github.com/shm11C3/HardwareVisualizer/issues/2698). Only S7 names these models, with two temperature and two fan inputs. Neither S15 nor S16 defines their discovery, bank layout, routing or serial-bus behavior. Needed: an independent source for those facts and a named-board dump. (S7, lead-only) |
| IT8696E | [#2698](https://github.com/shm11C3/HardwareVisualizer/issues/2698). The Linux WMI temperature scope in [`superio-ite-b850m-wifi6e-ice-linux-wmi.md`](superio-ite-b850m-wifi6e-ice-linux-wmi.md) is unchanged. S15 groups IT8696E with IT8689E for register locations but states no FAN2 or FAN3 routing fields for it. S16 does not list it. That specification records a thermal-zone method in one BIOS for that board that can reach fan-control writes. Needed: routing fields, a Windows-side dump on a named board, and firmware evidence about periodic AML use of the primary pair. (S15, lead-only) |
| IT8620E and IT8628E additional inputs | [#2698](https://github.com/shm11C3/HardwareVisualizer/issues/2698). FAN4/FAN5 share the high-index aliasing hazard above. S17 lists IT8620E's FAN6 count bytes in the opposite order from IT8628E and IT8686E, so its byte order is unresolved. TMPIN4–6 depend on EC `0x77`. Keep these channels separate until each model's routing, byte order and index width are established. (S6, S15, S17, lead-only) |
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
  read-to-clear behavior for this group. S13 independently specifies it
  for IT8772E FX, S22 for IT8718F and S24 for IT8721F DX. (S1
  §§9.5.2.2.2–9.5.2.2.4, printed p. 92, S13 §§9.5.2.2.2–9.5.2.2.4,
  printed pp. 73–74, S22 §§9.6.2.2.2–9.6.2.2.4, p. 84, S24
  §§9.6.2.2.2–9.6.2.2.4, p. 88)
- On IT8718F, never write EC `0x0C` to enable a 16-bit counter. Inputs
  that firmware left in 8-bit mode stay omitted. (S22 §9.6.2.2.13)
- On the banked modern discovery and sampling path, never write EC
  `0x06` and never read or write LDN `0x04` `CR F3`. Only the opt-in
  capture in [Manual compatibility evidence](#manual-compatibility-evidence)
  reads `CR F3`, once. The leads clear serial-bus access and
  select a bank there (S15, S16). Those writes change shared state that
  another controller may be using. Never read an index at or above
  `0x80` or any bank-2 register. A non-zero bank makes the sample
  unavailable instead of triggering a correction. (S7, S15, S17, S18)
- OEM AML on the inspected Gigabyte boards uses the same primary
  index/data pair under the ACPI Global Lock. PawnIO clients cannot take
  that lock. S20 observed wrong readings that matched this interleaving.
  Repeated reads apply to fan counts on every profile and to
  banked-modern temperatures. They are a consistency filter that
  reduces interleaving errors but does not eliminate them. An
  interference that persists across both reads of a value passes the
  filter. For example, firmware could select another bank before both
  count pairs are read and restore bank 0 before the closing EC `0x06`
  check. Both pairs would then match the same wrong-bank bytes, and the
  sample would accept a wrong value. S20 shows wrong readings but does
  not establish which interleavings repeated reads catch. Repeated reads
  also cannot prevent an interleaved index write from redirecting a
  firmware access. See Open questions. (S20, S21)
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
- Non-blocking for Phase 4 primary LPC expansion: the IT8716F row uses version D and the separate S12 specification owns IT8728F FAN1–2.
  The S1 DX/EX alias cannot select a single version. S12 covers both
  versions under the shared code with a lead-only fact, so no version
  selection is needed there. The S8 cover
  describes future versions without enumerating their revision codes.

- Non-blocking for Phase 4 primary LPC expansion: Experimental FX sampling uses only the two vendor-defined count pairs with model-specific fixed-width corroboration.
  S13 §9.5.3.5 describes 8- or 16-bit counting and gives divisor 2 as
  the default. The FX register table supplies both count bytes and no
  documented width selector. S6 uses a fixed 16-bit path and divisor 2
  for exact IT8772E (lead-only). An independent FX dump with paired RPM
  evidence is still needed for verification.

- Non-blocking for Phase 4 primary LPC expansion: the profile never writes the bank and discards samples whose bracket checks detect a bank change; the undetected case is a stated residual for this Experimental row.
  The profile detects a bank change only at the bracket reads. An
  embedded controller with serial-bus access can change the bank after
  the opening check and restore it before the closing check (S17,
  S18). A banked read could then return a byte from another bank at the
  same index. The double reads and the temperature range are a
  consistency filter, not a guarantee. They reject a change that falls
  between the two reads of one value, and a wrong-bank temperature
  outside +1 to +125 °C. If the other bank stays selected for both
  reads of a value, both reads return the same wrong-bank byte. The
  value then passes the filter and is published, unless it is a
  temperature outside the accepted range. No lead describes a read side effect
  at the sampled indexes in banks 1–3. S7 reads all four banks of
  IT8689E for its own dumps. Whether the serial-bus enables in LDN
  `0x04` `CR F3` are set on the target boards is unknown. The manual
  dump below records them. A dump that shows frequent bank changes
  would be grounds to withdraw the profile or require those enables to
  be clear.
- Non-blocking for Phase 4 primary LPC expansion: the exposure already applies to every approved ITE primary path; repeated reads reduce but do not eliminate its wrong readings.
  Firmware use of the shared primary pair is not specific to this
  revision. It also affects the IT8620E, IT8628E and IT8728F paths on
  Gigabyte boards. OEM `GSWApp` AML declares the primary pair `0x0A45`/
  `0x0A46` on the S21 boards. It serializes through the ACPI Global
  Lock, which PawnIO clients cannot take. S20 shows that interleaving
  produces wrong readings. It does not establish which interleavings
  the double reads here catch. The double reads reject a disturbance
  that affects only one of the two reads. A disturbance that persists
  across both reads yields a matching wrong value, which is accepted.
  Examples are a firmware index or bank selection that stays in place
  for both reads. In the banked profile, that includes another bank
  restored to bank 0 before the closing EC `0x06` check. The maintainer's
  acceptance covers this residual of occasional wrong readings. An
  interleaved index write from this client could also redirect a
  firmware data access. Thermal zones were examined on one
  S21 board, X670E AORUS MASTER F41, where `_TMP` uses only AMD
  processor sources. S20 reports, without evidence, that fan-stop SMM
  routines use the pair.
  No report attributes a misdirected firmware write to a monitoring
  client. The maintainer must explicitly accept this shared exposure
  when approving this revision. A cross-profile follow-up should
  inspect the fan-stop SMM path on one IT8686E, IT8688E or IT8689E
  firmware image.
- Non-blocking for Phase 4 primary LPC expansion: generic labels and a +1 to +125 °C range avoid any dependence on the unread source selection.
  The physical or external source of TMPIN1–3 is selected partly in
  bank 2 (S15, S17), which this profile does not read. A thermistor,
  diode, PECI or AMD-TSI source each yields a temperature byte. An
  unconfigured input in S19 reads −55 °C, and S7 treats zero and below
  as no reading. The narrower range therefore hides those values.
  Labels and source attribution need a board mapping or a dump that
  includes the bank-2 bytes.
- Non-blocking for Phase 4 primary LPC expansion: ADR 0023 lets lead-only facts back this Experimental row without an accepted dump.
  S19 is one independent IT8686E dump with tool-computed readings. No
  IT8688E or IT8689E register dump is available. Graduation needs
  maintainer-accepted dumps under [#2500](https://github.com/shm11C3/HardwareVisualizer/issues/2500)
  that cover every fact the row depends on.
- Non-blocking for Phase 4 primary LPC expansion: ADR 0023 lets the lead-only coverage fact back the Experimental IT8718F row, and no primary source assigns a different fan map to any IT8718F version.
  **IT8718F versions after C.** S22 applies to C and future versions
  but lists no code after `0x1`. ITE issued separate J and K documents
  whose public copies contain no register section (S27). Field logs
  report low nibbles 2 and 4 (S29, S30), and S6/S7 treat every value
  alike. The full J or K document, or a maintainer-accepted dump from
  a part with a marked version and its `CR22` byte, EC `0x0C`, EC
  `0x13`, raw count pairs and paired RPM, would confirm the coverage.
  If such evidence shows a different fan map for a version, that
  version's code becomes a mechanism gate and the fact moves here.
- Non-blocking for Phase 4 primary LPC expansion: an omitted 8-bit input is a safe availability outcome, and the profile never writes the counter mode.
  **IT8718F firmware counter mode.** The row reads only inputs whose
  EC `0x0C` bit is set. Whether firmware on IT8718F boards commonly
  sets these bits is unknown. S30 cannot show it, because the S6 driver
  sets them itself. A board whose firmware leaves them clear reports no
  IT8718F fan under this row. An 8-bit path would need the divisor
  that S22 does not document.
- Non-blocking for Phase 4 primary LPC expansion: the overlap arises at the pre-ID probe that every slot-0 profile already performs; the DX profile adds bounded configuration reads to the same session, and the maintainer must accept it explicitly.
  **IT8721F configuration-port ownership.** One audited Coreboot-15h
  image for the ASUS Crosshair V Formula-Z runs AML that enters ITE
  configuration mode on `0x2E`/`0x2F` under a private mutex
  ([`superio-ite-it8721f-coreboot-ownership.md`](superio-ite-it8721f-coreboot-ownership.md)).
  The host ISA mutex does not exclude that AML. The existing slot-0
  probe already enters configuration mode before it reads the chip ID.
  On raw `0x8721` the DX profile adds reads of `CR22`, `CR27` and
  `CR29`, LDN selection and the base reads before the same exit. That
  lengthens the session in which an AML transaction can interleave. The
  audited image is one firmware build. It is an evidence identity and
  cannot gate the read under ADR 0031. No report shows an actual
  Windows race. Evidence that excludes the AML access, or a pre-entry
  ownership check, would remove this residual.
- Non-blocking for Phase 4 primary LPC expansion: the exposure is the shared-pair residual already accepted for the other ITE primary paths, and repeated reads bound it the same way.
  **Firmware use of the EC pair on IT8718F and IT8721F boards.** No OEM
  ACPI or SMM image of an IT8718F or IT8721F board was audited for this
  revision. An OEM method that uses the EC index/data pair without a
  lock that PawnIO clients can take would interleave with this profile
  as described for the Gigabyte boards above. The 8S661FXMP-RZ record
  shows such unserialized AML on an earlier ITE model. An audit of one
  IT8718F and one IT8721F board firmware would narrow this residual.
- Non-blocking for Phase 4 primary LPC expansion: the row is Experimental and keyed by the reported chip ID and version code, which is the mechanism that S24 defines.
  **Other parts reporting raw `0x8721`.** S6 names IT8758E together
  with IT8721F (S6 line 28) and gives both names one profile. S24 does
  not mention IT8758E. A part that reports raw `0x8721` with
  `CR22[3:0]=0x3` is admitted whatever its marketing name. If such a
  part had different fan fields, the repeated reads and the count table
  would not detect it. A vendor document or an accepted dump for
  IT8758E would settle this.

## Readiness review

- The scoped rows name the exact model IDs, applicable revision gate,
  and register dependencies. Every applicable copyleft-only fact is attributed to S6, S7, S10, S14, S15, S16, S17 or S18 and
  confined to Experimental coverage. The IT8718F and IT8721F DX rows
  take their register fields from the vendor documents S22–S24. Their
  lead-only facts are the fixed divisor, the read order, the count
  dispositions and, on IT8718F, the coverage of later version codes. The runtime gate table tags every
  runtime condition as mechanism or safety.
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

For the IT8718F and IT8721F DX profiles, the opt-in dump also records
the full `CR22` byte, `CR27`, `CR29`, EC `0x00`, EC `0x13` and, on
IT8718F, EC `0x0C`, together with the marking printed on the package.
These are the facts that would confirm the version coverage and the
firmware counter mode in Open questions.

For the banked modern profile, the opt-in dump also records the full
`CR22` byte and the LDN `0x04` `CR F3` byte, both read during the
discovery configuration session. It records EC `0x06` at both bracket
points of at least three samples, at least 1.5 seconds apart. The
capture procedure reads `CR F3` once and never writes it, and it never
reads bank 1–3, EC `0x01`–`0x03` or any index at or above `0x80`.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-07 | Drafted exact primary-slot FAN1–3 for IT8728F/EX and IT8716F and TMPIN1–3 and FAN1–3 for IT8620E and IT8628E. Recorded deferred models and transport requirements without expanding secondary slot access. |
| 2 | 2026-11-08 | Propose Implementation-ready status for the four Experimental primary LPC rows. Pin IT8716F pin routing directly to the vendor specification, constrain modern EC base validation and authorization, and add synthetic scope, partial-result, and boundary cases. |
| 3 | 2026-11-08 | Require the vendor-defined IT8716F D revision nibble. Defer IT8728F fan reads because the E-only source shares the same revision value with DX. Retain the exact IT8620E and IT8628E model-scoped Experimental paths and add revision rejection cases. |
| 4 | 2026-11-08 | Keep the primary expansion Implementation-ready and assign the exact GA-B75M-D3H FAN1–2 profile to its separate specification. Limit the IT8728 no-fan fixture and deferred row to cases outside that profile. Retain the revision 3 primary expansion profiles and transport rules. |
| 5 | 2026-11-08 | Propose Implementation-ready status for Experimental primary IT8772E FX FAN2–3. Pin the exact revision, per-input routing and counts, and model-specific 16-bit evidence. Add partial, invalid, and authorization fixtures. |
| 6 | 2026-11-28 | Follow the chip-level S12 revision 2 (ADR 0031). The IT8728F scope row, the IT8728 no-fan fixture, the deferred row and the DX/EX open question now point to its `CR22[3:0]=0x1` FAN1–2 row instead of the GA-B75M-D3H board profile. The revision 5 profiles, gates and transport rules are unchanged. |
| 7 | 2026-11-28 | Propose Implementation-ready status for an Experimental bank-guarded TMPIN1–3 and FAN1–3 profile on exact slot-0 IT8686E, IT8688E and IT8689E ([#2698](https://github.com/shm11C3/HardwareVisualizer/issues/2698)). Add lead-only sources S15–S18, the S19 independent IT8686E dump, the S20 IT8688E firmware-interleaving observation and the S21 repository OEM evidence. Add a runtime gate table for every profile, banked fixtures, quirks, safety notes and open questions. These include the shared exposure to firmware use of the primary pair, which the maintainer must accept explicitly. Repeated reads reduce the resulting wrong readings but cannot filter an interference that persists across both reads. Give every #2698 target a disposition. The revision 6 profiles, gates and transport rules are unchanged. |
| 8 | 2026-11-28 | Propose Implementation-ready status for two Experimental fan rows ([#2695](https://github.com/shm11C3/HardwareVisualizer/issues/2695)): firmware-enabled 16-bit FAN1–3 on exact slot-0 IT8718F with `CR22` for diagnostics only, and FAN1–3 on exact slot-0 IT8721F DX (`CR22[3:0]=0x3`) with always-enabled 16-bit counters. Add the ITE datasheets S22–S24 as primary sources for their register fields, S25–S28 for dispositions and S29–S30 as IT8718F evidence identities. Extend S6/S7 with the matching lead-only line pins. Add the IT8718F applicability facts, gate rows, discovery rows, the register table, synthetic fixtures, quirks, safety notes and open questions. These include the IT8721F configuration-port overlap of one Coreboot image, which the maintainer must accept explicitly. Give every #2695 target a disposition and move those rows from #2518 to #2695. The revision 7 profiles, gates and transport rules are unchanged. |
