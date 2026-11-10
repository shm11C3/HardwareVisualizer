# Spec: Intel Rocket Lake core temperature via digital thermal sensor MSRs

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental read-only Windows core DTS for full CPUID signature `0x000A0671` in one active Windows processor group and one physical package. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

This supplement proposes one additional signature under the
[common Intel core DTS specification](cpu-intel-core-dts-msr.md)
revision 6. It uses the common collection and safety requirements.
Maintainer approval is required under the
[specification gate](README.md#status-transition-draft--implementation-ready).

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel, [SDM Volume 4, 335592-093US, September 2026](https://cdrdv2.intel.com/v1/dl/getContent/671098), SHA-256 `79f54b67b9bb3836856f04e0c45f50406c4d4b932c64800eb788b6b705a83dbd`, Tables 2-1 and 2-2 | Primary display model `06_A7` and architectural `0x19C` fields for a DTS-capable core. |
| S2 | Intel, [Rocket Lake desktop and workstation datasheet Volume 1, 634648-004, May 2022](https://cdrdv2-public.intel.com/634648/634648-004.pdf), cover and §§5.1.5.1.1, 5.1.5.2 (pp. 92–95) | Primary Core desktop and Xeon workstation Tjmax, per-core DTS and offset-independent temperature reference. |
| S3 | Intel, [Xeon E-2300 datasheet Volume 1, 662318-001, October 2021](https://cdrdv2-public.intel.com/662318/662318_Intel_Xeon%20E-2300%20Datasheet%20Vol1_Rev001.pdf), §§5.1.4.1.1, 5.1.4.2 (pp. 86–88) | Primary independent server product definition of the same reference and core readout. |
| S4 | Intel, [Rocket Lake-S specification update, 634808-012, April 2023](https://cdrdv2-public.intel.com/634808/634808_012.pdf), Table 1 (p. 7), Rocket Lake product list reference (p. 8), summary and RKL025/RKL026 (pp. 10, 18) | Primary exact S stepping and its thermal errata. |
| S5 | Intel, [Xeon E-2300 specification update, 709192-004US, May 2023](https://cdrdv2-public.intel.com/709192/709192_Intel%C2%AE%20Xeon%C2%AE%20E-2300%20Processor%20Family%20Specification%20Update_Rev004US.pdf), §3.1 (p. 7), summary and RKL25/RKL26 (pp. 10, 17) | Primary exact E-2300 stepping and its thermal errata. |
| S6 | Intel, [Affected Processors by CPU Model](https://www.intel.com/content/www/us/en/developer/topic-technology/software-security-guidance/processors-affected-consolidated-product-cpu-model.html), `06_A7` stepping 1 entry, consulted November 2026 | Primary exact signature shared by 11th-generation Core desktop, Xeon E-2300 and Xeon W-1300. |
| S7 | InstLatx64, raw captures at commit `17be9a9793c306cf96f8c286e34e705b780a0226`, [Core i9-11900K](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00A0671_RocketLake_01_CPUID.txt#L1262-L1265) and [Xeon E-2356G](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00A0671_RocketLakeE_01_CPUID.txt#L930-L933) | Independent same-system exact CPUID and thermal MSR observations. |
| S8 | [Common Intel core DTS](cpu-intel-core-dts-msr.md) revision 6 (`c16b2b2eea8ada07d43567116ed75bf2ed49af75`), sources S2–S6 and S21, Detection, Read procedure and decode, and Project collection constraints | Architectural capability bits, Windows affinity/topology and service contract, transport fault handling and project sample gates. |
| S9 | [PawnIO interface](pawnio-interface.md#intelmsr) revision 7 (`2495a1b0fbe4a8048feac9d122b98201e7970cbb`), signed IntelMSR module release `0.2.11` | Existing Intel vendor gate, synchronous calling-processor read and allow-list for `0x19C` and `0x1A2`. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Intel x86-64, available CPUID leaves, full `CPUID.01H:EAX = 0x000A0671`, and core DTS `CPUID.06H:EAX[0]` set at discovery and on each pinned representative | mechanism | S1–S8. Selects the documented Rocket Lake read mechanism. |
| Exactly one active Windows processor group and one physical package, with nonempty, disjoint core masks that cover the active package mask | mechanism | S8. Qualifies the common physical-core attribution path. |
| Approved signed IntelMSR transport available, one confirmed pinned target for both reads, affinity restored, and matching topology snapshots before publication | safety | S8, S9. Applies the existing privileged read and core attribution contract. |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Rocket Lake, full signature `0x000A0671` | S2 and S6 cover 11th-generation Core desktop and Xeon W-1300. S3, S5 and S6 cover Xeon E-2300. S4 pins the S stepping. S7 observes the exact mechanism on Core i9-11900K and Xeon E-2356G. | Experimental under the common gates. |
| Other full signatures, absent core DTS or topology outside the common scope | No qualified mechanism or attribution under this supplement (S4–S6, S8). | Disabled for this core path. |

Package thermal management, `CPUID.06H:EAX[6]`, gates the existing
package path independently. Product names identify the evidence in the
scope row and fixtures. The full CPUID signature and core DTS bit select
the mechanism (S1, S6–S8).

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| MSR `0x19C` | `IA32_THERM_STATUS` | 31 | Valid local core digital readout when set | Flag | S1, S3–S5 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 22:16 | Local core digital readout below Tjmax | Unsigned degrees Celsius | S1–S5 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Calibrated Tjmax reference for the DTS | Degrees Celsius | S2, S3 |

## Read procedure and decode

1. Apply the common revision 6 request, topology, affinity, timeout and
   cache rules. Select one representative per Windows physical-core
   record and confirm the full signature and core DTS on that target
   before MSR access (S8).
2. Read `0x1A2` and `0x19C` as two 64-bit values through the same
   confirmed logical processor via signed IntelMSR. Follow the common
   recheck and restore rules. A failed transport status invalidates
   that sample even if it carries output bytes (S8, S9).
3. Require Reading Valid and a target byte in 50–120 °C inclusive.
   Subtract the unsigned core delta from the target byte and accept
   only a result greater than zero and at most the target. A zero
   delta with valid status returns Tjmax. The product DTS measures
   relative to calibrated Tjmax. TCC activation offset and the thermal
   averaging window affect control policy and are unused in the
   temperature conversion (S1–S3, S8).
4. Keep each core's existing Windows topology identity. A rejected
   core in a valid pass leaves its position unavailable and preserves
   siblings and the independent package or ACPI result. Apply the
   common whole-pass and cache invalidation rules on topology mismatch
   or affinity restore failure (S8).

### Synthetic acceptance fixtures

| Fixture | Expected behavior |
| --- | --- |
| Exact signature with core DTS, one group and package, and the S7 i9-11900K words `0x80641000` target and `0x884D0000` status, or E-2356G words `0x80640A00` and `0x88458800` | Decode 23 °C and 31 °C respectively. Repeat with an unobserved brand and PTM clear. Core collection stays eligible and issues no package MSR read. |
| Adjacent signature `0x000A0670` or `0x000A0672`, or exact signature with core DTS clear, with either PTM state | Skip this core topology query and both core MSR reads. The existing package path follows its own gates. |
| Exact discovery with changed signature or clear core DTS on one pinned representative | Skip both reads for that target. Preserve valid siblings. |
| Same target 100 °C and delta 30 °C with zero or nonzero TCC offset, then another averaging window | Return 70 °C in each case. A valid zero delta returns 100 °C. |
| Zero or all-ones target word, target byte 49/50/120/121 °C, or digital delta equal to the target | Reject zero, all-ones and out-of-range targets. Accept the 50 °C and 120 °C bounds with valid positive results. Reject a decoded zero. |
| Clear Reading Valid, all-ones status word, or failed target/status read alongside a valid sibling | Mark only the failed core unavailable and preserve valid siblings and independent package or ACPI data. |
| Two logical threads for one physical core, multiple groups or packages, topology change after sampling, or failed affinity restore | Sample one representative for the SMT core. Reject unsupported topology before reading. Invalidate the core set and cache for topology changes or restoration failure as specified in S8. |

## Quirks

- RKL025 and RKL25 describe temperature gradients between the thermal
  sensor and other circuits that can impair throttling under heavy
  workloads. This readout is the local core DTS value. It does not
  establish the hottest circuit temperature or whether thermal
  protection works correctly. The errata describe throttling behavior
  and give no alternative conversion for the local reading (S4, S5).
- RKL026 and RKL26 concern writes of nonzero values to read-only
  `0x19C` fields. The two-register path performs reads only (S4, S5,
  S8, S9).

## Safety notes

Read only the two documented MSRs on a confirmed eligible core.
Never write status, clear log bits, change limits, or change thermal
control settings. The existing demand, rate limit, timeout and
disposable-thread rules bound privileged collection (S1, S8, S9).

## Open questions

Non-blocking for #2514 Experimental core DTS: the register and attribution contract is documented independently of an application runtime trace.

The cited observations cover Core desktop and Xeon E-2300. The
Xeon W-1300 path relies on the shared desktop/workstation thermal
datasheet and Intel's exact signature table. There is no independent
Xeon W-1300 raw capture or Windows PawnIO runtime validation cited here.
The published thermal-protection errata are documented for S and E-2300.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-10 | Propose Implementation-ready Experimental coverage for exact Rocket Lake signature `0x000A0671` using the existing Windows IntelMSR core DTS path. |
