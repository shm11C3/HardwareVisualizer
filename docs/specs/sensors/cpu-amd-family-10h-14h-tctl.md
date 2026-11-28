# Spec: AMD Family 10h/11h/12h/14h CPU control temperature (Tctl) via AMDFamily10

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Windows CPU control temperature (Tctl) on pre-Zen AMD Family 10h (all models), 11h (all models), 12h (all models) and 14h models 00h–0Fh, read from `D18F3xA4` on node 0 through the signed PawnIO `AMDFamily10` module. One physical package only. Family 10h carries erratum 319 safety gates. Excludes: Family 13h, Family 14h models 10h and above, Family 10h Socket F (1207), Family 10h G34 dual-node parts, Family 15h/16h (owned by [`cpu-amd-family-15h-16h-tctl.md`](cpu-amd-family-15h-16h-tctl.md)), Family 0Fh (#2513), per-core or per-node temperatures, multi-package systems, SB-TSI, and every write, control or frequency-measurement surface of the module. |
| Issue phase | #2512 (epic #2510) |

## Sources

Every register fact below rests on an AMD document. Every access fact
rests on the PawnIO project's own published interface definitions or
release artifacts. Every OS fact rests on Microsoft or DMTF
documentation. The one copyleft lead (L1) is used only in Open
questions. No fact row and no scope row depends on it.

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD, *BIOS and Kernel Developer's Guide (BKDG) for AMD Family 10h Processors*, publication 31116 Rev 3.62 (January 11, 2013): §2.6.4.2.2 p. 85 (NodeId enumeration for dual-node processors); §2.7.1.5 p. 97 (G34 dual-node); §2.10.1 p. 162 (Tctl scale); §2.11 p. 164 (bus 0, devices 24–31 = nodes 0–7); §2.11.3 p. 165 (unimplemented functions read all F's); §3.1 p. 200 (`FYxXXX` mnemonic, one register set per node); F2x[1, 0]94 `Ddr3Mode` pp. 252–255; F3x00 p. 287; F3xA4 pp. 320–321; F3xE8 `IntNodeNum`/`MultiNodeCpu` p. 330; CPUID Fn[8000_0001, 0000_0001]_EAX p. 386; CPUID Fn8000_0001_EBX `PkgType` p. 387 | Primary. Fetched from `https://www.amd.com/content/dam/amd/en/documents/archived-tech-docs/programmer-references/31116.pdf` on 2026-11-28, SHA-256 `80554786942e004cc15c2f27231c0df309aa4961bcb3b1a81b4bc801ea590780` |
| S2 | AMD, *BKDG for AMD Family 11h Processors*, publication 41256 Rev 3.00 (July 07, 2008): §2.10.1 p. 73; §2.11 p. 75 (configuration space at bus 0 device 24); §3.1 p. 93 (`FYxXXX` mnemonic); F3x00 p. 140; F3xA4 pp. 156–157; CPUID Fn0000_0001_EAX p. 198; CPUID Fn8000_0001_EBX p. 200 (`PkgType` = 2h, S1g2) | Primary. Same archive path, file `41256.pdf`, SHA-256 `7c9585b848e11ea61e56dc772197e39661267f9963a2cac6239ffc71709e6375` |
| S3 | AMD, *BKDG for AMD Family 12h Processors*, publication 41131 Rev 3.02 (October 06, 2011): §2.10.1 p. 120; §3.1 p. 148 (`DZFYxXXX` mnemonic: device Z, function Y); D18F3x00 p. 303; D18F3xA4 p. 316; CPUID Fn0000_0001_EAX p. 421 | Primary. Same archive path, file `12h_bkdg_pub.pdf`, SHA-256 `68ccad2458144a8f76c6486eb9e2aa1179a63e19e7539211b43bf1525b17d59d` |
| S4 | AMD, *BKDG for AMD Family 14h Models 00h-0Fh Processors*, publication 43170 Rev 3.13 (February 17, 2012): §1.5.1 p. 28 (revision B = model 1h, C = model 2h); §2.10.1 pp. 114–115; §3.1 p. 131 (`DZFYxXXX` mnemonic, "D18F3x40 specifies the register of device 18h, function 3"); D18F3x00 p. 282; D18F3xA4 pp. 294–295; CPUID Fn0000_0001_EAX pp. 380–381 ("This document applies only to Family 14h Models 00h-0Fh processors") | Primary. Same archive path, file `43170_14h_Mod_00h-0Fh_BKDG.pdf`, SHA-256 `be45b947d779d5bbd27ddc9e84dd9ba2232fff23858ea1f746b37190029a40c5` |
| S5 | AMD, *Revision Guide for AMD Family 10h Processors*, publication 41322 Rev 3.92 (March 2012): Tables 2–11 pp. 11–13 (CPUID per package and revision); Table 27 pp. 30–33 (erratum 319 row on p. 31); erratum 319 "Inaccurate Temperature Measurement" p. 74 | Primary. Fetched 2026-11-28 from `https://www.amd.com/content/dam/amd/en/documents/archived-tech-docs/revision-guides/41322_10h_Rev_Gd.pdf`, SHA-256 `6cc58481d898139e58c1659fe5cc6f45d9a74eea9ee07807541a4a2ee1edc1aa` |
| S6 | AMD, *Revision Guide for AMD Family 11h Processors*, publication 41788 Rev 3.08 (July 2009): Table 1 p. 8; Table 6 pp. 15–16 (errata cross-reference) | Primary (AMD-authored). Not on AMD's current archive path. Retrieved 2026-11-28 from the Internet Archive capture `http://web.archive.org/web/20111011045754/http://support.amd.com/us/Processor_TechDocs/41788.PDF`, SHA-256 `7b172ca096cbf59cd63345a2774cb60924657328cf98c00fee3745d7d7c17e94` |
| S7 | AMD, *Revision Guide for AMD Family 12h Processors*, publication 44739 Rev 3.10 (March 2012): Table 2 p. 9 (FS1), Table 3 p. 10 (FM1); Table 11 pp. 18–19 (errata cross-reference) | Primary. Fetched 2026-11-28 from `.../archived-tech-docs/revision-guides/44739_12h_Rev_Gd.pdf`, SHA-256 `7b956bcfc8ea44a340abaada9d6fae248c1cd44306d0cea42466dde84f595785` |
| S8 | AMD, *Revision Guide for AMD Family 14h Models 00h-0Fh Processors*, publication 47534 Rev 3.18 (February 2013): Table 2 p. 9; Table 8 pp. 18–20 (errata cross-reference) | Primary. Fetched 2026-11-28 from `.../archived-tech-docs/revision-guides/47534_14h_Mod_00h-0Fh_Rev_Guide.pdf`, SHA-256 `01b5e3761c54ee0f7ec1ced2d8a0e14f7af4445a529f3b1f5f4be2b7494ebb6a` |
| S9 | PawnIO.Modules `AMDFamily10.p` (LGPL-2.1-or-later) at tag `0.2.11` (commit `52a7e536dff3e53c96917a28caac5e0fa6510696`), byte-identical at tag `0.2.12`; re-read 2026-11-28 (file SHA-256 `caa05c100bf8ebea9a3de1205b574bb793ef2b682630396c063799dd11d99763`) | Upstream-published interface definition of the module this project calls: `ioctl_*` names, cell counts, the `ioctl_read_miscctl` device-ID and offset allow-lists, the family gate, load-time behavior. Not used for any hardware register fact. No code copied. Contract recorded in [`pawnio-interface.md`](pawnio-interface.md) (section `AMDFamily10`) |
| S10 | PawnIO.Modules release `0.2.11`, asset `release_0_2_11.zip` (SHA-256 `43608cb89bc84247fef1368a139013f7d043e17db6d6c8dfc9b46bf0905a81f4`), containing signed `AMDFamily10.bin`, 16,868 bytes, SHA-256 `6443080b2968474ffbc38aa4356cc56f9664349fa4b917afdb33027d6bb50525`; byte-identical in `release_0_2_12.zip` | Primary (signed distribution). Same artifact as [`cpu-amd-family-15h-16h-tctl.md`](cpu-amd-family-15h-16h-tctl.md) S11 |
| S11 | PawnIO driver source `PawnIO/src/natives_impl_windows.cpp` at PawnIO tag `2.2.0` (commit `5cdf470831fdfff3f7f1d06363ca6b230f3bf35a`) | Driver interface fact: each module PCI configuration read reads the target function's vendor ID at offset 0 first; an invalid vendor ID returns `STATUS_DEVICE_DOES_NOT_EXIST` without the second read. No code copied |
| S12 | PawnIO.Modules issue #32, "Amd10Cpu freeze", <https://github.com/namazso/PawnIO.Modules/issues/32> (open at 2026-11-28) | Upstream freeze reports attributed to `ioctl_measure_tsc_multiplier`. Used only for a safety rule (never invoke) |
| S13 | Microsoft Learn, `GetLogicalProcessorInformationEx` and `LOGICAL_PROCESSOR_RELATIONSHIP` (`RelationProcessorPackage`), <https://learn.microsoft.com/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex> | OS API documentation for the package-count gate |
| S14 | Microsoft Learn, *Win32_PhysicalMemory class*, <https://learn.microsoft.com/windows/win32/cimwin32prov/win32-physicalmemory> (page dated 2018-05-31, fetched 2026-11-28): class represents "a physical memory device located on a computer system and available to the operating system"; `Capacity` (uint64, bytes, from Type 17 Size / Extended Size); `SMBIOSMemoryType` (uint32, MappingStrings "SMBIOS\|Type 17\|Memory_Type", "The raw SMBIOS memory type", not supported before Windows 10 / Windows Server 2016) | OS API documentation for the Family 10h DDR3 gate |
| S15 | DMTF, *System Management BIOS (SMBIOS) Reference Specification*, DSP0134 3.9.0: §7.18 Table 75 p. 100 (Type 17 Size: 0 = no memory device installed); §7.18.2 Table 77 pp. 104–105 (Memory Type `13h` = DDR2, `18h` = DDR3) | Primary (public specification). Fetched 2026-11-28 from `https://www.dmtf.org/sites/default/files/standards/documents/DSP0134_3.9.0.pdf`, SHA-256 `366e92f087db1b68b453e24a27dd74e4ead8a11a53a9fb03789402f49a44bf1d` |
| L1 | Linux kernel documentation, *Kernel driver k10temp*, <https://docs.kernel.org/hwmon/k10temp.html> (GPL-2.0), read 2026-11-28 | **lead-only (copyleft)**. Documentation, not code. Used only in Open questions; no fact or scope row depends on it |

