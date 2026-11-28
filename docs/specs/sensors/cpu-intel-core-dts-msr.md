# Spec: Intel core temperature via digital thermal sensor MSRs

| Field | Value |
| --- | --- |
| Revision | 9 |
| Status | Implementation-ready (rev 9) |
| Scope | Read-only core DTS for the listed Intel x86-64 display models and full signatures in one Windows processor group and physical package. Includes DTS without package thermal management. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514), [#2700](https://github.com/shm11C3/HardwareVisualizer/issues/2700) (epic [#2510](https://github.com/shm11C3/HardwareVisualizer/issues/2510)) |

Revision 9 proposes Experimental rows for three more exact signatures of
the Alder Lake and Raptor Lake hybrid architecture: Alder Lake U and PS
`0x000906A4`, Raptor Lake S/HX C-0 `0x000B06F2` and Raptor Lake S H-0
`0x000B06F5`. They reuse the existing signed `IntelMSR` reads, the
hybrid per-target procedure and the single-group topology contract.
Implementation requires maintainer approval under the
[specification gate](README.md#status-transition-draft--implementation-ready).

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | [Intel SDM Volume 4, 335592-078US, December 2022](https://cdrdv2-public.intel.com/671098/335592-sdm-vol-4.pdf), Tables 2-1 and 2-2 (`19CH`, pp. 2-18–2-19), §2.4 and Table 2-6 (`19CH`, `1A2H`, p. 2-95), §2.4.1 and Table 2-10 (p. 2-107), §2.8 and Table 2-15 (`19CH`, `1A2H`, pp. 2-146–2-147), §2.11 and Table 2-20 (`19CH`, `1A2H`, pp. 2-183–2-185) | Primary model signatures and MSR scopes and fields |
| S2 | [Package DTS specification](cpu-intel-dts-msr.md) revision 3, sources S1–S3 and Detection/Register map | Intel SDM 325462-075US, June 2021, Volume 2A CPUID leaf 06H and Volume 3B §14.8.5.2, core DTS availability, semantics and field layout |
| S3 | [PawnIO interface](pawnio-interface.md) revision 7, `IntelMSR` section | `0x19C` and `0x1A2` read authorization and synchronous calling-processor execution |
| S4 | Microsoft Learn, [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex), [SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-system_logical_processor_information_ex), [PROCESSOR_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_relationship), [GROUP_RELATIONSHIP](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-group_relationship) and [PROCESSOR_GROUP_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_group_info) | Primary Windows active-core, package and group mask relationships |
| S5 | Microsoft Learn, [Processor Groups](https://learn.microsoft.com/en-us/windows/win32/procthread/processor-groups), [GROUP_AFFINITY](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-group_affinity), [SetThreadGroupAffinity](https://learn.microsoft.com/en-us/windows/win32/api/processtopologyapi/nf-processtopologyapi-setthreadgroupaffinity) and [GetCurrentProcessorNumberEx](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getcurrentprocessornumberex) | Primary Windows processor identity and thread affinity contract |
| S6 | [ADR 0029](../../adr/0029-windows-sensor-service.md), [service request handling](../../../core/src/sensor_service/windows/server.rs), [sampling cache](../../../core/src/sensor_service/windows/providers.rs), [service client](../../../core/src/sensor_service/windows/client.rs), [core affinity sampler](../../../core/src/infrastructure/providers/windows/pawn_io/cpu/core_temperature.rs) and [PawnIO client](../../../core/src/infrastructure/providers/windows/pawn_io/client.rs) | Project boundary and current call chain. The core pass uses a disposable thread and an independent synchronous PawnIO client. The service client has a two-second exchange deadline. |
| S7 | InstLatx64 [Bloomfield CPUID dump](https://instlatx64.github.io/InstLatx64/GenuineIntel/GenuineIntel00106A4_Bloomfield_CPUID.txt) and [Atom C2750 CPUID dump](https://instlatx64.github.io/InstLatx64/GenuineIntel/GenuineIntel00406D8_Avoton_CPUID2.txt), leaves 1 and 6 | Independent observations of the DTS-only capability combination. Neither dump establishes thermal readback or application behavior. |
| S8 | Intel, [Atom C2000 Product Family Specification Update, 329460-020US, June 2018](https://www.intel.com/content/dam/www/public/us/en/documents/specification-updates/atom-c2000-family-spec-update.pdf), Table 9 (p. 16) and erratum AVR32 (p. 26) | Primary retail CPUID corroboration and a low-temperature reporting limit on C2308 and C2508 |
| S9 | Intel, [SDM Documentation Changes, 252046-081, October 2025](https://cdrdv2-public.intel.com/868136/252046-081-sdm-change-document.pdf), replacement Volume 4 §2.17 and Table 2-39 (pp. 2-353–2-355), §2.17.5 (p. 2-393), and Volume 1 Tables 21-17 and 21-63 (pp. 21-29 and 21-66) | Primary hybrid display-model coverage, core scope for `0x19C`, hybrid MSR exceptions, hybrid flag and per-logical-processor core type. |
| S10 | Intel, [12th Generation Intel Core Processor Datasheet Vol. 1, 655258-011, May 2025](https://cdrdv2-public.intel.com/655258/655258-011.pdf), SHA-256 `826f5d275896887026720a413f2a913594ef070891dd864fd8e576685fe7204b`: Table 1 (pp. 13–14), §§4.1.3.1–4.1.3.2 (pp. 76–79), Table 18 (p. 92) and §15.1 Table 100 (p. 205), and [Specification Update, 682436-037, January 2026](https://cdrdv2-public.intel.com/682436/682436_037.pdf), §2.1 Table 2-1 and §3 Summary Tables of Changes | Primary exact CPUID for Alder Lake S and HX, core temperature reference and TCC offset, and errata applicability. Table 100 assigns the U15 and U9 2+8 lines to `906A4h`. Table 1 lists P- and U-line configurations from 6+8 down to 1+4. Table 18 lists a 100 °C junction maximum for the P/U 15 W lines, and a note that applies to U 9 W asks for a programmed TCC offset of 10. The DTS text applies to every processor line in the datasheet. |
| S11 | Intel, [13th and 14th Generation Intel Core Processor Datasheet Vol. 1, 743844-015, May 2025](https://cdrdv2-public.intel.com/743844/743844-015.pdf), SHA-256 `30644ce0bf2fe4eed956162f0d8d4fce4402e6694f1ea09be2b154517412f31e`: §§4.1.3.1–4.1.3.2 (pp. 80–83), §15.1 Table 101 (p. 219), and [Specification Update, 740518-028, February 2026](https://cdrdv2-public.intel.com/740518/740518_028.pdf), §2.1 and §3 Summary Tables of Changes | Primary exact Raptor Lake S/HX and S/HX Refresh CPUID, core temperature reference and TCC offset, and errata applicability. Table 101 also records the signature shared by the non-hybrid E line. It lists S/S Refresh 8P+8E C-0 and HX 8P+8E C-0 at `0xB06F2`, and S/S Refresh 6P+0E H-0 at `0xB06F5`. |
| S12 | Intel, [Best Practices for Real-Time Optimizations With the 12th Generation Intel Core Processors, 737973-1.1, December 2022](https://cdrdv2-public.intel.com/737973/737973_best_practices_for_real-time_optimizations_rev1.1.pdf), §§1.0, 2.1 and 2.4.1 and Table 2 (pp. 5–6 and 8–9), and [Affected Processors by CPU Model, April 2025 table](https://www.intel.com/content/dam/develop/public/us/en/include/software-security/affected-processors-2025tab-april2025.html), entries for `90672` and `B0671` | Primary corroboration of the hybrid core microarchitectures and the CPUID hybrid flag that distinguishes Raptor Lake S/HX from the same-signature non-hybrid Xeon E. |
| S13 | Intel, [12th Generation Intel Core Processor Specification Update, 682436-041, October 2026](https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/alder-lake-desktop/682436/041/identification-information/), Table 2-1 and [Summary Tables of Changes, H/P column](https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/alder-lake-desktop/682436/041/summary-tables-of-changes/), and [Affected Processors by CPU Model, February 2025](https://www.intel.com/content/dam/develop/public/us/en/include/software-security/affected-processors-2025tab-february2025.html), `906A3` entry | Primary exact Alder Lake H/P signature, errata applicability, and hybrid identification. S10's datasheet covers the H and P lines, including junction specifications in Table 18 (p. 92) and the DTS reference in §§4.1.3.1–4.1.3.2. |
| S14 | Intel, [12th Generation Intel Core for IoT Edge Specification Update, 743110-1.1, June 2023](https://cdrdv2-public.intel.com/743110/743110_Alder%20Lake%20-PS_Public_Specification%20Update_rev1.1.pdf), Tables 3–4 (pp. 8–9), §3.1 (p. 11), §4.1 Table 5 (pp. 12–15) and §5 (p. 16), and [IoT Edge Datasheet Vol. 1, 743329-001, September 2022](https://cdrdv2-public.intel.com/743329/743329_12thGen_Intel_Core_PS_Processor_Datasheet_Vol1_Rev001.pdf), §§4.1.3.1–4.1.3.2 (pp. 68–71) and §14.1 Table 62 (p. 143) | Primary exact Alder Lake-PS L-0 signature, separate errata applicability and explicit PS thermal register semantics. Table 3 of the specification update and Table 62 of the datasheet assign the PS 2+8 configuration to `906A4h`. Table 4 lists every `906A4` part as stepping R-0, with 2+8, 2+4 and 1+4 cores. Its §3.1 states that PS uses Alder Lake-P silicon on an LGA package. |
| S15 | Intel, [SDM Volume 4, 335592-093US, September 2026](https://cdrdv2.intel.com/v1/dl/getContent/671098), SHA-256 `79f54b67b9bb3836856f04e0c45f50406c4d4b932c64800eb788b6b705a83dbd` | Primary exact model mappings and readable thermal MSR definitions. See the scoped rows for the relevant sections and tables. |
| S16 | Intel, [Ivy Bridge desktop thermal guide, 326767-005, January 2013](https://www.intel.com/content/dam/www/public/us/en/documents/design-guides/3rd-gen-core-lga1155-socket-guide.pdf), §§6.2.2–6.2.3 (pp. 53–56), [mobile datasheet Vol. 1, 326768-006, June 2013](https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/3rd-gen-core-family-mobile-vol-1-datasheet.pdf), Digital Thermal Sensor (p. 80), and [Xeon E5 v2 datasheet Vol. 1](https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/xeon-e5-v2-datasheet-vol-1.pdf), §5.1.2 (p. 105) | Primary. Client DTS uses the calibrated reference independently of the activation offset. The E5 v2 text describes the DTS relative to the TCC and identifies the target MSR. |
| S17 | Intel, Haswell [desktop 328897-010](https://cdrdv2-public.intel.com/328897/4th-gen-core-family-desktop-vol-1-datasheet.pdf), §§5.6–5.9, [mobile M/H 328901-007](https://cdrdv2-public.intel.com/328901/4th-gen-core-family-mobile-m-h-processor-lines-vol-1-datasheet.pdf), §§5.6.1–5.6.2, and [mobile U/Y 329001-007](https://cdrdv2-public.intel.com/329001/4th-gen-core-family-mobile-u-y-processor-lines-vol-1-datasheet.pdf), §§5.6.1–5.6.2 | Primary product thermal reference. The DTS readout is independent of the TCC activation offset. |
| S18 | Intel, Broadwell [desktop 332376-001v1](https://www.mouser.com/datasheet/2/612/desktop-5th-gen-core-family-datasheet-vol-1-765515.pdf) (Intel document mirror), §5.9 (p. 76), [mobile 332378-001v1](https://www.intel.com.br/content/dam/www/public/us/en/documents/datasheets/mobile-5th-gen-core-family-datasheet-vol-1.pdf), §5.6.2 (p. 76), and [Core M 330834-001v1](https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/core-m-processor-family-datasheet-vol-1.pdf), §§5.6.1–5.6.2 | Primary product thermal reference. The DTS readout is independent of the TCC activation offset. |
| S19 | InstLatx64, commit `17be9a9793c306cf96f8c286e34e705b780a0226`: [Clarkdale](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel0020652_Clarkdale_CPUID.txt#L180-L183), [Gulftown](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00206C2_Gulftown_CPUID.txt#L468-L471), [Ivy Bridge](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00306A9_IvyBridge_CPUID.txt#L350-L353), and [Ivy Bridge-E](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00306E4_IvyBridgeE_CPUID.txt#L515-L518) | Independent register observations. They corroborate the model mechanisms and do not establish Windows PawnIO operation or the accuracy of individual readings. |
| S20 | Intel, [6th Generation Intel Processor Family Specification Update, 332689-024](https://www.scribd.com/document/557109734/332689-024-6th-generation-intel-processor-family-spec-update) (Intel document mirror), erratum SKL135 | Primary erratum describing DDR4 systems with wider DTS error than specified. |
| S21 | PawnIO 2.2.0 [driver read native](https://github.com/namazso/PawnIO/blob/5cdf470831fdfff3f7f1d06363ca6b230f3bf35a/PawnIO/src/natives_impl_windows.cpp#L437-L448) with [PawnIOLib status conversion](https://github.com/namazso/PawnIO/blob/5cdf470831fdfff3f7f1d06363ca6b230f3bf35a/PawnIOLib/PawnIOLib.cpp#L109-L115), and the [IntelMSR 0.2.11 read contract](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/IntelMSR.p#L102-L118) from S3 | Upstream interface behavior only. The documented signed read path returns a failing status when the native MSR read faults. |
| S22 | Intel, [Core i7 LGA2011-v3 Specification Update, 330841-001, August 2014](https://images-na.ssl-images-amazon.com/images/I/71JArr6MenL.pdf), Table 2 (p. 7) and errata HSH11 and HSH18 (pp. 13–14), and [Core i7 LGA2011-3 Thermal Guide, 330842-001, August 2014](https://szarka.ssgg.sk/Vyuka/2014/HighEnd-LGA2011/core-i7-lga2011-3-tmsdg.pdf), §§4.1.1, 4.2.5 and 4.3.2 (pp. 27, 31–32) (Intel document mirrors) | Primary desktop stepping, errata, DTS reference and TCC offset field. |
| S23 | Intel, [Xeon E5 v3 Specification Update, 330785-011, September 2017](https://www.mouser.com/pdfdocs/xeon-e5-v3-spec-update.pdf), Table 1 (pp. 7–8), and [330785-014US, April 2022](https://discuss.88.io/uploads/short-url/mACN9UgMYkSAc3qJfZ0dMtSc4VA.pdf), Table 1 (pp. 7–8) and HSE14 and HSE22 (pp. 20, 22) (Intel document mirrors). Intel [microcode update guidance, August 2018](https://www.intel.com/content/dam/www/public/us/en/documents/sa00115-microcode-update-guidance.pdf), Haswell E, Server EP/EP4S and Server EX rows. | Primary signature overlap and errata. The 2017 revision includes embedded and four-socket models as well as the E5-1600/2600 v3 lines. The microcode table distinguishes E7 v3 Server EX at `0x000306F4`. |
| S24 | Intel, [Xeon E5-1600/2600/4600 v3 Thermal Guide, 330786-003, October 2015](https://cdrdv2-public.intel.com/330786/xeon-e5-v3-thermal-guide.pdf), §§4.1.1, 4.2.1 and 4.3.2 (pp. 34, 36–37), and [Xeon E5-2400 v3 Thermal Guide, 331593-001US, January 2015](https://manualzz.com/doc/10867174/intel-xeon-e5-2400-v3-processor-thermal-mechanical-specif...), §§5.3.2, 5.4 Table 6 and 5.8.4 (pp. 38, 41, 46) (Intel document mirror) | Primary server thermal reference. The first guide locates the offset field. The second defines the target-plus-signed-DTS relationship for the embedded products, includes E5-1428L v3 in its thermal table and refers to the shared Xeon E5/E7 v3 register specification. |
| S25 | InstLatx64, commit `17be9a9793c306cf96f8c286e34e705b780a0226`, [Core i7-5820K](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00306F2_HaswellE2_CPUID.txt#L564-L567), [Xeon E5-2630 v3](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00306F2_HaswellEP_00_CPUID.txt#L765-L770), [Xeon E5-2660 v3](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00306F2_HaswellEP2_CPUID.txt#L1774-L1779), and [Xeon E5-4669 v3](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00306F2_HaswellEP_05_CPUID.txt#L1624-L1629) | Independent CPUID and register observations on the full signature with DTS capability, a valid core delta and a zero-offset target. They do not verify the Windows PawnIO path. |
| S26 | Linux commit [6ae0092ca7ad](https://github.com/torvalds/linux/commit/6ae0092ca7adfce79336c6525cb0da561ecd4f04), Intel-authored [model masks](https://github.com/torvalds/linux/blob/6ae0092ca7adfce79336c6525cb0da561ecd4f04/drivers/thermal/intel/intel_tcc.c#L34-L77) and [CPUID model mapping](https://github.com/torvalds/linux/blob/6ae0092ca7adfce79336c6525cb0da561ecd4f04/arch/x86/include/asm/intel-family.h#L87-L92) | **lead-only (copyleft)**. Corroborates the four-bit offset and seven-bit core delta for display model `06_3F`, including the offset width for embedded E5 v3 parts whose public guide refers to the shared register specification. |
| S27 | Linux [Intel-authored thermal mask change](https://github.com/torvalds/linux/commit/6ae0092ca7adfce79336c6525cb0da561ecd4f04), June 2024, [author's v2 test summary](https://lkml.rescloud.iu.edu/2404.3/02686.html), and Linux v6.18 commit `7d0a66e4bb9081d75c82ec4957c50034cb0ea449` [model IDs](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/arch/x86/include/asm/intel-family.h#L137-L140), [mask](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/thermal/intel/intel_tcc.c#L38-L43) and [model selection](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/thermal/intel/intel_tcc.c#L96-L98) | **lead-only (copyleft)** for the conflicting eight-bit readout mask explicitly selected for `06_97`, `06_9A` and `06_B7`. The author reports testing on Alder Lake without giving an exact CPUID or raw status words. That test summary does not identify Raptor Lake. |
| S28 | InstLatx64, commit `17be9a9793c306cf96f8c286e34e705b780a0226`: [Core i9-12900K](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel0090672_AlderLake_01_LC_BC_CPUID.txt), [Core i7-12800HX](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel0090672_AlderLake_03_CPUID.txt) and [Core i9-12900H](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00906A3_AlderLakeP_00_CPUID.txt) | Independent exact-signature P- and E-core CPUID and status observations. The captured words with Reading Valid set all have bit 23 clear. |
| S29 | InstLatx64, commit `17be9a9793c306cf96f8c286e34e705b780a0226`: [Core i9-13900K](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00B0671_RaptorLake_01_CPUID.txt) and [Core i9-14900K](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00B0671_RaptorLake_13_CPUID.txt) | Independent desktop `0x000B0671` CPUID and P- and E-core status observations. Both captures have bit 23 clear in every captured status word with Reading Valid set. HX coverage comes from S11 and S15. |
| S30 | Intel, *13th Generation Intel Core, Intel Core 14th Generation, Intel Core Processor (Series 1) and (Series 2), Intel Xeon E 2400 and Intel Xeon 6300 Processor Specification Update*, 740518 revision 033, November 1, 2026: [Identification Information, Table 2-1](https://edc.intel.com/content/www/us/en/design/products/platforms/details/raptor-lake-s/13th-generation-core-processor-specification-update/identification-information/), [Summary Tables of Changes](https://edc.intel.com/content/www/us/en/design/products/platforms/details/raptor-lake-s/13th-generation-core-processor-specification-update/summary-tables-of-changes/) and [Errata Details](https://edc.intel.com/content/www/us/en/design/products/platforms/details/raptor-lake-s/13th-generation-core-processor-specification-update/errata-details/), consulted 2026-11-29 | Primary. Table 2-1 assigns `B06F2h` to RPL-S 8P+8E, RPL-S Refresh 8P+8E and RPL-HX 8P+8E, and `B06F5h` to RPL-S 6P+0E and RPL-S Refresh 6P+0E. The errata table has S/S Refresh columns for 8+16, 8+8 and 6+0, and HX/HX Refresh columns for 8+16 and 8+8. It lists RPL001–RPL081. No title names a thermal, temperature, DTS or PROCHOT condition, and no detail names `IA32_THERM_STATUS` or `MSR_TEMPERATURE_TARGET`. |
| S31 | Intel, *12th Generation Intel Core Processor Specification Update*, 682436 revision 042, October 1, 2026: [Identification Information, Table 2-1](https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/alder-lake-desktop/682436/042/identification-information/), [Summary Tables of Changes](https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/alder-lake-desktop/682436/042/summary-tables-of-changes/), U column, and [Errata Details](https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/alder-lake-desktop/682436/042/errata-details/), consulted 2026-11-29 | Primary. Table 2-1 assigns `0x906A4` to ADL-U15W 2+8 and ADL-U9W 2+8. The errata table lists ADL001–ADL095. ADL034, the only title that names PROCHOT#, a thermal condition or temperature, describes a hang when PROCHOT# is asserted during BIOS initialization. No detail names `IA32_THERM_STATUS` or `MSR_TEMPERATURE_TARGET`. |
| S32 | Intel, [Affected Processors: Consolidated Product CPU Model](https://www.intel.com/content/www/us/en/developer/topic-technology/software-security-guidance/processors-affected-consolidated-product-cpu-model.html), 2025–2026 tab, rows for `06_9AH` stepping 4 and `06_BFH` steppings 2 and 5, consulted 2026-11-29 | Primary product grouping and hybrid identification. `06_9AH` stepping 4 has two rows. The Alder Lake U row (12th generation Core, Pentium Gold and Celeron, mobile and embedded) gives `CPUID.0x7.EDX[15] = 1`. The Arizona Beach row (Gracemont, Intel Atom, embedded) gives `EDX[15] = 0`. The `06_BFH` row (Raptor Lake S and Bartlett Lake S) gives `EDX[15] = 1` and lists CPUID values `B06F2`, `B06F5`, `B06F6` and `B06F7`. |
| S33 | InstLatx64, commit `17be9a9793c306cf96f8c286e34e705b780a0226`: [Core i5-13500](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00B06F2_RaptorLakeS_02_CPUID.txt) (lines 6, 14–15, 57, 67–69, 1001, 1595–1597 and 2161), [Core i3-1220P](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00906A4_AlderLakeP_00_CPUID.txt) (lines 1–49, 55, 63–64, 106, 115–117, 414, 993–998 and 1411–1414) and [Pentium Gold 8505](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00906A4_AlderLakeP_01_CPUID.txt) (lines 6, 14–15, 57, 67–69, 211, 492–497 and 710–713) | Independent exact-signature observations. Each file associates CPUID and MSR words by logical processor within one system. The i3-1220P capture records Windows 11. The i5-13500 file has `0x19C` but no `0x1A2` words. They do not verify the HardwareVisualizer Windows PawnIO path. |
| S34 | Linux v6.18 commit `7d0a66e4bb9081d75c82ec4957c50034cb0ea449`, [model identity](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/arch/x86/include/asm/intel-family.h#L142), [thermal mask](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/thermal/intel/intel_tcc.c#L42) and [model selection](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/thermal/intel/intel_tcc.c#L100) | **lead-only (copyleft)**. Linux selects the same eight-bit core readout mask, which includes bit 23, for display model `06_BF` at every stepping. This conflicts with S15 Table 2-39. S27 already records the same selection for `06_9A`. |

## Detection

- CPUID leaf 0 reports `GenuineIntel` and a maximum basic leaf at least
  06H. Use the CPUID.01H display family and display model from Table 2-1
  (S1, S2).
- CPUID.06H:EAX[0] advertises core DTS. CPUID.06H:EAX[6] independently
  advertises the package MSR. The latter may be clear on a usable core
  DTS path. A clear bit 6 permits no package-status MSR read (S2).
  The Bloomfield and Atom C2750 observations in S7 report leaf-6 EAX
  values `0x00000003` and `0x00000005`, respectively. Both have bit 0
  set and bit 6 clear.
- Windows returns one `RelationProcessorCore` record per active
  physical core. Its group mask contains the logical siblings of that
  core. A `RelationProcessorPackage` record identifies the logical
  processors of a physical package. Use the group number and
  group-relative processor number together (S4, S5).
- Haswell-E and Xeon E5 v3 qualify only at full signature
  `0x000306F2` on discovery and on the pinned processor (S15, S22,
  S23). The signature also covers embedded Xeon E5-1428L v3 and
  E5-2400 v3 parts. S24 documents their thermal reference separately.
- The hybrid rows require the full CPUID.01H:EAX signature below and
  CPUID.07H.00H:EDX[15] set. Apply these gates before topology discovery
  and check them again on each pinned representative. A shared signature
  without the hybrid bit does not qualify. The exact `0x000B06F5` row is
  the one exception, defined in its own bullet below. The required Windows
  topology checks are defined under Project collection constraints
  (S9–S14, S30–S32).
- Alder Lake H/P and Alder Lake-PS L-0 share full signature
  `0x000906A3` (S13, S14). Both have separate product evidence for the
  temperature reference (S10, S14), so the profile uses their shared
  signature without a product-name test. The signature selects the
  documented profile. A sample still requires the hybrid flag, per-target
  core type and DTS capability, valid MSR results and the topology gates.
  S14 lists PS L-0 products with both P- and E-cores. S13 lists the
  Alder Lake U15W and U9W lines at `0x000906A4`, and S14 gives that
  signature to PS R-0. The exact `0x000906A3` row excludes both groups
  even when they advertise hybrid and DTS capability. They qualify only
  through the separate `0x000906A4` row.
- Alder Lake U and PS R-0 share full signature `0x000906A4` (S10
  Table 100, S14, S31). The datasheets for both product groups specify
  the temperature reference (S10, S14). The signature selects the
  documented hybrid profile without a product-name test. S32 also lists
  an Arizona Beach Intel Atom product at display model `06_9AH` stepping 4
  with the hybrid flag clear. The required hybrid flag excludes it.
- Raptor Lake S, S Refresh and HX 8P+8E C-0 share full signature
  `0x000B06F2` (S11 Table 101, S30). Their datasheet specifies the
  temperature reference (S11). The common hybrid gates apply.
- Raptor Lake S and S Refresh 6P+0E H-0 use full signature `0x000B06F5`
  (S11 Table 101, S30). S11 specifies its temperature reference. The
  configuration has no E-cores. S32 lists `CPUID.0x7.EDX[15] = 1` for
  `06_BFH` steppings 2 and 5, and no independent capture of this
  signature is cited. S15 §2.17 applies Table 2-39 to display model
  `06_BFH`. §2.17.5 applies hybrid Tables 2-46–2-48 to the same model,
  and none of them lists `19CH` or `1A2H` (pp. 2-400–2-405). The core
  status and target fields therefore do not depend on the hybrid flag.
  This row admits a target with the flag in either state. With the flag
  set, the pinned target must report core type `0x40`. With the flag
  clear, leaf 1AH is not consulted. The pinned target must report the
  same flag state that discovery found. Discovery requires a maximum
  basic leaf of at least 07H in both states.

The runtime gates follow [ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md).

| Condition | Gate kind | Source |
| --- | --- | --- |
| Intel x86-64, available CPUID leaves, and a listed display family/model or required full signature | mechanism | S1, S9–S15, S22–S23, S30–S31. Selects the applicable core status and target MSRs. |
| Core DTS bit set on discovery and on the pinned processor | mechanism | S2. Package thermal management independently gates the package path. |
| For a listed hybrid signature other than `0x000B06F5`, hybrid flag set and the pinned processor reports a supported core type | mechanism | S9–S14, S30–S32. Selects the applicable hybrid status and reference mechanism. |
| For exact `0x000B06F5`, the hybrid flag state found at discovery, also reported by the pinned processor. A set flag requires core type `0x40` | mechanism | S11, S15, S30, S32. The flag selects whether leaf 1AH qualifies the target. The status and target fields are the same in both states. |
| Validated single active Windows group/package, with non-overlapping physical-core masks covering all active processors | mechanism | S4, S5 and project attribution policy. |
| Pinned processor and expected CPUID signature match before reads, processor still matches after reads, and topology still matches before publication | safety | S4, S5 and project attribution policy. Rejects samples with unconfirmed attribution. |
| Existing signed IntelMSR transport is available, and the dedicated sampling thread successfully restores affinity | safety | S3, S5, S6. Restricts privileged access and prevents further work on a thread with uncertain affinity. |
| Reading Valid is set and the row's offset condition passes | mechanism | S1, S2, S9–S11, S14–S18, S22, S24, S26, S30–S31. Qualifies the applicable DTS reference and status. |
| Both reads succeed and the reference and result pass the existing plausibility bounds | safety | S3, S21 and project sample policy in the read procedure. |

The legacy rows select the documented mechanism by display family/model
or an explicitly required full signature. Each pinned representative
must match the full discovered signature for attribution. Observed
product names are evidence identities. For a signature with no
applicable row in this scope, skip both core MSRs before topology
discovery even if CPUID advertises core DTS. Separately approved core DTS scopes retain their own gates.

| Scope | Status | Default enablement |
| --- | --- | --- |
| Nehalem `06_1A`, `06_1E`, `06_1F`, `06_2E` | Verified by S1 §2.8 and Table 2-15 for signatures and target. S2 pins the core DTS and separate PTM bit. | Enabled with direct target decode and the capability, affinity and validity gates. |
| Sandy Bridge `06_2A`, `06_2D` | Verified by S1 §2.11 and Table 2-20 for signatures and target. S2 pins the core DTS and separate PTM bit. | Enabled with direct target decode and the capability, affinity and validity gates. |
| Silvermont Atom C2000 `06_4D` | Verified by S1 §2.4/§2.4.1 and Tables 2-6/2-10 for the signature, core DTS and package target. S2 pins the capability bits. | Enabled only with a zero target offset and the common gates. |
| Hybrid Alder Lake S/HX `06_97`, full signature `0x00090672` (stepping ID 2) | S9/S10 pin the signature, P- and E-core status and reference. S12 corroborates hybrid identification. S28 observes bit 23 clear on both core types. S27 is Linux lead-only for the conflicting status width addressed in the read procedure. | Experimental, subject to the common gates and per-target core type below. |
| Hybrid Alder Lake H/P and PS `06_9A`, full signature `0x000906A3` (stepping ID 3) | S13/S14 pin the H/P and PS identities and applicable errata. S9 documents core thermal status for `06_9A`, S10/S14 pin the reference for the respective products, and S12/S13 corroborate hybrid identification. S28 observes bit 23 clear on both core types. S27 is Linux lead-only for the conflicting status width addressed in the read procedure. | Experimental, subject to the common gates and per-target core type below. |
| Hybrid Raptor Lake S/HX and S/HX Refresh `06_B7`, full signature `0x000B0671` (B-0) | S9/S11 pin the signature, P- and E-core status and reference. S11/S12 identify the non-hybrid signature overlap. S29 observes bit 23 clear on both core types on desktop. S27 is Linux lead-only for the conflicting status width addressed in the read procedure. | Experimental, subject to the common gates and per-target core type below. |
| Hybrid Alder Lake U and PS R-0 `06_9A`, full signature `0x000906A4` (stepping ID 4) | S10 Table 100 and S31 Table 2-1 assign the U15 and U9 2+8 lines, and S14 assigns PS R-0. S15 §2.17 Table 2-39 gives the core status for `06_9AH`. S10 and S14 §§4.1.3.1–4.1.3.2 give the calibrated reference independent of the TCC offset. The S31 U column and S14 list no erratum for the scoped reads. S32 identifies the hybrid flag for the Alder Lake U row. S33 observes the signature, the hybrid flag, P and E core types and valid `0x19C`/`0x1A2` pairs with bit 23 clear on a Core i3-1220P and a Pentium Gold 8505. These product names are evidence identities and best-evidenced members. They do not gate the read. S27 is the lead-only (copyleft) source for the conflicting status width, and no scope depends on it. | Experimental, subject to the common gates and per-target core type below. The same signature with the hybrid flag clear (S32 Arizona Beach) is disabled. |
| Hybrid Raptor Lake S, S Refresh and HX 8P+8E C-0 `06_BF`, full signature `0x000B06F2` (stepping ID 2) | S11 Table 101 and S30 Table 2-1 pin the signature and product lines. S15 §2.17 Table 2-39 gives the core status for `06_BFH`, and its hybrid tables add no thermal row. S11 §§4.1.3.1–4.1.3.2 give the calibrated reference independent of the TCC offset. The S30 8+8 columns list no erratum for the scoped reads. S32 identifies the hybrid flag. S33 observes the signature, the hybrid flag, P and E core types, and valid status words with bit 23 clear on a Core i5-13500. That capture has no target word. The product name is an evidence identity. S34 is the lead-only (copyleft) source for the conflicting status width, and no scope depends on it. | Experimental, subject to the common gates and per-target core type below. |
| Raptor Lake S and S Refresh 6P+0E H-0 `06_BF`, full signature `0x000B06F5` (stepping ID 5) | S11 Table 101 and S30 Table 2-1 pin the signature and its P-core-only configuration. S15 and S11 give the same status fields and reference as the C-0 row. The S30 6+0 column lists no erratum for the scoped reads. S32 lists the hybrid flag set for this stepping. No independent capture of this signature is cited. S34 is the lead-only (copyleft) source for the conflicting status width, and no scope depends on it. | Experimental, with the flag-state gate of the Detection table: core type `0x40` when the flag is set, and no leaf 1AH check when it is clear. |
| Westmere `06_25`, `06_2C`, `06_2F` | S15 §§2.9–2.10 (pp. 2-221–222) apply Table 2-15 (pp. 2-182–183). Core status, thread target, and no offset field in the target layout. S19 observes both MSRs on the Core i5-650 `0x00020652` and Core i7-990X `0x000206C2`. | Experimental. Direct target decode. |
| Ivy Bridge `06_3A` | S15 §2.12 (p. 2-264) applies Table 2-20 (pp. 2-230, 2-232). S16 documents the offset-independent reference for desktop and mobile. S19 observes both MSRs on the Core i7-3770K `0x000306A9`. | Experimental. Direct target decode. |
| Ivy Bridge-E `06_3E` | S15 §§2.12.1–2.12.2 (pp. 2-267, 2-279) apply Tables 2-20 and 2-26 (p. 2-269 for target). S16 corroborates the reference for Xeon E5 v2. S19 observes both MSRs with zero target offset on the Core i7-4930K `0x000306E4`. | Experimental. Zero offset in bits 27:24. |
| Haswell client `06_3C`, `06_45`, `06_46` | S15 §2.13 (p. 2-287) applies Table 2-20. S17 documents the offset-independent reference for desktop and mobile. | Experimental. Direct target decode. |
| Haswell-E and Xeon E5 v3, full signature `0x000306F2` | S15 Table 2-1 and §2.14 apply the core status and target fields from Table 2-20 (pp. 2-230–232, 2-305). S22–S24 pin the desktop and server scope and thermal reference. Lead-only S26 corroborates the offset width. S25 observes the mechanism on a Core i7-5820K and Xeon E5-2630, E5-2660 and E5-4669 v3. | Experimental. Zero offset in bits 27:24. |
| Broadwell client `06_3D`, `06_47` | S15 §2.15 (p. 2-333) applies Table 2-20. S18 documents the offset-independent reference for desktop, mobile and Core M. | Experimental. Direct target decode. |
| Broadwell Xeon `06_4F`, `06_56` | S15 §§2.16–2.16.2 (pp. 2-337, 2-348, 2-351) document the E5/E7 v4 and Xeon D layout in Table 2-36 (pp. 2-340–341). Both the core status and package target are explicit. | Experimental. Zero offset in bits 27:24. |
| Silvermont `06_37`, `06_4A`, `06_5A`, `06_5D` and Airmont `06_4C` | S15 §§2.4 and 2.4.2 (pp. 2-123, 2-142) apply Table 2-6 (p. 2-127) with core status and package target. | Experimental. Zero offset in bits 29:24, as for the existing `06_4D` mechanism. |
| Goldmont `06_5C`, Goldmont Plus `06_7A`, and Tremont `06_86`, `06_96`, `06_9C` | S15 §§2.5–2.7 (pp. 2-144, 2-170, 2-175) retain the Table 2-6 thermal rows. Table 2-12 also identifies bits 27:24 as programmable when its platform-info bit is set. The full Table 2-6 field supplies the conservative zero gate. | Experimental. Zero in bits 29:24. |
| Pre-Nehalem Core 2 and earlier Atom | S15 §2.2 Table 2-3 and §2.3 Table 2-4 list core thermal status without establishing a readable calibrated `0x1A2` target. | Disabled. Follow-up in #2700. |
| Other signatures, unrecognized hybrid core types, or multiple processor groups/packages | Outside the evidence or attribution scope. Existing package collection remains covered by the [package DTS specification](cpu-intel-dts-msr.md). | Disabled for core reads. Follow-up in #2700. |

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| MSR `0x19C` | `IA32_THERM_STATUS` | 31 | Core digital readout valid when set | Flag | S1 and S15 Table 2-2, S2, S9 Table 2-39 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 22:16 | Core digital readout below the DTS reference | Unsigned degrees Celsius | S1 and S15 Table 2-2, S2, S9 Table 2-39 |
| MSR `0x1A2`, listed legacy models | `MSR_TEMPERATURE_TARGET` | 23:16 | Reference temperature for the row's applicable offset condition | Degrees Celsius | S15 Tables 2-6, 2-15, 2-20, 2-26 and 2-36. S16–S18 specify offset independence for the direct-decode client rows. |
| MSR `0x1A2`, listed Atom models | `MSR_TEMPERATURE_TARGET` | 29:24 | Target offset field in the inherited register definition | Degrees Celsius | S1 and S15 Table 2-6 |
| MSR `0x1A2`, models `06_3E`, `06_4F`, `06_56` | `MSR_TEMPERATURE_TARGET` | 27:24 | Programmable TCC activation offset | Degrees Celsius | S15 Tables 2-26 and 2-36 |
| MSR `0x1A2`, exact `0x000306F2` | `MSR_TEMPERATURE_TARGET` | 27:24 | Programmable TCC activation offset | Degrees Celsius | Product-specific S22 and S24, with lead-only S26 for the embedded width. The generic Table 2-20 in S15 marks bits 63:24 reserved and does not define the product offset. |
| MSR `0x1A2`, the listed hybrid signatures | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated Tjmax, the DTS reference | Degrees Celsius | S10/S11/S14 §§4.1.3.1–4.1.3.2 |
| MSR `0x1A2`, the listed hybrid signatures | `MSR_TEMPERATURE_TARGET` | 29:24 | Programmable TCC activation offset, which affects the thermal control threshold | Degrees Celsius | S10/S11/S14 §4.1.3.1 |

## Read procedure and decode

1. Select one representative logical processor per Windows core record.
   Use the least set bit in its group mask as its session-local identity.
   Keep core identity independent of sampling order or sample validity.
   The mask's other bits are SMT siblings, so they do not produce
   additional temperature channels (S4, S5).
2. On the synchronous sampling thread, set the representative's group
   affinity with just its bit and capture the previous affinity. Confirm
   the running processor with `GetCurrentProcessorNumberEx`. Skip the
   core if affinity could not be set or the group/number differs.
   Confirm the expected CPUID vendor, signature and DTS support on the
   target processor before reading. For a hybrid signature other than
   `0x000B06F5`, also require the hybrid flag, a supported CPUID.1AH.00H
   leaf on that processor and EAX[31:24] of `0x40` (Intel Core, P-core)
   or `0x20` (Intel Atom, E-core). Skip the target before MSR access if
   the flag is clear, the leaf is absent or it reports another type.
   On exact `0x000B06F5`, require the pinned processor to report the
   hybrid flag state found at discovery. With the flag set, require the
   leaf and accept only `0x40`. With the flag clear, do not consult
   leaf 1AH.
   The type is a qualification input. The existing core identity
   remains the Windows group and representative processor number
   (S1, S2, S4, S5, S9, S12, S15, S30, S32).
3. Read `0x1A2` and `0x19C` through the same calling processor using the
   signed `IntelMSR` module. Check the running group and processor
   again after the reads while still pinned, and discard the core
   result if either differs. Require successful reads, a target byte
   in 50–120 °C inclusive and core status bit 31 set. On exact
   `0x00090672`, `0x000906A3`, `0x000906A4`, `0x000B0671`, `0x000B06F2`
   and `0x000B06F5`, leave the core unavailable if status bit 23 is set.
   Apply this check before interpreting the seven-bit readout (S15
   Table 2-39, S27–S29, S33–S34, project gate). For a
   zero-offset row, reject the sample when the selected target mask is nonzero.
   Use mask `0x0F000000` for `06_3E`, exact `0x000306F2`,
   `06_4F` and `06_56`, and `0x3F000000` for the listed Atom models,
   including `06_4D`. Direct-decode rows apply no offset mask
   (S1–S3, S5, S15–S18, S22, S24, S26).
4. Subtract the unsigned digital readout from the target. Apply the
   existing plausibility gate `0 < temperature ≤ target`. A valid
   digital readout of zero returns the target temperature. The
   offset-independent legacy and hybrid references use the target byte
   at both zero and nonzero TCC offsets. For the hybrid rows this also
   holds when the TCC averaging window changes.
   A faulted read returns a failing transport status and leaves the
   core unavailable. A zero target, clear validity bit or failed
   plausibility gate likewise leaves it unavailable (S2, S10/S11/S14,
   S15–S18, S21, project gate).
5. Restore the previous affinity after each successful pin, including
   failed read paths. Admit each core result only after affinity
   restoration succeeds (S5, project gate).

## Project collection constraints

- Use the existing typed sensor-service boundary and CPU acquisition
  cadence. Core MSR reads require current monitoring demand and a due
  sample. Share a rate-limited acquisition among concurrent requests.
  A cache hit retains its original acquisition time. Without a request
  that needs core temperatures, perform no core topology queries or
  affinity changes (S6, project policy).
- For every request that needs core temperatures on a matching CPU,
  validate the active-group, package and core relationships, including
  on a cache hit. Accept exactly one active group and one physical
  package in that group. The package mask must equal that group's
  active mask. Each core mask must be nonempty, contained in the
  package mask and disjoint from the other core masks. Their union
  must equal the package mask.
  Use the group number, active/package masks and unordered set of core
  masks to compare topology snapshots. A malformed or failed enumeration
  yields no core data (S4, project gates).
- Collect on a dedicated disposable service thread, never on the shared
  blocking pool. Allow one core pass in flight per service, with at
  most one target and two MSR reads per validated core and no retry
  sweep. The wait budget must fit within the existing service exchange
  deadline and allow the batch to return without core results. On
  expiry, stop scheduling targets, discard the expired result and wait
  for that pass to exit before starting another one. An already-running
  synchronous PawnIO call can continue past the deadline. A timed-out
  core call must not block unrelated providers through a shared
  executor or sampler lock (S3, S6, project policy).
- An affinity restore failure invalidates the entire core pass and its
  cached core results. Terminate the dedicated thread without performing
  any more sensor operations on it. A later pass requires a fresh
  thread and fresh topology validation. A pinned thread must never
  return to a shared worker pool (S5, S6, project policy).
- Recheck the topology before publishing newly acquired core readings.
  If enumeration fails or the snapshots differ, discard the core set
  and invalidate the core cache. If the current topology differs from
  the topology of a cached set, discard that cache even when it is still
  within its rate limit. Core identifiers are session-local and scoped
  to the validated topology. Never transfer a previous temperature to a
  new core because its representative processor number was reused
  (S4, project policy).
- Preserve usable siblings for isolated read failures within a
  successfully validated pass. Core-pass failure leaves existing
  package readings and ACPI fallback independently available. Core
  temperatures never populate package temperature or package
  thermal-status fields.

### Synthetic acceptance fixtures

| Fixture | Expected behavior |
| --- | --- |
| Exact supported signature, core DTS set, PTM clear | Eligible for core target/status reads. No `0x1B1` read. A valid core temperature never replaces the package/ACPI value. |
| Exact supported signature, core DTS set, PTM set | Eligible for core reads. The existing package path samples independently. |
| Exact signature, core DTS clear, PTM clear or set | No core MSR reads in either case. Package behavior remains governed by its existing gates. |
| Unsupported signature, otherwise valid capabilities and topology, including Core 2, earlier Atom, or deferred Skylake | No core topology query or core MSR reads. Core DTS alone does not qualify a target read. |
| Each added legacy display model with a valid target of 100 °C and a valid 30 °C digital readout | 70 °C under its documented mechanism. Use an unobserved synthetic brand and repeat with PTM clear to verify that neither a product name nor PTM gates core collection. |
| Ivy Bridge, Haswell client and Broadwell client with a valid nonzero activation offset, 100 °C target and valid 30 °C digital readout | 70 °C. The activation offset does not change the reference. |
| Zero-offset legacy rows with bit 24 set, then only bit 28 set, with otherwise valid reads | Bit 24 rejects all zero-offset rows. Bit 28 rejects the Atom rows only. Ivy Bridge-E, exact Haswell-E/Xeon E5 v3 and Broadwell Xeon ignore bits outside their 27:24 mask. |
| Exact `0x000306F2` with DTS, one active package and group, and S25 Core i7-5820K or Xeon E5-2630 v3 words. Repeat with an unobserved brand and PTM clear. | Decode 31 °C and 27 °C respectively. The full signature and capability select the core mechanism. The brand is fixture evidence, and PTM controls only the independent package path. |
| Exact `0x000306F2` with a successful all-zero target read, another out-of-range target, bit 27 set, invalid status, or a fault, with a valid sibling | Leave only the failing core unavailable. Preserve its sibling and independent package or ACPI data. In particular, HSH18/HSE22's all-zero target cannot pass the range gate. |
| Exact `0x000306F2` with valid Windows topology masks and an inconsistent CPUID.0BH core-level count, then malformed Windows masks or a second active group/package | Select only the Windows core records in the first case. Reject the invalid or unsupported topology before core MSR access. |
| `0x000306F4` or any other `06_3F` signature with core DTS set, or discovery at `0x000306F2` and a pinned signature that differs | No core MSR reads for those targets. Discovery of an unsupported signature also skips core topology enumeration. |
| Supported legacy discovery, pinned representative reports a different signature or either MSR read fails | For a signature mismatch, skip both MSRs on that target. For a transport failure, omit that core even if an output cell is zero or populated. Preserve valid core siblings and independent package/ACPI results. |
| Each exact hybrid signature except `0x000B06F5` (P-cores only) with one SMT P-core and multiple E-cores in one group and package, all advertising core DTS | One temperature per Windows physical-core record. The P-core's second logical processor adds no channel. Each E-core keeps its own slot even when several E-cores share other hardware resources. |
| Hybrid full signature `0x000B0671`, `0x000906A3`, `0x000906A4` or `0x000B06F2` with hybrid flag clear, or another full signature such as `0x000906A2`, `0x000906A5`, `0x000B06F1`, `0x000B06F6` or `0x000B06F7` | No new core topology query or core MSR reads under this specification, even if an unsupported stepping advertises DTS and the hybrid flag. `0x00090675` follows its own [specification](cpu-intel-alder-lake-s-h0-dts-msr.md). |
| Exact hybrid discovery succeeds, but an individual pinned target has a different full signature, no core DTS, no leaf 1AH where leaf 1AH is required (every hybrid-flag-set target; not `0x000B06F5` with the flag clear), or an unknown core type | Skip that target before MSR access. Preserve eligible core siblings and the independent package/fallback result. |
| Each hybrid signature with Tjmax 100 °C, TCC offset 10 °C, and valid digital readouts of 30 and 0 | 70 °C and 100 °C respectively. Changing the TCC offset or its averaging window does not change either decoded value. |
| `0x000906A3` with valid P- and E-core targets and different per-target Tjmax words, each sampled with zero and nonzero TCC offsets | Use each target's own reference with the same 7-bit delta and validity gate on both core types. An invalid E-core status leaves its slot unavailable and preserves a valid P-core and independent package data. |
| Each exact `0x00090672`, `0x000906A3`, `0x000906A4`, `0x000B0671`, `0x000B06F2` and `0x000B06F5`, synthetic 100 °C target with status `0x889E0000` on one physical core and `0x881E0000` on another, both with Reading Valid set | Leave the bit-23-set sample unavailable and return 70 °C for the other core. Repeat with the two core types exchanged where the row has both types. |
| Exact `0x000906A4` with the hybrid flag set in one group and package. Use the S33 Core i3-1220P pairs `0x80640000 / 0x883A2A82` (P-core) and `0x80640000 / 0x88322800` (E-core), and the S33 Pentium Gold 8505 pairs `0x80640000 / 0x883F0282` (P-core) and `0x80640000 / 0x88400282` (E-core). Repeat with an unobserved brand and PTM clear | 42 °C, 50 °C, 37 °C and 36 °C. Neither the brand nor PTM changes admission. |
| Exact `0x000906A4` with the hybrid flag clear, as S32 lists for Arizona Beach, and otherwise valid capabilities and topology | No core topology query or core MSR reads under this specification. |
| Exact `0x000906A4` with target word `0x8A640000` (TCC activation offset 10, as a note to S10 Table 18 asks for U 9 W) and status `0x881E0000` | 70 °C. The offset does not change the decode. |
| Exact `0x000B06F2` with the hybrid flag set, the S33 Core i5-13500 status words `0x88520000` on a P-core and `0x884C0000` on an E-core, and a synthetic target word `0x00640000` | 18 °C and 24 °C. The live target of each representative is used. |
| Exact `0x000B06F5` with the hybrid flag set, one group and package, and four or six SMT P-cores whose pinned targets report core type `0x40` | One target per physical core. The second logical processor of each core adds no channel. |
| Exact `0x000B06F5` with the hybrid flag set, where one pinned target reports `0x20`, an unknown type, or no leaf 1AH | Skip both reads on that target. Preserve eligible siblings and independent package and fallback data. |
| Exact `0x000B06F5` with the hybrid flag clear, and leaf 1AH reporting zero, `0x40`, or a maximum basic leaf below 1AH | The same admission in all three cases. Leaf 1AH is not consulted. |
| Exact `0x000B06F5` where a pinned target reports a hybrid flag state that differs from discovery | Skip both reads on that target. Preserve eligible siblings. |
| Exact `0x000B06F5` with target word `0x00640000` and status `0x881E0000`, once with the flag set and once with it clear | 70 °C in both cases. |
| One package with SMT siblings, reordered enumeration and one failed target or core read | One representative per physical core. Reordering alone preserves identities. The isolated failure leaves its position unavailable and preserves valid siblings. |
| Topology changes during a fresh pass or between cache acquisition and reuse | No output from the mismatched core set. Invalidate the cached core set even if its age is acceptable. Verify that reused representative processor numbers do not inherit old values. |
| Restore fails after one successful core and one attempted core | Discard the entire core set and cache. No subsequent reads on the disposable thread. A later pass uses a fresh thread. Package and ACPI results remain independent. |
| No demand, concurrent requests, rate-limit hit or expired wait | No idle reads. Requests share at most one in-flight pass. Cache age stays tied to acquisition. No second pass starts while a timed-out pass is still in flight. |
| Synthetic target and status words at validity and temperature boundaries | Accept targets of 50 °C and 120 °C and reject 49 °C and 121 °C. Accept a valid zero digital readout at the target. Reject missing or zero targets, clear Reading Valid, and a decoded temperature of zero or less. |

## Quirks

- The same-target read follows the model-specific scope in S1 and S15.
  Table 2-15 labels `0x1A2` Thread for Nehalem and Westmere.
  Table 2-20 labels it Unique for Sandy Bridge and the rows that
  inherit its target definition, including `0x000306F2`. Tables 2-6,
  2-26 and 2-36 label it Package for the corresponding Atom, Ivy Bridge-E
  and Broadwell Xeon rows. S3 documents calling-processor execution for the signed module.
  Its package-scope wording applies where S1 or S15 documents a
  package-scope target.
- S9 specifies core scope for hybrid `IA32_THERM_STATUS` and lists
  P- and E-core MSR exceptions separately. Use the same pinned processor
  for both thermal reads, as on the existing path. On `06_9A`, the
  core status, digital readout and valid flag apply to both core types.
  The H/P and PS datasheets independently describe the DTS delta as
  relative to the factory-calibrated target with a separate TCC offset.
  The H/P and PS errata tables list no exception to the scoped DTS
  reads (S9, S10, S13, S14).
  Core numbering and labeling remain based on Windows core records (S4).
- The rev 9 signatures have these S33 observations. Every capture
  reports leaf 6 EAX `0x00DF8FF7` (core DTS and PTM set) and a leaf 7
  EDX word with bit 15 set. Every status word with Reading Valid set has
  bit 23 clear.

| S33 capture | Logical CPUs and core types | P-core words (`0x1A2` / `0x19C`, lines) | E-core words (`0x1A2` / `0x19C`, lines) | Decoded |
| --- | --- | --- | --- | --- |
| Core i5-13500, `0x000B06F2` | 20: CPUs 0–11 report `0x40` (6 SMT P-cores), CPUs 12–19 report `0x20` (8 E-cores) | none / `0x88520000` (CPU 0, 1597) | none / `0x884C0000` (CPU 12, 2161) | Readouts 82 and 76. No target word was captured. |
| Core i3-1220P, `0x000906A4` | 12: CPUs 0–3 report `0x40` (2 SMT P-cores), CPUs 4–11 report `0x20` (8 E-cores) | `0x80640000` / `0x883A2A82` (CPU 0, 998 and 995) | `0x80640000` / `0x88322800` (CPU 4, 1414 and 1411) | 100 °C target and zero offset. 42 °C and 50 °C. |
| Pentium Gold 8505, `0x000906A4` | 6: CPUs 0–1 report `0x40` (1 SMT P-core), CPUs 2–5 report `0x20` (4 E-cores) | `0x80640000` / `0x883F0282` (CPU 0, 497 and 494) | `0x80640000` / `0x88400282` (CPU 2, 713 and 710) | 100 °C target and zero offset. 37 °C and 36 °C. |

- The i3-1220P is a P-line 2+8 product (S10 Table 1 lists a 28 W 2+8
  P configuration). It reports `0x000906A4`, although S10 Table 100
  names only the U15 and U9 lines at that signature. The row is keyed by
  the signature and hybrid mechanism, so this product is an observed
  member, not an additional gate (S10, S33, ADR 0031).
- A note to S10 Table 18 that applies to the U 9 W line asks designs to
  program a TCC offset of 10 and a Tau value into MSR `1A2h`. The DTS
  readout remains relative to the calibrated target (S10 §4.1.3.2), so
  the decode ignores that offset.
- Intel errata HSH18 and HSE22 document an erroneous all-zero
  `0x1A2` result. Reject it through the per-read target range even if
  the transport succeeds. HSH11 and HSE14 affect the CPUID.0BH
  core-level EBX[15:0] logical processor count. Attribution uses
  validated Windows active-core, package and group masks and confirms
  the running processor. It does not use that CPUID count (S4, S5,
  S22, S23).
- No universal TjMax is assumed. Pre-Nehalem Core and earlier Atom
  signatures that lack an evidenced target MSR need a separate
  processor-specific temperature reference before they can be enabled.
- Intel erratum AVR32 documents a core DTS lower limit of −27 °C on
  Atom C2308 and C2508, with no workaround (S8). The core path's
  positive-temperature gate excludes that range. The separate uncore
  sensor covered by that erratum is outside this specification.
- The core value represents the Windows topology record it came from.
  A missing core cannot shift another reading into its position. No
  relationship to a persistent system-wide physical core ID is claimed.

## Safety notes

- Read only the documented MSRs, on a confirmed processor and with the
  corresponding capability and model gates. The transport fault path
  handles failure of an eligible read. Never use it to probe an
  undocumented target. Do not write an MSR, change thresholds, clear
  logs, or override firmware configuration (S1–S3, S15, S21).
- Affinity is per-thread. Keep the affinity interval synchronous and
  bounded to the target's reads. Do not repurpose a process-wide
  affinity mask as the physical core map (S3–S5).

## Open questions

- Non-blocking for #2514 single-group scope: only the listed mechanisms and one physical package and active group qualify.
  Other hybrid signatures need their own applicability evidence.
  Multi-package/group systems need a separately validated attribution
  and affinity-restore contract.
- Non-blocking for the legacy extension: each zero-offset row rejects a nonzero offset within its specified mask.
  Wider nonzero-offset coverage requires a model-specific DTS reference.
  The signed Windows path has no real-hardware validation for the new rows.
- Non-blocking for the legacy extension: Skylake `06_4E` and `06_5E` remain outside this table.
  S20 records an asymmetric DTS error on DDR4. Separate approved
  specifications define any admitted scope.
- Non-blocking for Haswell-E and Xeon E5 v3: this scope requires exact `0x000306F2` and zero activation offset.
  Other `06_3F` signatures and nonzero offsets require applicable
  product evidence. Windows PawnIO operation has not been verified on
  this signature.
- Non-blocking for #2514 initial scope: topology attribution uses matching validated observations and confirmed read targets.
  The Windows topology interface cannot distinguish a topology change
  that occurs and reverses between observations.

- Non-blocking for the Alder Lake H/P and PS scope: Intel documents the exact signature and both product lines' DTS semantics.
  The supported Windows PawnIO path has not been verified on real
  hardware for this signature.
- Non-blocking for the exact Alder Lake and Raptor Lake hybrid scope: collection rejects core status bit 23 when set before applying Intel's seven-bit decode.
  Intel SDM 335592-093US §2.17 Table 2-39 (pp. 2-359–2-361)
  marks bit 23 reserved for `06_97`, `06_9A`, `06_B7` and `06_BF`. The
  applicable hybrid common and core-type tables supply no replacement
  for this register (§2.17.5, Tables 2-46–2-48).
  Linux includes bit 23 in its masks for all four display models (S27
  and S34, lead-only). S28, S29 and S33 corroborate clear bit 23 on P and
  E targets across `0x00090672`, `0x000906A3`, `0x000906A4`,
  `0x000B0671` and `0x000B06F2`. No status word is cited for
  `0x000B06F5`. The meaning of a set bit remains unresolved. Reject only
  the affected per-core sample on these exact signatures. Package status
  follows the existing independent contract.
- Non-blocking for the #2700 exact `0x000B06F5` Experimental scope: both hybrid flag states lead to the same status and target decode, and each pinned target must match the discovered state.
  S32 lists the flag set for `06_BFH` steppings 2 and 5. The
  [Alder Lake-S H0 specification](cpu-intel-alder-lake-s-h0-dts-msr.md)
  records four captures of the separate `0x00090675` signature with the
  flag clear, although the same Intel table lists the flag set for that
  client row (S5 of that specification).
  No capture of `0x000B06F5` is cited, so the flag state on shipped
  parts is not established. This row admits either state on the basis
  of S15 §2.17, which applies the same status and target definitions to
  display model `06_BFH` regardless of core type. The Alder Lake-S H0
  specification still disables `0x00090675` with the flag set. Aligning
  that row is a separate revision of that specification.
- Non-blocking for the #2700 exact `0x000906A4` Experimental scope: the hybrid flag excludes the same-signature Arizona Beach product, and Intel documents the U and PS thermal reference.
  S32 lists an Arizona Beach Intel Atom product at `06_9AH` stepping 4
  with the flag clear. This revision cites no thermal datasheet for that
  product, so it stays disabled. S33 records a P-line 2+8 product at
  this signature, which S10 Table 100 does not list there. The S31 U
  column contains ADL034, a hang when PROCHOT# is asserted during BIOS
  initialization. It does not concern the scoped reads. No
  HardwareVisualizer Windows PawnIO run exists for this signature.
- Non-blocking for the #2700 exact `0x000B06F2` Experimental scope: the reference comes from the S11 datasheet and is read live on every target.
  The S33 i5-13500 capture contains status words but no `0x1A2` word,
  so no observed target value is cited for this signature. No HX 8P+8E
  capture and no HardwareVisualizer Windows PawnIO run exist. Graduation
  of a member to Verified uses the
  [#2500](https://github.com/shm11C3/HardwareVisualizer/issues/2500)
  Hardware Compatibility Report path.

## Deferred-scope disposition (#2700)

This table records the state of each #2700 target after revision 9. A
disposition is not a fact row. The cited sources and specifications
hold the facts.

| #2700 target | Disposition after revision 9 |
| --- | --- |
| Pre-Nehalem Core 2 and earlier Atom | Disabled and unchanged. #2700 records that the published Atom D400/D500 and Core 2 E8000/E7000 design limits are not the per-unit calibrated TCC activation point. Enablement needs a vendor-documented readable per-device reference, or an exact alternate path that already exports attributable absolute core temperatures. |
| Alder Lake U and PS R-0 `0x000906A4`, hybrid flag set | Experimental in this revision. |
| Raptor Lake S, S Refresh and HX 8P+8E C-0 `0x000B06F2` | Experimental in this revision. |
| Raptor Lake S and S Refresh 6P+0E H-0 `0x000B06F5` | Experimental in this revision, with the flag-state gate. |
| Alder Lake-S H0 `0x00090675` | Experimental with the hybrid flag clear in [its specification](cpu-intel-alder-lake-s-h0-dts-msr.md) revision 1. The set-flag state stays disabled there. |
| `0x000906A4` with the hybrid flag clear (S32 Arizona Beach Intel Atom) | Disabled. No product thermal datasheet is cited. |
| `0x00090672` with the hybrid flag clear | Disabled. No identification or capture of that state is cited. |
| Arrow Lake H `0x000C0652` and U `0x000B0650` | Deferred to a revision of the [Arrow Lake specification](cpu-intel-arrow-lake-dts-msr.md). InstLatx64 commit `17be9a9793c306cf96f8c286e34e705b780a0226` has a Core Ultra 9 285H capture (`GenuineIntel00C0652_ArrowLakeH_04_CPUID.txt`) and a Core Ultra 7 265U capture (`GenuineIntel00B0650_ArrowLakeU_03_CPUID.txt`) with per-processor `0x19C`/`0x1A2` words. That revision needs the H and U datasheet thermal sections, the H and U errata columns, and low-power E-core attribution as in the [Meteor Lake specification](cpu-intel-meteor-lake-dts-msr.md). |
| Lunar Lake LP E-cores (type `0x20` on `0x000B06D1`) | Unavailable and unchanged. The [Lunar Lake specification](cpu-intel-lunar-lake-dts-msr.md) needs the physical granularity of the shared LP E module. |
| Panther Lake `06_CCH` and Wildcat Lake `06_D5H` | Deferred. S15 lists both display models in Table 2-1 only. §2.17 does not name them. The one public Panther Lake capture located (InstLatx64 `GenuineIntel00C06C3_PantherLakeL_01_CPUID.txt`, Core Ultra 7 355 at `0x000C06C3`) has no MSR section. Enablement needs a product datasheet DTS section and a register observation. |
| Bartlett Lake S `0x000B06F6` and `0x000B06F7` | Deferred. S32 lists both under `06_BFH`, but the S11 and S30 identification tables do not. Enablement needs the product identification and thermal documents. |
| Other `06_BAH` and `06_B7H` steppings | Disabled. S30 Table 2-1 lists only `B06A2h`, `B06A3h` and `B0671h` for these display models. |
| Emerald Rapids `06_CFH`, Granite Rapids `06_ADH`/`06_AEH`, Sierra Forest `06_AFH` and other Sapphire Rapids steppings | Deferred. S15 §2.17 includes these display models, but each needs product thermal reference evidence and, for most configurations, multi-group topology. They rank below the client targets for this application. |
| Larger or multi-package topology | Unchanged. The exact two-group `0x000806F8` scope is in the [Sapphire Rapids specification](cpu-intel-sapphire-rapids-dts-msr.md) revision 2. Every other multi-group or multi-package topology stays disabled for core reads. Package readings and partial core results keep their existing contracts. |

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-07 | Initial fact-only draft for the listed single-group, single-package models and independent DTS/PTM detection. |
| 2 | 2026-11-07 | Documented service placement, bounded request-driven collection, topology and affinity failure handling, and synthetic acceptance fixtures. |
| 3 | 2026-11-08 | Proposed transition to Implementation-ready for the exact single-group, single-package scope. Pinned the model-specific target scopes and Windows group-mask definitions and recorded DTS-only CPUID observations and the scoped Atom erratum. |
| 4 | 2026-11-08 | Propose Implementation-ready scope for two exact Alder Lake and Raptor Lake hybrid signatures. Pin per-core qualification, Tjmax and its relationship to the TCC offset, and the existing single-group topology contract. |
| 5 | 2026-11-08 | Propose Implementation-ready Experimental coverage for exact `0x000906A3` Alder Lake H/P and PS. Pin separate product evidence and reuse the existing hybrid and single-group topology gates. |
| 6 | 2026-11-09 | Propose additional Experimental legacy DTS mechanisms with explicit target decoding and offset masks. Classify runtime gates under ADR 0031. |
| 7 | 2026-11-10 | Propose Implementation-ready Experimental coverage for exact `0x000306F2` at zero TCC activation offset. Pin desktop, server and embedded product evidence, the zero-target errata and the topology count exclusion. |
| 8 | 2026-11-10 | Propose a fail-closed core DTS status check for disputed bit 23 on exact hybrid signatures `0x00090672`, `0x000906A3` and `0x000B0671`. |
| 9 | 2026-11-29 | Propose Implementation-ready Experimental coverage for exact Alder Lake U and PS R-0 `0x000906A4` and Raptor Lake S/HX C-0 `0x000B06F2`, both with the hybrid flag set, and Raptor Lake S H-0 `0x000B06F5` with a flag-state gate, pending maintainer approval. Pin the identities from the current Intel specification updates and datasheets, the hybrid identification, three independent captures and the lead-only `06_BF` mask conflict. Extend the bit 23 rejection to the three signatures, and add fixtures and the #2700 disposition. |
