# Spec: Intel Arrow Lake core temperatures via digital thermal sensors

| Field | Value |
| --- | --- |
| Revision | 2 |
| Status | Implementation-ready (rev 2) |
| Scope | Experimental read-only Windows core DTS for Intel Core Ultra processors with full signature `0x000C0662` (the 200S, 200S Plus, 200HX and 200HX Plus product lines), one active Windows processor group and one physical package. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

This supplement proposes one signature-keyed scope under the common
[Intel core DTS specification](cpu-intel-core-dts-msr.md) revision 5.
Revision 2 replaces the five exact brand gates of revision 1 with the
mechanism gates required by
[ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md);
the five observed brands remain in the Status column as evidence
identities. The
[specification gate](README.md#status-transition-draft--implementation-ready)
requires maintainer approval before this revision can be used for
implementation.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel, *Core Ultra 200S, 200S Plus, 200HX and 200HX Plus Processor Datasheet*, volume 1, 832586-007, March 17, 2026, [CPUID](https://edc.intel.com/content/www/us/en/design/products/platforms/details/arrow-lake-s/core-ultra-200s-series-processors-datasheet-volume-1-of-2/007/cpuid/), [Adaptive Thermal Monitor](https://edc.intel.com/content/www/us/en/design/products/platforms/details/arrow-lake-s/core-ultra-200s-series-processors-datasheet-volume-1-of-2/007/adaptive-thermal-monitor/) and [Digital Thermal Sensor](https://edc.intel.com/content/www/us/en/design/products/platforms/details/arrow-lake-s/core-ultra-200s-series-processors-datasheet-volume-1-of-2/007/digital-thermal-sensor/) | Primary product-line identity, instantaneous DTS readout and calibrated reference independent of TCC offset. The datasheet covers all four product lines that carry the scoped signature. |
| S2 | Intel, *Core Ultra Processors (Series 2) Specification Update*, 834774-023, September 1, 2026, [§2.1 Table 2-1](https://edc.intel.com/content/www/us/en/design/products/platforms/details/arrow-lake-s/core-ultra-200s-series-processors-specification-update/identification-information/), [Errata Summary Table](https://edc.intel.com/content/www/us/en/design/products/platforms/details/arrow-lake-s/core-ultra-200s-series-processors-specification-update/summary-tables-of-changes/), columns S/S Plus 8+16, S 6+8 and HX/HX Plus, and [Errata Details](https://edc.intel.com/content/www/us/en/design/products/platforms/details/arrow-lake-s/core-ultra-200s-series-processors-specification-update/errata-details/) | Primary full signature, the product lines that carry it, and exact errata applicability. |
| S3 | Intel, [SDM Documentation Changes, 252046-080](https://cdrdv2-public.intel.com/858441/252046-080-sdm-change-document.pdf), replacement volume 2A, CPUID instruction, How Brand Strings Work (pp. 3-242–3-243), and [SDM, June 2024](https://cdrdv2-public.intel.com/825743/325462-sdm-vol-1-2abcd-3abcd-4.pdf), volume 3B §§15.8.5.1–15.8.5.2 (pp. 15-42–15-43) | Primary architectural brand encoding (used only to read the evidence identities in S5), core DTS enumeration and status bit definitions. S1 supplies the Arrow Lake readout reference. |
| S4 | Intel, [Affected Processors by CPU Model](https://www.intel.com/content/www/us/en/developer/topic-technology/software-security-guidance/processors-affected-consolidated-product-cpu-model.html), `06_C6H` stepping 2 entry, consulted November 2026 | Primary hybrid bit and broader labeling of the same signature. |
| S5 | InstLatx64, raw CPUID and MSR captures at `17be9a9793c306cf96f8c286e34e705b780a0226`, linked by exact SKU below | Independent observations. Each file associates its own brand and signature with readings on P and E logical processors. |
| S6 | [Intel core DTS](cpu-intel-core-dts-msr.md) revision 5 at `cdd809aac2ceae942c5b6bcdddc7ecc537c07cdb`, sources S2 and S4–S6 and S9, Detection, Read procedure and decode, and Project collection constraints | Common Windows affinity, topology, identity, request, partial-result and safety contract. Its own signature allow-list is separate. |
| S7 | [PawnIO interface](pawnio-interface.md#intelmsr) revision 7 at `2495a1b0fbe4a8048feac9d122b98201e7970cbb`, `IntelMSR`, signed module release `0.2.11` | Existing Intel vendor gate, synchronous calling-processor read and allow-list for `0x19C` and `0x1A2`. |

## Detection

Every runtime condition below is a mechanism gate: it selects the
documented hybrid core DTS decode using what the processor reports about
itself. No brand string, product name or other evidence identity gates
the read (ADR 0031).

| Condition | Gate kind | Source |
| --- | --- | --- |
| CPUID leaf 0 reports `GenuineIntel`, with basic leaves 01H, 06H, 07H and 1AH available | mechanism | S3, S6 |
| Full `CPUID.01H:EAX = 0x000C0662` | mechanism | S1, S2 |
| `CPUID.06H:EAX[0]` (core DTS) set; `CPUID.06H:EAX[6]` (package thermal management) is independent and not required | mechanism | S3, S6 |
| `CPUID.07H.00H:EDX[15]` (hybrid) set | mechanism | S3, S4 |
| On each pinned target, `CPUID.1AH.00H:EAX[31:24]` is `0x40` (Intel Core, P-core) or `0x20` (Intel Atom, E-core) | mechanism | S3, S5, S6 |
| One active Windows processor group and one physical package, per the common topology contract | mechanism | S6 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Vendor `GenuineIntel`, full `CPUID.01H:EAX = 0x000C0662`, core DTS and hybrid capability present | S1 documents the DTS readout and calibrated reference for the 200S, 200S Plus, 200HX and 200HX Plus lines, and S2 documents that these lines carry the signature. S5 independently observes the signature, both core types and P/E `0x19C`/`0x1A2` reads on five SKUs, recorded here as evidence identities that do not gate the read: `Intel(R) Core(TM) Ultra 5 245K` (200S; [capture](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00C0662_ArrowLake_07_CPUID.txt), signature line 55, brand lines 130–132), `Intel(R) Core(TM) Ultra 7 265K` (200S; [capture](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00C0662_ArrowLake_08_CPUID.txt), signature line 55, brand lines 122–124), `Intel(R) Core(TM) Ultra 7 270K Plus` (200S Plus; [capture](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00C0662_ArrowLakeR_01_CPUID.txt), signature line 6, brand lines 73–75), `Intel(R) Core(TM) Ultra 9 275HX` (200HX; [capture](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00C0662_ArrowLakeHX_01_CPUID.txt), signature line 6, brand lines 73–75) and `Intel(R) Core(TM) Ultra 9 290HX Plus` (200HX Plus; [capture](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00C0662_ArrowLakeR_11_CPUID.txt), signature line 6, brand lines 73–75). These five are the first graduation candidates toward Verified once a maintainer-accepted hardware report confirms the HardwareVisualizer path on the product. Other SKUs that carry the signature have no public or HardwareVisualizer readback observation yet. | Experimental under the common topology and per-target gates. |
| Other signature (S2 lists standard H at `0x000C0652` and U at `0x000B0650`), wrong vendor, unavailable required CPUID leaf, absent core DTS or hybrid capability, unrecognized target type, or more than one active group or physical package | Outside the supported scope (S2–S6). | Disabled for this core path. |

- At discovery, require the vendor, the exact full signature,
  `CPUID.06H:EAX[0]` (core DTS), `CPUID.07H.00H:EDX[15]` (hybrid) and
  availability of the CPUID leaves needed for the path. Package thermal
  management, `CPUID.06H:EAX[6]`, is independent (S2–S6).
- Repeat the vendor, full signature and capability checks on every
  confirmed pinned target before either MSR read. Require
  `CPUID.1AH.00H:EAX[31:24]` of `0x40` (Intel Core, P-core) or `0x20`
  (Intel Atom, E-core). These types qualify a target. Its identity
  remains the existing Windows group and representative processor
  number (S3, S5, S6).
- The processor brand (leaves `0x80000002`–`0x80000004`, S3) is not a
  gate. A missing or unrecognized brand does not change admission.

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| MSR `0x19C` | `IA32_THERM_STATUS` | 31 | Valid digital readout when set | Flag | S3 §15.8.5.2, S5 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 22:16 | Instantaneous core DTS readout below the calibrated maximum operating temperature | Unsigned degrees Celsius | S1, S3 §15.8.5.2, S5 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated maximum operating temperature (Tjmax) | Degrees Celsius | S1, S5 |

## Read procedure and decode

Use the common revision 5 hybrid procedure and project constraints (S6).
For each physical core, pin one confirmed Windows representative. Once
the target passes Detection, read `0x1A2` followed by `0x19C` through
the signed `IntelMSR` module on that same logical processor (S7).
Subtract the unsigned digital readout from the target byte for the
temperature in degrees Celsius. TCC activation offset and averaging
settings are thermal-control inputs. The instantaneous DTS readout is
relative to the calibrated target regardless of those settings (S1, S3).

Use each representative's own target read. Apply the common 50–120 °C
inclusive target bound, Reading Valid flag and `0 < temperature ≤ target`
gate (S6). The common demand, deadline, cache, topology recheck, processor
confirmation and affinity restore requirements apply. Isolated failures
leave usable sibling cores and independent package and ACPI results
available. A topology or affinity restoration failure invalidates the
core set and its cache as specified in S6.

### Synthetic acceptance fixtures

| Fixture | Expected behavior |
| --- | --- |
| `0x000C0662` with both capability flags set, valid P and E representatives, and one active group/package, with the brand leaves reporting an observed brand, a synthetic unobserved brand, or no brand at all | Admit one channel per validated Windows physical core in every case. The brand does not change admission. |
| Observed or synthetic brand with `0x000C0661`, `0x000C0664` or `0x000C0652`, or wrong vendor | Reject the Arrow Lake scope before core topology queries or MSR reads. |
| `0x000C0662` with core DTS or hybrid flag clear, regardless of package thermal management. `0x000C0662` with package thermal management clear and both required flags set | Reject the first case. The second remains eligible for core reads independently of the package path. |
| Discovery passes and a pinned target reports a wrong signature or vendor, missing capabilities or type leaf, or unsupported type | Skip both MSRs on that target and retain other eligible targets. |
| A P and E representative use target words `0x00691400` and `0x82690000` with `0x19C = 0x88352800` on each | Report 52 °C on both. Changing unused target bits does not change the result. A valid zero digital readout returns 105 °C with either word. |
| Targets 49, 50, 120 and 121 °C, invalid bit 31, delta equal to or greater than target, or a target-specific fault or processor mismatch | Accept the 50 and 120 °C boundary targets with valid plausible readouts. Leave a rejected core unavailable and preserve valid siblings and independent package data. |
| Changed topology, deadline expiry, failed affinity restoration, or no current core demand | Apply the common revision 5 admission, discard, cache and no-work rules. |

## Quirks

S5 logical CPU 0 reports `CPUID.1AH:EAX = 0x40000003` and CPU 2
reports `0x20000003` in each capture. Both have DTS and hybrid capability.
The low 32 bits below describe those systems. A target of 105 °C is an
observation in these captures.

| SKU | P/E core-type lines | P/E `0x19C` observations | P/E `0x1A2` observations |
| --- | --- | --- | --- |
| 245K | 106, 288 | `0x88470000`, `0x88470000` (1395, 1591) | `0x00691400` on both (1398, 1594) |
| 265K | 107, 273 | `0x88470000`, `0x88490000` (1763, 1979) | `0x00691400` on both (1766, 1982) |
| 270K Plus | 58, 224 | `0x883D2800`, `0x883D2800` (2046, 2262) | `0x00691400` on both (2049, 2265) |
| 275HX | 58, 224 | `0x88352800`, `0x883B2800` (2046, 2262) | `0x02690000` on both (2049, 2265) |
| 290HX Plus | 58, 224 | `0x88352802`, `0x88392800` (2046, 2262) | `0x82690000` on both (2049, 2265) |

S2's S/S Plus 8+16, S 6+8 and HX/HX Plus columns and errata details
identify no erratum for the two scoped thermal reads or their CPUID
gates. ARL035 is for the HWP status MSR `0x777`. ARL051 concerns reading
the core P0 ratio through the BIOS mailbox. ARL056 concerns processor
trace MSRs `0x570` and `0x572`. The scoped procedure uses neither the
mailbox nor those MSRs.

## Safety notes

- Read only `0x1A2` and `0x19C` on eligible confirmed representatives
  through the existing service and signed transport (S6, S7). S3 defines
  the status fields used here as read-only and the sticky thermal logs
  as cleared by a write of zero. This scope performs no MSR writes, log
  clears, thermal-limit changes or control operations.
- Core results retain the common session-local Windows identities and
  never populate package temperatures or package thermal status (S6).

## Open questions

### Other products sharing the signature

Non-blocking for #2514 Experimental core DTS: the scope is keyed by the signature and capability mechanism, and S1/S2 document the DTS semantics for every product line that carries the signature.

S2 lists standard H with `0x000C0652` and U with `0x000B0650`; both are
outside this signature and need their own evidence. Products with
`0x000C0662` other than the five observed SKUs share the documented
decode but have no public readback observation. A maintainer-accepted
Hardware Compatibility Report
([#2500](https://github.com/shm11C3/HardwareVisualizer/issues/2500))
from such a product, or from one of the five, is the graduation evidence
toward a Verified member of this row.

### HardwareVisualizer validation

Non-blocking for #2514 Experimental core DTS: the five independent readbacks establish both core types and the existing service contract bounds access.

The public captures do not exercise the HardwareVisualizer service,
its Windows affinity lifecycle or its fallback behavior.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-09 | Initial Experimental readiness proposal for five exact Arrow Lake SKUs with signature `0x000C0662`. |
| 2 | 2026-11-09 | Replace the five exact brand gates with the signature, capability and core-type mechanism gates of ADR 0031. Keep the five observed brands and their S5 captures as evidence identities inside the Status column, add the gate-kind table, and rewrite the brand fixtures so the brand never changes admission. |
