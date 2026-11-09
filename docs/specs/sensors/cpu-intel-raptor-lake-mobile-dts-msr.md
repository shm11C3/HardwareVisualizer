# Spec: Intel Raptor Lake mobile J-0 core DTS

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental Windows physical-core temperatures on hybrid Raptor Lake H, H Refresh, P and PX with exact signature `0x000B06A2`, in one active Windows processor group and one physical package. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel, [13th and 14th Generation Intel Core Processor Datasheet, Volume 1](https://cdrdv2-public.intel.com/743844/743844-015.pdf), 743844-015, May 2025, §§4.1.3.1–4.1.3.2 (pp. 80–83) and §15.1 Table 101 (p. 219) | Primary exact H, H Refresh, P and PX J-0 signature, separate U/U Refresh signature, calibrated Tjmax and DTS readout independent of the thermal-control offset. |
| S2 | Intel, [SDM Documentation Changes](https://cdrdv2-public.intel.com/868136/252046-081-sdm-change-document.pdf), 252046-081, October 2025, replacement Volume 4 §2.17 and Table 2-39 (pp. 2-353–2-355), §2.17.5 (p. 2-393) and Volume 3B core DTS fields | Primary display-model applicability, core scope, status fields and P/E applicability. |
| S3 | Intel, [13th and 14th Generation Intel Core Processor Specification Update](https://cdrdv2-public.intel.com/740518/740518_032.pdf), 740518-032, October 2026, §2.1 (pp. 8–9), §3 errata matrix and §4 errata details | Primary H, H Refresh and P exact identity and errata. SHA-256 `a1c699fb9a6141ec9c91d05410898afc932e8b98f30ce68a22fd69b7fd2302f2`. |
| S4 | Intel, [Microcode release history](https://github.com/intel/Intel-Linux-Processor-Microcode-Data-Files/blob/bdc92abe5c499c3fd7988b76a128d05c9120a520/releasenote.md#L361-L364), and [Affected Processors by CPU Model](https://www.intel.com/content/www/us/en/developer/topic-technology/software-security-guidance/processors-affected-consolidated-product-cpu-model.html), `06_BAH` hybrid row, consulted November 2026 | Primary corroboration of the H/P/PX 6P+8E J-0 group and separate Q-0 U line. The security table groups multiple steppings and identifies the hybrid capability for the model. It does not establish a stepping-specific thermal register contract. |
| S5 | InstLatx64 raw observations at `17be9a9793c306cf96f8c286e34e705b780a0226`, [Core i7-1370P](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00B06A2_RaptorLakeP_03_CPUID.txt) and [Core i9-13900H](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00B06A2_RaptorLakeP_04_CPUID.txt) | Independent exact-signature, hybrid capability, local core types and per-logical-processor MSR observations. Each file associates CPUID and MSRs by logical processor number within that system. |
| S6 | [Intel core DTS](https://github.com/shm11C3/HardwareVisualizer/blob/c16b2b2eea8ada07d43567116ed75bf2ed49af75/docs/specs/sensors/cpu-intel-core-dts-msr.md) revision 6, Detection, Read procedure and decode, Project collection constraints, and Safety notes | Common Windows topology, identity, affinity, request budget, cache, fault and partial-result contract. The signature selection in that document remains separate. |
| S7 | [PawnIO interface](https://github.com/shm11C3/HardwareVisualizer/blob/2495a1b0fbe4a8048feac9d122b98201e7970cbb/docs/specs/sensors/pawnio-interface.md#intelmsr) revision 7, `IntelMSR` and signed release `0.2.11` | Existing synchronous calling-processor read interface and read allow-list for `0x19C` and `0x1A2`. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, CPUID leaf 0 vendor `GenuineIntel` and full `CPUID.01H:EAX = 0x000B06A2` | mechanism | S1–S5 |
| `CPUID.06H:EAX[0]` (core DTS) and `CPUID.07H.00H:EDX[15]` (hybrid) set | mechanism | S2, S4–S6 |
| On each confirmed pinned target, repeat vendor, signature, DTS and hybrid checks. Require `CPUID.1AH.00H:EAX[31:24]` to equal `0x40` for P-core or `0x20` for E-core. | mechanism | S2, S4–S6 |
| One active Windows processor group and one physical package with complete, consistent core topology | mechanism | S6 |
| The existing signed `IntelMSR` read path is available and the disposable sampling thread can confirm and restore affinity | safety | S6, S7 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Hybrid Raptor Lake H, H Refresh, P and PX, exact J-0 `0x000B06A2` | Intel identifies the exact product lines and stepping (S1, S3, S4) and supplies a shared register contract (S1, S2). The Core i7-1370P and Core i9-13900H observations cover both core types (S5). | Experimental under the capability, topology, affinity and read-validity gates. |
| Other signatures or a missing gate | U/U Refresh Q-0 `0x000B06A3` and other `06_BA` steppings require a separate disposition. Previously qualified signatures follow their own specifications (S1, S3, S4, S6). | Disabled under this specification. |

Package thermal management (`CPUID.06H:EAX[6]`) is independent of core
DTS. Product name, firmware version and core count are evidence
identities. The existing Windows group and representative processor
number remain the core identity (S1, S4–S6).

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| MSR `0x19C` | `IA32_THERM_STATUS` | 31 | Core digital readout valid when set | Flag | S2, S5 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 22:16 | Core digital readout below Tjmax | Unsigned degrees Celsius | S1, S2 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated Tjmax, the core readout reference | Degrees Celsius | S1, S5 |

## Read procedure and decode

Use the common revision 6 hybrid procedure and project constraints
(S6). On the confirmed representative, read 64-bit `0x1A2` followed by
`0x19C` through `IntelMSR` (S7). Check the representative after
both reads and restore affinity. Require successful reads, a target in
50–120 °C inclusive, and a set status bit 31. Subtract the unsigned
readout from the target and accept only a positive result no greater
than the target. A valid zero readout returns Tjmax. The calculation
uses the calibrated target independently of the TCC activation offset
and its averaging window (S1, S2, S6).

Reuse the common demand, rate limit, topology recheck, cache,
disposable-thread and timeout rules (S6). A failed or invalid core
result leaves that core unavailable and preserves independent readings.
A topology mismatch or failed affinity restoration invalidates the
core set under S6.

### Synthetic acceptance fixtures

| Fixture | Expected behavior |
| --- | --- |
| Eligible J-0, one P-core with two SMT threads and one E-core, core DTS and hybrid set, package thermal management clear | Use one target per physical core and return both eligible results through the existing identity. No core-path `0x1B1` request. |
| `0x000B06A3`, `0x000B06A8`, any other full signature, wrong vendor, missing required leaf, or core DTS or hybrid bit clear at discovery | No core topology query or MSR requests under this scope, regardless of package thermal management. |
| Discovery passes and a representative has an unexpected signature, missing core DTS or hybrid flag, or missing or unsupported core type | Skip both reads for that core and preserve sibling results. |
| Valid readout 50 with target 100 °C and a zero or nonzero TCC offset, including differing averaging windows | Return 50 °C in each case. A valid readout of zero returns 100 °C. |
| A read faults, target is zero, 49 °C or 121 °C, Reading Valid is clear, or readout is at least the target | Mark the affected core unavailable. Accept targets of 50 °C and 120 °C when the remaining fields qualify. |
| Multi-group or multi-package discovery, changing topology during a pass, timeout, or failed affinity restoration | Apply the common revision 6 no-result and cache rules. Preserve independent package and fallback results. |

## Quirks

| Capture in S5 | Core type and logical CPU | Core-type line | Low 32 bits of `0x19C` and `0x1A2` | Thermal lines |
| --- | --- | --- | --- | --- |
| Core i7-1370P | P, CPU 0 | 57 | `0x88322A82`, `0x03640000` | 1598, 1601 |
| Core i7-1370P | E, CPU 12 | 1001 | `0x88302A82`, `0x03640000` | 2766, 2769 |
| Core i9-13900H | P, CPU 0 | 57 | `0x883F280A`, `0x0D640000` | 1598, 1601 |
| Core i9-13900H | E, CPU 12 | 1001 | `0x883A280A`, `0x0D640000` | 2814, 2817 |

Both captures show a calibrated 100 °C reference with nonzero TCC
activation offsets. They establish register observations on P and E
targets (S5). The temperature conversion follows the product
datasheet (S1).

The S3 errata matrix and details identify no core DTS status,
calibrated reference or local read protocol exception for P/H/H
Refresh J-0. RPL063 concerns internal voltage requests and is marked
N/A in that column. Intel lists PX with the same J-0 signature and
microcode group as P/H (S1, S4).

## Safety notes

- Read only the two named registers on the confirmed eligible
  representative. Used status fields are read-only and the log bits
  use write-zero-to-clear semantics (S2, S7). The scope includes no
  register writes, event-log clears or changes to thermal protection.
- Apply the common per-request topology and affinity rules before
  accepting a reading (S6). Package temperature and ACPI retain their
  own eligibility and fallback behavior.

## Open questions

Non-blocking for #2514 Experimental core DTS: the H Refresh and PX lines share the exact Intel-documented J-0 mechanism and product thermal contract, with P and H raw observations.

The cited raw observations cover H and P. This revision has no
independent PX or H Refresh thermal capture and no HardwareVisualizer
runtime trace. The common service, affinity and fault rules apply
to every eligible target.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-10 | Proposes Implementation-ready status for exact hybrid Raptor Lake mobile J-0 core DTS, pending maintainer approval. Pins the H, H Refresh, P and PX mechanism and P/E register observations. |
