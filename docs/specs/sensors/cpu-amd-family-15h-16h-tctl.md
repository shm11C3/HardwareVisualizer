# Spec: AMD Family 15h/16h CPU control temperature (Tctl) via AMDFamily10

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Windows CPU control temperature (Tctl) on pre-Zen AMD Family 15h models 00h–0Fh, 10h–1Fh, 30h–3Fh, 60h–6Fh and 70h–7Fh, and Family 16h models 00h–0Fh and 30h–3Fh, read through the signed PawnIO `AMDFamily10` module. Two register maps: `D18F3xA4` (map A) and the SMU-indirect `D0F0xBC_xD820_0CA4` (map B). One physical package only. Excludes: Family 10h–14h (#2512) and 0Fh (#2513), per-core or per-node temperatures, multi-package systems, SB-TSI, any use of the `RyzenSMU` module on Family 15h, and every write, control or frequency-measurement surface of the module. |
| Issue phase | #2511 (epic #2510) |

## Sources

No copyleft monitoring implementation was used. Every register fact
below rests on an AMD document; every access fact rests on the PawnIO
project's own published interface definitions or release artifacts.

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD, *BIOS and Kernel Developer's Guide (BKDG) for AMD Family 15h Models 00h-0Fh Processors*, publication 42301 Rev 3.14 (January 23, 2013): §2.8 p. 93 (configuration space, device 18h = node 0); §2.11.1 pp. 219–220 (Tctl scale); D18F3x00 p. 374; D18F3xA4 pp. 393–394; CPUID Fn0000_0001_EAX p. 476 | Primary. Fetched from `https://www.amd.com/content/dam/amd/en/documents/archived-tech-docs/programmer-references/42301_15h_Mod_00h-0Fh_BKDG.pdf` on 2026-11-28, SHA-256 `5948b5827cb8752f3151a8b581c1a362d2f7155879919ee78970980e155fe0b3` |
| S2 | AMD, *BKDG for AMD Family 15h Models 10h-1Fh Processors*, publication 42300 Rev 3.12 (July 14, 2015): §2.8 p. 80; §2.10.1 p. 136; D18F3x00 p. 419; D18F3xA4 pp. 432–433; CPUID Fn0000_0001_EAX p. 493; §1.5.3 (revision RL-A1) | Primary. Same archive path, file `42300_15h_Mod_10h-1Fh_BKDG.pdf`, SHA-256 `05e174bb2772a1da8d4abc15ea05a64f8563e8c03a4347b70083c5586cc26dc4` |
| S3 | AMD, *BKDG for AMD Family 15h Models 30h-3Fh Processors*, publication 49125 Rev 3.06 (February 10, 2015): §2.8 p. 88; §2.10.1 p. 145; D18F3x00 p. 442; D18F3xA4 pp. 459–460; CPUID Fn0000_0001_EAX p. 535 | Primary. File `49125_15h_Models_30h-3Fh_BKDG.pdf`, SHA-256 `35ecdc7f6eecc85ce86390a86f85aadaea2acf79290299c98b1b99730de96b27` |
| S4 | AMD, *BKDG for AMD Family 15h Models 60h-6Fh Processors*, publication 50742 Rev 3.05 (May 21, 2016): §2.8 p. 80; §2.10.1 p. 154; D0F0xB8 / D0F0xBC p. 233; D0F0xBC_xD820_0CA4 pp. 238–239; §3.13 D18F3 register list (D18F3xA0 is followed by D18F3xA8); D18F3x00 p. 478; CPUID Fn0000_0001_EAX p. 600 | Primary. File `50742_15h_Models_60h-6Fh_BKDG.pdf`, SHA-256 `3cc06aa632c669d042a588892ed890498731d256b015e7746d4036a15c979888` |
| S5 | AMD, *BKDG for AMD Family 15h Models 70h-7Fh Processors*, publication 55072 Rev 3.09 (June 20, 2018): §2.8 p. 78; §2.10.1 p. 143; D0F0xB8 / D0F0xBC p. 212; D0F0xBC_xD820_0CA4 pp. 216–217; §3.13 D18F3 register list; D18F3x00 p. 451; CPUID Fn0000_0001_EAX p. 565 | Primary. File `55072_AMD_Family_15h_Models_70h-7Fh_BKDG.pdf`, SHA-256 `818e9cd3dcf32c052be6aff1689dd79401b8a1dd1950d43ad2ac40b32d1ce3d7` |
| S6 | AMD, *BKDG for AMD Family 16h Models 00h-0Fh Processors*, publication 48751 Rev 3.03 (February 19, 2015): §2.8 p. 80; §2.10.1 p. 141; D18F3x00 p. 395; D18F3xA4 pp. 412–413; CPUID Fn0000_0001_EAX p. 467 (family and model formulas) | Primary. File `48751_16h_bkdg.pdf`, SHA-256 `0bdc6eaaef6427877a73f0e0f5c335f2e4b774c2872930dc8ab3502d8d7a4737`. The publicly hosted file carries an "AMD Internal-Only Specification / AMD Confidential - Advance Information" page footer |
| S7 | AMD, *BKDG for AMD Family 16h Models 30h-3Fh Processors*, publication 52740 Rev 3.06 (March 18, 2016): §2.8 p. 82; §2.10.1 p. 145; D18F3x00 p. 437; D18F3xA4 pp. 454–455; CPUID Fn0000_0001_EAX p. 529 | Primary. File `52740_16h_Models_30h-3Fh_BKDG.pdf`, SHA-256 `1aaeb108fe91fc8ca1fe2bfe02511ac90bbd1fa5e84b4d098e2179e1ef421a7b` |
| S8 | AMD Revision Guides: 48063 Rev 3.24 (September 2014, Family 15h 00h–0Fh) Tables 2–4 pp. 9–10; 48931 Rev 3.08 (August 2012, 15h 10h–1Fh) Tables 2–4 pp. 9–10; 51603 Rev 1.06 (April 2014, 15h 30h–3Fh) Tables 2–3 pp. 9–10; 51810 Rev 3.06 (June 2013, 16h 00h–0Fh) Table 2 p. 9; 53072 Rev 3.00 (April 2014, 16h 30h–3Fh) Table 5 p. 12; 55370 Rev 3.00 (July 2016, 15h 70h–7Fh) Table 2 p. 11 | Primary. Fetched 2026-11-28 from `https://www.amd.com/content/dam/amd/en/documents/archived-tech-docs/revision-guides/` (file names `48063_15h_Mod_00h-0Fh_Rev_Guide.pdf`, `48931_15h_Mod_10h-1Fh_Rev_Guide.pdf`, `51603_Rev_Guide_15h_Models_30h-3Fh.pdf`, `51810_16h_00h-0Fh_Rev_Guide.pdf`, `53072_Rev_Guide_16h_Models_30h-3Fh.pdf`, `55370_Rev_Guide_For_Family_15h_Models_70h-7Fh_Processors.pdf`). Used for CPUID evidence identities and the errata review. Revision Guide 53564 (15h 60h–6Fh) could not be obtained; see Open questions |
| S9 | PawnIO.Modules `AMDFamily10.p` (LGPL-2.1-or-later) at tag `0.2.11` (commit `52a7e536dff3e53c96917a28caac5e0fa6510696`); last changed by commit `57648839a70cb3842330766e193cbfff326f8c76` (2026-07-27); byte-identical at tag `0.2.12` (commit `40a95f0c812b09f78e05b2d5aa4614cbcef67b0a`) | Upstream-published interface definition of the module this project calls: `ioctl_*` names, cell counts, allow-lists, family gate, caller-mutex `@warning` docs, load-time behavior. Not used for any hardware register fact. No code copied. Contract recorded in [`pawnio-interface.md`](pawnio-interface.md) rev 8 |
| S10 | PawnIO.Modules `RyzenSMU.p` at tag `0.2.11` (byte-identical at `0.2.12`) | Interface definition; used only to explain why `RyzenSMU` is not the access path here |
| S11 | PawnIO.Modules release `0.2.11`, asset `release_0_2_11.zip` (69,582 bytes, SHA-256 `43608cb89bc84247fef1368a139013f7d043e17db6d6c8dfc9b46bf0905a81f4`): contains signed `AMDFamily10.bin`, 16,868 bytes, SHA-256 `6443080b2968474ffbc38aa4356cc56f9664349fa4b917afdb33027d6bb50525`. Release `0.2.12` (published 2026-11-17), asset `release_0_2_12.zip` (83,556 bytes, SHA-256 `09698897ce19b92f1043a2e322cfcf0b1fe823e0b576971b178a65244fb0af9d`) contains a byte-identical `AMDFamily10.bin` | Primary (signed distribution). Downloaded and hashed 2026-11-28 |
| S12 | PawnIO driver source `PawnIO/src/natives_impl_windows.cpp` at PawnIO tag `2.2.0` (commit `5cdf470831fdfff3f7f1d06363ca6b230f3bf35a`; the PCI read routine is unchanged at commit `079c43431148631b894a09d6b38ee0cfc5001e8b`) | Driver interface fact: module PCI configuration reads go through the Windows HAL bus-data routine one access at a time, and a function whose vendor ID reads as invalid returns `STATUS_DEVICE_DOES_NOT_EXIST`. No code copied |
| S13 | PawnIO.Modules issue #32, "Amd10Cpu freeze", <https://github.com/namazso/PawnIO.Modules/issues/32> (open at 2026-11-28) | Upstream user and contributor reports: whole-system freezes on FX-6300, FX-8350, FX-9590 and A8-7600 systems, attributed by a contributor to `ioctl_measure_tsc_multiplier`; the module owner notes that this ioctl waits for a tick-count change after disabling interrupts. Used only for a safety rule (never invoke); not a register fact |
| S14 | Microsoft Learn, `GetLogicalProcessorInformationEx` and `LOGICAL_PROCESSOR_RELATIONSHIP` (`RelationProcessorPackage`), <https://learn.microsoft.com/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex> | OS API documentation for the package-count gate |

## Access path (facts)

The PawnIO modules this project already ships cannot read these
registers:

- `AMDFamily17` loads only on families `0x17`–`0x1A`. (pawnio-interface.md, `AMDFamily17`)
- `RyzenSMU` recognizes family `0x15` only for the exact models `0x60`,
  `0x65` and `0x70`, and rejects every other family `0x15` model at load.
  Its SMN read allow-list (`0x3B10000`–`0x3B10FFF`, `0x56000`–`0x5AFFF`,
  `0x6F000`–`0x6FFFF`, and on those three models `0x13000000`–`0x130000F0`,
  an MMIO mailbox page and `0xC0100000`–`0xC010FFFF`) does not contain
  `0xD8200CA4`. Its load on those models also searches for an integrated
  GPU at PCI `00:01.0` or `01:00.0` and maps one page of that device's
  BAR5; load fails if no matching device is found. (S10)
- `RyzenSMU` recognition of models `0x60`/`0x65`/`0x70` is therefore an
  access lead only. The Zen SMN Tctl register (`0x00059800`) is a Family
  17h/19h fact ([`cpu-amd-zen-smn.md`](cpu-amd-zen-smn.md)); no AMD
  document consulted here defines it for Family 15h.

The access path for this document is the signed `AMDFamily10` module
(S9, S11). Its contract is in
[`pawnio-interface.md`](pawnio-interface.md) (section `AMDFamily10`).
The two operations used:

| Function | Input cells | Output cells | Used for |
| --- | --- | --- | --- |
| `ioctl_read_miscctl` | `in[0]` = node index (the module reads PCI bus 0, device `0x18 + in[0]`, function 3), `in[1]` = offset | `out[0]` = 32-bit value | Map A and the identity read; offsets `0x00` and `0xA4` only |
| `ioctl_read_smu` | `in[0]` = SMU address, must be `0xD8200CA4` | `out[0]` = 32-bit value | Map B |

## Detection

Every runtime condition is a mechanism gate (ADR 0031). No brand string,
firmware version or board identity is read.

| Condition | Gate kind | Source |
| --- | --- | --- |
| CPUID leaf 0 vendor is `AuthenticAMD` | mechanism | S1–S7 (CPUID); S9 (module rejects other vendors) |
| Effective family = `BaseFamily + ExtendedFamily` and model = `{ExtendedModel, BaseModel}` from CPUID.01H:EAX. Family `0x15` with model `0x00`–`0x0F`, `0x10`–`0x1F` or `0x30`–`0x3F`, or family `0x16` with model `0x00`–`0x0F` or `0x30`–`0x3F`, selects **map A** | mechanism | S1, S2, S3, S6, S7 (document titles and CPUID Fn0000_0001_EAX pages) |
| Family `0x15` with model `0x60`–`0x6F` or `0x70`–`0x7F` selects **map B** | mechanism | S4, S5 |
| `GetLogicalProcessorInformationEx(RelationProcessorPackage)` succeeds and yields exactly one physical package | mechanism (node 0 is the only node attributed to a package; see Read procedure step 3) | S1 §2.8, S14 |
| `pawnio_load` of the signed `AMDFamily10.bin` succeeds (the module rejects non-x64, non-AMD and family outside `0x10`–`0x16`) | mechanism (access path present) | S9, S11 |
| `ioctl_read_miscctl(0, 0x00)` returns exactly the D18F3x00 value of the selected group (table below) | mechanism (device identity register) | S1–S7 D18F3x00, S9 |
| After each read: bits 17:16 of the register equal `00b` | mechanism (the register's own select field); post-read check, not an admission gate | S1–S7 |

| Group | CPUID family / model | Map | D18F3x00 32-bit value (DeviceID 31:16, VendorID 15:0) | Source |
| --- | --- | --- | --- | --- |
| 15h 00h–0Fh | `0x15` / `0x00`–`0x0F` | A | `1603_1022h` | S1 p. 374 |
| 15h 10h–1Fh | `0x15` / `0x10`–`0x1F` | A | `1403_1022h` | S2 p. 419 |
| 15h 30h–3Fh | `0x15` / `0x30`–`0x3F` | A | `141D_1022h` | S3 p. 442 |
| 15h 60h–6Fh | `0x15` / `0x60`–`0x6F` | B | `1573_1022h` | S4 p. 478 |
| 15h 70h–7Fh | `0x15` / `0x70`–`0x7F` | B | `15B3_1022h` | S5 p. 451 |
| 16h 00h–0Fh | `0x16` / `0x00`–`0x0F` | A | `1533_1022h` | S6 p. 395 |
| 16h 30h–3Fh | `0x16` / `0x30`–`0x3F` | A | `1583_1022h` | S7 p. 437 |

All seven device IDs are on the module's `ioctl_read_miscctl`
allow-list (S9).

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| Map A: `D18F3xA4` on node 0 through `ioctl_read_miscctl`, groups 15h 00h–0Fh, 10h–1Fh, 30h–3Fh, 16h 00h–0Fh, 16h 30h–3Fh, one physical package, identity read matched | Verified: register, fields, scale, select field and Tctl semantics from S1, S2, S3, S6, S7; access contract from S9, S11, S12. No lead-only fact. Revision-guide errata review (S8) found no erratum naming the reported-temperature register for 15h 00h–0Fh, 10h–1Fh, 30h–3Fh, 16h 00h–0Fh or 16h 30h–3Fh. Evidence identities (not gates) from S8: OR-B2 `00600F12h` and OR-C0 `00600F20h` (AMD FX, Opteron 3200/3300 on AM3r2; Opteron 4200/4300 C32r1 and 6200/6300 G34r1 are multi-package-capable server parts and are covered only in single-package systems); TN-A1 `00610F01h` (A-Series, R-Series, FirePro APU); KV-A1 `00630F01h`; KB-A1 `00700F01h` (A-, E-, G-Series, Opteron X1100/X2100); ML-A1 `00730F01h`. Revisions inside the documented model ranges without a revision-guide entry here (for example RL-A1, named in S2 §1.5.3) share the map. No hardware observation by this project yet | Enabled (Verified), with the identity, select-field and plausibility gates |
| Map B: `D0F0xBC_xD820_0CA4` through `ioctl_read_smu` under `Global\Access_PCI`, groups 15h 60h–6Fh and 70h–7Fh, one physical package, identity read matched | Verified: register, fields, scale, select field, Tctl semantics and the D0F0xB8/D0F0xBC index protocol from S4, S5; absence of D18F3xA4 on these groups from the S4/S5 register lists; access contract from S9, S11, S12. No lead-only fact. Errata review: S8 (55370) found no erratum naming the register for 70h–7Fh; the 60h–6Fh revision guide (53564) was not obtained (Open questions). Evidence identity (not a gate) from S8: ST-A0 `00670F00h`. No hardware observation by this project yet | Enabled (Verified), with the identity, select-field and plausibility gates |
| Family `0x15` models `0x20`–`0x2F`, `0x40`–`0x5F`, `0x80`–`0xFF`; family `0x16` models `0x10`–`0x2F`, `0x40`–`0xFF` | No AMD document consulted defines a register map for these models (ADR 0031 rule 3) | Unsupported |
| More than one physical package (for example dual-socket Opteron 4200/4300/6200/6300) | Node-to-package attribution is not specified: the module accepts a node index but no package selector, and this document states no node-numbering fact for multi-package systems | Unsupported; follow-up under #2511 |
| Family `0x10`–`0x14` | Out of scope here (#2512), although the same module loads on them | Unsupported by this document |
| Any Family 15h/16h read through `RyzenSMU` or the Zen SMN decode | Not a documented map for these families (Access path) | Unsupported |

## Register map (facts)

Map A — PCI configuration space, bus 0, device `18h` (node 0),
function 3, offset `A4h` (S1 §2.8; S1–S3, S6, S7 D18F3xA4):

| Address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| D18F3x00 | Device/Vendor ID | 31:16, 15:0 | DeviceID (per group table), VendorID `1022h` | read-only value | S1–S7 |
| D18F3xA4 | Reported Temperature Control, `CurTmp` | 31:21 | "Provides the current control temperature, Tctl, after the slew-rate controls have been applied." Read-only, updated by hardware, when `CurTmpTjSel` is not `11b`; read-write when it is `11b` | With `CurTmpTjSel` ≠ `11b`: Tctl = `CurTmp` × 0.125 (000h = 0, 7FFh = 255.875) | S1 pp. 393–394, S2 pp. 432–433, S3 pp. 459–460, S6 pp. 412–413, S7 pp. 454–455 |
| D18F3xA4 | `CurTmpTjSel` | 17:16 | "Current temperature select." Reset 00b; "may be used for diagnostic software". `00b`: CurTmp provides the read-only Tctl value. `01b`, `10b`: Reserved. `11b`: CurTmp is a read-write register that specifies a value used to create Tctl; its two LSBs read as zero | select field | same pages |
| D18F3xA4 | slew controls | 12:8, 7, 6:5, 4:0 | PerStepTimeDn, TmpSlewDnEn, TmpMaxDiffUp, PerStepTimeUp (read-write, programmed by BIOS) | not decoded | same pages |
| D18F3xA4 | Reserved / other | 20:18, 15:13 | Reserved (S1, S3, S6, S7); on S2 bit 20 is TcenPwrDnCc6En and 19:18 Reserved | not decoded | same pages |

Map B — SMU indirect space through the host bridge index/data pair
(S4 p. 233, S5 p. 212): write the 32-bit SMU address to D0F0xB8
(`NbSmuIndAddr`, PCI bus 0 device 0 function 0 offset `B8h`), then read
D0F0xBC (`NbSmuIndData`, offset `BCh`).

| Address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| D0F0xBC_xD820_0CA4 | Reported Temperature Control, `CUR_TEMP` | 31:21 | "Provides the current control temperature, Tctl, after the slew-rate controls have been applied." Read-only, updated by hardware, unless `CUR_TEMP_TJ_SEL` is `11b` | With `CUR_TEMP_TJ_SEL` ≠ `11b`: Tctl = `CUR_TEMP` × 0.125 (000h = 0, 7FFh = 255.875) | S4 pp. 238–239, S5 pp. 216–217 |
| D0F0xBC_xD820_0CA4 | `CUR_TEMP_TJ_SEL` | 17:16 | Same encoding as `CurTmpTjSel` above | select field | same pages |
| D0F0xBC_xD820_0CA4 | slew controls | 12:8, 7, 6:5, 4:0 | PER_STEP_TIME_DN, TMP_SLEW_DN_EN, TMP_MAX_DIFF_UP, PER_STEP_TIME_UP | not decoded | same pages |
| D0F0xBC_xD820_0CA4 | Reserved | 20, 19:18, 15:13 | Reserved | not decoded | same pages |

Notes:

- On 15h 60h–6Fh and 70h–7Fh the D18F3 register list has no D18F3xA4
  (D18F3xA0 is followed by D18F3xA8); Tctl is defined only at
  D0F0xBC_xD820_0CA4. (S4, S5 §3.13)
- For `CurTmpTjSel` = `11b` the documents give an encoding offset by
  −49 (S1: `CurTmp[10:2]` × 0.5 − 49; S2, S3, S6, S7, S4, S5:
  `CurTmp` × 0.125 − 49). This document never decodes that case; see
  Read procedure.
- S1 (multi-node parts, `D18F3xE8[MultiNodeCpu]` = 1): the field
  should be read on internal node 0 to obtain the processor's
  temperature; internal node 1 gives only its own temperature.
  (S1 p. 393)

## Read procedure and decode

1. Evaluate the admission gates in Detection. Select map A or B from
   family and model.
2. Load `AMDFamily10.bin` on its own executor handle. A load failure
   leaves CPU temperature unavailable from this path (fallback as for
   other PawnIO paths).
3. Identity read, once per opened handle: `ioctl_read_miscctl` with
   `in[0]` = 0, `in[1]` = `0x00`. Take the low 32 bits of `out[0]`. It
   must equal the group's D18F3x00 value. On mismatch or error, do not
   read further on this handle and report the path unavailable. Node 0
   is the only node this document reads.
4. Read the register:
   - Map A: `ioctl_read_miscctl` with `in[0]` = 0, `in[1]` = `0xA4`.
     The module documents no caller mutex for this function (S9); the
     driver performs it as one HAL configuration read (S12). No mutex
     is required.
   - Map B: acquire `Global\Access_PCI` (bounded timeout;
     open-before-create, per pawnio-interface.md), call
     `ioctl_read_smu` with `in[0]` = `0xD8200CA4`, release the mutex.
     On timeout, skip the sample.
   Take the low 32 bits of `out[0]` as `raw`.
5. Decode:
   - If `raw` is `0x00000000` or `0xFFFFFFFF`, the sample is invalid
     (project policy: failed transaction).
   - `select` = bits 17:16 of `raw`. If `select` ≠ `00b`, the sample is
     unavailable: `01b`/`10b` are Reserved encodings, and `11b` means
     the field holds a software-specified value rather than the
     hardware-updated Tctl. (S1–S7)
   - `CurTmp` = bits 31:21 of `raw` (unsigned, 0–2047).
   - Tctl = `CurTmp` × 0.125, on the Tctl scale (0 to 255.875).
     (S1–S7)
6. Plausibility gate (project policy, matching the Zen path): publish
   only 0 ≤ Tctl ≤ 120; drop anything else.
7. Publish Tctl as the CPU control temperature. Apply no offset and do
   not derive a die temperature from it.

### Meaning and label

- "Tctl is a processor temperature control value used for processor
  thermal management... Tctl is a temperature on its own scale aligned
  to the processors cooling requirements. Therefore Tctl does not
  represent a temperature which could be measured on the die or the
  case of the processor. Instead, it specifies the processor
  temperature relative to the maximum operating temperature,
  Tctl_max." Above Tctl_max the part is (Tctl − Tctl_max) over its
  maximum operating temperature; below, (Tctl_max − Tctl) under it.
  Tctl_max is specified in the power and thermal data sheet. (S1
  §2.11.1; S2–S7 §2.10.1)
- The value is slew-filtered ("after the slew-rate controls have been
  applied"), so it lags the measured temperature. (S1–S7)
- Required presentation: the reading is a control temperature. Its
  label must name it as Tctl or a control temperature (for example
  "CPU Tctl"), never as a die, case, or calibrated package
  temperature, and it must not be classified with independently
  calibrated die readings. Low idle values are expected on this scale
  and are not an error. This document provides no Tctl_max, so no
  "distance to limit" may be derived.

## Quirks

- Map choice depends on the model group: 15h 60h–7Fh have no
  D18F3xA4 and use the SMU-indirect register. The module's
  `ioctl_read_miscctl` allow-list still accepts offset `0xA4` for
  device IDs `1573h`/`15B3h` (S9); the implementation must not use it
  on those groups. (S4, S5, S9)
- `ioctl_read_smu` returns `STATUS_NOT_SUPPORTED` unless family is
  `0x15` and model bits 7:4 are `6h` or `7h`, and
  `STATUS_ACCESS_DENIED` for any address other than `0xD8200CA4`.
  (S9)
- On 15h 00h–0Fh multi-node parts (G34r1, `MultiNodeCpu` = 1), node 0
  reports the processor temperature. (S1 p. 393; S8 48063 Table 2
  places Opteron 6200/6300 on G34r1)

## Safety notes

- Operations used: `pawnio_load`, `ioctl_read_miscctl` (offsets
  `0x00` and `0xA4` only, node index 0 only) and `ioctl_read_smu`
  (address `0xD8200CA4` only). Nothing else in the module is invoked.
- Never invoke `ioctl_measure_tsc_multiplier`: it disables interrupts,
  writes MSRs `C001_0015h` (HWCR), `C001_0000h` and `C001_0004h` and
  spins on a tick count (S9); upstream reports whole-system freezes on
  FX and Kaveri systems attributed to it (S13). Never invoke
  `ioctl_read_cstate_residency`: it writes I/O port `0xCD6` and
  documents no mutex (S9). `ioctl_read_msr` and offset `0xD4` are not
  needed.
- Map B performs one write that the read requires: the module writes
  the SMU address to D0F0xB8 before reading D0F0xBC, and does not
  restore the previous index. Hold `Global\Access_PCI` around each
  call, as the module's `@warning` requires (S9). D0F0xB8/D0F0xBC is
  the same index pair `RyzenSMU` uses on its family `0x15` models
  (S10).
- Load-time effects of `AMDFamily10` (S9): reads CPU vendor, family and
  model; emits a debug print; reads the dword at PCI `00:14.0` offset 0
  and, only when it equals `4385_1002h`, the byte at offset `08h`. No
  write, mapping, MSR access or interrupt change happens at load. The
  module defines no unload routine.
- Never write D18F3xA4 or D0F0xBC_xD820_0CA4. Setting
  `CurTmpTjSel` = `11b` would replace the hardware Tctl with a
  software value (S1–S7).

## External component requirements

This document adds a fifth module to the four the repository installs
today (`IntelMSR.bin`, `RyzenSMU.bin`, `AMDFamily17.bin`, `LpcIO.bin`).
Installing those four does not make `AMDFamily10.bin` available. Through
the existing external-component owner
([`docs/design/external-component-setup.md`](../../design/external-component-setup.md),
ADR 0024, ADR 0026, ADR 0029), an implementation must:

- obtain `AMDFamily10.bin` from the already pinned
  `release_0_2_11.zip` (S11): 16,868 bytes, SHA-256
  `6443080b2968474ffbc38aa4356cc56f9664349fa4b917afdb33027d6bb50525`;
- add that file name and hash to setup detection, placement and
  outdated-module refresh, and to the sensor service's set of known
  signed module hashes;
- report a missing or unrecognized `AMDFamily10.bin` as its own
  missing-module state, distinct from the other four, and fall back as
  for other PawnIO paths;
- keep the LGPL `COPYING` notice already carried for the archive.

A later move to release `0.2.12` does not change this module's bytes
(S11); other modules in that release changed and need their own
re-verification.

## Open questions

- Non-blocking for #2511: Revision Guide 53564 (Family 15h 60h–6Fh) could not be obtained, so no errata review exists for that group; reads are read-only and plausibility-gated, and an erratum would change accuracy, not safety. Fetch 53564 and record the review in a revision.
- Non-blocking for #2511: whether any firmware leaves `CurTmpTjSel` at `11b` at runtime is unknown; this document reports such samples as unavailable, which is truthful. If Compatibility Reports (#2500) show that state, a revision decides from primary evidence whether any value can be published.
- Non-blocking for #2511: multi-package systems are Unsupported. Supporting them needs a stated node-to-package mapping and a module input that can address each package's node 0.
- Non-blocking for #2511: whether other agents on Windows (firmware SMM, AMD graphics or chipset drivers) use the D0F0xB8/D0F0xBC index pair without `Global\Access_PCI` is undocumented. The mutex covers cooperating monitors only; the same limit already applies to the `RyzenSMU` family `0x15` path.
- Non-blocking for #2511: no hardware observation exists for either map. A user Compatibility Report or a maintainer-accepted register dump per group should corroborate the identity read and a plausible Tctl.
- Non-blocking for #2511: whether setup installs `AMDFamily10.bin` on every machine or only on Family 15h/16h CPUs is a design decision of the external-component owner, not a hardware fact.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-28 | Initial version. Map A (`D18F3xA4`) and map B (`D0F0xBC_xD820_0CA4`) for Family 15h 00h–0Fh, 10h–1Fh, 30h–3Fh, 60h–6Fh, 70h–7Fh and 16h 00h–0Fh, 30h–3Fh, from AMD BKDGs 42301, 42300, 49125, 50742, 55072, 48751, 52740 and six revision guides. Access through signed `AMDFamily10.bin` from PawnIO.Modules 0.2.11. Proposed as Implementation-ready (rev 1); the maintainer's PR approval is the sign-off. |