## Access path (facts)

The signed `AMDFamily10` module loads on AMD x86-64 CPUs of families
`0x10`–`0x16` (S9; [`pawnio-interface.md`](pawnio-interface.md),
section `AMDFamily10`). This document uses one operation:

| Function | Input cells | Output cells | Used for |
| --- | --- | --- | --- |
| `ioctl_read_miscctl` | `in[0]` = node index (the module reads PCI bus 0, device `0x18 + in[0]`, function 3), `in[1]` = offset | `out[0]` = 32-bit value | Offsets `0x00` (identity) and `0xA4` (temperature) only, node index 0 only |

- The module allows the read only when the dword at offset `0x00` of
  the addressed function is on its device-ID list. That list contains
  `0x12031022`, `0x13031022` and `0x17031022`, the three values in the
  group table below. Allowed offsets are `0x00`, `0xA4` and `0xD4`.
  Any other device ID or offset returns `STATUS_ACCESS_DENIED`. (S9)
- `ioctl_read_smu` returns `STATUS_NOT_SUPPORTED` on every family
  other than `0x15` (S9). This document does not call it.
- The other modules this project ships do not reach these registers.
  `AMDFamily17` loads only on families `0x17`–`0x1A`. `RyzenSMU`
  recognizes no family `0x10`–`0x14` part. `LpcIO` never allows ports
  `0xCF8`–`0xCFF`. ([`pawnio-interface.md`](pawnio-interface.md))
