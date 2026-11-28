# Spec: Nuvoton NCT668x firmware-mediated temperature and fan readings

| Field | Value |
| --- | --- |
| Revision | 6 |
| Status | Draft — not implementation-ready |
| Scope | Existing firmware-enabled EC Space monitoring on MSI PRO B850-P WIFI (MS-7E56), raw chip ID `0xD592`, customer `0x020D`, EC 0.0 build 2025-04-18. Records separate Linux hwmon evidence, an installed-base ranking of the #2696 targets, and the per-platform dispositions for ASRock and AMD NCT6686D, ASRock NCT6683D, Intel NCT6683D and additional MSI NCT6687D boards and for direct Windows access. Excludes changing EC or Super I/O configuration, voltage, limits and fan control. |
| Issue phase | [#2520](https://github.com/shm11C3/HardwareVisualizer/issues/2520), [#2696](https://github.com/shm11C3/HardwareVisualizer/issues/2696) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Nuvoton, [NCT6686D Hardware Datasheet v0.5](https://www.nuvoton.com/export/resource-files/NCT6686D_HW_Datasheet_V0_5.pdf), April 21, 2017, §§6.1 and 7.2–7.3, pp. 52–58, and §§8 and 18.11 | Primary for that documented hardware's configuration and EC host interface. The sensor map is firmware-defined. This document does not identify the MSI chip or settle production NCT6686D chip IDs. |
| S2 | MSI, [PRO B850-P WIFI User Guide](https://download.msi.com/archive/mnu_exe/mb/PROB850-PWIFI_English.pdf), Specifications, Fan Connectors and Regulatory Notices | Primary for board name, board number and available physical connectors. MSI lists the I/O controller as NCT6687-R. |
| S3 | Linux [v6.18 NCT6683-series hardware monitor](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/nct6683.c), GPL-2.0-or-later | **lead-only (copyleft)**. Raw-ID family match, EC configuration discovery and application-port transaction, firmware identity, channel configuration, temperature and RPM words. Its mutation paths are outside this draft. |
| S4 | 3mdeb, [Porting MSI PRO B850-P to coreboot, Part 1](https://blog.3mdeb.com/2026/2026-02-09-msi_pro_b850p_part1/#super-io-initialization), February 9, 2026, and its [pinned article source](https://github.com/3mdeb/news-and-ideas/blob/df347e0fd731df6832c19abaeb5dcc594e3b4dc7/blog/content/post/2026-02-09-msi_pro_b850p_part1.md) | Published independent board observations and provenance for the full OEM capture in S8. |
| S5 | [MSI PRO B850-P WIFI hardware report](https://lore.landau.one/lkml/0c8f35ac-3c92-4b1d-a027-7d8912751109%40roeck-us.net/T/), October 4, 2026 | **lead-only (copyleft)** for its report on the exact board, BIOS and EC build, customer ID and fan channels. No control transactions are specified here. |
| S6 | [PawnIO interface](pawnio-interface.md) revision 7, `LpcIO` and mutex sections, and [pinned upstream module interface](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/LpcIO.p) at tag 0.2.11 | Interface and allow-list behavior for discovery and byte I/O. Existing signed module, no additional blob required by this candidate. |
| S7 | Linux [NCT6683-series documentation](https://www.kernel.org/doc/html/v6.18/hwmon/nct6683.html), Tested Boards and Firmware Versions, and [MSI MAG B850M MORTAR WIFI report](https://lists.openwall.net/linux-kernel/2026/09/21/3030) | **lead-only (copyleft)**. Firmware and variant inventory for follow-up, not an enablement list. |
| S8 | 3mdeb, [full OEM Super I/O and EC capture](https://paste.dasharo.com/?ba0b5d09afbacfc1#6vRmb9aVC4cG4LVfGVEpD3KV6a5wKVCrui8wveLru5gT), linked by S4 | First-hand board observation of raw chip ID, enabled EC window, firmware identity bytes and channel readback. The capture gives no system BIOS version. |
| S9 | Dasharo/coreboot at `d08d5eaa98`, [Nuvoton diagnostic](https://github.com/Dasharo/coreboot/blob/d08d5eaa98/util/superiotool/nuvoton.c) and [common configuration operations](https://github.com/Dasharo/coreboot/blob/d08d5eaa98/util/superiotool/superiotool.c), pinned by the tool banner in S8 | **lead-only (copyleft)** for the operations used to obtain the OEM observation. The utility performs other hardware operations outside the project's read-only scope. |
| S10 | Dasharo [MSI PRO B850-P firmware v0.9.0](https://docs.dasharo.com/variants/msi_b850/releases/), coreboot `713a0b76`, [board device tree](https://github.com/Dasharo/coreboot/blob/713a0b76/src/mainboard/msi/ms7e56/devicetree.cb), [chip configuration](https://github.com/Dasharo/coreboot/blob/713a0b76/src/superio/nuvoton/nct6687d/superio.c), [configuration mode](https://github.com/Dasharo/coreboot/blob/713a0b76/src/superio/common/conf_mode.c) and [firmware EC port](https://github.com/Dasharo/coreboot/blob/713a0b76/src/superio/nuvoton/nct6687d/nct6687d_ec.c) | **lead-only (copyleft)** for how separate firmware on the same board exits configuration mode and uses the firmware EC port. |
| S11 | [Windows SMBIOS board identification report](https://xmrig.com/benchmark/5hgiH1) and MSI [B850 GAMING PLUS WIFI User Guide](https://download.msi.com/archive/mnu_exe/mb/B850GAMINGPLUSWIFI_German.pdf), Regulatory Notices | Windows reports the full PRO board product and manufacturer. MSI also uses MS-7E56 for a different product. |
| S12 | Linux v7.0 [NCT6683-family identity and register ranges](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c#L37-L185), [read and refresh](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c#L503-L654), [fan input](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c#L718-L725), [temperature input](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c#L822-L831), [probe](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c#L1100-L1317), [chip match](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c#L1350-L1410) and [resource conflict check](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c#L1450-L1490) | **lead-only (copyleft)** for masked identity, customer admission, driver and attribute behavior, and the full input refresh read set. These sources describe driver accesses, with firmware address semantics still dependent on the EC build. |
| S13 | Linux [BC-250 customer support commit](https://github.com/torvalds/linux/commit/f392611e268f5462b51d90857c72058c31282706), first in v6.15, and [v7.1-rc5 documentation](https://www.kernel.org/doc/html/v7.1-rc5/hwmon/nct6683.html#tested-boards-and-firmware-versions) | **lead-only (copyleft)** for native customer `0x162b` acceptance and reported NCT6686D EC 1.0 build 2021-07-28. |
| S14 | ASRock AMD BC-250 [December 2024 Fedora 41 observation](https://github.com/mothenjoyer69/bc250-documentation/issues/3#issue-2734591172) and [separate Fedora CoreOS boot log](https://github.com/elektricM/amd-bc250-docs/blob/98e48d9a346cb04ba1cabc62e54a0ae5363357a6/docs/linux/fedora-coreos.md#L132-L150) | First-hand raw ID, live values and a separate firmware/version and platform-instance observation. The documented captures use the driver's force option. |
| S15 | Linux [Z590 Taichi customer support commit](https://github.com/torvalds/linux/commit/c0fa7879c9850bd4597740a79d4fac5ebfcf69cc) and [stable submission with the author's two-board report](https://lists.openwall.net/linux-kernel/2026/02/28/1014) | **lead-only (copyleft)** for native customer `0x1621` acceptance and reported instantiation on EC 1.0 build 2021-01-25. |
| S16 | Linux v7.0 [Intel firmware note and tested boards](https://www.kernel.org/doc/html/v7.0/hwmon/nct6683.html#usage-note), [fan attribute visibility](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c#L746-L769) and [v3.16 refresh](https://github.com/torvalds/linux/blob/v3.16/drivers/hwmon/nct6683.c#L629-L636) | **lead-only (copyleft)** for the reported Intel EC builds, firmware-dependent limits, and fan-minimum reads in both kernel versions. |
| S17 | [DH87MC contributed sensor configuration](https://github.com/sqrdevl/nct6683/blob/4c57b3c54f60f73cf23f5bd0e612e6c6921136d0/conf/intel-dh87mc.conf#L1-L17), credited to Adam Nielsen, and Intel [DH87MC Technical Product Specification G94453-003](https://gzhls.at/blob/ldb/8/f/2/5/188cfe3a34e80c2326718304c43d92d324fe.pdf), §§1.10 and 1.11.2.2 (Intel document mirror) | **lead-only (copyleft)** for the configuration's device name, base and channel labels. Intel is the primary source for physical fan headers and lists the controller as NCT6883D-T. |
| S18 | ASRock [X670E Steel Legend manual](https://download.asrock.com/Manual/X670E%20Steel%20Legend.pdf#page=14), §1.5 p. 10, [Arch Linux report](https://github.com/lm-sensors/lm-sensors/issues/499), [probe follow-up](https://github.com/lm-sensors/lm-sensors/issues/499#issuecomment-2145526709) and [separate forced Ubuntu capture](https://github.com/lm-sensors/lm-sensors/issues/499#issuecomment-2338377717) | OEM block diagram identifies NCT6686D. The Arch report gives exact board and BIOS identity and raw chip ID. The separate Ubuntu report gives EC build and live values with force enabled. |
| S19 | ASRock [X670E Steel Legend BIOS 3.01 ZIP](https://download.asrock.com/BIOS/AM5/X670E%20Steel%20Legend%283.01%29ROM.zip), SHA-256 `61ab1b123f6a21336b7946283976b9eb45ea765277b30525c8563280c983f29e`. Extracted 32 MiB `X670ESL3.01.ROM`, SHA-256 `47601a2767e5ae7a38ca2e649bf78bafd48eb16a47ba5702fb74fe8d52419f34`. OEM [BIOS setup guide](https://download.asrock.com/Manual/Software/AMD%20X670/Software_BIOS%20Setup%20Guide_English.pdf), §§2.3.2 and 3.6 | Static firmware evidence limited to the extracted checksum-valid tables. The guide describes the UEFI monitoring UI. |
| S20 | Intel [DZ87KLT-75K Technical Product Specification G89979-001](https://manualzz.com/doc/48591817/intel-dz87klt-75k-specification), p. iii and §§1.11, 1.12.2.2, [Intel BIOS 0459 release notes](https://ftp.infania.net/pub/Windows%20Drivers/Intel/downloadmirror.intel.com/KL_0459_ReleaseNotes.pdf), BIOS 0446 entry, and [independent board inspection](https://techplayboy.com/754/intel-dz87klt-75k-lga1150-desktop-motherboard-review/) | OEM board and firmware family identity, monitoring ASIC wiring, and threshold corrections. The independent inspection identifies NCT6683D. |
| S21 | [Original driver test record](https://github.com/sqrdevl/nct6683/blob/89c8ec9ac2749b579af6436a2073f5effef10ae0/README), May 17, 2014, and stock Linux [v7.3 refresh](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/nct6683.c#L633-L646) and [v7.4-rc2 refresh](https://github.com/torvalds/linux/blob/v7.4-rc2/drivers/hwmon/nct6683.c#L632-L645) | **lead-only (copyleft)** for the older DZ87KLT test and the generic fan-minimum read retained in the stock refresh. |
| S22 | Linux v7.3 [customer admission list](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/nct6683.c#L172-L186), unchanged in v7.4-rc4, and the admitting commits [`bd433537fef8`](https://github.com/torvalds/linux/commit/bd433537fef8) (`0x0E2C`, X570 Phantom Gaming-ITX/TB3), [`d55532f77137`](https://github.com/torvalds/linux/commit/d55532f77137) (`0x0E1B`, Z370M Pro4 rev. 1.01 with NCT6683D-T), [`05b68e18ec64`](https://github.com/torvalds/linux/commit/05b68e18ec64) (`0x1631`, X670E Taichi), [`ff708b549c4d`](https://github.com/torvalds/linux/commit/ff708b549c4d) (`0x1633`, B650I Lightning WiFi, first in v7.1) and [`705ec98e8a48`](https://github.com/torvalds/linux/commit/705ec98e8a48) (`0x163D`, Z890 Pro-A, first in v7.2) | **lead-only (copyleft)** for native customer admission and the board each commit names. S12, S13, S15 and the [B650 Steel Legend WiFi specification](superio-nuvoton-nct6686-asrock-linux-hwmon.md) S5 cover `0x162B`, `0x1621` and `0x163E`. |
| S23 | [ASRock X600DC-ITX customer patch](https://lkml.iu.edu/hypermail/linux/kernel/2611.1/21560.html), November 14, 2026, and the [maintainer reply](https://lkml.iu.edu/hypermail/linux/kernel/2611.1/22223.html) | First-hand BIOS 2.02, NCT6686D at `0x2e:0xa20`, forced binding of customer `0x165F` as `nct6683.2592`, EC 1.0 build 12/23/25, and plausible readings including fan input 1 near 1450 RPM and an AMD TSI temperature. The reply reports the patch as applied. The admission is absent from v7.4-rc4 (S22). |
| S24 | [ASRock B850 Steel Legend WiFi fan-control series thread](https://ratatoskr.run/lkml/2026/08/17457921/t), August 25 to November 10, 2026 | First-hand customer `0x164A`, EC 1.0 build 10/29/24 and BIOS 4.10, unadmitted in v7.4-rc4. The series adds fan-control writes, which are outside this project. |
| S25 | LibreHardwareMonitor at `9e2d677c`, [EC Space byte read](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/9e2d677cefff5c614ecc38da88fbc447eb478e2a/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/Nct677X.cs#L1259-L1302) | **lead-only (copyleft)** for a Windows reader that waits up to 500 ms for the application page selector to read `0xFF`, writes `0xFF` on timeout regardless, and writes `0xFF` after each byte. |
| S26 | linux-hardware.org computer listings by baseboard model, for example [ASRock X670E Taichi](https://linux-hardware.org/?view=computers&vendor=ASRock&model=X670E+Taichi), [MSI MAG B550 TOMAHAWK](https://linux-hardware.org/?view=computers&vendor=MSI&model=MAG+B550+TOMAHAWK+%28MS-7C91%29) and [Intel DH87MC](https://linux-hardware.org/?view=computers&vendor=Intel&model=DH87MC), whose model filters list the per-model counts for the whole vendor, retrieved November 29, 2026 | Count of distinct computers per model whose owners submitted a Linux hardware probe. Intel counts sum the listed board revisions. A model absent from the filter is "Not listed". Used only to rank targets. |
| S27 | Linux NCT6683-family driver customer admission at tags [v5.11](https://github.com/torvalds/linux/blob/v5.11/drivers/hwmon/nct6683.c), [v5.12](https://github.com/torvalds/linux/blob/v5.12/drivers/hwmon/nct6683.c), [v5.16](https://github.com/torvalds/linux/blob/v5.16/drivers/hwmon/nct6683.c), [v6.5](https://github.com/torvalds/linux/blob/v6.5/drivers/hwmon/nct6683.c), [v6.7](https://github.com/torvalds/linux/blob/v6.7/drivers/hwmon/nct6683.c), [v6.9](https://github.com/torvalds/linux/blob/v6.9/drivers/hwmon/nct6683.c), [v6.14](https://github.com/torvalds/linux/blob/v6.14/drivers/hwmon/nct6683.c), [v6.15](https://github.com/torvalds/linux/blob/v6.15/drivers/hwmon/nct6683.c), [v7.0](https://github.com/torvalds/linux/blob/v7.0/drivers/hwmon/nct6683.c), [v7.1](https://github.com/torvalds/linux/blob/v7.1/drivers/hwmon/nct6683.c) and [v7.2](https://github.com/torvalds/linux/blob/v7.2/drivers/hwmon/nct6683.c), each compared with the preceding release, and [commit `7fce84a991cd`](https://github.com/torvalds/linux/commit/7fce84a991cd), which names MSI PRO X670-P WIFI with NCT6687D for `0x0207` | **lead-only (copyleft)** for the first release that admits each customer without force: `0x0201` v5.11, `0x0E2C` v5.12, `0x0E1B` v5.16, `0x0200` v6.5, `0x1631` v6.7, `0x0207` v6.9, `0x020D` and `0x163E` v6.14, `0x162B` v6.15, `0x1621` v7.0, `0x1633` v7.1 and `0x163D` v7.2. The Intel customer `0x0805` is admitted from the driver's introduction. |
| S28 | linux-hardware.org sensors logs of MSI MAG B550 TOMAHAWK (MS-7C91) [probe `2cfb05371e`](https://linux-hardware.org/?log=sensors&probe=2cfb05371e) and MSI MPG B550 GAMING PLUS (MS-7C56) [probe `2e5644f065`](https://linux-hardware.org/?log=sensors&probe=2e5644f065), retrieved November 29, 2026 | Independent captures of an `nct6687-isa-0a20` endpoint with the stock driver's labels in the same order on both boards: `AMD TSI Addr 98h`, `Diode 0 (curr)` of type thermal diode, then `Thermistor 15`, `Thermistor 1`, `Thermistor 16`, `Thermistor 0` and a second `Thermistor 15`. Each lists ten fan inputs. The TOMAHAWK capture shows 43,554 RPM on fan input 3 beside values from 605 to 933 RPM. The GAMING PLUS capture shows values from 744 to 2,390 RPM on fan inputs 1–4 and 6. The probes' kernel, firmware build and DMI records were not retrieved. |
| S29 | [MSI PRO B650-P WIFI report](https://bbs.archlinux.org/viewtopic.php?id=296903), Arch Linux forums, 2024, Linux 6.9.1 | First-hand stock-driver capture with `Thermistor 0` at 80 °C and no hot component found on inspection, beside the `AMD TSI Addr 98h`, `Diode 0 (curr)`, `Thermistor 1`, `Thermistor 15` and `Thermistor 16` sources. Fan input 1 at 718 RPM and fan inputs 2–10 at 0 RPM. |
| S30 | [ASRock X670E Taichi Linux thread](https://forum.level1techs.com/t/asrock-x670e-taichi-no-sensors-on-linux/199354?page=2), Level1Techs forums, July 2023 to January 2024 | First-hand reports that the driver found the chip but published no sensors before the customer admission, and that on Linux 6.7 the endpoint loaded with every fan at 0 RPM. No capture with labels and values is posted. |

## Detection

- Require manufacturer `Micro-Star International Co., Ltd.` and full
  board product `PRO B850-P WIFI (MS-7E56)` from Windows board
  identification. The board number `MS-7E56` is shared with another
  product (S2, S11).
- Require raw chip ID `0xD592` in configuration registers `0x20` and
  `0x21` at `0x4E/0x4F`, enabled logical device `0x0B` with base
  `0x0A20`, customer ID `0x020D`, and EC version 0.0 build
  2025-04-18. S8 observes that complete controller tuple under OEM
  firmware. S5 independently reports the same EC build on BIOS 2.A93.
  Firmware identity for this candidate is the EC tuple. Keep other
  raw IDs unsupported even if they match the Linux family mask
  `0xFFF0` against `0xD590` (S3, S5, S8).
- S2, S5 and S8 use different package suffixes for the controller.
  Use the exact board and raw controller tuple for attribution.

| Scope | Status | Default enablement |
| --- | --- | --- |
| MSI PRO B850-P WIFI (MS-7E56), raw `0xD592`, customer `0x020D`, EC 0.0 (2025-04-18) | S8 pins OEM routing, identity bytes and sample readback. Decode relies on lead-only S3. The entry, exit and application-port sequence rely on lead-only S9 and S10 as described below. Safe selector handling and signed-transport confirmation remain open. | Disabled. Experimental candidate after the blocking questions are resolved and the spec is approved. |
| Other MSI NCT6687D/-R deployments | S7 reports other builds, including MAG B850M MORTAR WIFI with EC 0.0 (2025-03-10). A separate profile needs raw ID, firmware identity and channel evidence. The installed-base ranking below records the evidence for the most common boards (S26, S28, S29). | Unsupported by this revision. Follow-up in #2696. |
| Direct Windows EC Space access on NCT6686D, including ASRock deployments | S1 pins the host interface, but its stated chip ID `0xC73A` conflicts with the `0xD441` observed on ASRock units (S14, S18) and the `0xD440` family match of lead-only S3. Application-port selector release and the S1 §7.2 protection lock are unresolved for readers outside the ISA mutex (see Open questions). S7 names multiple firmware builds. | Unsupported by this revision. The bound Linux endpoint is the selected first path. |
| ASRock B650 Steel Legend WiFi, bound Linux `nct6683` | Specified in [`superio-nuvoton-nct6686-asrock-linux-hwmon.md`](superio-nuvoton-nct6686-asrock-linux-hwmon.md) revision 1, with lead-only customer `0x163E` admission and a non-Intel refresh audit. | Experimental in that specification. |
| ASRock AMD BC-250, bound Linux `nct6683` | Specified in [`superio-nuvoton-nct6686-asrock-linux-hwmon.md`](superio-nuvoton-nct6686-asrock-linux-hwmon.md) revision 2. Two hardware probes there supply the exact DMI baseboard strings, a native binding, the numeric customer `0x162B` and labeled channel captures. Lead-only S12/S13 document native admission of the customer and the EC 1.0 (2021-07-28) build. | Experimental in that specification. |
| ASRock Z590 Taichi, bound Linux `nct6683` | Lead-only S12/S15 document native admission of customer `0x1621` and successful instantiation on two NCT6686D units with EC 1.0 (2021-01-25). | Disabled. Exact DMI strings and an attributable channel capture are missing. |
| ASRock X670E Taichi (`0x1631`), B650I Lightning WiFi (`0x1633`) and Z890 Pro-A (`0x163D`), bound Linux `nct6683` | Lead-only S22 records native admission and the board named by each commit. No exact-board channel capture with labels and values on a qualified endpoint was found. | Disabled pending exact DMI strings and a channel capture. |
| ASRock X600DC-ITX, bound Linux `nct6683` | S23 records BIOS 2.02, customer `0x165F`, EC 1.0 build 12/23/25 and plausible forced readings. The admission is applied to the hwmon tree but absent from released tags (S22). The report gives one fan value and no temperature channel labels beyond an AMD TSI source. | Disabled until a released admission and a labeled channel capture exist. |
| ASRock X670E Steel Legend and B850 Steel Legend WiFi, bound Linux `nct6683` | S18 needs force on EC 1.0 build 2022-07-19 with the customer unrecorded. S24 reports unadmitted customer `0x164A`. | Disabled. No native admission. |
| ASRock NCT6683D boards: X570 Phantom Gaming-ITX/TB3 (`0x0E2C`, EC 1.0 build 2019-06-28) and Z370M Pro4 (`0x0E1B`) | Lead-only S7/S22 record native admission and the tested EC build. These customers take the non-Intel refresh. No exact-board channel capture was found. | Disabled pending exact DMI strings and a channel capture. |
| Intel DH87MC, DH87RL, DB85FL and DZ87KLT-75K, bound Linux `nct6683` | Lead-only S12/S16 document the default Intel customer `0x0805` branch and tested NCT6683D EC 1.0 (2013-04-03) units. S17 records DH87MC's contributed channel mapping and the OEM controller description. S20 identifies the DZ87KLT-75K controller. Lead-only S7 and S16 state that this firmware uses undisclosed register locations, while every input refresh reads the generic fan-minimum words and reverse-engineered limit addresses. | Unsupported. The refresh cannot be stated as a read-only fact for this firmware. |
| Direct-access NCT6683D and other NCT6683D mechanisms | S7 reports firmware-dependent differences and an Intel OEM layout outside the public specification. No primary NCT6683D host-interface document was found. | Unsupported by this revision. |

### ASRock Linux bound-hwmon research

S14's BC-250 Fedora 41 observation reports `nct6686-isa-0a20` with
fan input 2 at 2,877 RPM and `Thermistor 14` and `Thermistor 15` at
45.5 °C and 45 °C. Those labels identify sensor sources and provide no
physical temperature location. The separate CoreOS log reports
configuration port `0x2e`, EC base `0x0a20`, platform instance
`nct6683.2592` and EC 1.0 build 2021-07-28. Revision 6 found two
linux-hardware.org probes that supply the missing same-unit BIOS,
DMI strings, customer and native binding. The
[ASRock specification](superio-nuvoton-nct6686-asrock-linux-hwmon.md)
revision 2 cites them. The Z590 Taichi two-unit report in S15 gives no
complete raw ID, EC base, BIOS version or individual channel values.

S12 confirms the native kernel matches `0xD440` under mask `0xFFF0`
and accepts both customer IDs without the force option. Normal binding
checks for ACPI conflicts and claims the I/O region. Binding can also start hardware
monitoring if it was disabled. A future Linux path would consume an
already bound hwmon endpoint and request no module actions. The
ordinary input attributes do not expose the raw ID, customer or EC
build. The force option is not exported as a sysfs parameter (S12).

Both input callbacks can refresh a shared cache. For the two customer
IDs, the enabled-channel refresh reads voltages and their high/low
limits, temperatures and their minimum, maximum, hysteresis and critical
limits, RPM and fan minimums, and PWM readback. The reads lie in
`0x0100`–`0x0167` and `0x0330`–`0x03D7`. The driver writes EC
page/index selectors and reads EC data. It makes no EC data or host
event writes during the refresh and does not select the driver's
designated monitor/fan status addresses (S12). These are driver access
facts. S1 §§7.2–7.3 and §8 place the underlying sensor semantics in the
separate EC Space specification. The [MSI NCT6687D Linux evidence](superio-nuvoton-nct6687-linux-hwmon.md)
covers its own OEM firmware profiles.

### Intel Linux bound-hwmon research

S12 matches NCT6683D as `0xC730` under mask `0xFFF0`. S17 names
`nct6683-isa-0a20` for DH87MC and labels fan inputs 1–3 as CPU,
front and rear. It provides neither live values nor a complete raw ID
for the unit. The Intel board document in S17 identifies the physical
headers and uses a different controller name. The public records do
not establish the same-unit BIOS, raw controller ID, native readings
and firmware ownership of the EC application window (S12, S16, S17).

The Intel branch reads measured voltages without their limits. It
reads temperature values, repeats each Intel maximum read for the
minimum and hysteresis positions, and reads the Intel critical limit.
It also reads RPM, PWM readback, and a generic fan-minimum word from
`0x03B8 + 2n` for each enabled zero-based fan input `n`. Both temperature and
RPM inputs can trigger this refresh. The driver hides `fanN_min` for
Intel and documents that its Intel address and encoding are unknown.
The generic minimum read also occurs in v3.16. The v7.0 refresh
writes selectors and has no EC data write or explicit monitor/fan
status access (S12, S16).

### Additional exact-board research

- **ASRock X670E Steel Legend, BIOS 3.01.** The Arch report identifies
  raw `0xD441` at the primary configuration port `0x2E`. Its follow-up
  reports EC base `0x0A20` and no new sensor entries after the driver
  found the chip. A separate Ubuntu unit exposes fan and thermistor
  values from EC 1.0 build 2022-07-19 with force enabled (S18). The
  static audit of the exact 3.01 ROM found 50 distinct checksum-valid
  AML tables. The only tables with `_WDG` were AMD CPMTBASL and
  AMD AOD. The extraction yielded no valid ECDT, recognized ACPI EC region
  or `PNP0C09` device, `_FST`, `_FPS`, `_FSL`, or attributable
  board-monitor query (S19). This finding is limited to the extracted
  tables. A native unforced binding with exact firmware identity and
  documented read effects, or a safe firmware-owned input path,
  remains unverified.
- **Intel DZ87KLT-75K.** Intel documents AA `G74721-003` and BIOS
  family `KLZ8711D.86A`. The OEM's BIOS 0446 notes identify corrections
  to two fan under-speed thresholds. The board inspection identifies
  NCT6683D (S20). The older driver test records EC 1.0 build 2013-04-03
  on DZ87KLT without a paired BIOS version, raw customer ID or live
  sensor values (S21, lead-only). Linux v7.3 and v7.4-rc2 retain the
  generic fan-minimum reads in the shared Intel refresh described
  above (S21, lead-only). The exact firmware's read effects and an
  independently attributable read-only firmware endpoint remain
  unverified. Both board candidates remain disabled.

### Path selection for the deferred profiles (#2696)

- **Linux, ASRock and AMD NCT6686D and ASRock NCT6683D.** The bound
  kernel endpoint is the selected first path. The kernel requests the
  application-port addresses, checks ACPI resource conflicts and
  serializes refreshes. The application performs no port I/O (S12).
  S1 §7.2 assigns that port set to application software and a separate
  set to BIOS and ACPI. Every admitted ASRock and AMD customer takes the
  non-Intel refresh, whose read set the
  [B650 Steel Legend WiFi specification](superio-nuvoton-nct6686-asrock-linux-hwmon.md)
  audits. It is the read set already accepted for the MSI profiles.
  A board profile still needs exact DMI strings, native admission of
  its customer in a released kernel, and a labeled same-board channel
  capture. Board identity is a mechanism gate there because the EC
  assignment differs by board. S7 of that specification lists two
  thermistor sources before the AMD TSI source. S14 lists the AMD TSI
  source first on BC-250. Revision 2 of that specification adds the
  BC-250 as a second board profile on the same endpoint and refresh
  audit. It is the second-ranked NCT6686D target by installed base and
  the first with all three prerequisites (see the ranking below).
- **Linux, Intel NCT6683D.** The kernel documents a special Intel
  firmware with undisclosed register locations. The shared refresh
  still reads the generic fan-minimum words and reverse-engineered
  limit addresses on every input read (S12, S16). No source states the
  semantics of those addresses on this firmware. The profile stays
  Unsupported.
- **Windows direct EC Space access.** The application would choose its
  own read set, limited to values, assignments and identity. The
  remaining blockers are chip identity and selector arbitration. S1's
  chip ID conflicts with observed hardware. The application port is
  shared with other Windows readers. Lead-only S25 shows one such
  reader writing `0xFF` to the page selector after a timeout, which
  releases another holder's selection under S1 §7.2. The exact signed
  `LpcIO` authorization of the EC window is unconfirmed on any ASRock
  tuple (S6). These remain Open questions.
- **Additional NCT6687D/-R firmware.** The MSI profiles in the
  [MSI NCT6687D Linux specification](superio-nuvoton-nct6687-linux-hwmon.md)
  are unchanged. Other MSI boards and the MSI direct-access candidate
  keep the requirements stated above. The ranking below records why
  the most common MSI boards are not in this slice.

### Installed-base ranking and dispositions (#2696)

S26 approximates installed base by the number of distinct computers
that linux-hardware.org lists for each baseboard model. The count
covers Linux users who submitted a probe. It ranks the targets and is
not a market share. The admission column gives the first Linux release
that admits the board's customer without force (S27). Rows are grouped
by #2696 target and sorted by count within each group. Every target
below that is not already Experimental, or selected here, stays
disabled or unsupported. The selected slice for this revision is the
AMD BC-250 profile alone. It is the smallest set that reaches the
Experimental gate, and it is the most common admitted NCT6686D target.

| #2696 target | Board | Controller | Customer, first admitting release | Computers (S26) | Disposition |
| --- | --- | --- | --- | --- | --- |
| NCT6686D on ASRock and AMD | ASRock X670E Steel Legend | NCT6686D (S18) | Unrecorded. Not admitted through v7.4-rc4. | 59 | Disabled. Needs a released admission of its customer, then a labeled capture on an admitting kernel. |
| NCT6686D on ASRock and AMD | ASRock AMD BC-250 | NCT6686D | `0x162B`, v6.15 | 49 | **Selected.** Experimental for `Thermistor 14`, `Thermistor 15` and fan inputs 1–5 in the [ASRock specification](superio-nuvoton-nct6686-asrock-linux-hwmon.md) revision 2. |
| NCT6686D on ASRock and AMD | ASRock B650I Lightning WiFi | NCT6686D (S22) | `0x1633`, v7.1 | 36 | Disabled. The admission thread posts no capture and no labeled same-board capture was found. |
| NCT6686D on ASRock and AMD | ASRock X670E Taichi | NCT6686D (S22) | `0x1631`, v6.7 | 26 | Disabled. No labeled capture was found. S30 reports every fan at 0 RPM on Linux 6.7, which a capture must explain before fan inputs are scoped. |
| NCT6686D on ASRock and AMD | ASRock B850 Steel Legend WiFi | Not established | `0x164A`, not admitted (S24) | 13 | Disabled. Needs a released admission. Its fan-control series is outside this project. |
| NCT6686D on ASRock and AMD | ASRock X670E Taichi Carrara | Not established | Unrecorded | 10 | Disabled. No controller, customer or capture evidence. |
| NCT6686D on ASRock and AMD | ASRock B650 Steel Legend WiFi | NCT6686D | `0x163E`, v6.14 | 9 | Experimental in the ASRock specification since revision 1. Unchanged. |
| NCT6686D on ASRock and AMD | ASRock Z590 Taichi | NCT6686D (S15) | `0x1621`, v7.0 | Not listed | Disabled. Needs exact DMI strings and a labeled capture. |
| NCT6686D on ASRock and AMD | ASRock Z890 Pro-A | Not established | `0x163D`, v7.2 | Not listed | Disabled. Needs exact DMI strings and a labeled capture. |
| NCT6686D on ASRock and AMD | ASRock X600DC-ITX | NCT6686D (S23) | `0x165F`, applied upstream but in no tag through v7.4-rc4 | Not listed | Disabled. Needs the released admission and a labeled capture. |
| NCT6683D on ASRock | ASRock X570 Phantom Gaming-ITX/TB3 | NCT6683D (S7) | `0x0E2C`, v5.12 | 27 | Disabled. No labeled capture was found. S22's commit notes that the customer also appears on other ASRock boards, so the customer alone does not identify a board's channel map. |
| NCT6683D on ASRock | ASRock Z370M Pro4 | NCT6683D-T (S22) | `0x0E1B`, v5.16 | 15 | Disabled. No labeled capture was found. |
| NCT6683D on Intel | Intel DH87RL, DB85FL, DH87MC and DZ87KLT-75K | NCT6683D (S17, S20) | `0x0805`, Intel branch | 26, 13, 11 and 1 | Unsupported. The Intel refresh reads addresses whose semantics are unknown on this firmware (Open questions). |
| Additional NCT6687D/-R | MSI MAG B550 TOMAHAWK | NCT6687D (S28) | Unrecorded. The first MSI customer, `0x0201`, is admitted from v5.11. | 373 | Deferred. Highest installed base of every target. Its S28 capture shows an implausible 43,554 RPM on fan input 3 that the common 1–65,534 RPM bound admits. A fan scope needs a second capture or a tighter validity rule. The probe's kernel, firmware build and DMI record were not retrieved. |
| Additional NCT6687D/-R | MSI MPG B550 GAMING PLUS | NCT6687D (S28) | Unrecorded. The first MSI customer, `0x0201`, is admitted from v5.11. | 318 | Deferred. Next candidate. Its S28 capture has labels and plausible values. A profile needs the same probe's DMI strings, kernel version and EC build, or another capture that records them. |
| Additional NCT6687D/-R | MSI PRO B650-P WIFI | NCT6687D (S29) | Unrecorded. MSI customers are admitted from v5.11. | 85 | Deferred. S29 reports a stock `Thermistor 0` value of 80 °C with no hot component, so that source needs a board-specific exclusion. |
| Additional NCT6687D/-R | MSI PRO X670-P WIFI | NCT6687D (S27) | `0x0207`, v6.9 | 52 | Deferred. No labeled capture was found. |
| Additional NCT6687D/-R | MSI MAG B850 TOMAHAWK MAX WIFI, MPG X870E CARBON WIFI, PRO B850-P WIFI and MAG B850M MORTAR WIFI | NCT6687D | `0x020D`, v6.14 | 48, 45, 27 and 13 | Experimental in the [MSI specification](superio-nuvoton-nct6687-linux-hwmon.md) revision 6. Unchanged. |
| Additional transports | Direct Windows EC Space access on every board above | NCT6686D and NCT6687D | Not applicable | Not applicable | Unsupported. The remaining blockers are the chip-ID conflict and application-selector arbitration with other readers (Open questions). These are missing mechanism and safety facts, not evidence identities, so no ADR 0031 mechanism gate can stand in for them. |

## Register map (facts)

EC addresses below are page in the high byte and index in the low byte.

| Address | Meaning | Units / encoding | Source |
| --- | --- | --- | --- |
| Super I/O configuration `0x07` | Logical device select. EC Space is logical device `0x0B`. | Byte | S3, corroborated for the documented NCT6686D by S1 §18.11 |
| EC logical device `0x30` bit 0 | EC Space enabled | Flag. S8 observes `0x01` for the full byte. | S3, S8 |
| EC logical device `0x60` and `0x61` | EC Space I/O base | Big-endian word. An eight-port window starting at `0x0A20` for this candidate. | S1, S3, S8 |
| Base + `4`, `5`, `6` | Application EC page selector, index selector and data, respectively | Byte I/O | S1 §7.2.5–7.2.7, S3, S8, S9 |
| EC `0x0602` and `0x0603` | Customer ID | Big-endian word | S3, S8 |
| EC `0x0604`, `0x0605`, `0x0606` | Build year within century, month, day | MSI uses binary byte values. The candidate tuple is decimal 25, 4, 18. | S3, S5, S8 |
| EC `0x0608`, `0x0609` | Major and minor firmware version | Binary byte each | S3, S5, S8 |
| EC `0x0180` bit 7 | Monitoring enabled | Flag. S8 observes `0x80` for the full byte. | S3, S8 |
| EC `0x01A0 + n` bits 6:0 | Assignment of monitoring input `n` (0–31). Bit 7 is separate. | Source code | S3, S4 |
| EC `0x0100 + 2n` and following byte | Temperature value of configured monitoring input `n` | Signed big-endian word. S3 reports half-degree increments after truncating the signed word divided by 128, then multiplying by 0.5 °C. | S3; S4 corroborates whole-degree values for a sample |
| EC `0x01C0 + n` bit 7 | Fan tachometer input `n` (0–15) enabled | Flag | S3 |
| EC `0x0140 + 2n` and following byte | Tachometer value for fan input `n` | Unsigned big-endian direct RPM | S3 |

| Sample input from S8 | Assignment or enable byte | Readback word |
| --- | --- | --- |
| Temperature slot 1 | `0x82` | `0x1A00` (26 °C) |
| Temperature slot 2 | `0x89` | `0x1600` (22 °C) |
| Temperature slot 3 | `0x8C` | `0x1A00` (26 °C) |
| Fan input 0 | `0xA1` | `0x0206` (518 RPM) |
| Fan input 1 | `0xA3` | `0x0000` (physical state unrecorded) |

## Read procedure and decode

1. Scope discovery to the identified MSI board and configuration pair
   at `0x4E/0x4F` (S6, S8). Acquire the ISA mutex before slot selection
   or any configuration operations. The candidate enters with two
   writes of `0x87` to the configuration index port and exits with
   `0xAA` to that port. S10 corroborates this sequence on the same
   board with Dasharo firmware. S9 performs a generic exit that also
   writes an ITE-specific register after `0xAA`. Those additional
   writes are outside the candidate scope.
2. Check the exact raw chip ID before logical-device selection. With
   logical device `0x0B` selected, verify EC Space is already enabled
   and its base is exactly `0x0A20`. `LpcIO` discovers already assigned
   BARs in configuration mode and restricts subsequent byte I/O to
   authorized windows.
   Discovery success alone does not confirm that the EC ports are
   authorized. Slot selection resets this executor's authorization
   state, so recheck access after selecting another slot or loading a
   new executor (S1, S3, S6).
3. Confirm the signed executor permits byte I/O to the discovered
   application EC window after exiting configuration mode. In the OEM
   capture, each EC byte read starts by writing `0xFF` to the application
   page selector, then selects the page and index and reads the data
   port (S8, S9). Use high byte first for each pair. S10 writes `0xFF`
   again at the end on the firmware EC port. The application port's
   post-read release behavior and arbitration require separate evidence.
4. Validate the customer, version and build tuple and check the
   monitoring-enabled bit before accessing channel readings (S3, S8).
   An unexpected tuple or a disabled monitor yields no readings. Do
   not start a disabled monitor.
5. Scope the first candidate temperature inputs to slots 1, 2 and 3.
   S8 observed their assignment bytes as `0x82`, `0x89`, `0x8C`.
   The corresponding seven-bit assignments are temperature diode or
   thermistor sources (S3). Check the low seven bits for the expected
   assignment at each slot. Keep a generic slot label pending evidence
   of its physical location. Skip disabled or different assignments.
   Reject all-ones reads and values outside −55 to +125 °C as
   defensive project gates.
6. Scope the first fan candidate to inputs 0 and 1 (S5, S8). Check
   the enable flag before reading. Emit only verified valid positive
   RPM. Treat zero, all-ones, failed or incomplete reads as unavailable
   under project gates. S8 observes a zero for an enabled input with
   unknown physical state. Keep generic input labels and defer any
   stopped-fan claim, header mapping and other channels.
7. Preserve independently valid channels and keep temperature and
   fan identity tied to the EC input slot. An EC failure does not
   suppress readings from a separate NCT679x controller.

## Quirks

- S4 shows slots configured differently from other MSI boards and
  separate AMD temperature telemetry on slot 0. Slot 0 and virtual
  temperatures are excluded from the initial motherboard profile.
- S1 states that the EC hardware-monitor implementation is provided
  by firmware. EC Space transport agreement does not establish an
  interchangeable sensor map.

## Safety notes

- Require `Global\Access_ISABUS.HTP.Method` with a bounded acquisition
  timeout throughout the configuration, BAR and multi-byte EC read
  sequences. When the lock is unavailable, skip collection (S6). The
  mutex serializes participants that acquire it. Establish protection
  for the application selector against OEM firmware and ACPI access
  separately before enabling this candidate.
- Use application EC ports only. Never write EC data, host events,
  logical-device enable, BARs, monitoring-enable, limits, tachometer
  configuration, fan PWM or power state. Only the documented
  selectors and mode transitions necessary for reads are candidates.

## Fixture acceptance for the candidate

| Case | Required result |
| --- | --- |
| Exact controller tuple, authorized window and synthetic valid samples matching S8 | Three independent temperatures and 518 RPM on fan input 0. Fan input 1 is unavailable. |
| Different board product with the same board number, unknown raw ID within the family, wrong base, or different EC tuple | Reject at the first mismatched gate before any channel reads. |
| Mutex timeout, denied BAR authorization, disabled logical device, or disabled monitoring | Skip channel reads and preserve results from other providers. |
| One temperature assignment mismatch, unavailable byte, all-ones word or out-of-range decode | Skip that input and preserve independently valid results. Cover both limits of the temperature gate. |
| Disabled fan input, zero, all-ones, invalid positive value or one failed byte of a pair | Skip that input and preserve independently valid results, without renumbering any slots. |
| Configuration or selector failure partway through collection | Exercise the evidenced cleanup path. Stop further transactions if it cannot release the mode or selector safely. |

## Open questions

- Blocking transport evidence: confirm configuration exit with only
  `0xAA` and scoped reads via the shipped signed `LpcIO` module on
  this exact OEM tuple with the monitor already enabled. S8 used
  ordinary byte port I/O for its EC reads. It does not establish
  signed executor authorization, timing or error handling (S6, S9).
- Blocking selector safety: verify whether the MSI firmware enables
  a selector protection mechanism like the one documented for
  NCT6686D in S1 §7.2. Establish the exact application page/index
  release behavior on success, access failure and timeout. Determine
  whether OEM firmware or ACPI can use the same window concurrently
  and what prevents interference for the proposed read path.
- Blocking fan validity: establish how a positive word is acquired
  coherently and how to reject invalid or stale positive readback on
  the exact firmware. The S8 snapshot establishes sample words and
  enable flags. It does not establish the firmware's error flag
  semantics or a stable snapshot protocol.
- Non-blocking for a verified positive-only RPM slice: a labeled
  observation or exact firmware documentation is needed to distinguish
  stopped and unpopulated inputs. Until then both are unavailable
  when readback is zero. Any later use of error/status registers needs
  evidence of their meaning and read side effects.
- Non-blocking for the proposed MSI slice: Intel and ASRock variants
  require their own profiles. The Nuvoton NCT6686D document shows
  `0xC7` and differing lower bytes in its global table and detailed
  description. S3 attributes an `0xD440` family to the production
  NCT6686D. Do not resolve this conflict by masking or aliasing IDs.
- **Linux BC-250 is resolved for Experimental use** in revision 2 of the
  [ASRock specification](superio-nuvoton-nct6686-asrock-linux-hwmon.md),
  which records the exact DMI strings, the native binding and the
  labeled captures. An EC 1.0 (2021-07-28) firmware map is still needed
  for Verified status.
- **Additional MSI NCT6687D boards remain blocking.** MAG B550 TOMAHAWK
  and MPG B550 GAMING PLUS have the largest installed base of every
  #2696 target (S26) and labeled stock captures (S28). Each needs exact
  DMI strings and a kernel and EC build record from the same unit. The
  TOMAHAWK fan inputs also need a rule that rejects the implausible
  positive value S28 shows. PRO B650-P WIFI needs its `Thermistor 0`
  source excluded or explained (S29). PRO X670-P WIFI needs a labeled
  capture.
- **Linux Z590 Taichi evidence remains blocking.** Associate EC 1.0
  (2021-01-25) with exact DMI strings and a native hwmon instance, and
  establish attributable published inputs with labels and types.
- **Other admitted ASRock boards remain blocking.** X670E Taichi,
  B650I Lightning WiFi, Z890 Pro-A, X570 Phantom Gaming-ITX/TB3 and
  Z370M Pro4 each need exact DMI strings and a labeled channel capture
  (S22). X600DC-ITX also needs its admission in a released kernel
  (S23). X670E Steel Legend and B850 Steel Legend WiFi need native
  admission (S18, S24).
- **Linux Intel NCT6683D is Unsupported.** Establish non-destructive
  semantics for the enabled refresh addresses on Intel EC 1.0
  (2013-04-03), especially the generic fan-minimum bank at
  `0x03B8`–`0x03D7` and the Intel limit addresses. Pin a board/BIOS
  association with a qualified native endpoint and an attributable
  reading, including how the OS and firmware share the application
  window. The lead-only sources describe these addresses as unknown for
  this firmware (S12, S16).
- **Windows direct NCT6686D access remains blocking.** Resolve the S1
  chip ID conflict with observed hardware. Establish whether ASRock
  firmware enables the S1 §7.2 protection lock and how a reader that
  holds the ISA mutex can avoid releasing another reader's selection,
  given the forced-release behavior in lead-only S25. Confirm that the
  signed `LpcIO` module authorizes the EC window on an exact ASRock
  tuple (S6).

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-07 | Initial draft of the MSI candidate and its firmware-dependent access, with unsupported scopes and unresolved read-only safety conditions. |
| 2 | 2026-11-08 | Pin the exact OEM controller capture, a configuration exit candidate, and sample data. Narrow the fan proposal to verified positive RPM and retain transport, selector and fan validity blockers. |
| 3 | 2026-11-09 | Record separate BC-250, Z590 Taichi and Intel DH87MC/DH87RL Linux hwmon research, native customer acceptance and unresolved firmware refresh evidence. |
| 4 | 2026-11-10 | Record ASRock X670E Steel Legend BIOS 3.01 and Intel DZ87KLT-75K evidence and their unresolved read paths. |
| 5 | 2026-11-28 | Select the bound Linux endpoint as the first path for ASRock NCT6686D and point B650 Steel Legend WiFi to its own specification. Record dispositions for the remaining #2696 targets, including admitted ASRock customers, X600DC-ITX and unadmitted boards. Mark Intel NCT6683D Unsupported because its refresh reads addresses with unknown semantics, and record the Windows chip-identity and selector blockers. |
| 6 | 2026-11-29 | Rank every #2696 target by Linux installed base (S26) with its first admitting release (S27) and a disposition. Select the AMD BC-250 as the next Experimental slice, specified in the ASRock specification revision 2, and resolve its Open question for Experimental use. Record MSI B550-series and PRO B650-P WIFI captures (S28, S29) and the X670E Taichi fan report (S30), and add the blockers for additional MSI boards. |
