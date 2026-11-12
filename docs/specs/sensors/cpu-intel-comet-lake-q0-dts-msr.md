# Spec: Intel Comet Lake-S Q0 core DTS

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental Windows physical-core DTS on exact Comet Lake-S Q0 signature `0x000A0655`, including Core desktop and Xeon W-1200 workstation parts, in one active processor group and one physical package. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel [10th Generation Intel Core Processors Datasheet, Volume 1](https://cdrdv2.intel.com/v1/dl/getContent/615211), 615211-008, February 2022, §5.1.5.2 (p. 82), §5.4.6 (p. 107) and §9.1 Tables 9-2 and 9-3 (p. 136). SHA-256 `0468134adb02156c2addbc3494f111b3cbf166a09469f63b0dc31a5c34120de9`. | Primary client and Xeon Q0 signature and offset-independent instantaneous DTS reference. |
| S2 | Intel [10th Generation Intel Core Processor Datasheet, Volume 2](https://cdrdv2.intel.com/v1/dl/getContent/615212), 615212-003, May 2020, §9.3 (pp. 207–208). SHA-256 `0b4936ca7eeed4fbcdfd741cdf4c0a138b70d07041cba73c8c1aab8af633c345`. | Primary Comet Lake reference to the thermal target and offset MSR. The package interrupt register described there is outside this scope. |
| S3 | Intel [10th Generation Intel Core Processor Specification Update](https://cdrdv2.intel.com/v1/dl/getContent/615213), 615213-013, April 2023, Q0 S102 column of Summary Tables of Changes and Errata Details CML071, CML091 and CML093. SHA-256 `27593f34008f6627fc14eaa5b0310e5fb7dfd20fc82f60c6ff9faa5a81c520ce`. | Primary Q0 errata for HWP thermal interrupts, PROCHOT output and writes to thermal status. |
| S4 | Intel [SDM Documentation Changes](https://cdrdv2-public.intel.com/868136/252046-081-sdm-change-document.pdf), 252046-081, October 2025, replacement Volume 4 §2.17 Table 2-39 (pp. 2-353–2-355), and [SDM Volume 3B](https://cdrdv2-public.intel.com/874250/253669-090-sdm-vol-3b.pdf), 253669-090, §17.8.5 | Primary Comet Lake `06_A5` applicability, architectural DTS capability and Core-scope status and readout fields. |
| S5 | Intel [Processor Servicing](https://www.intel.com/content/www/us/en/support/topics/support-and-servicing-for-processors.html), desktop Core and workstation Xeon W-1200 tables, and [microcode release history](https://github.com/intel/Intel-Linux-Processor-Microcode-Data-Files/blob/bdc92abe5c499c3fd7988b76a128d05c9120a520/releasenote.md#L518-L521) | Corroborates `A0655` on Core i5/i7/i9 desktop and Xeon W-1200 workstation SKUs and distinguishes the H R1 and S G1 signatures. |
| S6 | InstLatx64 [Core i9-10900K raw capture](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00A0655_CometLake_CPUID.txt) and [Core i7-10700K raw capture](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00A0655_CometLake_CPUID2.txt), commit `17be9a9793c306cf96f8c286e34e705b780a0226` | Independent exact-signature CPUID and topology, plus raw status and target on the i9 Windows capture. The i7 capture corroborates raw status. |
| S7 | [Xeon W-1290 Windows inventory](https://xmrig.com/benchmark/EmjbS), benchmark ID `EmjbS` | Independent exact CPUID and one-package workstation topology. No thermal readout. |
| S8 | Linux v6.18 commit `7d0a66e4bb9081d75c82ec4957c50034cb0ea449`, [model ID](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/arch/x86/include/asm/intel-family.h#L106-L107), [model mapping and masks](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/thermal/intel/intel_tcc.c#L38-L83), [target](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/thermal/intel/intel_tcc.c#L174-L196), and [status decode](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/thermal/intel/intel_tcc.c#L268-L304) | **lead-only (copyleft)** for the `0x1A2[23:16]` target field on `06_A5` and corroboration of its seven-bit digital readout. |
| S9 | [Intel core DTS](https://github.com/shm11C3/HardwareVisualizer/blob/c16b2b2eea8ada07d43567116ed75bf2ed49af75/docs/specs/sensors/cpu-intel-core-dts-msr.md) revision 6 and [PawnIO interface](https://github.com/shm11C3/HardwareVisualizer/blob/2495a1b0fbe4a8048feac9d122b98201e7970cbb/docs/specs/sensors/pawnio-interface.md#intelmsr) revision 7, signed `IntelMSR` release `0.2.11` | Common Windows topology, affinity, demand, time, partial-result and failure rules, plus signed calling-processor allow-listed reads. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, `GenuineIntel`, exact `CPUID.01H:EAX = 0x000A0655` and core DTS `CPUID.06H:EAX[0] = 1` at discovery and on each confirmed representative | mechanism | S1, S4–S7, S9 |
| Exactly one active Windows processor group and one physical package, with complete and consistent physical-core topology | mechanism | S9 |
| Existing signed `IntelMSR` path, disposable sampling thread and confirmed affinity, restoration and topology recheck | safety | S9 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Comet Lake-S Q0 Core desktop and Xeon W-1200, full `0x000A0655` | Intel identifies the client and Xeon mechanism and exact stepping (S1, S3–S5). S6 records raw client reads, S7 independently corroborates Xeon identity, and S8 is lead-only for the target field. | Experimental under the common gates. |
| Other signatures or missing gates | Neighboring Comet Lake signatures, including S G1 `0x000A0653`, S P1 `0x000A0654`, H R1 `0x000A0652` and U `0x000A0660`, require separate qualification (S1, S5). | Disabled under this specification. |

Use the observed Windows physical-core masks and representative
processors. Product and board names are evidence identities, and the
scope uses no model-name filter or fixed core count. Package thermal
management (`CPUID.06H:EAX[6]`) has separate eligibility (S4, S9).

## Register map and read procedure

| Address | Bits | Meaning | Source |
| --- | --- | --- | --- |
| MSR `0x19C`, `IA32_THERM_STATUS` | 31 | Core DTS readout valid when set | S4, S6 |
| MSR `0x19C`, `IA32_THERM_STATUS` | 22:16 | Unsigned digital readout in degrees below Tjmax | S1, S4, S8 (lead-only corroboration) |
| MSR `0x1A2`, `MSR_TEMPERATURE_TARGET` | 23:16 | Runtime Tjmax reference in degrees Celsius | S1, S2, S6, S8 (lead-only field definition) |

On the confirmed representative, use the common revision 6
non-hybrid procedure to read 64-bit `0x1A2` and then `0x19C` through
the signed module (S9). Require successful reads, a target in
50–120 °C inclusive and a set status bit 31. Subtract the unsigned
readout from the target. A valid zero readout returns the target.
Accept only a positive result at most equal to the target. Intel
defines the DTS relative to Tjmax independently of the TCC activation
offset (S1), so the core decode applies no offset adjustment or
zero-offset gate.

Recheck the running processor after the pair and restore affinity.
Follow the common demand, rate limit, topology recheck, cache and
timeout rules. A failed or invalid core sample leaves that core
unavailable and preserves independent readings. A topology mismatch
or failed affinity restoration invalidates the core set (S9).

## Synthetic fixture acceptance

| Fixture | Expected behavior |
| --- | --- |
| Exact signature, core DTS set, package thermal management clear, one package and an SMT pair in a core mask | Sample one representative per physical core. The core path requests only `0x1A2` and `0x19C`. |
| Other signatures including `0x000A0652`, `0x000A0653`, `0x000A0654` or `0x000A0660`, wrong vendor, absent required leaf or core DTS clear | Make no core-path topology query or MSR request under this scope. |
| Discovery qualifies and a pinned target's signature or capability differs | Skip both reads for that target. Preserve independent core results. |
| Target 100 °C and valid delta 50, with zero and nonzero TCC offsets | Both return 50 °C. A valid zero delta returns 100 °C. |
| Either read faults, target 0, 49 °C or 121 °C, Reading Valid clear, or delta at least the target | Leave that core unavailable. Accept target endpoints 50 °C and 120 °C when the remaining fields qualify. |
| Multiple groups or packages, inconsistent or changed topology, timeout or failed restoration | Apply S9's no-result and cache rules. Preserve independent package and fallback results. |

## Quirks and safety

The i9-10900K Windows capture identifies one package with ten physical
cores and twenty logical CPUs (S6 lines 1168–1189). Logical CPU 0
reports low words `0x883D0000` and `0x00641200` (lines 1217, 1220).
Logical CPU 2, on a distinct physical core, reports `0x883C0000`
and `0x00641200` (lines 1421, 1424). Both target fields are 100 °C.
The i7-10700K capture independently supplies exact Q0 and raw valid
core status (S6, second capture lines 6, 14 and 930).

The Q0 errata distinguish HWP thermal interrupts (CML071), the
external PROCHOT output in package C10 (CML091), and a possible
fault when writing to read-only thermal status fields (CML093).
The two permitted reads use the digital readout and calibrated
target fields. Thermal control, interrupt programming and status-log
writes are outside this scope (S3, S4, S9).

## Open questions

Non-blocking for #2514 Experimental core DTS: the raw register observation covers a desktop Q0, while the workstation mechanism is identified by Intel's exact Xeon Q0 table and supported by independent Windows inventory.

No raw Xeon thermal words or HardwareVisualizer runtime trace were
available. The lead-only target field supports Experimental coverage
under ADR 0023.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-10 | Proposes Implementation-ready status for exact Comet Lake-S Q0 core DTS, pending maintainer approval. |
