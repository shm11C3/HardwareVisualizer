# Spec: Intel Gracemont 06_BE core temperatures via digital thermal sensors

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental Windows core DTS for the exact non-hybrid Gracemont signature `0x000B06E0` with one active processor group and one physical package. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

This supplement proposes a new exact mechanism under the
[specification gate](README.md#status-transition-draft--implementation-ready).
It follows the non-hybrid procedure and collection constraints of
[Intel core DTS](cpu-intel-core-dts-msr.md) revision 6.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel, [SDM Volume 4, 335592-093US, September 2026](https://cdrdv2.intel.com/v1/dl/getContent/671098), SHA-256 `79f54b67b9bb3836856f04e0c45f50406c4d4b932c64800eb788b6b705a83dbd`, Table 2-1 (p. 2-3, PDF p. 13) and Table 2-2 (pp. 2-21–22) | Primary for the Gracemont display-model mapping to the N-series and Atom x7000 and for the architectural `0x19C` availability, readout and validity fields. |
| S2 | Intel, [Intel Processor and Intel Core i3 and Intel Core 3 N-Series Datasheet, Volume 1, 759603-002, January 2025](https://cdrdv2-public.intel.com/759603/759603-002.pdf), Table 1 (pp. 17–18) and §§12.1.3.1–12.1.3.2 (pp. 134–137) | Primary for the Alder Lake-N and Twin Lake E-core configurations, the factory-calibrated `0x1A2` reference, and the independence of the instantaneous DTS readout from TCC offset and averaging. |
| S3 | Intel, [N-Series Specification Update, 764616-004, January 2025](https://cdrdv2-public.intel.com/764616/764616_004.pdf), §2.1 Table 2-1 (p. 6), Table 3-2 (pp. 8–9) and §4, and [Affected Processors by CPU Model, April 2025](https://www.intel.com/content/dam/develop/public/us/en/include/software-security/affected-processors-2025tab-april2025.html), `B06E0` row | Primary for the exact consumer signature, N0 errata and clear hybrid flag for the Gracemont N-series and Atom X-series at this signature. |
| S4 | Intel, [Atom x7000 and N-Series Edge Specification Update Addendum, 766970-1.6, April 2025](https://cdrdv2-public.intel.com/766970/766970_ADL-N_ASL_TWL_for_ECG_Spec_Update_Addendum_Public_rev1p6.pdf), §2 Tables 1–4 (pp. 7–10), Table 6 (p. 12) and §4 (p. 14), and [Edge Datasheet Addendum, 767454-1.7, February 2026](https://cdrdv2-public.intel.com/767454/767454_Intel_Atom_x7000_Processor_Series_Datasheet_Addendum_Rev1p7.pdf), §1.4 Tables 3–6 (pp. 39–41) and Ch. 2 (pp. 43–49) | Primary for the N0 signatures in the edge update, exact supported Atom SKUs including the later FE additions and explicit applicability of S2 Chapter 12 for thermal capabilities without an override. |
| S5 | InstLatx64, [N95](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00B06E0_AlderLakeN_02_CPUID.txt), [Core i3-N305](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00B06E0_AlderLakeN_03_CPUID.txt) and [Core 3 N355](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00B06E0_TwinLake_01_CPUID.txt) at commit `17be9a9793c306cf96f8c286e34e705b780a0226`, and [x7433RE Windows inventory observation](https://browser.geekbench.com/v6/cpu/19041544) | Independent raw CPUID, processor topology, core status and target MSRs on the three N-series processors. The Atom report corroborates family, model, stepping and observed topology only. |
| S6 | [Intel core DTS](cpu-intel-core-dts-msr.md) revision 6 at `c16b2b2eea8ada07d43567116ed75bf2ed49af75`, sources S2–S6 and S21, Read procedure and decode and Project collection constraints | Architectural per-core status semantics, existing Windows affinity and topology, range and request bounds, and failure policy. |
| S7 | [PawnIO interface](pawnio-interface.md#intelmsr) revision 7, signed IntelMSR `0.2.11` at [`52a7e536`](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/IntelMSR.p), and PawnIO 2.2.0 [native read status](https://github.com/namazso/PawnIO/blob/5cdf470831fdfff3f7f1d06363ca6b230f3bf35a/PawnIO/src/natives_impl_windows.cpp#L437-L448) | Existing signed, synchronous calling-processor read authority for both MSRs and a failing status when a native read faults. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Intel x86-64, `GenuineIntel`, basic CPUID leaf 07H available and exact `CPUID.01H:EAX = 0x000B06E0` | mechanism | S1, S3–S5. Selects the documented Gracemont mechanism at stepping ID 0. |
| `CPUID.01H:EDX[22] = 1` and `CPUID.06H:EAX[0] = 1` | mechanism | S1, S5, S6. Advertise the thermal status MSR and its core DTS fields. The package thermal management bit is independent. |
| `CPUID.07H.00H:EDX[15] = 0` | mechanism | S3, S5. A set hybrid flag excludes this non-hybrid path. |
| One active Windows group and one physical package with validated core masks | mechanism | S6. Defines physical-core attribution and identity. |
| Existing signed IntelMSR route and confirmed affinity to the selected representative with vendor, signature, thermal status, DTS and non-hybrid gates rechecked there | safety | S6, S7. Restricts privileged access and confirms the target before reads. |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Non-hybrid Gracemont, full signature `0x000B06E0` | S1–S4 establish the shared mechanism. S5 independently observes Alder Lake-N N95 and Core i3-N305 and Twin Lake Core 3 N355. S4 applies the thermal chapter to Atom x7211E, x7213E, x7425E, x7211RE, x7213RE, x7433RE, x7835RE, x7203C, x7405C, x7809C, x7433FE and x7835FE. S4 identifies the E, RE and C N0 signatures directly. Its later datasheet lists FE within the same Gracemont Atom series in S1. Every target still requires the full signature and advertised capabilities. | Experimental under the listed gates. |
| Other signatures or a hybrid target | Outside this supplement's evidenced mechanism. Separate approved core DTS scopes retain their own gates. | Disabled for this core path. |

Product names identify evidence. The full signature and capabilities
select the common mechanism without a name, board or firmware test
([ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md)).
S2 Table 1 distinguishes the Alder Lake-N and Twin Lake product sets.
S5 confirms that a clear hybrid bit can coexist with a nonzero
CPUID.1AH leaf. The three captures report core type `0x20` and native
model `1` on each captured processor. This corroborates the E-core
mechanism. Leaf 1AH is unnecessary for attributing the homogeneous
topology.

## Register map (facts)

| Address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| MSR `0x19C` | `IA32_THERM_STATUS` | 22:16 | Digital readout below the calibrated target for the calling core | °C, unsigned | S1, S2, S4–S6 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 31 | Reading Valid | Flag, 1 indicates a valid readout | S1, S6 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated target | °C, unsigned | S2, S4, S5 |

## Read procedure and decode

1. Apply the discovery gates before requesting the core topology. Use
   the existing bounded, demand-driven non-hybrid collection contract
   in S6. Pin the dedicated sampling thread to one representative per
   validated physical core and recheck the gates on it.
2. Read `0x1A2` and `0x19C` on that same calling processor using S7.
   Apply S6's processor confirmation, affinity restoration and topology
   recheck before publication.
3. Require successful reads, a target in the existing inclusive
   50–120 °C range, and Reading Valid set. Subtract the unsigned
   digital readout from the target and require a positive temperature
   within the shared result bounds. A valid zero readout returns the
   target temperature. S2 defines TCC offset and averaging as thermal
   control parameters separate from the instantaneous DTS reference.
   Ignore them in the calculation (S1, S2, S4, S6).
4. Preserve eligible sibling readings after an individual target failure
   under S6's partial-result rules. Keep the existing independent
   package temperature and fallback behavior. No package-status MSR
   read is required for the core path (S6, S7).

## Quirks

- N95 and Core i3-N305 show target word `0x91690000` with a
  105 °C target and a 17 °C TCC offset under S2. The independently
  captured Twin Lake Core 3 N355 shows `0x80690000` with the same
  target and zero offset. Each capture contains valid status words
  from multiple cores (S5). These are observations, and the live target
  is required for every result.
- The embedded thermal addendum lists separate operating limits for
  some Atom SKUs. Its Chapter 2 refers unmodified thermal capabilities
  to S2. Those operating limits do not supply a core DTS reference.
  The same run-time target and existing result bounds apply (S2, S4, S6).
- The N0 errata in S3 and the edge addendum in S4 list no exception
  to the scoped core status, target or decode. S3 erratum 010 concerns
  entry to deep package C-states with certain USB hotplug conditions.
  Erratum 016 concerns the microcode update trigger write. These
  conditions do not describe either scoped read.

## Safety notes

- Use only the existing signed allow-listed reads in S7. Never write
  thermal MSRs, thresholds or logs. The core path requires neither
  a platform-ID MSR nor a board-specific access path (S1–S7).
- Inherit S6's active topology, bounded affinity interval, rechecks
  and fail-closed restoration rules. Use the existing Windows core
  identities and typed demand-driven path.

## Acceptance fixtures

All cases below are synthetic unless citing S5 register observations.

| Case | Required result |
| --- | --- |
| Exact `0x000B06E0`, advertised thermal status and DTS, clear hybrid flag, valid single-group/package topology with one logical processor per core | The existing non-hybrid path produces one slot per physical core. Repeat with PTM clear to verify core independence. |
| Same valid discovery with leaf 1AH returning type `0x20`, or with no 1AH data | The same core path qualifies without requiring hybrid enumeration or labeling. |
| `0x000B06E1`, a different display model, a set hybrid flag or a missing required capability | No new core MSR access. Recheck on the pinned representative and preserve eligible siblings when only one representative fails qualification. |
| S5 N95, N305 and N355 example pairs `0x91690000 / 0x88410000`, `0x91690000 / 0x882C0800` and `0x80690000 / 0x88352000` | 40 °C, 61 °C and 52 °C respectively. Changing only offset or averaging leaves the decoded value unchanged. |
| Valid zero delta at each accepted target boundary of 50 °C and 120 °C | Accept the corresponding target temperature. |
| Zero target, target 49 °C or 121 °C, Reading Valid clear, nonpositive result or fault of either MSR | Leave the affected core unavailable. Preserve valid siblings and independent package/fallback data for an isolated rejection. |
| Topology changes or affinity restoration fails | Apply S6's existing attribution and restoration rules. A batch with unconfirmed topology supplies no core results. |

## Open questions

- Non-blocking for the exact `0x000B06E0` scope: Intel documents the shared thermal mechanism for the listed populations.
  The public raw MSR observations cover N95, Core i3-N305 and Core 3
  N355. No independent Atom thermal-register capture or HardwareVisualizer
  Windows PawnIO run has been established for this scope.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-10 | Propose Implementation-ready Experimental coverage for the shared exact non-hybrid Gracemont `0x000B06E0` mechanism. Pin the consumer and edge scope, offset-independent decode and separate Alder Lake-N and Twin Lake observations. |
