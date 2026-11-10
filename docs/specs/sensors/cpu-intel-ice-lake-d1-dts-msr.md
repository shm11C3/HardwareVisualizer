# Spec: Intel Ice Lake D1 core DTS

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental Windows physical-core DTS on exact Ice Lake-U/Y D1 signature `0x000706E5`, in one active processor group and one physical package. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel [10th Generation Core Processor Families Datasheet, Volume 1](https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/10th-gen-core-families-datasheet-vol-1-datasheet.pdf), 341077-005, July 2020, cover and §§4.2.3.1–4.2.3.2 (pp. 60–63) | Primary thermal contract covering Ice Lake U/Y Core, Pentium and Celeron. The internal DTS reports instantaneous temperature relative to a factory-calibrated software-readable reference regardless of the TCC activation offset. |
| S2 | Intel [10th Generation Core Processor Specification Update](https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/ice-lake-ultra-mobile-u/10th-generation-core-processor-specification-update/), 341079-023, April 2026, [Table 2-1](https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/ice-lake-ultra-mobile-u/10th-generation-core-processor-specification-update/identification-information/), [errata summary](https://edc.intel.com/content/www/id/id/design/ipla/software-development-platforms/client/platforms/ice-lake-ultra-mobile-u/10th-generation-core-processor-specification-update/summary-tables-of-changes/), [errata details](https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/ice-lake-ultra-mobile-u/10th-generation-core-processor-specification-update/errata-details/) and [specification changes](https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/ice-lake-ultra-mobile-u/10th-generation-core-processor-specification-update/specification-changes/) | Primary U/Y identification, affected datasheet and applicable errata. The Y row's binary stepping subcolumn disagrees with the full signature in the same row. |
| S3 | Intel [microcode release notes](https://github.com/intel/Intel-Linux-Processor-Microcode-Data-Files/blob/bdc92abe5c499c3fd7988b76a128d05c9120a520/releasenote.md#L1740-L1748), `06-7e-05/80`; [Microcode Update Guidance](https://software.intel.com/content/dam/www/public/us/en/security-advisory/documents/IPU-2019.2-microcode-update-guidance-v1.0.pdf), Ice Lake `706E5` row; and [processor support and servicing](https://www.intel.com/content/www/us/en/support/topics/support-and-servicing-for-processors.html), Mobile Processors, Ice Lake Client rows | Primary full D1 signature for both U and Y, including listed NG Core parts, Pentium 6805 and Celeron 5805. The microcode platform mask is identification evidence. |
| S4 | Intel [SDM Volume 4](https://cdrdv2-public.intel.com/929363/335592-093-sdm-vol-4.pdf), 335592-093US, September 2026, Tables 2-1 and 2-2 (pp. 2-1 and 2-21–2-22), §2.17 Table 2-39 (pp. 2-359–2-361), and §2.17.3 Table 2-44 | Primary `06_7E` model identification and Core-scope `0x19C` layout. The Ice Lake supplemental table supplies no replacement layout for this register. |
| S5 | InstLatx64, commit `17be9a9793c306cf96f8c286e34e705b780a0226`, [Core i7-1065G7](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00706E5_IceLakeY_CPUID.txt), [Core i5-1035G4](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00706E5_IceLakeY_CPUID2.txt) and [Core i3-1005G1](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00706E5_IceLakeY_CPUID3.txt) | Independent exact CPUID, topology and paired raw `0x19C`/`0x1A2` observations. Each capture has one MSR section with no logical-processor attribution. The CPU names in the latter two are encoded in the raw brand leaves. |
| S6 | Linux [Intel-authored model-specific thermal change](https://github.com/torvalds/linux/commit/6ae0092ca7adfce79336c6525cb0da561ecd4f04) and the pinned [thermal implementation](https://github.com/torvalds/linux/blob/e2457a664ea02c414c6b9828bff3a0df4c300f63/drivers/thermal/intel/intel_tcc.c#L38-L44) ([model selection](https://github.com/torvalds/linux/blob/e2457a664ea02c414c6b9828bff3a0df4c300f63/drivers/thermal/intel/intel_tcc.c#L85-L91), [target read](https://github.com/torvalds/linux/blob/e2457a664ea02c414c6b9828bff3a0df4c300f63/drivers/thermal/intel/intel_tcc.c#L181-L202)), [model ID](https://github.com/torvalds/linux/blob/e2457a664ea02c414c6b9828bff3a0df4c300f63/arch/x86/include/asm/intel-family.h#L115) and [MSR index](https://github.com/torvalds/linux/blob/e2457a664ea02c414c6b9828bff3a0df4c300f63/arch/x86/include/asm/msr-index.h#L1023) | **lead-only (copyleft)** for the exact model selection tying the readable calibrated target to MSR `0x1A2[23:16]`; S1 and S5 corroborate the field and address respectively. The explicit `06_7E` core readout width agrees with S4. |
| S7 | [Intel core DTS revision 6](https://github.com/shm11C3/HardwareVisualizer/blob/c16b2b2eea8ada07d43567116ed75bf2ed49af75/docs/specs/sensors/cpu-intel-core-dts-msr.md), Detection, Read procedure and decode, Project collection constraints and Safety notes, and [PawnIO interface revision 7](https://github.com/shm11C3/HardwareVisualizer/blob/2495a1b0fbe4a8048feac9d122b98201e7970cbb/docs/specs/sensors/pawnio-interface.md#intelmsr), signed `IntelMSR` `0.2.11` | Common Windows topology, affinity, demand, timeout, partial-result and fault contract. The signed module allows both synchronous calling-processor reads. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, `GenuineIntel`, exact `CPUID.01H:EAX = 0x000706E5`, and `CPUID.06H:EAX[0] = 1` at discovery and on each confirmed representative | mechanism | S1–S5, S7 |
| One active Windows processor group and one physical package with complete and consistent physical-core topology | mechanism | S7 |
| Existing signed `IntelMSR` transport, disposable sampling thread, confirmed affinity, successful restoration and topology recheck | safety | S7 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact Ice Lake-U/Y D1 `0x000706E5` | S1–S4 establish the U/Y mechanism including the same-signature Pentium 6805 and Celeron 5805. S5 supplies exact Core observations. S6 is lead-only for the calibrated target's MSR binding on this model. | Experimental under the common gates. |
| Other signatures or missing gates | Includes other `06_7E` steppings and the distinct Ice Lake Xeon models `06_6A` and `06_6C` (S2–S4). | Disabled under this specification. |

Use the existing non-hybrid physical-core sampling path and its
Windows core identities (S7). Product names, nominal core counts,
package types and microcode platform IDs record evidence identities.
The package thermal management capability is evaluated independently.

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x19C` | `IA32_THERM_STATUS` | 31 | Core DTS reading valid when set | Boolean | S4, S5 |
| `0x19C` | `IA32_THERM_STATUS` | 22:16 | Unsigned digital readout below Tjmax | Degrees Celsius | S1, S4, S5 |
| `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated Tjmax reference | Degrees Celsius | S1, S5, S6 |

## Read procedure and decode

Apply the common revision 6 sampling procedure (S7). On each
confirmed representative, read the 64-bit `0x1A2` followed by
`0x19C` through the signed module. Require both reads to succeed,
a target in 50–120 °C inclusive, and core status bit 31 set.
Subtract the seven-bit unsigned delta from the target. Accept a
positive result at most equal to the target. A valid zero delta
returns the target. The thermal-control activation offset and
averaging window are independent of this instantaneous DTS
reference (S1, S4–S7).

Recheck the running processor after the pair and restore affinity.
Leave a core unavailable on its isolated read or validity failure.
Apply S7's demand, rate limit, timeout, topology and cache rules.
Failed affinity restoration or inconsistent topology invalidates
the core set.

## Quirks

- The three S5 captures report a single package, four physical cores
  with SMT on the i7 and i5 and two physical cores with SMT on the
  i3. Their respective status/target low words are
  `0x88300A82`/`0x05640000` (lines 618–656),
  `0x88340002`/`0x07640000` (lines 571–609), and
  `0x88350808`/`0x00640000` (lines 283–317).
  The target byte is 100 °C in each. The data corroborates the
  register pair with zero and nonzero activation offsets.
- ICL042 concerns PROCHOT demotion, ICL044 concerns THRMTRIP on deep
  package-idle exit, and ICL052 concerns the Thunderbolt controller's
  TCSS DTS. S2 marks ICL042 and ICL044 fixed for U and Y and ICL052
  fixed for U and inapplicable to Y. Those errata do not redefine
  the admitted core readout or its reference.

## Safety notes

Issue only the two signed reads in S7. Preserve independent package
and fallback collection on partial or unavailable core results. The
scope permits no write, control operation or package/PECI proxy for
an unavailable core.

## Synthetic fixture acceptance

| Fixture | Expected behavior |
| --- | --- |
| Exact D1 with core DTS, one package and group, with SMT and without SMT, and a synthetic unlisted brand | Use one representative per observed physical core mask. Read only `0x1A2` and `0x19C` on the core path. Results are independent of PTM. |
| `0x000706E4`, `0x000706E6`, Ice Lake Xeon `0x000606A6`, wrong vendor, absent required leaf or core DTS clear | No core topology query or core-path MSR read under this scope. |
| Pinned representative changes signature, loses DTS capability or cannot be confirmed | Skip both reads for that target and preserve valid siblings. |
| Valid target 100 °C, delta 30 °C and target words with offset zero and nonzero, then a valid zero delta | Return 70 °C in both offset cases and 100 °C for zero delta. |
| Either read faults, target 0, 49 °C or 121 °C, clear bit 31, or delta at least the target | Leave that sample unavailable and preserve valid siblings. Admit target endpoints 50 °C and 120 °C when the other fields qualify. |
| Multiple groups or packages, changed topology, timeout, or failed restoration | Apply S7's no-result and cache rules. Preserve independent package and fallback results. |

## Open questions

- Non-blocking for #2514 Experimental core DTS: Intel's full-signature and microcode sources agree for the exact D1 gate.
  S2 Table 2-1 prints `706E5h` for Y while its separate Y stepping
  subcolumn prints `0100b`. S3 independently maps U/Y D1 to stepping 5
  and lists exact Y product examples with `706E5`. The profile admits
  the documented full `0x000706E5` only.
- Non-blocking for #2514 Experimental core DTS: Intel's shared thermal contract and the exact Core captures substantiate the admitted mechanism.
  S5 supplies no individually attributed raw thermal pair or
  Y, Pentium 6805 or Celeron 5805 raw thermal capture. The signed
  transport contract is pinned in S7. A HardwareVisualizer Windows
  service run has not been observed for this profile.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-10 | Proposes Implementation-ready status for exact Ice Lake-U/Y D1 core DTS, pending maintainer approval. |