- No shipped module can read function 2 (`F2x[1, 0]94`) or F3xE8. The
  Family 10h gates below use only CPUID and OS facts for that reason.

## Detection

Every runtime condition is a mechanism or safety gate (ADR 0031). No
brand string, firmware version or board identity is read.

| Condition | Gate kind | Source |
| --- | --- | --- |
| CPUID leaf 0 vendor is `AuthenticAMD` | mechanism | S1 p. 386; S9 (module rejects other vendors) |
| Effective family = `BaseFamily + ExtendedFamily` and model = `{ExtendedModel, BaseModel}` from CPUID Fn0000_0001_EAX. Family `0x10` (any model), `0x11` (any model), `0x12` (any model), or `0x14` with model `0x00`–`0x0F`, selects this document and its group row. Family `0x10` selects the decode with the select-field check; families `0x11`, `0x12` and `0x14` select the decode without it (Register map) | mechanism | S1 p. 386, S2 p. 198, S3 p. 421 ("This document applies only to family 1xh processors"); S4 pp. 380–381 (models 00h–0Fh only) |
| Family `0x10` only: the CPUID Fn0000_0001_EAX (model, stepping) pair is **not** model `0x00`–`0x03` (any stepping), model `0x04` with stepping `0x0`–`0x2`, or model `0x08` with stepping `0x0` | safety (erratum 319: these cover every revision Table 27 marks, DR-BA, DR-B2, DR-B3, RB-C2 and HY-D0, and every model below 4, for which the erratum states no package limit) | S5 Table 27 p. 31, erratum 319 p. 74, Tables 2–11 pp. 11–13 |
| Family `0x10` only: CPUID Fn8000_0001_EBX[31:28] (`PkgType`) is not `0000b` (Fr2, Fr5 or Fr6, socket F 1207) | safety (erratum 319: "software should not … utilize the value from CurTmp" when `PkgType` = 0) | S1 p. 387, S5 p. 74 |
| Family `0x10` only: `PkgType` is not `0011b` (G34) | mechanism (topology: G34 parts are dual-node; see Register map notes) | S1 p. 387, §2.7.1.5 p. 97 |
| Family `0x10` only: `PkgType` is one of `0001b` (AM2r2 or AM3), `0010b` (S1g3 or S1g4), `0100b` (ASB2), `0101b` (C32). Other encodings are reserved and Unsupported | mechanism | S1 p. 387 |
| Family `0x10` with `PkgType` = `0001b` only: DDR3 gate. A `Win32_PhysicalMemory` query of `Capacity` and `SMBIOSMemoryType` succeeds; at least one instance has `Capacity` > 0; and every instance with `Capacity` > 0 has `SMBIOSMemoryType` = 24 (`18h`, DDR3). An absent or null property, a failed or empty query, or any other value fails the gate | safety (erratum 319: "software should not … utilize the value from CurTmp" when `PkgType` = 1 and `Ddr3Mode` = 0; `Ddr3Mode` is set by BIOS from the connected DIMM type, and both DCTs carry the same type) | S5 p. 74; S1 F2x[1, 0]94 p. 255; S14; S15 Table 77 p. 105 |
| `GetLogicalProcessorInformationEx(RelationProcessorPackage)` succeeds and yields exactly one physical package | mechanism (node 0 is the only node read) | S1 §2.11 p. 164, S13 |
| `pawnio_load` of the signed `AMDFamily10.bin` succeeds | mechanism (access path present) | S9, S10 |
| `ioctl_read_miscctl(0, 0x00)` returns exactly the D18F3x00 value of the selected group (table below) | mechanism (device identity register) | S1 p. 287, S2 p. 140, S3 p. 303, S4 p. 282; S9 |
| Family `0x10` only, after each read: bits 17:16 of the register equal `00b` | mechanism (the register's own select field); post-read check, not an admission gate | S1 p. 320 |

| Group | CPUID family / model | D18F3x00 32-bit value (DeviceID 31:16, VendorID 15:0) | Select-field check | Source |
| --- | --- | --- | --- | --- |
| 10h | `0x10` / any, after the Family 10h gates | `1203_1022h` | yes | S1 p. 287 |
| 11h | `0x11` / any | `1303_1022h` | no (bits 20:13 Reserved) | S2 p. 140, pp. 156–157 |
| 12h | `0x12` / any | `1703_1022h` | no (bits 20:13 Reserved) | S3 p. 303, p. 316 |
| 14h 00h–0Fh | `0x14` / `0x00`–`0x0F` | `1703_1022h` | no (bits 20:13 Reserved) | S4 p. 282, pp. 294–295 |

12h and 14h share a device ID; CPUID family tells them apart. Both
decodes are the same.

### Scoped enablement

| Scope | Status | Default enablement |
| --- | --- | --- |
| Family 10h, `PkgType` S1g3/S1g4 (`0010b`), ASB2 (`0100b`) or C32 (`0101b`), outside the erratum 319 revision set, one physical package, identity read matched | Verified: register, fields, scale, select field and Tctl semantics from S1; erratum 319 boundaries from S5; access contract from S9, S10, S11. No lead-only fact. Evidence identities (not gates) from S5 Tables 9–11, 6: DA-C2 `00100F62h` (S1g3: Athlon II, Sempron, Turion II mobile), BL-C3 `00100F53h` and DA-C3 `00100F63h` (S1g4: V-Series, Athlon II, Turion II, Phenom II mobile), DA-C3 (ASB2: V-Series, Athlon II Neo, Turion II Neo), HY-D1 `00100F81h` (C32r1: Opteron 4100, covered only in single-package systems). No hardware observation by this project yet | Enabled (Verified), with the identity, select-field and plausibility gates |
| Family 10h, `PkgType` AM2r2/AM3 (`0001b`), outside the erratum 319 revision set, DDR3 gate passed, one physical package, identity read matched | Experimental. Register facts are as in the row above (S1, S5). The DDR3 gate infers `Ddr3Mode` = 1 from firmware-reported SMBIOS memory types (S14, S15) and the BKDG rule that BIOS sets `Ddr3Mode` from the connected DIMM type (S1 p. 255). It does not read the register itself. No lead-only fact. Evidence identities (not gates) from S5 Table 8: BL-C2 `00100F52h`, DA-C2 `00100F62h`, RB-C3 `00100F43h`, BL-C3 `00100F53h`, DA-C3 `00100F63h`, PH-E0 `00100FA0h`; Table 8's AM3 segments include Phenom II X2/X3/X4/X6, Athlon II X2/X3/X4 and Sempron. No hardware observation yet | Experimental (ADR 0011), with the identity, select-field and plausibility gates |
| Family 11h (any model), one physical package, identity read matched | Verified: register, fields, scale and Tctl semantics from S2; access contract from S9–S11. No lead-only fact. Revision-guide review (S6 Table 6) found no erratum naming F3xA4, CurTmp or Tctl. Evidence identity (not a gate) from S6 Table 1: LG-B1 `00200F31h` (S1g2: Turion X2 Ultra, Turion X2, Athlon, Athlon X2, Sempron, Sempron X2 mobile). No hardware observation yet | Enabled (Verified), with the identity and plausibility gates |
| Family 12h (any model), one physical package, identity read matched | Verified: register, fields, scale and Tctl semantics from S3; access contract from S9–S11. No lead-only fact. Revision-guide review (S7 Table 11) found no erratum naming D18F3xA4, CurTmp or Tctl. Evidence identity (not a gate) from S7 Tables 2–3: LN-B0 `00300F10h` (FS1: A-Series mobile APU; FM1: A-Series, E2-Series, Sempron, Athlon II). No hardware observation yet | Enabled (Verified), with the identity and plausibility gates |
| Family 14h models 00h–0Fh, one physical package, identity read matched | Verified: register, fields, scale and Tctl semantics from S4; access contract from S9–S11. No lead-only fact. Revision-guide review (S8 Table 8) found no erratum naming D18F3xA4, CurTmp or Tctl. Evidence identities (not gates) from S8 Table 2: ON-B0 `00500F10h` (C-, E-, G-Series), ON-C0 `00500F20h` (C-, E-, G-, Z-Series). No hardware observation yet | Enabled (Verified), with the identity and plausibility gates |
| Family 10h erratum 319 revision set: model `0x00`–`0x03`; model `0x04` stepping `0x0`–`0x2`; model `0x08` stepping `0x0` (documented members DR-BA `00100F2Ah`, DR-B2 `00100F22h`, DR-B3 `00100F23h`, RB-C2 `00100F42h`, HY-D0 `00100F80h`) | AMD states the sensor behind CurTmp "may report inconsistent values" on these revisions (S5 p. 74, Table 27) | Unsupported (safety) |
| Family 10h `PkgType` `0000b` (Fr2, Fr5, Fr6: socket F Opteron) | AMD's software rule says not to use CurTmp (S5 p. 74). These are also multi-socket server parts | Unsupported (safety) |
| Family 10h `PkgType` `0001b` with the DDR3 gate failed (DDR2 system such as an AM2r2 board, or memory type unknown) | AMD's software rule says not to use CurTmp when `Ddr3Mode` = 0 (S5 p. 74), and this document cannot read `Ddr3Mode` | Unavailable on that system (safety) |
| Family 10h `PkgType` `0011b` (G34: HY-D1 `00100F91h`, Opteron 6100) | Dual-node part. Device 18h is the BSC node (S1 §2.6.4.2.2 p. 85), and no document consulted states that it is internal node 0, the node that reports the processor temperature (S1 p. 320). `F3xE8[IntNodeNum]` is not on the module's allow-list | Unsupported; see Open questions |
| Family 10h reserved `PkgType` encodings | No package defined (S1 p. 387) | Unsupported |
| Family `0x13`; Family `0x14` models `0x10`–`0xFF` | No AMD document consulted defines a register map (ADR 0031 rule 3) | Unsupported |
| More than one physical package | The module accepts a node index but no package selector, and this document states no node-to-package mapping | Unsupported |
| Family `0x15`/`0x16` | Owned by [`cpu-amd-family-15h-16h-tctl.md`](cpu-amd-family-15h-16h-tctl.md) | Out of scope here |

## Register map (facts)

PCI configuration space, bus 0, device `18h` (node 0), function 3.
Family 10h: devices 24–31 correspond to nodes 0–7 (S1 §2.11 p. 164).
Family 11h: configuration space is at bus 0 device 24 (S2 §2.11 p. 75).
Families 12h and 14h: the mnemonic `D18F3xA4` names device 18h,
function 3, offset `A4h` (S3 §3.1 p. 148, S4 §3.1 p. 131). Families
10h and 11h name the same register `F3xA4` (S1 p. 200, S2 p. 93).

| Address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| D18F3x00 | Device/Vendor ID | 31:16, 15:0 | DeviceID (per group table), VendorID `1022h` | read-only value | S1 p. 287, S2 p. 140, S3 p. 303, S4 p. 282 |
| D18F3xA4 | Reported Temperature Control, `CurTmp` | 31:21 | 10h and 11h: "Provides the current control temperature, Tctl (after the slew-rate controls have been applied)". 12h and 14h: "Specifies the current control temperature with the slew-rate controls applied". Read-only, reset X | Tctl = `CurTmp` × 0.125, 0 to 255.875 (12h and 14h: "7FFh-000h <CurTmp*0.125>") | S1 p. 320, S2 p. 156, S3 p. 316, S4 p. 295 |
| D18F3xA4 (10h only) | `CurTmpSel` | 17:16 | "Current temperature select." Read-write, reset 00; "may be used for diagnostic software". `00b`: CurTmp provides the read-only Tctl value. `01b`, `10b`: Undefined. `11b`: CurTmp is a read-write register that specifies a value used to create Tctl; its two LSBs read as zero | select field | S1 p. 320 |
| D18F3xA4 (10h) | Reserved | 20:18, 15:13 | Reserved | not decoded | S1 p. 320 |
| D18F3xA4 (11h, 12h, 14h) | Reserved | 20:13 | Reserved. These families have no select field | not decoded | S2 p. 156, S3 p. 316, S4 p. 295 |
| D18F3xA4 | slew controls | 12:8, 7, 6:5, 4:0 | PerStepTimeDn, TmpSlewDnEn (11h: bit 7 Reserved), TmpMaxDiffUp, PerStepTimeUp; read-write, programmed by BIOS | not decoded | S1 pp. 320–321, S2 pp. 156–157, S3 p. 316, S4 pp. 294–295 |

Notes:

- Family 10h dual-node processors are identified by
  `F3xE8[MultiNodeCpu]` (revision D and later), and the G34 package
  supports them. On such a part, CurTmp "should be accessed on internal
  node 0 (F3xE8[IntNodeNum]=00b) to obtain temperature information for
  the processor. Reading this field on internal node 1 provides only
  the temperature for internal node 1." (S1 p. 97, p. 320, p. 330)
- Family 10h NodeId assignment starts at the node containing the BSC,
  which has NodeId 0 by definition (S1 §2.6.4.2.2 p. 85).
- Family 10h, 11h: "Accesses to unimplemented functions are also
  ignored: writes are dropped; however, reads return all F's." (S1
  §2.11.3 p. 165, S2 §2.11.3 p. 76)
