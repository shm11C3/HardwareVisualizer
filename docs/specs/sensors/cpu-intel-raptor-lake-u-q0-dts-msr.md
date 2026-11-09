# Spec: Intel Raptor Lake-U Q-0 core DTS

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental Windows physical-core DTS on hybrid Raptor Lake U and U Refresh with exact signature `0x000B06A3`, in one active processor group and one physical package. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel, [13th and 14th Generation Intel Core Processor Datasheet, Volume 1](https://cdrdv2-public.intel.com/743844/743844-015.pdf), 743844-015, May 2025, §§4.1.3.1–4.1.3.2 (pp. 80–83) and §15.1 Table 101 (p. 219) | Primary U/U Refresh Q-0 signature and instantaneous MSR DTS, calibrated reference and offset independence. |
| S2 | Intel, [SDM Documentation Changes](https://cdrdv2-public.intel.com/868136/252046-081-sdm-change-document.pdf), 252046-081, October 2025, replacement Volume 4 §2.17 Table 2-39 (pp. 2-353–2-355) and §2.17.5 (p. 2-393), and [SDM Volume 3B](https://cdrdv2-public.intel.com/874250/253669-090-sdm-vol-3b.pdf), 253669-090, §17.8.5 | Primary hybrid `06_BA` model applicability, Core scope for `0x19C`, P/E-specific exceptions, capability enumeration and DTS field semantics. |
| S3 | Intel, [13th and 14th Generation Intel Core Processor Specification Update](https://cdrdv2-public.intel.com/740518/740518_032.pdf), 740518-032, October 2026, §2.1 Table 2-1 (pp. 8–9), §3 errata matrix and §4 errata details | Primary exact U and U Refresh product line, including Core Series 1 and Series 2, and applicability of the U/U Refresh errata column. SHA-256 `a1c699fb9a6141ec9c91d05410898afc932e8b98f30ce68a22fd69b7fd2302f2`. |
| S4 | Intel [microcode release history](https://github.com/intel/Intel-Linux-Processor-Microcode-Data-Files/blob/bdc92abe5c499c3fd7988b76a128d05c9120a520/releasenote.md#L358-L365), and [hybrid processor identification](https://www.intel.com/content/www/us/en/developer/topic-technology/software-security-guidance/processors-affected-consolidated-product-cpu-model.html), `06_BA` row, consulted November 2026 | Corroborates the Raptor Lake U Q-0 signature and the model's hybrid flag. The grouped security row does not assign individual marketed SKUs to a stepping. |
| S5 | InstLatx64 [Core i7-1365U CPUID and MSR capture](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00B06A3_RaptorLakeP_01_CPUID.txt), commit `17be9a9793c306cf96f8c286e34e705b780a0226`, lines 1–322 and 324–653 | Independent exact signature, core DTS and hybrid flags, and raw target/status from two P-cores with SMT. All four logical CPUs are type `0x40`. The raw CPUID brand identifies the i7-1365U. The acquisition OS is unspecified. |
| S6 | [Topton Core i7-1355U Linux report](https://wiki.liutyi.info/display/DEVOPS/Topton%2Bi7-1355U%2Bupgraded%2Bgeekbench), `/proc/cpuinfo` and `sensors` sections, and [independent Core i7-1355U Linux report](https://github.com/htop-dev/htop/issues/1335), November 24, 2023 | First-hand Linux signature, topology and decoded `coretemp` channel observations. They corroborate E-core enumeration and separately addressed readings. The Linux reports do not supply raw MSR words or attest Reading Valid. |
| S7 | ASRock Industrial [IMB-1235 User Manual](https://download.asrock.com/IPC/Manual/IMB-1235.pdf#page=32), v1.1, February 6, 2024, §3.3.1 p. 26, and Intel [Time Coordinated Computing User Guide](https://cdrdv2-public.intel.com/851159/Public%20TCC%20User%20Guide%20-%20Q4%202025%20-%20RDC-831067.pdf), 831067, January 2026, Appendix A.2.2 p. 50 | OEM setup capture identifies the embedded i3-1315UE as `0x000B06A3` with 2 P-cores and 4 E-cores. Intel identifies it and the other listed UE parts as 13th Generation U Series. |
| S8 | [Intel core DTS](https://github.com/shm11C3/HardwareVisualizer/blob/c16b2b2eea8ada07d43567116ed75bf2ed49af75/docs/specs/sensors/cpu-intel-core-dts-msr.md) revision 6, Detection, Read procedure and decode, Project collection constraints, and Safety notes | Common Windows topology, affinity, request budget, cache, fault and partial-result contract. This scope has separate signature admission. |
| S9 | [PawnIO interface](https://github.com/shm11C3/HardwareVisualizer/blob/2495a1b0fbe4a8048feac9d122b98201e7970cbb/docs/specs/sensors/pawnio-interface.md#intelmsr) revision 7, `IntelMSR` and signed release `0.2.11` | Existing calling-processor synchronous read interface and allow-list for `0x19C` and `0x1A2`. |
| S10 | [OpenBSD 7.3 ThinkPad X1 Carbon Gen 11 log](https://gist.githubusercontent.com/reyk/82aaabe49add48d1afbd4920046b19b5/raw/2b6dfd2bba9684011447b40ef65c26bae814c5ac/openbsd-thinkpad-x1-carbon-gen11-2023), lines 22–83, and OpenBSD [flag decoding](https://github.com/openbsd/src/blob/d7fb83ec8e606829eabfddeca63857805f59d4fc/sys/arch/amd64/amd64/identcpu.c#L663-L664) and [bit definition](https://github.com/openbsd/src/blob/d7fb83ec8e606829eabfddeca63857805f59d4fc/sys/arch/amd64/include/specialreg.h#L253-L255) | Independent i7-1365U `06-ba-03` boot log with the DTS capability on all twelve logical CPUs, including the eight E-core targets. The log reports capabilities and topology. |
| S11 | Independent XMRig Windows inventory reports for [Core i7-1365U](https://xmrig.com/benchmark/5PS6UY) and [Core 7 150U](https://xmrig.com/benchmark/6RebzH) | Corroborates exact `0x000B06A3` and ten cores/twelve threads on both U and U Refresh. These reports supply no thermal data. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, `GenuineIntel` and full `CPUID.01H:EAX = 0x000B06A3` | mechanism | S1, S3–S7 |
| `CPUID.06H:EAX[0]` (core DTS) and `CPUID.07H.00H:EDX[15]` (hybrid) set | mechanism | S2, S4, S5, S8 |
| On each confirmed pinned target, repeat vendor, signature, DTS and hybrid checks. Require `CPUID.1AH.00H:EAX[31:24]` of `0x40` for P-core or `0x20` for E-core. | mechanism | S2, S4, S5, S8 |
| One active Windows processor group and one physical package with complete, consistent core topology | mechanism | S8 |
| The existing signed `IntelMSR` read path is available and a disposable sampling thread can confirm and restore affinity | safety | S8, S9 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact Q-0 `0x000B06A3` hybrid U and U Refresh | Intel supplies the exact signature-to-platform and thermal contracts (S1–S4). S5 captures P-core registers. S6 and S10 corroborate E-core enumeration, capability and Linux decoded readings. S7 establishes embedded U-series overlap with fewer active cores, and S11 corroborates the U Refresh signature. | Experimental under the mechanism and safety gates. |
| Other signatures or missing capability, topology or transport gates | J-0 `0x000B06A2`, `0x000B06A8` and other signatures are outside this scope. Other approved signatures follow their own specifications. | Disabled under this specification. |

Product, board, firmware and core counts are evidence identities.
The CPU lineup table in the specification update describes the
2P+8E configuration. The same signature can have other enabled core
counts (S7). Use the actual Windows physical-core representatives.
Package thermal management (`CPUID.06H:EAX[6]`) is independent of
core DTS (S2, S8).

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| MSR `0x19C` | `IA32_THERM_STATUS` | 31 | Core digital readout valid when set | Flag | S2, S5 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 22:16 | Core digital readout below Tjmax | Unsigned degrees Celsius | S1, S2 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated Tjmax, the core DTS reference | Degrees Celsius | S1, S5 |

Intel's model table includes hybrid `06_BA` and gives `0x19C`
Core scope. The hybrid-specific section lists distinct P/E MSRs and
exceptions, with no exception for this status register. The product
datasheet supplies the exact U/Q-0 calibrated reference and defines
MSR DTS readouts relative to it (S1, S2). Each result describes the
architectural core DTS readout addressed on its confirmed
representative. The measurement does not identify a physical diode
or its placement on the die.

## Read procedure and decode

Use the common revision 6 hybrid procedure and project constraints
(S8). On the confirmed representative, read 64-bit `0x1A2` and then
`0x19C` through `IntelMSR` (S9). Check the representative after both
reads and restore affinity. Require both reads to succeed, a target
of 50–120 °C inclusive and a set status bit 31. Subtract the unsigned
readout from the target and accept only a positive result at most
equal to the target. A valid zero readout returns Tjmax. The TCC
activation offset and averaging window do not enter this conversion
(S1, S2, S8).

Reuse the common demand, rate limit, topology recheck, cache,
disposable-thread and timeout rules (S8). A failed or invalid core
result leaves that core unavailable and preserves independent readings.
A topology mismatch or failed affinity restoration invalidates the
core set under S8.

### Synthetic acceptance fixtures

| Fixture | Expected behavior |
| --- | --- |
| Exact Q-0 with two SMT threads on one P-core and independent E-cores. Core DTS and hybrid set, package thermal management clear. | One target per physical core and one reading per eligible representative. Use the existing core identity and make no core-path `0x1B1` request. |
| Exact Q-0 with fewer active P/E cores | Use the complete actual topology. Do not infer expected core count from the SKU or the evidence capture. |
| `0x000B06A2`, `0x000B06A8`, another signature, wrong vendor, missing required discovery leaf or clear core DTS/hybrid bit | Make no topology query or MSR request under this scope, regardless of package thermal management. |
| Eligible discovery followed by unexpected signature, absent capability or unsupported type on a pinned representative | Make neither read on that target. Preserve independent eligible core results. |
| Valid delta 50 and reference 100 °C with either zero or nonzero TCC offset, including a nonzero averaging window | Return 50 °C for each. A valid zero delta returns 100 °C. |
| A register faults, target is zero, 49 °C or 121 °C, status bit 31 is clear, or delta is at least the target | Mark that core unavailable. Accept targets of 50 °C and 120 °C when the remaining fields qualify. |
| Multiple groups or packages, inconsistent or changing topology, timeout or failed affinity restoration | Apply S8's no-result and cache rules. Preserve independent package and fallback results. |

## Quirks

| S5 logical CPU | CPUID core type and line | Low 32 bits of `0x19C` and `0x1A2` | Thermal lines |
| --- | --- | --- | --- |
| 0 | `0x40`, line 57 | `0x88180A82`, `0x03640000` | 350, 353 |
| 2 | `0x40`, line 215 | `0x881B0A82`, `0x03640000` | 554, 557 |

The P-core words show a calibrated 100 °C reference with a nonzero
TCC offset (S1, S5). The i7-1365U capture reports the hybrid flag
and samples only the P-core logical CPUs. The i7-1355U Linux report
shows family 6, model 186, stepping 3 and separate decoded channels
for cores 8–15, including 42 °C on core 8 and 43 °C on core 11
(S6). That report corroborates E-core targeting without establishing
raw validity bits or Windows collection.

The S3 U/U Refresh column and corresponding errata details identify
no exception for core DTS readout or calibrated target. RPL063,
concerning an internal voltage request, is marked N/A in that
column (pp. 18, 38).

## Safety notes

- Use only the two named reads on each confirmed representative.
  The used fields are read-only and the status log fields clear
  through a write, which this scope excludes (S2, S9).
- Apply the common topology and affinity rules before accepting a
  sample (S8). Package and ACPI fallback retain their own eligibility.

## Open questions

Non-blocking for #2514 Experimental core DTS: Intel specifies the Q-0 U/U Refresh core-scoped mechanism, with raw P-core evidence and independent Linux E-core channel observations.

The evidence includes no E-core raw thermal words or
HardwareVisualizer Windows service run. The Linux observations establish
neither raw Reading Valid nor the location of a sensor on the die.
Retail Refresh and embedded observations do not include raw thermal
register captures.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-10 | Proposes Implementation-ready status for exact hybrid Raptor Lake U/U Refresh Q-0 core DTS, pending maintainer approval. Records the raw P-core, Linux E-core and embedded identity evidence boundaries. |
