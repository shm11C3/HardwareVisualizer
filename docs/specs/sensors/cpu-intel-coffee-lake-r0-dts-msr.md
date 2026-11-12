# Spec: Intel Coffee Lake Refresh R0 core DTS

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental Windows physical-core DTS on exact Coffee Lake R0 signature `0x000906ED`, including documented S, H and Xeon E overlap. One active processor group and one physical package. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel [8th and 9th Generation Core Processor Families Datasheet, Volume 1](https://cdrdv2.intel.com/v1/dl/getContent/337344), 337344-009, February 2022, §§5.1.5.1–5.1.5.2 (pp. 95–98), SHA-256 `bc38a3497e5a1cec25632768a1a43da8da8767ce251d2b89ee742035380ddce7` | Primary client thermal contract for Coffee Lake S/H, including the Celeron family. Factory calibration, target byte and instantaneous DTS relative to the offset-independent reference. |
| S2 | Intel [Xeon E-2100 and E-2200 Processor Family Datasheet, Volume 1](https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/xeon-e-2100-datasheet-vol-1.pdf), 338012-002, July 2019, §§5.1.4.1–5.1.4.2 (pp. 84–87), SHA-256 `c8339f23de313f9ded82b98b0a8ef4b54f308bbe08df4a5a6200a7c788edf69c` | Primary Xeon contract identifies both MSRs, target bits and reference independence. |
| S3 | Intel [SDM Volume 4](https://cdrdv2-public.intel.com/929363/335592-093-sdm-vol-4.pdf), 335592-093US, September 2026, Table 2-2 (pp. 2-21–2-22), §2.17 and Table 2-39 (pp. 2-359–2-361) | Primary architectural fields and explicit 8th/9th generation Core and Xeon E Coffee Lake `06_9E` applicability. The status is Core-scoped. |
| S4 | Intel [IPU 2019.2 Microcode Update Guidance](https://www.intel.com/content/dam/www/public/us/en/security-advisory/documents/IPU-2019.2-microcode-update-guidance-v1.0.pdf), pp. 5–6, [microcode release notes](https://github.com/intel/Intel-Linux-Processor-Microcode-Data-Files/blob/bdc92abe5c499c3fd7988b76a128d05c9120a520/releasenote.md#L1745-L1752), and [Processor Servicing](https://www.intel.com/content/www/us/en/support/topics/support-and-servicing-for-processors.html), `906ED` Coffee Lake entries consulted November 2026 | Exact R0 overlap across Core S/H and Xeon E, with examples including i9-9900K and E-2288G. The servicing table also lists G4930/G4930T under `906ED`. |
| S5 | Intel [8th/9th Generation Core Specification Update](https://cdrdv2-public.intel.com/337346/337346_008.pdf), 337346-008, November 2025, R0 S/H columns and CFL069, CFL071, CFL097–098 (pp. 15, 17, 40–42 and 48), and [Xeon E-2100/E-2200 Specification Update](https://cdrdv2-public.intel.com/338014/338014_Intel%C2%AE%20Xeon%C2%AE%20E-2100%20and%20E-2200%20Processor%20Family%20Specification%20Update_rev019US.pdf), 338014-019US, September 2023, R-0 column and CFW069, CFW071, CFW094–095 (pp. 13–14, 34–35 and 41) | Primary applicable HWP notifications, PROCHOT output and thermal status write errata. |
| S6 | InstLatx64 [CC150 raw capture](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00906ED_CoffeeLake_CPUID.txt), lines 6, 14, 42–44, 865–886 and 904–909 | Independent exact-signature CPUID, topology and raw thermal words from one system. |
| S7 | Independent inventory observations for [Core i9-9900KS](https://xmrig.com/benchmark/2pm9Em), [Core i9-9980HK](https://xmrig.com/benchmark/6R6hD7), [Xeon E-2288G](https://xmrig.com/benchmark/4EKV9X) and [Celeron G4930](https://xmrig.com/benchmark/4Fg9bY) | The Core S, Core H and Xeon examples report exact `906ED`. This G4930 reports `906EB`, which demonstrates why a product name cannot establish R0. These reports supply no raw thermal MSRs. |
| S8 | [Intel core DTS revision 6](https://github.com/shm11C3/HardwareVisualizer/blob/c16b2b2eea8ada07d43567116ed75bf2ed49af75/docs/specs/sensors/cpu-intel-core-dts-msr.md), Detection, Read procedure and decode, Project collection constraints and Safety notes, and [PawnIO interface revision 7](https://github.com/shm11C3/HardwareVisualizer/blob/2495a1b0fbe4a8048feac9d122b98201e7970cbb/docs/specs/sensors/pawnio-interface.md#intelmsr), signed `IntelMSR` 0.2.11 | Common single-group, single-package topology and read contract, bounded demand, affinity, partial results and allow-listed calling-processor reads. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, `GenuineIntel`, full `CPUID.01H:EAX = 0x000906ED` and core DTS `CPUID.06H:EAX[0] = 1` at discovery and on each confirmed core representative | mechanism | S1–S4, S6–S8 |
| One active Windows processor group and one physical package with complete, consistent core topology | mechanism | S8 |
| Existing signed `IntelMSR` transport, disposable sampling thread, confirmed affinity, successful restoration and topology recheck | safety | S8 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact Coffee Lake R0 `0x000906ED` core DTS | S1–S5 establish the Coffee Lake client and Xeon E contracts, exact overlaps and relevant errata. S6 records the paired CC150 reads. S7 corroborates the exact S/H/Xeon identities. The Celeron servicing entries are identity evidence (S4), and a G4930 observation with a different stepping remains outside this scope (S7). | Experimental under the common mechanism and safety gates. |
| Other signatures or missing gates | Adjacent Coffee Lake signatures `0x000906EA`, `0x000906EB` and `0x000906EC` require separate qualification. | Disabled under this specification. |

Use the existing Windows physical-core representative identities. The
topology determines the number of targets. Product names record evidence
identity. Package thermal management has separate eligibility (S8).

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x19C` | `IA32_THERM_STATUS` | 31 | Core DTS reading valid when set | Boolean | S3, S6 |
| `0x19C` | `IA32_THERM_STATUS` | 22:16 | Unsigned digital readout below Tjmax | Degrees Celsius | S1–S3, S6 |
| `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated Tjmax reference | Degrees Celsius | S1–S2, S6 |

## Read procedure and decode

Apply the common revision 6 non-hybrid sampling procedure (S8). On
each confirmed representative, read the 64-bit `0x1A2` followed by
`0x19C` using the signed module. Require successful reads, a target in
50–120 °C inclusive and status bit 31 set. Subtract the unsigned delta
from the target and accept a positive result at most equal to the
target. A valid zero delta returns the target. The DTS reference is
independent of the TCC activation offset and averaging window (S1–S2).

Recheck the running processor after the pair and restore affinity.
An isolated fault or invalid sample leaves that core unavailable and
preserves other readings. Apply the common demand, rate limit, deadline,
topology and cache rules, including discarding the core set on failed
affinity restoration or inconsistent topology (S8).

## Quirks

- S6 reports exact signature and DTS capability, one package with eight
  physical cores, and status low words `0x88420800` and `0x88410800`
  alongside target low word `0x00640E41`. The target is 100 °C. The dump
  provides one MSR section without a logical CPU label, so it corroborates
  the register pair without identifying individual core temperatures.
- The exact R0 errata cover HWP thermal interrupts, the external
  PROCHOT signal during package C10 and writes of nonzero values to
  read-only `0x19C` fields. The two permitted reads use the calibrated
  target and digital readout fields (S1–S3, S5).

## Safety notes

The core path only issues the signed allow-listed reads `0x1A2` and
`0x19C` under S8. No thermal, interrupt, log or power-control write is
permitted. Preserve the independent package path and fallback on partial
or unavailable core collection.

## Synthetic fixture acceptance

| Fixture | Expected behavior |
| --- | --- |
| Exact R0, core DTS set, package thermal management clear, one group and package with an SMT pair | Sample one representative per physical core. The core path reads only the two allowed MSRs. |
| Same evidence product name with signature `0x000906E9`, `0x000906EA`, `0x000906EB` or `0x000906EC`, wrong vendor, absent required leaf, or core DTS clear | No core topology query or core-path MSR read under this scope. |
| Admitted discovery and a pinned representative with mismatched identity or missing DTS capability | Skip both MSRs for that target and preserve other readings. |
| Valid target 100 °C and delta 30 with zero and nonzero TCC offsets, then valid zero delta | Return 70 °C for both offsets and 100 °C for the zero delta. |
| A fault, zero target, target 49 °C or 121 °C, clear bit 31, or delta at least the target, with a valid sibling | Leave that core unavailable and preserve the sibling. Accept targets 50 °C and 120 °C when the other fields qualify. |
| Multiple groups or packages, changed topology, timeout, or failed restoration | Apply S8's no-result and cache rules. Preserve independent package and fallback results. |

## Open questions

Non-blocking for #2514 Experimental core DTS: the paired raw evidence is CC150, while Intel's product documents and independent inventory establish the shared R0 mechanism and exact Core and Xeon product overlaps.

The collected sources do not include separately attributed per-core raw
words for the Core or Xeon examples, a raw `906ED` G4930/G4930T
observation or a HardwareVisualizer Windows service trace. The servicing
table is evidence of reported model overlap only. Runtime admission uses
the full signature and hardware capability (S4, S6–S8).

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-10 | Proposes Implementation-ready status for exact Coffee Lake R0 core DTS, pending maintainer approval. |
