# Spec: Intel Meteor Lake C0 core temperatures via digital thermal sensors

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental read-only Windows core DTS for hybrid Intel Meteor Lake H, U, U Type4 and PS with full CPUID signature `0x000A06A4`, one active processor group and one physical package. |
| Issue phase | #2514 |

This supplement adds one exact signature to the common [Intel core DTS
specification](cpu-intel-core-dts-msr.md) revision 4. The common
collection and safety requirements apply, with the model-specific evidence
and fixtures below.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel, [SDM Documentation Changes, 252046-076, June 2024](https://cdrdv2-public.intel.com/825745/252046-sdm-change-document.pdf), replacement Volume 4 §2.17 and Table 2-39 (pp. 2-332–2-333), §2.17.9 and Tables 2-53–2-55 | Primary display-model MSR applicability, core status scope, status bit fields, and separate P/E MSR lists. |
| S2 | Intel, [Core Ultra Processor Datasheet Volume 1, 792044-008, May 2025](https://edc.intel.com/output/DownloadPdfDocument?id=8207), §§16.1.2.1–16.1.2.2 (pp. 179–182) and [Meteor Lake core and tile explanation, article 000099551](https://www.intel.com/content/www/us/en/support/articles/000099551/processors/intel-core-ultra-processors.html) | Primary H/U/U Type4 thermal reference, offset and readout semantics, and independent descriptions of compute-tile and SoC-tile core placement. |
| S3 | Intel, [Core Ultra Processors (PS Series) Datasheet, 819636-001, April 2024](https://cdrdv2-public.intel.com/819636/819636_Intel_Core_Ultra_Proc_PS_Datasheet__Rev001.pdf), §§15.1.2.1–15.1.2.2 (pp. 142–145) | Primary PS thermal reference, offset and readout semantics. |
| S4 | Intel, [Core Ultra Processor Specification Update, 792254-024, September 2026](https://edc.intel.com/content/www/us/en/design/products/platforms/details/meteor-lake-u-p/core-ultra-processor-specification-update/identification-information/), Table 2-1, [Errata Summary Table](https://edc.intel.com/content/www/us/en/design/products/platforms/details/meteor-lake-u-p/core-ultra-processor-specification-update/summary-tables-of-changes/) and [Errata Details](https://edc.intel.com/content/www/jp/ja/design/products/platforms/details/meteor-lake-u-p/core-ultra-processor-specification-update/errata-details/) | Primary exact H/U/U Type4 C0 signature and errata for those lines. |
| S5 | Intel, [Affected Processors by CPU Model](https://www.intel.com/content/www/us/en/developer/topic-technology/software-security-guidance/processors-affected-consolidated-product-cpu-model.html), `06_AAH` stepping 4 row, consulted November 2026 | Primary hybrid flag and exact shared `A06A4` H/U/PS signature, including the embedded overlap. |
| S6 | InstLatx64, raw CPUID and MSR captures at commit `17be9a9793c306cf96f8c286e34e705b780a0226`, [Core Ultra 5 125H](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00A06A4_MeteorLake_07_CPUID.txt), [Core Ultra 7 155H](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00A06A4_MeteorLake_10_CPUID.txt) and [Core Ultra 7 155U](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00A06A4_MeteorLake_09_CPUID.txt) | Independent same-system observations of per-logical-processor CPUID and register readback. Each file documents one system. |
| S7 | [Intel core DTS](cpu-intel-core-dts-msr.md) revision 4 (`38643ed9c9d38e8ee744de58208d8c71e267f84b`), sources S2–S6 and S9, Detection, Read procedure and decode, and Project collection constraints | Primary CPUID bit and core-type definitions, Windows affinity/topology contract, and shared project constraints. |
| S8 | [PawnIO interface](pawnio-interface.md#intelmsr) revision 7 (`2495a1b0fbe4a8048feac9d122b98201e7970cbb`), `IntelMSR`, signed module release `0.2.11` | Existing vendor gate, synchronous calling-processor read and allow-list for `0x19C` and `0x1A2`. |

## Detection

| Scope | Status | Default enablement |
| --- | --- | --- |
| Hybrid Meteor Lake H, U, U Type4 and PS, display family/model `06_AA`, full signature `0x000A06A4` | S4 and S5 identify the exact stepping and product lines. S1–S3 define the MSR contract. S6 independently observes P-, E- and low-power E-core readbacks on H and U. | Experimental under the per-target capability, affinity, validity and topology gates. |
| Other signatures, absent DTS or hybrid flag, unrecognized target core type, or multiple active groups/packages | Outside this supplement's eligibility scope (S1, S4, S5, S7). | Disabled for this core path. |

- Require `GenuineIntel`, full `CPUID.01H:EAX = 0x000A06A4`,
  `CPUID.06H:EAX[0]` and `CPUID.07H.00H:EDX[15]` both set. Require
  those gates at discovery and again on each confirmed representative.
  Package thermal management (`CPUID.06H:EAX[6]`) is independent (S4–S7).
- On the pinned target, require `CPUID.1AH.00H` to be available and
  EAX[31:24] equal to `0x40` (Intel Core) or `0x20` (Intel Atom).
  The S6 captures report `0x40` on P-cores and `0x20` on both
  compute-tile E-cores and SoC-tile low-power E-cores. A low-power
  E-core exposed as a distinct eligible Windows physical-core record
  gets its own existing core identity. Use the existing Windows group
  and representative processor number, without a tile-based label
  or a cache or APIC-ID filter (S2, S6, S7).

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| MSR `0x19C` | `IA32_THERM_STATUS` | 31 | Valid core digital readout when set | Flag | S1 Table 2-39, S7 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 22:16 | Core digital readout below the calibrated Tjmax | Unsigned degrees Celsius | S1 Table 2-39, S2, S3 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated Tjmax, the reference for the DTS readout | Degrees Celsius | S2, S3 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 29:24 | Thermal Control Circuit activation offset | Degrees Celsius | S2, S3 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 6:0 | Thermal-control averaging window parameter (Tau) | Encoded time parameter | S2, S3 |

## Read procedure and decode

Use the common revision 4 hybrid procedure (S7). Validate one active
Windows processor group and one physical package. Confirm one pinned
representative per core record. Use the same representative for both
64-bit MSR reads, `0x1A2` followed by `0x19C`, via `IntelMSR` (S8).
The resulting temperature in degrees Celsius is the target byte minus
the unsigned digital readout. Decode independently of TCC offset and
Tau. The target byte is the reference and can vary. The project accepts
targets 50–120 °C inclusive, a set Reading Valid bit and a decoded value
greater than zero and at most the target (S1–S3, S7).

The common revision 4 request, topology-recheck, disposable-thread,
affinity restore, cache, timeout and partial-result rules apply.
An isolated rejected target leaves other cores and the independent
package and ACPI paths available (S7).

### Synthetic acceptance fixtures

| Fixture | Expected behavior |
| --- | --- |
| `0x000A06A4`, hybrid and core DTS set, one group/package, one SMT P-core and separate E and low-power E physical-core records | Use one eligible representative per physical core. The two `0x20` records remain separate channels under the existing identity. The P-core's second thread adds no channel. |
| Adjacent full signatures `0x000A06A3` and `0x000A06A5`, or exact signature with hybrid or core DTS clear, regardless of package thermal management | No Meteor Lake core topology collection or core MSR reads. |
| Exact eligible signature with package thermal management clear | Core reads remain eligible. This path never requests `0x1B1`. |
| Eligible pass with one low-power E target lacking leaf 1AH, reporting an unknown type, faulting on a read or returning clear Reading Valid | Reject that core only. For an unsupported target type, skip both reads. Preserve the other positions and the independent package/fallback result. |
| Same valid readout with nonzero versus zero TCC offset, and different Tau; target 110 °C and digital readouts 59 and 0 | Return 51 °C and 110 °C respectively in each case. |
| Target 49/50/120/121 °C, digital readout equal to target, or changed topology or affinity restore failure | Reject out-of-bounds targets and a decoded zero. Follow the common revision 4 rules to reject the whole core set and cache on a topology mismatch or restore failure. |

## Quirks

- The public H and U captures report distinct P, compute-tile E and
  low-power E logical processors. The two low-power E observations
  per machine have APIC IDs `0x40` and `0x42`, each with its own core
  entry and no L3 entry in CPUID leaf 04H. These characteristics
  describe those captures and corroborate the low-power core
  attribution alongside Intel's topology description (S2, S6).

| S6 capture | LP E logical CPUs | Low 32 bits of LP E `0x19C`, by logical CPU | Low 32 bits of `0x1A2` on both LP E targets |
| --- | --- | --- | --- |
| 125H, lines 3518–3521 and 3646–3649 | 16, 17 | `0x883B2800`, `0x88392800` | `0x066E0000` |
| 155H, lines 4144–4147 and 4272–4275 | 20, 21 | `0x88382800`, `0x88363800` | `0x806E0000` |
| 155U, lines 2742–2745 and 2870–2873 | 12, 13 | `0x883B2800`, `0x883A2800` | `0x056E0000` |

- S4 MTL023 describes thermal throttling behavior at maximum turbo
  frequency. The published entry does not change the scoped DTS
  reference, status or read protocol. MTL049 names other MSRs and
  concerns writes. The S4 summary and details identify no erratum
  for the two scoped readouts on H/U/U Type4.

## Safety notes

- Read only the target and core status registers on an eligible pinned
  processor. The status fields used here are read-only and S1 defines
  log bits as write-zero-to-clear. No writes, threshold changes, log
  clears, or thermal-policy changes are part of the scope (S1, S7, S8).
- Each core keeps its Windows topology identity. The common scope and
  collection limits in S7 apply even if a matching package advertises
  more topology than is eligible for a single validated pass.

## Open questions

Non-blocking for #2514 Experimental core DTS: the exact model and read contract are documented independently of a HardwareVisualizer runtime trace.

S6 covers three H and U systems. There is no independent PS or U Type4
readback cited here and the public errata review in S4 covers H/U/U Type4.
S3 and S5 provide the independent PS thermal and identity contract.
The captures do not exercise the HardwareVisualizer service or its
affinity restoration path.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-08 | Initial Experimental readiness proposal for exact Meteor Lake C0 H, U, U Type4 and PS core DTS. |