- Family 10h erratum 319 (S5 p. 74): the internal sensor behind CurTmp
  (F3xA4[31:21]), HTC, the STC thermal zone and SB-TSI "may report
  inconsistent values". For model 4 and higher it occurs "only on
  AM2r2, Fr2, Fr5 and Fr6 package processors". The software rule: when
  (`PkgType` = 1 (AM2r2 or AM3) and `F2x[1, 0]94[Ddr3Mode]` = 0) or
  `PkgType` = 0 (F (1207)), "software should not modify HtcTmpLmt …,
  utilize the value from CurTmp, or enable any of the STC thermal zone
  features". Table 27 (p. 31) marks the erratum for DR-BA, DR-B2,
  DR-B3, RB-C2 and HY-D0. Tables 2–11 (pp. 11–13) list the AM2r2
  package revisions as DR-B2, DR-B3 and RB-C2 only.
- `F2x[1, 0]94[Ddr3Mode]` (bit 8): "This bit must be set by BIOS based
  on the types of DIMMs connected to the DCT. 0=DDR2 mode. 1=DDR3 mode.
  Both DCTs must be programmed to the same DIMM type." (S1 p. 255)

## Read procedure and decode

Admission (steps 1–4) runs in this order. The path is admitted only
after step 4 passes. A failure in steps 1–4 leaves CPU temperature
unavailable from this path (fallback as for other PawnIO paths). No
register read happens before step 4.

