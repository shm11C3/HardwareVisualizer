# Spec: Intel Skylake U/Y core temperatures via digital thermal sensors

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental Windows core DTS for the Skylake U, U23e and Y mechanism with full CPUID signature `0x000406E3`, one active processor group and one physical package. |
| Issue phase | [#2514](https://github.com/shm11C3/HardwareVisualizer/issues/2514) |

This supplement proposes one exact signature under the
[specification gate](README.md#status-transition-draft--implementation-ready).
It uses the non-hybrid procedure and collection constraints in
[Intel core DTS](cpu-intel-core-dts-msr.md) revision 6. Of the Skylake
population deferred in that revision, this supplement qualifies only
`0x000406E3`.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel, [SDM Volume 4, 335592-079US, March 2023](https://cdrdv2-public.intel.com/774500/335592-sdm-vol-4.pdf), Table 2-2, §2.17 (p. 2-284), and Table 2-39 (pp. 2-285–2-286). Table 2-20 (p. 2-185) is among the inherited interfaces in §2.17. | Primary display model and core `0x19C` layout. The Skylake product datasheet S2 specifies the applicable target reference and offset behavior. |
| S2 | Intel, [6th Generation Intel Processor for U/Y-Platforms Datasheet, Volume 1, 332990-009, February 2022](https://cdrdv2-public.intel.com/332990/332990-6th-gen-core-family-mobile-u-y-processor-lines-datasheet-vol-1-rev009.pdf), §1 (p. 11) and §§5.1.5.1–5.1.5.2 (pp. 83–85) | Primary U/Y platform scope including the U configurations with on-package cache, factory-calibrated reference and independence from TCC offset and averaging window. |
| S3 | Intel, [Retpoline: A Branch Target Injection Mitigation](https://www.intel.com/content/www/us/en/developer/articles/technical/software-security-guidance/technical-documentation/retpoline-branch-target-injection-mitigation.html), Table 5, and [INTEL-SA-00329 microcode update guidance](https://software.intel.com/content/dam/www/public/us/en/security-advisory/documents/sa00329-microcode-update-guidance.pdf), pp. 5–6 | Primary full signature by product line, including U23e and the separate H/S/Xeon E3 signature. |
| S4 | Intel, [6th Generation Intel Processor Family Specification Update, 332689-030, July 2022](https://web.archive.org/web/20230308232303id_/https://cdrdv2-public.intel.com/332689/332689_030.pdf), PDF SHA-256 `4f0a9fd86c8ef4ea39694554069826e326f7844acca87a0b9651698b820c6238`, Tables 1–3 and errata SKL135 (p. 63) and SKL160 (p. 69) | Primary stepping and errata applicability. Table 1 p. 16 places SKL135 in the S columns only. |
| S5 | [Acer F5-573G-59AJ report](https://community.acer.com/en/discussion/comment/471608), i5-6200U thread 0, and InstLatx64 [Core i7-6500U](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00406E3_Skylake_CPUID.txt) and [Core m3-6Y30](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel00406E3_Skylake_CPUID5.txt) at `17be9a9793c306cf96f8c286e34e705b780a0226` | Independent same-boot CPUID and MSR words on U and Y. S1/S2 define their interpretation. |
| S6 | [Intel core DTS](cpu-intel-core-dts-msr.md), revision 6 at `c16b2b2eea8ada07d43567116ed75bf2ed49af75`, sources S2–S6, Read procedure and decode and Project collection constraints | CPUID capability meanings, Windows processor identity and affinity, and shared collection policy. The [current core selector](https://github.com/shm11C3/HardwareVisualizer/blob/0157e66a3b8b2692087d23680a1694d5191768ff/core/src/infrastructure/providers/windows/pawn_io/cpu/core_temperature.rs#L51-L94) separates non-hybrid and hybrid paths. |
| S7 | [PawnIO interface](pawnio-interface.md#intelmsr), revision 7, signed IntelMSR `0.2.11`, and PawnIO 2.2.0 [native read status](https://github.com/namazso/PawnIO/blob/5cdf470831fdfff3f7f1d06363ca6b230f3bf35a/PawnIO/src/natives_impl_windows.cpp#L437-L448), [library status conversion](https://github.com/namazso/PawnIO/blob/5cdf470831fdfff3f7f1d06363ca6b230f3bf35a/PawnIOLib/PawnIOLib.cpp#L109-L115) and [module read contract](https://github.com/namazso/PawnIO.Modules/blob/52a7e536dff3e53c96917a28caac5e0fa6510696/IntelMSR.p#L102-L118) | Existing signed, synchronous calling-processor path authorizes both MSRs and returns failure for a faulted native read. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| `GenuineIntel`, basic CPUID leaf 06H available, and exact `CPUID.01H:EAX = 0x000406E3` | mechanism | S1, S3, S4, S6. Selects the documented U/Y target and core status mechanism. |
| `CPUID.06H:EAX[0] = 1` | mechanism | S1, S6. Advertises core DTS. The package thermal management bit is independent. |
| No hybrid capability enumerated in `CPUID.07H.00H:EDX[15]` | mechanism | S5, S6. A set hybrid bit excludes this non-hybrid path. An unavailable leaf 07H does not enumerate the bit. |
| One active Windows processor group, one physical package, and validated core masks | mechanism | S6. Defines core attribution and identity. |
| Existing signed IntelMSR route and confirmed affinity to the selected representative with vendor, signature, DTS and non-hybrid gates rechecked there | safety | S6, S7. Limits privileged reads and prevents attributing a different processor's data. |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Skylake U, U23e and Y, full signature `0x000406E3` | S1–S4 establish the common mechanism and errata boundary. S5 records i5-6200U, i7-6500U and m3-6Y30 as evidence identities. S6/S7 establish the shared collection and signed access. | Experimental under the listed gates. |
| Other signatures, including Skylake H, S and Xeon E3 `0x000506E3` | S3/S4 identify different product populations outside this supplement. | Disabled for this core path. |

S2 includes U GT3 processors with on-package cache. S3 assigns U23e
to the same signature. The Intel [lifecycle listing](https://www.intel.com/content/www/us/en/support/topics/support-and-servicing-for-processors.html)
groups i5-6300HQ and i5-6440HQ with `406E3`. S4 places both in the H
line and defines its separate display model in Table 3. S3's named H
microcode rows explicitly assign those SKUs `506E3`.

## Register map (facts)

| Address | Name | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| MSR `0x19C` | `IA32_THERM_STATUS` | 31 | Reading Valid for the physical core | Flag | S1 Tables 2-2 and 2-39 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 22:16 | Distance below the calibrated DTS reference | Unsigned degrees Celsius | S1 Tables 2-2 and 2-39, S2 §5.1.5.2 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated Tjmax used as the reference | Degrees Celsius | S2 §§5.1.5.1–5.1.5.2 |

## Read procedure and decode

Apply the revision 6 demand, topology-recheck, disposable-thread,
affinity-restore, cache, timeout and partial-result rules (S6). Use one
representative per physical core and confirm its identity before and
after reading `0x1A2` followed by `0x19C` through signed IntelMSR on
that same processor (S7). Both reads are 64-bit. Use each target's
own reference word.

Subtract the unsigned digital readout from the target byte. The TCC
activation offset lowers the thermal-control threshold while the
digital readout remains relative to Tjmax. Temperature conversion and
admission are independent of the offset and its averaging window (S2).
The revision 6 zero-offset masks apply only to their own legacy rows.
Require successful reads, Reading Valid set, target in 50–120 °C inclusive
and result greater than zero and at most the target (S6).
A faulted or invalid reading leaves only that core unavailable if the
common pass checks succeed.

## Quirks

| Scope | Consequence | Source |
| --- | --- | --- |
| SKL135 on DDR4 | Intel gives an error range of −15 to +5 °C on S R-0 and S S-0. Table 1 leaves U D-1/K-1 and Y D-1 unmarked. This signature needs no memory-type gate for this erratum. | S4 |
| SKL160 | Writing nonzero values to read-only fields in `0x19C` can fault. The sampling procedure reads only. | S4, S7 |

## Observed readback

| Evidence identity | CPUID.06H:EAX | `0x1A2` | `0x19C` | Interpretation |
| --- | --- | --- | --- | --- |
| Core i5-6200U | `0x000007F7` | `0x03640000` | `0x88200800` | 100 °C reference, 3 °C TCC offset, valid 32 °C delta, giving 68 °C. |
| Core i7-6500U | `0x000027F7` | `0x03640000` | `0x88380800` | 100 °C reference, 3 °C TCC offset, valid 56 °C delta, giving 44 °C. |
| Core m3-6Y30 | `0x000027F7` | `0x8A640080` | `0x88253800` | 100 °C reference, 10 °C TCC offset, valid 37 °C delta, giving 63 °C. |

Each row is one system with full signature `0x000406E3` (S5).
The TCC offset interpretation comes from S2 §5.1.5.1.1. These
observations corroborate the fields. Sensor accuracy was not measured.

## Synthetic acceptance fixtures

| Fixture | Expected behavior |
| --- | --- |
| Exact signature and DTS capability with one group/package and two physical cores with two logical siblings each | One channel per physical core with the existing session-local identity. |
| Valid exact signature with an unobserved brand string and PTM clear or set | The core path is eligible in both cases. The existing package path operates independently. |
| `0x000406E2`, `0x000406E4`, `0x000506E3`, or exact signature with DTS clear or the hybrid bit set | No core topology discovery or MSR reads for this supplement. |
| i5-6200U or m3-6Y30 words from the S5 rows and variants with zero offset or another averaging window | 68 °C or 63 °C respectively, unchanged by the synthetic offset and window changes. |
| Pinned representative loses its signature or DTS match, reports the hybrid bit, one read faults, or Reading Valid clears | Leave that core unavailable and preserve usable siblings and independent package/ACPI results. Skip reads when the pinned capability check fails. |
| Target 49, 50, 120 or 121 °C with a valid zero delta, or delta at least as large as a valid target | Accept 50 and 120 °C for the zero delta. Reject 49 and 121 °C and a nonpositive decoded result. |
| Multiple groups or packages, invalid core masks, topology change, or affinity restore failure | Apply the common revision 6 rejection and cache rules. Keep the existing package and ACPI results independent. |

## Open questions

- Non-blocking for the Experimental scope: S1–S5 establish the mechanism
  and S6/S7 establish the approved read path. The independent captures
  do not exercise the HardwareVisualizer Windows sensor service.
  Application-level hardware validation remains separate.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-09 | Propose the exact Skylake U/U23e/Y core DTS mechanism as Experimental. |
