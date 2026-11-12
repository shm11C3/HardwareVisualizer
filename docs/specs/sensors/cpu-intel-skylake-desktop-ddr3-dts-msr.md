# Spec: Intel Skylake desktop core DTS on the ASUS Z170-P D3

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Proposed Experimental Windows core DTS for exact signature `0x000506E3` on the factory ASUS Z170-P D3 DDR3-only board, with one active processor group and one physical package. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

This proposal applies the non-hybrid procedure in [Intel core DTS](cpu-intel-core-dts-msr.md) revision 6 to a single desktop board with a documented DDR3-only memory interface.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel, [SDM Volume 4, 335592-079US, March 2023](https://cdrdv2-public.intel.com/774500/335592-sdm-vol-4.pdf), Table 2-2, §2.17 p. 2-284 and Table 2-39 pp. 2-285–2-286, and [affected processors by CPU model](https://www.intel.com/content/www/us/en/developer/topic-technology/software-security-guidance/processors-affected-consolidated-product-cpu-model.html), `06_5EH` stepping 3 row | Primary core thermal status layout and scope. The full signature `0x000506E3` covers Skylake S, H and Xeon E3. |
| S2 | Intel, [6th Generation Intel Processor Families for S-Platforms Datasheet Vol. 1, 332687-010, February 2022](https://cdrdv2-public.intel.com/332687/332687_SKL%20Datasheet%20Vol%201_rev010.pdf), §§2.1.1 and 5.1.5.1–5.1.5.2.1 pp. 19 and 85–88 | Primary processor DDR3L/-RS and DDR4 support, calibrated Tjmax target and DTS reference independent of the TCC activation offset. |
| S3 | Intel, [6th Generation Intel Processor Family Specification Update, 332689-030, July 2022](https://web.archive.org/web/20230308232303id_/https://cdrdv2-public.intel.com/332689/332689_030.pdf), Tables 1 and 3 pp. 16–17 and 21, S-line identification pp. 26–29, and SKL135 and SKL160 pp. 63 and 69 | Primary errata and stepping applicability. Archived PDF SHA-256 `4f0a9fd86c8ef4ea39694554069826e326f7844acca87a0b9651698b820c6238`. |
| S4 | ASUS, [Z170-P D3 user guide E10766](https://dlcdnets.asus.com/pub/ASUS/mb/LGA1151/Z170-P-D3/E10766__Z170-P-D3_UM_WEB.pdf), §§1.3 and 1.4 pp. 1-3 and 1-7, [CPU support table](https://www.asus.com/uk/supportonly/z170p_d3/helpdesk_cpu/), and [Z170-P user guide E12045](https://dlcdnet.asus.com/pub/ASUS/mb/LGA1151/Z170-P/E12045_Z170-P_UM_V3_WEB.pdf), §1.4 p. 1-7 | Primary exact board mechanics. Z170-P D3 uses LGA1151 and four 240-pin DDR3 sockets. ASUS lists the Core i7-6700K R-0 and Core i3-6100 S-0 for all PCB revisions. The distinct Z170-P uses 288-pin DDR4 sockets. |
| S5 | [Windows Core i5-6600K system report](https://browser.geekbench.com/v4/compute/531843), System Information and [GECID DDR3L test system](https://ru.gecid.com/ram/hyperx_ddr3l-1600_hx316lc10fbk2_16/?s=2) with Core i7-6700K | Independent evidence. The Windows report shows family 6, model 94, stepping 3 with one processor and identifies `ASUSTeK COMPUTER INC.` and `Z170-P D3`. The separate test uses that board with Core i7-6700K and DDR3L-1600 memory. Neither includes a raw DTS trace. |
| S6 | [Core i7-6700K same-system raw CPUID and MSR report](https://gist.githubusercontent.com/seanvree/13f92db466bef85514a85509c2510993/raw/2500723503fa3a6ec36df5742418934ff5ea8b9a/openHardwareMon), lines 227–238, 410–417 and 738–785, and GIGABYTE [GA-H170N-WIFI specifications](https://www.gigabyte.com/Motherboard/GA-H170N-WIFI-rev-10/sp), Memory | Independent exact-signature readback. The observed system is a different, DDR4-only board. The capture corroborates register fields and eligibility bits. It provides no accuracy measurement. |
| S7 | Intel, [EDS addendum 334408-001US, May 2016](https://cdrdv2-public.intel.com/334408/6th-gen-core-pentium-celeron-xeon-e3-v5-processor-h-s-u-series-eds-addendum.pdf), Tables 20–21 pp. 21–22, [H-platform Datasheet Vol. 1, 332986-012, February 2022](https://cdrdv2-public.intel.com/332986/332986_SKL-H%20Datasheet%20Vol%201_rev012.pdf), Table 1-1 and §1.4, and [Xeon E3-1200 v5 Datasheet Vol. 1](https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/xeon-e3-1200v5-vol-1-datasheet.pdf), §§1.4 and 5.1.4.1–5.1.4.2 | Primary overlap evidence. Skylake H and mobile Xeon E3-1500M v5 use BGA1440. Socketed Xeon E3-1200 v5 uses LGA1151 and the same Tjmax-relative decode. |
| S8 | Intel, [Xeon E3-1200 v5 Specification Update, 333133-040, September 2023](https://cdrdv2-public.intel.com/333133/333133_Intel%20Xeon%20E3-1200%20v5%20Processor%20Family%20Spec%20Update_Rev040.pdf), errata summary and SKW134 p. 53 and SKW152 p. 57 | Primary Xeon R-0 DDR4-only accuracy erratum and write-specific thermal status erratum. |
| S9 | Microsoft, [Win32_BaseBoard](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-baseboard), Manufacturer and Product, and [LocalSystem](https://learn.microsoft.com/en-us/windows/win32/services/localsystem-account) | Primary local Windows identity and service account contracts. Product identifies the SMBIOS Type 2 board. |
| S10 | [Intel core DTS](cpu-intel-core-dts-msr.md) rev 6 at `c16b2b2eea8ada07d43567116ed75bf2ed49af75`, and [PawnIO interface](pawnio-interface.md#intelmsr) rev 7 | Shared non-hybrid core selection, affinity, restoration, topology checks, demand, timeout, partial results, and the signed synchronous IntelMSR read contract. |
| S11 | [Windows sensor service architecture](../../adr/0029-windows-sensor-service.md), [complete local board query](https://github.com/shm11C3/HardwareVisualizer/blob/db901f80c9ad4efcabf963d43cf321a610352455/core/src/infrastructure/providers/windows/wmi_provider.rs#L173-L175) and its existing [sampler use](https://github.com/shm11C3/HardwareVisualizer/blob/db901f80c9ad4efcabf963d43cf321a610352455/core/src/infrastructure/providers/windows/pawn_io/super_io/motherboard.rs#L204-L221) in the [service process](https://github.com/shm11C3/HardwareVisualizer/blob/db901f80c9ad4efcabf963d43cf321a610352455/core/src/sensor_service/windows/providers.rs#L175-L183) | Repository evidence that the service can obtain the complete local WMI result asynchronously and cache a board decision without an interactive App call. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, `GenuineIntel`, exact `CPUID.01H:EAX = 0x000506E3`, leaf 06H available and `CPUID.06H:EAX[0] = 1` | mechanism (documented Skylake core DTS) | S1–S3, S6, S10 |
| `CPUID.07H.00H:EDX[15]` does not enumerate hybrid capability | mechanism (non-hybrid path) | S6, S10 |
| The service's complete local `Win32_BaseBoard` result has exactly one record. With surrounding whitespace removed, its Manufacturer equals `ASUSTeK COMPUTER INC.` and its Product equals `Z170-P D3`. | safety (selects the physically DDR3-only board and excludes the affected DDR4 condition) | S3–S5, S8, S9, S11 |
| One active Windows processor group, one physical package, and validated core masks | mechanism (physical core attribution) | S10 |
| Existing signed IntelMSR route and confirmed affinity to each selected representative with vendor, signature, DTS and non-hybrid checks repeated there | safety (bounded read on the attributed core) | S10 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact signature `0x000506E3` with the DDR3-only ASUS Z170-P D3 factory board and the gates above | S1–S4 establish the processor and board mechanisms. S5 independently corroborates board identity and operation with DDR3L. The 6700K readback in S6 corroborates field correspondence on DDR4 only. S7–S8 establish scope for other product lines with that signature. S9–S11 establish the available local paths. | Experimental. |
| Same signature on any other, unavailable or ambiguous board, and all other signatures | Board gate needed to exclude DDR4 applicability of SKL135 and SKW134 for this extension. Other approved scopes have their own gates. | Disabled for this extension. |

The ASUS board's physical LGA1151 socket excludes native BGA1440 H and
mobile Xeon E3-1500M v5 installations (S4, S7). LGA1151 alone does
not distinguish socketed Xeon E3-1200 v5. Its documented reference
and status layout support the same decode, and its SKW134 accuracy
erratum has the same DDR4 condition (S1, S7, S8). The scope makes
no compatibility claim about installing a Xeon on this ASUS board.

Resolve the complete local query on demand for this CPU candidate
before taking affinity or performing any extension MSR read. The
Windows Sensor Service may reuse a successful result for its own
lifetime. A pending query yields no extension reads and does not block
other sensor sampling. Use the existing asynchronous query and
cached identity pattern (S11). The service owns the result independently
of motherboard sampling demand or preferences. Keep a single query
in flight and do not repeat it on each visible poll. The IPC caller
cannot supply board eligibility. Missing, malformed, failed or
multiple records fail closed. BIOS version, board version, and a
reported DIMM memory type are unnecessary for the documented factory
board's physical socket mechanism (S4, S9).

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| MSR `0x19C` | `IA32_THERM_STATUS` | 31 | Reading Valid for the physical core | Flag | S1 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 22:16 | Distance below the calibrated DTS reference | Unsigned degrees Celsius | S1, S2, S7 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated Tjmax reference | Degrees Celsius | S2, S7 |

## Read procedure and decode

1. Apply the revision 6 demand, topology validation and recheck,
   disposable-thread, cache, timeout, and affinity-restore rules (S10).
   The board decision is an additional precondition for this scope.
2. Use one representative per physical core, confirm its identity, and
   read `0x1A2` followed by `0x19C` through signed IntelMSR on that
   processor. Confirm attribution after the reads (S10).
3. Require both reads to succeed, Reading Valid set, the target within
   50–120 °C inclusive, and target minus the unsigned delta greater
   than zero and at most the target (S1, S2, S10). The result is
   relative to Tjmax regardless of TCC offset or averaging window
   (S2, S7). Other profiles' legacy offset masks do not apply here.
4. If a core fails independently, preserve valid siblings subject to
   the common whole-pass checks. Package temperature and ACPI fallback
   retain their independent paths (S10).

## Quirks

| Scope | Consequence | Source |
| --- | --- | --- |
| SKL135 on S-line R-0/S-0 and SKW134 on Xeon E3-1200 v5 R-0 | On DDR4, DTS error may range from −15 °C to +5 °C, with no published workaround. The exact factory ASUS Z170-P D3's physical memory layout rules out DDR4. Do not correct a sampled temperature using the error range. | S3, S4, S8 |
| SKL160 and SKW152 | Writing nonzero values to read-only thermal-status fields can fault. This path issues the two signed reads only. | S3, S8, S10 |

## Safety notes

Restrict the extension to the two 64-bit signed IntelMSR reads. The
existing transport handles faulted reads as unavailable. Keep the board
eligibility check local to the sampling host and outside the per-core
affinity window. The signed read interface is the only privileged transport for this extension.
Memory-controller MMIO, PCI, SPD and firmware operations are outside sampling (S4, S9–S11).

## Observed readback

The independent i7-6700K report in S6 records `0x000506E3`,
`CPUID.06H:EAX = 0x000027F7`, and four target words `0x00641400`.
Its four valid status words are `0x88490008`, `0x88470008`,
`0x88490008` and `0x88400008`. The formula yields 27, 29, 27
and 36 °C from the 100 °C target. These values are register-format
observations on an affected DDR4 configuration. They neither establish
absolute accuracy nor attest operation on the DDR3 board.

## Synthetic acceptance fixtures

| Fixture | Expected behavior |
| --- | --- |
| Exact signature and one local manufacturer/product match with the current core topology gates; independent valid readings on physical cores with SMT | One channel per physical core. The 100 °C target with valid 73 °C delta yields 27 °C, also with a nonzero synthetic TCC offset and Tau. Brand and PTM changes do not change this core gate. |
| `Z170-P`, another board, missing Manufacturer/Product, or zero, duplicate or multiple records | No extension MSR reads, even when a generic memory-inventory field reports DDR3. |
| Pending, failed or unavailable service board query, or a caller-provided board name | No extension MSR reads. Preserve ordinary package/ACPI results. Repeated requests use the same pending or cached result. A later valid result enables this path without requiring motherboard sampling. |
| Exact board but neighbor signature, no DTS, hybrid bit, malformed topology, or multiple groups or packages | No extension MSR reads. |
| Individual fault or invalid status, target 49/50/120/121 °C with valid zero delta, or delta at least as large as target | Preserve valid siblings. Accept 50 and 120 °C at zero delta. Reject 49 and 121 °C and nonpositive results. |
| Topology changes or affinity restoration fails | Apply revision 6 whole-pass rejection and cache rules. Preserve independent package and ACPI results. |

## Open questions

- Non-blocking for the exact-board Experimental scope: Intel documents the decode and DDR4-only erratum, and ASUS documents the independent physical memory constraint.
  A raw CPUID and DTS capture from a DDR3 Z170-P D3 system has not
  been found. Same-board validation through the Windows Sensor Service
  and a calibrated accuracy comparison remain separate.
- Non-blocking for the exact-board Experimental scope: other board and memory configurations remain outside this proposal.
  Intel documents an active memory-controller DDR-type field in
  [332688, §7.31](https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/desktop-6th-gen-core-family-datasheet-vol-2.pdf).
  The existing signed IntelMSR path does not provide that MMIO access.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-10 | Propose Implementation-ready status for one Experimental Skylake desktop DDR3-only board, pending maintainer approval. |