1. CPU and topology gates, before any PawnIO call: vendor, family and
   model selection, the Family 10h revision and `PkgType` gates, and
   the single-package gate (Detection). A CPU outside every admitted
   group is Unsupported, and the module is not loaded.
2. Family 10h with `PkgType` = `0001b` only: evaluate the DDR3 gate
   once. If it fails, the path is unavailable on this system.
3. Load `AMDFamily10.bin` on its own executor handle (the module-load
   gate in Detection).
4. Identity read gate, once per opened handle: `ioctl_read_miscctl`
   with `in[0]` = 0, `in[1]` = `0x00`. If the call returns an error
   status, or if the low 32 bits of `out[0]` (read only after a
   successful call) differ from the group's D18F3x00 value, do not
   read further on this handle and report the path unavailable.

Per sample, on an admitted path:

5. Call `ioctl_read_miscctl` with `in[0]` = 0, `in[1]` = `0xA4`. The
   module documents no caller mutex for this function. The call reads
   only the addressed function's ID register and the requested
   register and writes no index, so no mutex is required (S9, S11). If
   the call returns an error status, skip the sample. The module
   returns before writing `out[0]` on every error path (S9). Only after
   a successful call, take the low 32 bits of `out[0]` as `raw`.
6. Decode:
   - If `raw` is `0x00000000` or `0xFFFFFFFF`, the sample is invalid
     (project policy: failed transaction; S1 p. 165 and S2 p. 76 state
     that unimplemented functions read all F's).
   - Family 10h only: `select` = bits 17:16 of `raw`. If `select` ≠
     `00b`, the sample is unavailable. `01b`/`10b` are Undefined, and
     `11b` means the field holds a software-specified value rather than
     the hardware-updated Tctl (S1 p. 320). Families 11h, 12h and 14h:
     bits 17:16 are Reserved and are not examined.
   - `CurTmp` = bits 31:21 of `raw` (unsigned, 0–2047).
   - Tctl = `CurTmp` × 0.125, on the Tctl scale (0 to 255.875).
7. Plausibility gate (project policy, matching the Zen and Family
   15h/16h paths): publish only 0 ≤ Tctl ≤ 120; drop anything else.
8. Publish Tctl as the CPU control temperature. Apply no offset, and
   do not derive a die temperature from it.

### Decode examples (synthetic)

These values are constructed from the field layout above. They are not
hardware observations.

| Family | `raw` | Result |
| --- | --- | --- |
| 10h | `0x2D000FEF` | `CurTmp` = `0x168` (360), select `00b`: Tctl 45.000 |
| 10h | `0x2D030FEF` | select `11b`: unavailable |
| 10h | `0x2D010FEF` | select `01b` (Undefined): unavailable |
| 11h, 12h or 14h | `0x2D030FEF` | bits 17:16 not examined: Tctl 45.000 |
| any | `0x79000FEF` | `CurTmp` = `0x3C8` (968), Tctl 121.000: dropped by plausibility |
| any | `0x00000000` or `0xFFFFFFFF` | invalid |

### Meaning and label

- Family 10h and 11h: "Tctl is the processor temperature control
  value, used by the platform to control cooling systems… Tctl is a
  non-physical temperature on an arbitrary scale measured in degrees.
  It does not represent an actual physical temperature like die or
  case temperature." Below Tctl_max the part is (Tctl_max − Tctl)
  under the temperature for which maximum cooling is required; at or
  above it, (Tctl − Tctl_max) over. (S1 §2.10.1 p. 162, S2 §2.10.1
  p. 73)
- Family 12h and 14h: "Tctl is a temperature on its own scale aligned
  to the processors cooling requirements. Therefore Tctl does not
  represent a temperature which could be measured on the die or the
  case of the processor." (S3 §2.10.1 p. 120, S4 §2.10.1 p. 114)
- The value is slew-filtered, so it lags the measured temperature.
  (S1–S4 D18F3xA4)
- Required presentation, as in the Family 15h/16h path: the label
  names the reading as Tctl or a control temperature (for example
  "CPU Tctl"), never as a die, case or calibrated package temperature.
  It must not be classified with independently calibrated die
  readings. Low idle values are expected on this scale and are not an
  error. This document provides no Tctl_max, so no "distance to limit"
  may be derived.

## Quirks

- Family 10h erratum 319 (S5 p. 74, Table 27 p. 31): CurTmp is
  unreliable on DR-BA, DR-B2, DR-B3, RB-C2 and HY-D0, on socket F
  packages, and, by AMD's software rule, on AM2r2/AM3-type packages
  running DDR2. The Detection gates exclude all three cases. The gate
  set is broader than the erratum in two places: RB-C2 and HY-D0 are
  excluded on every package, and model 4 steppings 0–1 (no documented
  revision) are excluded with RB-C2.
- Only Family 10h has `CurTmpSel`. Families 11h, 12h and 14h leave bits
  17:16 Reserved. (S1 p. 320, S2 p. 156, S3 p. 316, S4 p. 295)
- Families 12h and 14h share D18F3x00 `1703_1022h`. (S3 p. 303, S4
  p. 282)

## Safety notes

- Operations used: `pawnio_load` and `ioctl_read_miscctl` (offsets
  `0x00` and `0xA4` only, node index 0 only). Nothing else in the
  module is invoked. Offset `0xD4` is not needed.
- Never invoke `ioctl_measure_tsc_multiplier`: it disables interrupts,
  writes MSRs `C001_0015h`, `C001_0000h` and `C001_0004h` and spins on
  a tick count (S9). Upstream reports whole-system freezes attributed
  to it (S12). Never invoke `ioctl_read_cstate_residency`: it writes
  I/O port `0xCD6` and documents no mutex (S9). `ioctl_read_msr` and
  `ioctl_read_smu` are not used.
- Load-time effects of `AMDFamily10` (S9): it reads the CPU vendor,
  family and model and emits a debug print. It reads the dword at PCI
  `00:14.0` offset 0, and only when that equals `4385_1002h` it reads
  the byte at offset `08h`. No write, mapping, MSR access or interrupt
  change happens at load. The module defines no unload routine.
- Never write D18F3xA4. On Family 10h, setting `CurTmpSel` = `11b`
  would replace the hardware Tctl with a software value (S1 p. 320).
  The erratum 319 rule also forbids modifying HtcTmpLmt or enabling STC
  features on the affected systems (S5 p. 74). This document never
  writes either.
- The DDR3 gate is a read-only WMI query.

## External component requirements

This path uses the same signed `AMDFamily10.bin` as
[`cpu-amd-family-15h-16h-tctl.md`](cpu-amd-family-15h-16h-tctl.md):
16,868 bytes, SHA-256
`6443080b2968474ffbc38aa4356cc56f9664349fa4b917afdb33027d6bb50525`,
from `release_0_2_11.zip` (S10). Installing the other four modules
(`IntelMSR.bin`, `RyzenSMU.bin`, `AMDFamily17.bin`, `LpcIO.bin`) does
not make it available. If the Family 15h/16h implementation has not yet
integrated this module, an implementation of this document must meet
the requirements stated in that document's External component
requirements section, through the external-component owner
([`docs/design/external-component-setup.md`](../../design/external-component-setup.md),
ADR 0024, ADR 0026, ADR 0029): file name and hash in setup detection,
placement, outdated-module refresh and the sensor service's known
signed hashes; a distinct missing-module state; fallback as for other
PawnIO paths; and the LGPL `COPYING` notice.

## Open questions

- Non-blocking for #2512: AMD's erratum 319 statements differ in breadth. The description and Table 27 confine the erratum for model 4 and later to AM2r2/Fr2/Fr5/Fr6 package processors and the five marked revisions. The software rule also covers any `PkgType` 1 part running DDR2, which includes AM3 processors on AM2+ boards. L1 (lead-only, copyleft) states that an AM3 processor on an AM2+ board is unaffected. That conflicts with the literal software rule, so the primary rule wins and such systems stay unavailable. A revision may relax this only on primary evidence.
- Non-blocking for #2512: the DDR3 gate relies on firmware-reported SMBIOS memory types, not on `F2x[1, 0]94[Ddr3Mode]`. Boards that report an unknown or wrong type fail closed (unavailable) or, if wrongly reporting DDR3 on DDR2, would expose an Experimental reading that the erratum calls unreliable. The row stays Experimental. Graduation to Verified needs an accepted dump that pairs the SMBIOS memory type with a `Ddr3Mode` readout, or a module that can read function 2.
- Non-blocking for #2512: Family 10h G34 (HY-D1 `00100F91h`, Opteron 6100) is Unsupported because no document consulted states that the BSC node (device 18h) is internal node 0, and `F3xE8[IntNodeNum]` cannot be read through the module. The same question applies to the Family 15h 00h–0Fh G34r1 row of `cpu-amd-family-15h-16h-tctl.md`. A module allow-list entry for `F3xE8`, or an AMD statement placing the BSC on internal node 0, would resolve it.
- Non-blocking for #2512: no document consulted states that only G34 Family 10h parts are dual-node. On any other single-package system the BSC node is NodeId 0, which is device 18h (S1 pp. 85, 164), so a reading would still be the Tctl of the node that holds the boot core.
- Non-blocking for #2512: multi-package systems (dual-socket C32 Opteron 4100) are Unsupported. Supporting them needs a stated node-to-package mapping and a module input that can address each package's node.
- Non-blocking for #2512: whether any firmware leaves Family 10h `CurTmpSel` at `11b` at runtime is unknown. Such samples are reported unavailable, which is truthful.
- Non-blocking for #2512: the Family 11h revision guide (S6, Rev 3.08, July 2009) was obtained only from an Internet Archive capture of AMD's former support site. No later revision was found. A later revision could add errata; reads stay read-only and plausibility-gated.
- Non-blocking for #2512: no hardware observation exists for any group. A user Compatibility Report (#2500) or a maintainer-accepted register dump per group should corroborate the identity read and a plausible Tctl.
- Non-blocking for #2512: Family 13h and Family 14h models 10h and above have no AMD register document in this research and stay Unsupported.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-28 | Initial version. `D18F3xA4` Tctl for Family 10h, 11h, 12h and 14h models 00h–0Fh, from AMD BKDGs 31116, 41256, 41131 and 43170 and revision guides 41322, 41788, 44739 and 47534. Family 10h erratum 319 safety gates (revision set, socket F, DDR3 gate for AM2r2/AM3-type packages) and G34 exclusion. Access through signed `AMDFamily10.bin` from PawnIO.Modules 0.2.11 (`ioctl_read_miscctl` only). Proposed as Implementation-ready (rev 1); the maintainer's PR approval is the sign-off. |
