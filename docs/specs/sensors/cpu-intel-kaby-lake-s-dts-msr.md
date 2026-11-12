# Spec: Intel Kaby Lake-S core DTS on the ASUS STRIX Z270F GAMING

| Field | Value |
| --- | --- |
| Revision | 2 |
| Status | Implementation-ready (rev 2) |
| Scope | Proposed Experimental Windows core DTS for `0x000906E9` on the factory ASUS STRIX Z270F GAMING LGA1151/Z270 platform, with one active processor group and one physical package. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel [SDM Volume 4, 335592-090US, February 2026](https://cdrdv2-public.intel.com/874253/335592-090-sdm-vol-4.pdf), Tables 2-2 and 2-39 and §§2.17–2.17.1 | Primary core scope, valid bit and digital readout layout. The model-level sections include `06_9E`, which has several steppings and product lines. |
| S2 | Intel [Kaby Lake S and Core X-series Datasheet Vol. 1, 335195-005, February 2022](https://cdrdv2-public.intel.com/335195/335195_005.pdf), Table 1-1 and §§5.1.5.1–5.1.5.2.1, pp. 85–88 | Primary calibrated reference and offset-independent DTS readout. The specified accuracy is ±5 °C within the operating range. |
| S3 | Intel [7th and 8th Generation Core Processor Specification Update, 334663-017, November 2024](https://cdrdv2-public.intel.com/334663/334663_017.pdf), Summary Tables of Changes and KBL071, KBL072, KBL074, KBL111 and KBL113, pp. 43–44, 53–54 | Primary errata for the covered client product lines. SHA-256 `6157ef8f454b96cdbd0dc48670567401816c693c04f2b3838e6a985514d97ca1`. |
| S4 | Intel [processor servicing table](https://www.intel.com/content/www/us/en/support/topics/support-and-servicing-for-processors.html), 7th Generation desktop rows, [SA-00329 exact product table](https://www.intel.com/content/dam/www/public/us/en/security-advisory/documents/sa00329-microcode-update-guidance.pdf), Kaby Lake S/H/G/X and Xeon E3 rows, and [August 2024 consolidated model table](https://www.intel.com/content/dam/develop/public/us/en/include/software-security/affected-processors-2022tab-august2024.html), `06_9E` stepping 9 row | Primary identity. `0x000906E9` covers Kaby Lake S, H, G, X and Xeon E3 v6. Intel's Coffee Lake entries have other full signatures. |
| S5 | ASUS [STRIX Z270F GAMING manual E12193](https://dlcdnet.asus.com/pub/ASUS/mb/LGA1151/STRIX_Z270F_GAMING/E12193_STRIX_Z270F_GAMING_UM_V2_WEB.pdf), specifications summary and §§1.1.3–1.1.4, pp. 1-4–1-5, [board specifications](https://rog.asus.com/us/motherboards/rog-strix/rog-strix-z270f-gaming-model/spec/) and [CPU support table](https://www.asus.com/us/supportonly/strix%20z270f%20gaming/helpdesk_cpu/), i7-7700K B0 entry, and Intel [LGA1151 desktop chipset compatibility](https://www.intel.com/content/www/us/en/support/articles/000027816/processors/intel-core-processors.html) | Primary factory board socket and chipset. ASUS documents i7-7700K B0 support on all PCB revisions from BIOS 0604. Intel distinguishes the 6th/7th Generation Intel Core desktop platform from the 8th/9th Generation platform despite the socket designation. |
| S6 | Intel [Kaby Lake H Datasheet Vol. 1, 335190-005, February 2022](https://cdrdv2-public.intel.com/335190/335190_7th-gen-core-family-mobile-h-processor-lines-datasheet-vol-1_rev005.pdf), Table 1-1 and §§1.4 and 5.1.5.1–5.1.5.2.1, Intel [Kaby Lake G product list](https://www.intel.com/content/www/us/en/ark/products/codename/136847/products-formerly-kaby-lake-g.html) and the linked Package Specifications for each listed part, Intel [FCLGA2066 compatibility](https://www.intel.com/content/www/us/en/support/articles/000005592/processors/intel-core-processors.html), and Intel [NUC8i7HV Technical Product Specification J89400-009](https://www.intel.com/content/dam/support/us/en/documents/mini-pcs/nuc-kits/NUC8i7HVK_TechProdSpec.pdf), Kit Identification Information and §1.3 | Primary package facts. H and mobile Xeon E3 v6 are BGA1440, the listed G parts are BGA2270, and both X parts are LGA2066. The NUC document identifies i7-8809G B0, S-spec SR3RL, and its soldered CPU package. |
| S7 | Intel [Xeon E3-1200 v6 Datasheet Vol. 1](https://www.intel.vn/content/dam/www/public/us/en/documents/datasheets/xeon-e3-1200v6-vol-1-datasheet.pdf), §§1.4 and 5.1.5.1–5.1.5.2.1, and [Specification Update 730694-029US, May 2023](https://cdrdv2-public.intel.com/730694/730694_Intel%C2%AE%20Xeon%C2%AE%20E3-1200%20v6%20Processor%20Family%20Specification%20Update_Rev029US.pdf), Summary Tables of Changes and KBW70, KBW71, KBW73, KBW104 and KBW106, printed pp. 33–34 and 40–41 | Primary socketed Xeon overlap, equivalent calibrated decode and errata. Specification Update SHA-256 `b19c906a4e6c71902cf91b00d36f354ca3ca854d79bcfdf5c700ed3ab9d3db40`. |
| S8 | [First-hand Windows i7-7700K CPUID and MSR report, May 2017](https://github.com/openhardwaremonitor/openhardwaremonitor/issues/981#issuecomment-304824870), CPUID, SMBIOS and four MSR Core sections | Independent hardware data. ASUS STRIX Z270F GAMING, BIOS 0604, Windows 10, one four-core package with SMT and DDR4. The source is a report from the running hardware. |
| S9 | [Intel core DTS](cpu-intel-core-dts-msr.md) rev 6 at `c16b2b2eea8ada07d43567116ed75bf2ed49af75` and [PawnIO interface](pawnio-interface.md#intelmsr) rev 7 at `2495a1b0fbe4a8048feac9d122b98201e7970cbb`, [signed IntelMSR 0.2.11 allow-list](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/IntelMSR.p#L102-L118) | Approved shared non-hybrid procedure, affinity, request bounds and failure handling. The signed transport permits both reads on the calling processor. |
| S10 | Microsoft [Win32_BaseBoard](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-baseboard), Manufacturer and Product, and [Windows Sensor Service architecture](../../adr/0029-windows-sensor-service.md), current local [board query](https://github.com/shm11C3/HardwareVisualizer/blob/25217ab627390c68fb59da80787789fbea26ac00/core/src/infrastructure/providers/windows/wmi_provider.rs#L173-L175), [one-retry board lookup](https://github.com/shm11C3/HardwareVisualizer/blob/25217ab627390c68fb59da80787789fbea26ac00/core/src/infrastructure/providers/windows/pawn_io/super_io/ite.rs#L47-L96) and [board-dependent sampler](https://github.com/shm11C3/HardwareVisualizer/blob/25217ab627390c68fb59da80787789fbea26ac00/core/src/infrastructure/providers/windows/pawn_io/super_io/motherboard.rs#L404-L422) in the [service process](https://github.com/shm11C3/HardwareVisualizer/blob/25217ab627390c68fb59da80787789fbea26ac00/core/src/sensor_service/windows/providers.rs#L175-L183) | Public local Windows identity contract and repository evidence of asynchronous WMI access from the sampling host. |
| S11 | [Independent i7-8809G/NUC8i7HVK report](https://gihyo.jp/admin/serial/01/ubuntu-recipe/0523), Linux sensor output | Observed four core readings and a 100 °C critical value on one G system. The report contains no raw MSR words and supplies no enablement for this scope. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, `GenuineIntel`, exact `CPUID.01H:EAX = 0x000906E9`, leaf 06H available and `CPUID.06H:EAX[0] = 1` | mechanism (documented Kaby Lake core DTS) | S1–S4, S8–S9 |
| Leaf 07H subleaf 0 is available and the hybrid bit EDX[15] is clear | mechanism (non-hybrid core attribution) | S8–S9 |
| Complete local `Win32_BaseBoard` response contains exactly one record whose Manufacturer and Product are `ASUSTeK COMPUTER INC.` and `STRIX Z270F GAMING` after trimming surrounding whitespace | mechanism (factory 6th/7th Generation LGA1151/Z270 processor routing, excluding same-signature BGA H/G and LGA2066 X packages) | S4–S6, S8, S10 |
| One active Windows processor group, one physical package and validated core masks | mechanism (one physical core per channel) | S9 |
| Signed IntelMSR transport, confirmed affinity to the representative, and repeated vendor, signature, DTS and non-hybrid checks there | safety (bounded read on the attributed core) | S9 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact `0x000906E9` on the qualified stock LGA1151/Z270 platform | S1–S7 establish the calibrated DTS mechanism, overlap and errata. S8 records a four-core i7-7700K B0 on ASUS STRIX Z270F GAMING, BIOS 0604, in the same boot as the raw observations. S9/S10 establish the Windows access and attribution paths. | Experimental. |
| Same signature outside this board mechanism or other signatures | G thermal accuracy and register evidence remains incomplete (S6/S11). H, X and other platforms are separate scope decisions. Existing approved signatures retain their own gates. | Disabled under this specification. |

The stock board uses LGA1151 and Z270, which excludes the native BGA and
LGA2066 packages. It can physically accept an LGA1151 Xeon E3-1200 v6.
Its equivalent thermal decode and read-specific errata audit are
documented in S7. This specification makes no claim that ASUS supports
booting that Xeon on this board. Product names and BIOS versions are
evidence identities.

On demand, use the existing local asynchronous board inventory query
from the Windows Sensor Service before per-core affinity or MSR access
(S10). Keep at most one query in flight without blocking other providers.
On a query error or worker disconnection, retry once on demand. Cache
the decision from a completed response or the unavailable result of a
failed retry for the service lifetime, independently of motherboard
sampling demand and preferences. Qualify only a complete response with
exactly one record matching the required identity. The IPC caller cannot
supply board eligibility. Other CPU profiles remain independent of this
decision.

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| MSR `0x19C` | `IA32_THERM_STATUS` | 31 | Reading Valid for the physical core | Flag | S1 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 22:16 | Distance below calibrated DTS reference | Unsigned degrees Celsius | S1, S2, S7 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated Tjmax reference | Degrees Celsius | S2, S7 |

## Read procedure and decode

1. Apply the shared revision 6 demand, topology validation and recheck,
   disposable thread, affinity restoration, cache and timeout rules
   (S9). The qualified board decision precedes the added core reads.
2. For each confirmed representative, read `0x1A2` followed by
   `0x19C` through signed IntelMSR on that processor. Recheck
   attribution after the reads (S9).
3. Require both reads to succeed, Reading Valid set, target within
   50–120 °C inclusive, and target minus unsigned delta greater than
   zero and at most the target (S1, S2, S7, S9). TCC offset and averaging
   affect the thermal-control threshold and leave this decode unchanged.
4. Preserve independently valid siblings subject to the shared
   whole-pass rules. Package temperature and ACPI fallback use their
   existing independent paths (S9).

## Quirks

| Scope | Consequence | Source |
| --- | --- | --- |
| KBL113 and KBW106 | A nonzero write to read-only thermal-status fields can fault. The path only reads. | S3, S7 |
| KBL071, KBL072 and KBL074, and Xeon counterparts KBW70, KBW71 and KBW73 | These describe HWP field and interrupt delivery behavior. The decode does not use HWP fields or interrupts. | S3, S7 |
| KBL111 and KBW104 | An erroneous output-only PROCHOT assertion in package C10 concerns the external signal. The decode uses the instantaneous digital readout and Reading Valid. | S1–S3, S7 |
| DDR4 | The pinned Kaby Lake and Xeon specification updates contain no DDR4-specific DTS accuracy erratum. The DDR4 errata concerning DIMM population, initialization or system stability do not change the documented DTS fields. | S2, S3, S7 |

## Safety notes

The extension issues two 64-bit reads on the attributed physical core
through the approved signed IntelMSR interface. Faults remain unavailable
through the existing transport. Board inventory runs outside the affinity
window. No register writes, control commands or new transport are needed
(S9, S10).

## Observed readback

S8 records `0x000906E9`, `CPUID.06H:EAX = 0x000027F7`,
`CPUID.07H.00H:EDX = 0` and four target words `0x0064140D`.
The four valid core status words are `0x88420800`, `0x88410800`,
`0x88410800` and `0x88340800`. They decode to 34, 35, 35 and
48 °C from the 100 °C target. The captured target has zero TCC offset
and a nonzero Tau field. The capture establishes register correspondence
on the exact DDR4 platform. Accuracy comparison and Windows Sensor
Service verification are separate.

## Synthetic acceptance fixtures

| Fixture | Expected behavior |
| --- | --- |
| Exact board and signature, supported topology, SMT and the four S8 core words | One channel per physical core. Report 34, 35, 35 and 48 °C. Changing the synthetic TCC offset or Tau, or the brand or PTM bit, leaves the core decode unchanged. |
| `STRIX Z270E GAMING`, another board, wrong or missing Manufacturer/Product, empty, duplicate or multiple board records, or caller-supplied identity | Zero reads under this scope. Cache a completed nonmatching response without retry. Other CPU profiles and package fallback are independent. |
| Pending board lookup, then a query error or worker disconnection followed by an exact match | Reuse the pending query across repeated requests and allow one retry, even if motherboard sampling is off. No scoped core reads before the match. After the match, apply the other gates and reuse the decision. Preserve unrelated readings. |
| Initial and retry queries both fail | Cache unavailability for the service lifetime. Later requests issue no more board queries or scoped core reads. Preserve unrelated readings. |
| Exact board with a neighboring signature, unavailable DTS bit, hybrid flag, malformed topology, or multiple processor groups or packages | Zero reads under this scope. |
| A core read faults or has invalid status. Target values are 49, 50, 120 and 121 °C at zero delta, or delta equals or exceeds target. | Preserve valid siblings. Accept the two inclusive target boundaries and reject nonpositive results. |
| Topology changes or affinity restoration fails | Apply revision 6 whole-pass rejection and cache rules. Keep independent package and ACPI results. |

## Open questions

- Non-blocking for the stock Z270F Experimental scope: the exact same-boot report and primary specifications establish the mechanism on the factory board.
  The project's Windows Sensor Service and calibrated accuracy have not
  been verified on that hardware.
- Non-blocking for the stock Z270F Experimental scope: the board mechanism excludes the native G package.
  Intel documents G package identity and S11 shows observed core
  readings. G-specific raw target/status evidence and authoritative
  processor errata applicability remain open for signature-wide scope.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-10 | Propose Implementation-ready status for the exact Kaby Lake-S board mechanism, pending maintainer approval. |
| 2 | 2026-11-11 | Reuse the existing one-retry board lookup behavior for query errors and worker disconnection. Pin fail-closed recovery and final cache fixtures. |
