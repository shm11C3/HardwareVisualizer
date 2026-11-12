# Spec: Intel Tiger Lake B1 core DTS

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental Windows physical-core DTS on exact Tiger Lake B1 signature `0x000806C1`, covering documented UP3, UP4, H35 and IoT UP3 overlap. One active processor group and one physical package. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel [11th Generation Core Processor Datasheet, Volume 1](https://cdrdv2-public.intel.com/631121/631121_TGL%20Datasheet%20Volume1of2%20rev012_PUBLIC.pdf), 631121-012, May 2023, §§4.1.3.1–4.1.3.2 (pp. 74–77), SHA-256 `37745c3e712a39b68a9759d0e62cd1778698de27859f0807e808aaf35fd87cf6` | Primary Core, Pentium and Celeron thermal mechanism for UP3, UP4 and H35. The local MSR reports instantaneous DTS relative to the calibrated target independently of the TCC activation offset. |
| S2 | Intel [11th Generation Core Processor Specification Update](https://cdrdv2-public.intel.com/631123/631123_034.pdf), 631123-034, April 2026, §1.1 (p. 6), §2.1 Table 2-1 (p. 8), §3 (pp. 12–17), and §4 TGL014 and TGL018–020 (pp. 21–22), SHA-256 `9eaacfd64a426fa384034a91a73d59b5262ce2d01381d71429adfad4779f3e4f` | Primary exact signature and applicable product and IoT columns. The update identifies S1 as its affected datasheet and distinguishes core status bit 12 from the readout. |
| S3 | Intel [INTEL-SA-01153](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-01153.html), May 2025, Affected Products `806C1` row, and [Intel microcode release notes](https://github.com/intel/Intel-Linux-Processor-Microcode-Data-Files/blob/bdc92abe5c499c3fd7988b76a128d05c9120a520/releasenote.md), `06-8c-01/80` | Primary exact IoT overlap, including Celeron 6305E/RE and listed embedded Core parts, and the B1 label. Platform and SKU names record evidence identity. |
| S4 | Intel [SDM Volume 4](https://cdrdv2-public.intel.com/929363/335592-093-sdm-vol-4.pdf), 335592-093US, September 2026, Table 2-2 (pp. 2-21–2-22), §2.17 Table 2-39 (pp. 2-359–2-361), and §2.17.4 Table 2-45 (pp. 2-397–2-399) | Primary architectural fields and Core-scope layout for exact display model `06_8C`. The Tiger Lake supplemental table supplies no replacement for `0x19C`. |
| S5 | InstLatx64, commit `17be9a9793c306cf96f8c286e34e705b780a0226`: [Core i5-1135G7](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00806C1_TigerLake_01_CPUID.txt), [Core i7-11375H](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00806C1_TigerLake_CPUID5.txt), [Pentium Gold 7505](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00806C1_TigerLake_CPUID6.txt), and [Celeron 6305](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00806C1_TigerLake_CPUID9.txt) | Independent exact CPUID, package and core topology, and paired per-logical-CPU `0x19C`/`0x1A2` observations. All valid status words in these captures have bit 23 clear. |
| S6 | Linux [Intel-authored thermal mask change](https://github.com/torvalds/linux/commit/6ae0092ca7adfce79336c6525cb0da561ecd4f04) and v6.18 commit `7d0a66e4bb9081d75c82ec4957c50034cb0ea449` [model ID](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/arch/x86/include/asm/intel-family.h#L119-L120), [core mask](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/thermal/intel/intel_tcc.c#L38-L43) and [model selection](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/thermal/intel/intel_tcc.c#L88-L93) | **lead-only (copyleft)** for the conflicting eight-bit core mask explicitly selected for `06_8C`. It is selected by display model and supplies no raw bit-23-set B1 observation. |
| S7 | [Intel core DTS revision 6](https://github.com/shm11C3/HardwareVisualizer/blob/c16b2b2eea8ada07d43567116ed75bf2ed49af75/docs/specs/sensors/cpu-intel-core-dts-msr.md), Detection, Read procedure and decode, Project collection constraints and Safety notes, and [PawnIO interface revision 7](https://github.com/shm11C3/HardwareVisualizer/blob/2495a1b0fbe4a8048feac9d122b98201e7970cbb/docs/specs/sensors/pawnio-interface.md#intelmsr), signed `IntelMSR` `0.2.11` | Common Windows topology, affinity, demand, timeout, partial-result and fault contract. The signed module allows the two synchronous calling-processor reads. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, `GenuineIntel`, exact `CPUID.01H:EAX = 0x000806C1`, and core DTS `CPUID.06H:EAX[0] = 1` at discovery and on each confirmed representative | mechanism | S1–S5, S7 |
| Exactly one active Windows processor group and one physical package, with complete and consistent physical-core topology | mechanism | S7 |
| Existing signed `IntelMSR` transport, disposable sampling thread, confirmed affinity, successful restoration and topology recheck | safety | S7 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Exact Tiger Lake B1 `0x000806C1` core DTS | S1–S4 establish the shared mechanism, UP3/UP4/H35 and documented IoT UP3 overlap. S5 supplies exact Core, Pentium and Celeron observations. Collection rejects samples with the disputed bit set. S6 is Linux lead-only for the conflicting mask. | Experimental under the common gates. |
| Other signatures or missing gates | UP3/H35 Refresh `0x000806C2`, H81 `0x000806D1` and other signatures require separate qualification (S2). | Disabled under this specification. |

Use the existing Windows physical-core representative identities and the
validated actual core masks. Product names identify evidence. Package
thermal management has separate eligibility (S7).

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| `0x19C` | `IA32_THERM_STATUS` | 31 | Core DTS reading valid when set | Boolean | S4, S5 |
| `0x19C` | `IA32_THERM_STATUS` | 22:16 | Unsigned digital readout below Tjmax | Degrees Celsius | S1, S4, S5 |
| `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated Tjmax reference | Degrees Celsius | S1, S5 |

## Read procedure and decode

Apply the common revision 6 non-hybrid sampling procedure (S7). On
each confirmed representative, read the 64-bit `0x1A2` followed by
`0x19C` through the signed module. Require successful reads, a target
in 50–120 °C inclusive and status bit 31 set. Leave the sample
unavailable if core status bit 23 is set, before applying Intel's
seven-bit decode (S4–S6, project safety gate). Subtract the unsigned
delta from the target. Accept only a positive result at most equal to
the target. A valid zero delta returns the target. The DTS reference
is independent of the TCC activation offset and averaging window (S1).

Recheck the running processor after the pair and restore affinity.
An isolated fault or invalid sample leaves that core unavailable and
preserves other readings. Apply S7's demand, rate limit, timeout,
topology and cache rules. Failed affinity restoration or inconsistent
topology invalidates the core set.

## Quirks

- The Core i5-1135G7 capture identifies logical CPUs 0 and 2 as distinct
  physical cores in package 0 (S5, lines 628–631). Their status low
  words are `0x88228282` and `0x881F8282`, and both target low words are
  `0x80640000` (lines 663, 666, 871 and 874). The i7-11375H capture
  shows target `0x05640000` with TCC offset 5 °C and Tjmax 100 °C
  (S1, S5 line 625). Pentium 7505 and Celeron 6305 independently
  corroborate the pair and distinct physical core topology (S5).
- TGL018 affects current-limit status bit 12 of `0x19C`. The decode
  reads the documented valid bit and digital readout fields. TGL014
  limits PECI signaling. TGL019 and TGL020 concern package-C10 exit
  and BIOS-time PROCHOT. S2 marks both fixed or inapplicable for
  UP3, IoT UP3, UP4 and H35.

## Safety notes

The core path issues only the two signed reads in S7. It performs no
thermal, interrupt, power-control or status-log write. Preserve the
independent package path and fallback on partial or unavailable
core collection.

## Synthetic fixture acceptance

| Fixture | Expected behavior |
| --- | --- |
| Exact B1 with core DTS and no package thermal management, one package and group, with SMT and without SMT | Use the observed physical core masks with one representative per core. The core path reads only `0x1A2` and `0x19C`. |
| `0x000806C0`, `0x000806C2`, `0x000806D1`, wrong vendor, absent required leaf, or core DTS clear | No core topology query or core-path MSR read under this scope. |
| Admitted discovery but mismatched pinned representative or missing core DTS | Skip both reads for that target and preserve valid siblings. |
| Valid target 100 °C and delta 30 with activation offset zero and 5 °C, then valid zero delta | Return 70 °C in both offset cases and 100 °C for the zero delta. |
| Synthetic valid-bit-set `0x889E0000` and `0x881E0000` on distinct cores with target 100 °C | The bit-23-set core is unavailable. The other returns 70 °C. |
| Fault, target 0, 49 °C or 121 °C, clear bit 31, or delta at least the target | Leave that sample unavailable and preserve valid siblings. Accept target endpoints 50 °C and 120 °C when the other fields qualify. |
| Multiple groups or packages, changed topology, timeout, or failed restoration | Apply S7's no-result and cache rules. Preserve independent package and fallback results. |

## Open questions

- Non-blocking for #2514 Experimental core DTS: the exact B1 path rejects ambiguous status bit 23 before decoding the documented seven-bit field.
  Intel reserves bit 23 (S4). Linux explicitly includes it in the core
  mask for `06_8C` (S6, lead-only). All valid exact raw observations in
  S5 have it clear. The bit's meaning when set remains unresolved.
- Non-blocking for #2514 Experimental core DTS: Intel's exact product and thermal documents cover the supported mechanism and the public captures corroborate the register pair.
  Separate raw UP4 and IoT UP3 reports and a HardwareVisualizer Windows
  service trace are unavailable. The named observations do not restrict
  runtime admission (S1–S5, S7).

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-10 | Proposes Implementation-ready status for exact Tiger Lake B1 core DTS, pending maintainer approval. |
