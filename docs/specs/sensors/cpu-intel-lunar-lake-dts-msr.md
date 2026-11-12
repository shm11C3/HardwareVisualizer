# Spec: Intel Lunar Lake B0 P-core temperatures via digital thermal sensors

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental read-only Windows P-core DTS for hybrid Intel Core Ultra 200V Lunar Lake B0, exact CPUID signature `0x000B06D1`, in one active processor group and physical package. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

This supplement uses the common [Intel core DTS specification](cpu-intel-core-dts-msr.md)
revision 6. The target core type and evidence below apply to this exact
signature.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel [SDM Volume 4, 335592-093US, September 2026](https://cdrdv2.intel.com/v1/dl/getContent/671098), Table 2-1 (p. 2-2), §2.17 and Table 2-39 (pp. 2-360–2-362), §2.17.12 and Tables 2-58–2-60 (pp. 2-498–2-503). | Primary model applicability, core MSR status scope, access, valid and delta fields. The Lunar Lake overrides list no replacement for `0x19C`. |
| S2 | Intel [Core Ultra 200V Series Processors Datasheet Volume 1, 829568-004, May 2025](https://cdrdv2-public.intel.com/829568/829568-004.pdf), §§11.7.2–11.7.5 (pp. 108–110), §§14.1.2.2–14.1.2.3 (pp. 127–131) and §14.2 (pp. 135–136). | Primary P/LP E topology, exact calibrated target field and offset-independent DTS semantics for the Core Ultra 5, 7 and 9 product groups. |
| S3 | Intel Core Ultra 200V Series Processors Specification Update 827538 rev 022, September 2026, [Table 2-1](https://edc.intel.com/content/www/us/en/design/products/platforms/details/lunar-lake-mx/827538/identification-information/), [Errata Summary Table](https://edc.intel.com/content/www/us/en/design/products/platforms/details/lunar-lake-mx/827538/summary-tables-of-changes/) and [Errata Details](https://edc.intel.com/content/www/us/en/design/products/platforms/details/lunar-lake-mx/827538/errata-details/). | Primary B0 identification, 4P+4E product scope and stepping errata. |
| S4 | Intel [Affected Processors by CPU Model](https://www.intel.com/content/www/us/en/developer/topic-technology/software-security-guidance/processors-affected-consolidated-product-cpu-model.html), `06_BDH` stepping 1, consulted November 2026. | Primary exact signature and hybrid flag for the Lunar Lake mobile Core Ultra 5, 7 and 9 groups. |
| S5 | InstLatx64 [Core Ultra 9 288V raw CPUID and MSR capture](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00B06D1_LunarLake_04_CPUID.txt), commit `17be9a9793c306cf96f8c286e34e705b780a0226`. | Independent Windows 11 machine observation. CPUID, topology and both thermal MSRs are recorded by logical processor. |
| S6 | First-hand [MSI Prestige 13 AI Evo A2VM Linux Core Ultra 7 258V report](https://github.com/lhl/linuxlaptops/wiki/2024-MSI-Prestige-13-AI--Evo-A2VM/48d7b6094274c569216bc9cd79fdbe6b64dbf231), temperature section. | Decoded coretemp output from a separate system. No raw per-core MSR words. |
| S7 | [Intel core DTS specification](cpu-intel-core-dts-msr.md) rev 6, `c16b2b2eea8ada07d43567116ed75bf2ed49af75`, Detection, Read procedure and decode, and Project collection constraints. | Existing Windows topology, identity, affinity, bounded request and partial-result contract. |
| S8 | [PawnIO interface](pawnio-interface.md#intelmsr) rev 7, `2495a1b0fbe4a8048feac9d122b98201e7970cbb`, signed IntelMSR module release `0.2.11`. | Existing synchronous calling-processor read, allow-list and failure contract for `0x1A2` and `0x19C`. |

## Detection

Apply the common rev 6 hybrid rules with the mechanism and safety
conditions below. The gate kinds follow [ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md).

| Condition | Gate kind | Source |
| --- | --- | --- |
| On x86-64 Windows, CPUID reports `GenuineIntel`, full CPUID.01H:EAX `0x000B06D1`, core DTS in CPUID.06H:EAX[0] and hybrid in CPUID.07H.00H:EDX[15]. Require the leaves and repeat these checks on each pinned target before thermal reads. | mechanism | S1, S3–S5, S7 |
| The pinned processor exposes CPUID.1AH.00H and EAX[31:24] equals `0x40` (P-core). Apply the type gate to the pinned representative. Discovery may run on either core type. | mechanism | S2, S5, S7 |
| The active Windows topology has one processor group and one physical package. The core masks are nonempty, disjoint and cover that package and the group's active mask. | mechanism | S7 |
| The existing signed IntelMSR read path is available. Confirm the selected processor before and after reading, restore previous affinity and validate the topology again before publication. | safety | S7, S8 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact Lunar Lake B0 `0x000B06D1` hybrid mechanism, target type `0x40` | S1–S4 document the exact Core Ultra 200V scope and DTS contract. S5 observes distinct P-core readouts and a calibrated reference on Core Ultra 9 288V. S6 corroborates distinct P-core readings on Core Ultra 7 258V. | Experimental under the gates above. |
| Same signature, target type `0x20` (LP E), missing core-type leaf or any other type | S2 documents one four-core LP module. S5 and S6 show equal values across its four cores in their respective captures. Independent physical-core temperature attribution is unresolved. | Core temperature unavailable for the affected positions. |
| Other full signatures, missing discovery capability, or invalid group/package topology | Outside this supplement (S1, S3, S4, S7). | No core temperature through this scope. |

## Register map

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| MSR `0x19C` | `IA32_THERM_STATUS` | 31 | Core digital readout valid when set | Flag | S1 Table 2-39 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 22:16 | Distance below the calibrated maximum operating temperature | Unsigned degrees Celsius | S1 Table 2-39, S2 §14.1.2.3 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated maximum operating temperature (Tjmax) | Degrees Celsius | S2 §§14.1.2.2–14.1.2.3 |

## Read procedure and decode

Use the common rev 6 acquisition, topology recheck, deadline,
cache and disposable-thread rules (S7). Keep every Windows physical
core's existing identity, including LP E cores. On an eligible pinned
P-core representative, read the 64-bit `0x1A2` followed by `0x19C` via
the signed module on the same calling processor (S8). Skip both MSRs
on an ineligible target. Its existing position has an unavailable
temperature, with no P-core, package or cluster proxy.

Subtract the unsigned digital readout from that representative's
target in degrees Celsius. The DTS reference is independent of the
programmed TCC activation offset and its averaging window (S2).
Apply the common project bounds. Require a successful read of both
registers, target in 50–120 °C inclusive, Reading Valid set and
a result greater than zero and at most the target. A valid zero
delta returns the target. A read failure or invalid response leaves
that core unavailable and preserves independently valid positions.
The package thermal management bit, package temperature path and
ACPI fallback are independent (S7).

## Quirks

- S5 reports one package with eight single-thread physical-core
  records (lines 720–727). Logical processors 0–3 report P-core
  type `0x40`, and 4–7 report LP E-core type `0x20`. All report
  core DTS and the hybrid flag.
- On the four P-cores in S5, `0x19C` low words
  `0x882A2800`, `0x881D0000`, `0x882B0000` and `0x88270000`
  yield 58, 71, 57 and 61 °C respectively against the observed
  `0x85640000` target word (100 °C calibrated target, 5 °C TCC
  activation offset). See lines 753–758, 867–870, 967–970 and
  1067–1070. The observations are successive, and the captured target
  does not replace each target's runtime read.
- S3 LNL039 and LNL040 concern a regulator alert and DRAM thermal
  throttling, respectively. LNL041 describes the HWP status register.
  LNL046 describes a general instruction-execution failure following
  Core C1 exit under complex microarchitectural conditions. The rev
  022 errata review identifies no `0x19C` or `0x1A2` readout or
  read-specific condition. The scope is measurement of the local
  DTS reading.

## Safety notes

- Read only the two allow-listed thermal MSRs on a confirmed P-core.
  S1 defines the used status fields as read-only and thermal log
  fields as write-zero-to-clear. Skip writes and thermal policy
  operations (S1, S7, S8).
- Apply rev 6 topology and affinity rejection rules to the whole
  core set. An isolated unsupported target or failed read leaves
  that position unavailable. Topology mismatch or failed affinity
  restoration rejects the core set and invalidates the cache (S7).

## Synthetic acceptance fixtures

| Fixture | Expected result |
| --- | --- |
| Eligible `0x000B06D1`, core DTS and hybrid set, one group/package, four P and four LP E core records | Four P-core targets qualify, even when discovery runs on an LP E core. All eight existing identities remain in order, and the four LP E positions are unavailable with no thermal MSR reads on them. |
| Adjacent full signatures `0x000B06D0` or `0x000B06D2`, neighboring `0x000B06E0`, or exact signature with DTS or hybrid clear | No topology collection or thermal MSR reads via this supplement. |
| Matching CPU with package thermal management clear, and one P-core has a changed signature, missing leaf 1AH, unknown type or read fault | Skip the ineligible core before either read, or leave the faulted core unavailable. Other P-cores remain eligible. The core path does not read `0x1B1`. |
| Valid target 100 °C with zero or nonzero offset and changed Tau, with deltas 42 and zero | Return 58 °C and 100 °C respectively for either target word. |
| Target 0, 49, 50, 120 or 121 °C with a valid zero delta, or core status with bit 31 clear or delta at least target | Accept only the bounded valid 50 and 120 °C cases. Preserve any other valid core results. |
| Topology changes during a pass or across cache reuse, or affinity restore fails after a P-core sample | Reject the core set and cache under rev 6. Preserve independent package or fallback results. |

## Open questions

### LP E-core thermal granularity

Non-blocking for #2514 Experimental P-core DTS: LP E-core sampling is excluded from this scope.

The SDM defines an architectural core-scope status register (S1),
and the datasheet puts all four LP E cores in one module (S2).
In S5, the four LP E targets return `0x88260000` at lines 1171,
1299, 1427 and 1555. S6 independently reports equal temperatures
on its four corresponding core channels. These observations do not
establish whether the physical temperature source is per core or
shared by the module. Establish the physical granularity before
assigning those values to individual core temperatures.

### Runtime verification

Non-blocking for #2514 Experimental P-core DTS: the exact model and P-core read contract are documented independently of HardwareVisualizer runtime verification.

S5 is a Windows register capture on one exact 288V machine. It
does not exercise HardwareVisualizer, its signed module or service
affinity restoration. S6 is a separate decoded Linux observation.
S1–S4 establish the shared product mechanism across the exact
signature.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-10 | Propose Implementation-ready Experimental P-core DTS for exact Lunar Lake B0, pending maintainer approval. |
