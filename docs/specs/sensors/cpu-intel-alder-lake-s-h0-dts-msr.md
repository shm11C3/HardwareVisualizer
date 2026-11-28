# Spec: Intel Alder Lake-S H0 (6+0) core DTS

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental Windows physical-core DTS on the non-hybrid Alder Lake-S 6+0 die, exact CPUID signature `0x00090675` with the hybrid flag clear, in one active processor group and one physical package. Excludes the same signature with the hybrid flag set, other Alder Lake and Raptor Lake steppings, and package temperature. |
| Issue phase | [#2700](https://github.com/shm11C3/HardwareVisualizer/issues/2700) (epic [#2510](https://github.com/shm11C3/HardwareVisualizer/issues/2510)) |

This supplement proposes a new exact mechanism under the
[specification gate](README.md#status-transition-draft--implementation-ready).
Maintainer approval of the PR that adds it is the sign-off. It follows the
non-hybrid procedure and collection constraints of
[Intel core DTS](cpu-intel-core-dts-msr.md) revision 8.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel, [SDM Volume 4, 335592-093US, September 2026](https://cdrdv2-public.intel.com/929363/335592-093-sdm-vol-4.pdf), SHA-256 `79f54b67b9bb3836856f04e0c45f50406c4d4b932c64800eb788b6b705a83dbd`: Table 2-1 (p. 2-1), Table 2-2 `19CH` (pp. 2-21–2-22), Table 2-20 `1A2H` (p. 2-232), §2.17 (pp. 2-358–2-359), Table 2-39 `19CH` (pp. 2-360–2-361) and §2.17.5 (p. 2-400) | Primary. Display model `06_97H` belongs to the 12th generation Core rows. §2.17 applies Tables 2-20 and 2-39 to `06_97H`. Table 2-39 gives `IA32_THERM_STATUS` Core scope, the readout in bits 22:16, bits 26:23 reserved and Reading Valid in bit 31. Table 2-20 defines the target in `MSR_TEMPERATURE_TARGET` bits 23:16. The hybrid P-core and E-core tables in §2.17.5 add no row for `19CH` or `1A2H`. Table 2-2 makes the readout and validity fields depend on `CPUID.06H:EAX[0]` and the register on `CPUID.01H:EDX[22]`. |
| S2 | Intel, [12th Generation Intel Core Processors Datasheet, Volume 1 of 2, 655258-011, May 2025](https://cdrdv2-public.intel.com/655258/655258-011.pdf), SHA-256 `826f5d275896887026720a413f2a913594ef070891dd864fd8e576685fe7204b`: Table 1 (pp. 13–14), §4.1.3.1 (pp. 76–77), §4.1.3.2 Digital Thermal Sensor (p. 79), and revision 011 history | Primary. Table 1 lists LGA1700 S-processor lines with 6, 4 and 2 P-cores and no E-cores. TjMAX is factory calibrated and readable in `0x1A2` bits 23:16. The TCC activation offset is in bits 29:24 and the averaging window in bits 6:0. Each is a thermal-control setting. The MSR DTS readout is instantaneous and relative to TjMAX regardless of the TCC activation offset. Revision 011 adds S 6+0 detail to the same datasheet. |
| S3 | Intel, [12th Generation Intel Core Processor Specification Update, 682436-041, October 2026](https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/alder-lake-desktop/682436/041/identification-information/), Table 2-1, and [Summary Tables of Changes](https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/alder-lake-desktop/682436/041/summary-tables-of-changes/), S column, errata ADL001–ADL094, consulted 2026-11-28 | Primary. Table 2-1 assigns `0x90675` (stepping ID `0101b`) to the ADL-S 6+0 configuration only. The other rows are ADL-S/HX 8+8 at `0x90672`, ADL-H/P 6+8 at `0x906A3` and ADL-U 2+8 at `0x906A4`. The errata table has one S column for the S processor line. ADL034, the only erratum whose title concerns PROCHOT#, thermal state or temperature, is N/A for S. No erratum title names the thermal status or target MSR. |
| S4 | Intel, [Linux Processor Microcode Data Files release notes](https://github.com/intel/Intel-Linux-Processor-Microcode-Data-Files/blob/bdc92abe5c499c3fd7988b76a128d05c9120a520/releasenote.md#L172-L193), commit `bdc92abe5c499c3fd7988b76a128d05c9120a520`, lines 172–173 and 192–193 | Primary corroboration. Intel names the `06-97-05` signature the ADL H0 stepping, separately from C0 at `06-97-02`. Lines 192–193 name `06-bf-02` RPL-HX/S C0 and `06-bf-05` RPL-S H0. |
| S5 | Intel, [Affected Processors: Consolidated Product CPU Model](https://www.intel.com/content/www/us/en/developer/topic-technology/software-security-guidance/processors-affected-consolidated-product-cpu-model.html), rows for CPUID `90675` and for `B06F2, B06F5`, consulted 2026-11-28 | Primary product grouping. There are two `06_97H` stepping 5 rows. The Alder Lake S row (12th generation Core, Pentium Gold and Celeron, desktop and embedded) gives `CPUID.0x7.EDX[15] = 1` as its identification. The Catlow server row (Pentium G7400/G7400T) gives `EDX[15] = 0` with `CPUID.0x1A.EAX[31:24] = 40`. Both rows conflict in part with S6. The `06_BFH` row (steppings 2 and 5) gives `CPUID.0x7.EDX[15] = 1`. See Open questions. |
| S6 | InstLatx64, commit `17be9a9793c306cf96f8c286e34e705b780a0226`: [Core i3-12100](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel0090675_AlderLake_00_CPUID.txt) (lines 1–53), [Core i5-12500T](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel0090675_AlderLake_01_CPUID.txt) (lines 1–53), [Pentium Gold G7400](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel0090675_AlderLake_02_CPUID.txt) (lines 6–69, 319–322, 348–353, 460–463, 558–561, 656–659) and [Core i5-12500](https://github.com/InstLatx64/InstLatx64/blob/17be9a9793c306cf96f8c286e34e705b780a0226/GenuineIntel/GenuineIntel0090675_AlderLake_03_CPUID.txt) (lines 6–68, 939–950, 976–981 and the `19CH`/`1A2H` lines of each logical CPU through 2068–2071) | Independent observations. All four captures report `0x00090675`, DTS set, the hybrid flag clear and an all-zero leaf 1AH on every logical processor. The G7400 and i5-12500 captures add per-processor `0x19C`/`0x1A2` words. They do not verify the HardwareVisualizer Windows PawnIO path. |
| S7 | [Intel core DTS](https://github.com/shm11C3/HardwareVisualizer/blob/8a70f4649fa66e46c0a18f3584e88f541e0fc3dd/docs/specs/sensors/cpu-intel-core-dts-msr.md) revision 8 at `8a70f4649fa66e46c0a18f3584e88f541e0fc3dd`: Detection, Read procedure and decode, Project collection constraints and Safety notes | Common Windows topology, affinity, request budget, cache, timeout, fault and partial-result contract. This scope has its own signature admission. |
| S8 | [PawnIO interface](https://github.com/shm11C3/HardwareVisualizer/blob/2495a1b0fbe4a8048feac9d122b98201e7970cbb/docs/specs/sensors/pawnio-interface.md#intelmsr) revision 7, `IntelMSR` and signed release `0.2.11` | Existing synchronous calling-processor read interface with `0x19C` and `0x1A2` on its allow-list. A native fault returns a failing status. |
| S9 | Linux v6.18 commit `7d0a66e4bb9081d75c82ec4957c50034cb0ea449`, [model identity](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/arch/x86/include/asm/intel-family.h#L137-L140), [thermal mask](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/thermal/intel/intel_tcc.c#L38-L43) and [model selection](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/drivers/thermal/intel/intel_tcc.c#L96-L98) | **lead-only (copyleft)**. Linux selects an eight-bit readout mask, which includes bit 23, for display model `06_97` at every stepping. This conflicts with S1, which marks bit 23 reserved. The project gate below follows S1. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows x86-64, vendor `GenuineIntel`, maximum basic leaf at least 07H, and exact `CPUID.01H:EAX = 0x00090675` | mechanism | S1, S3, S4, S6. Selects the 6+0 die at display model `06_97H`. |
| `CPUID.01H:EDX[22] = 1` and `CPUID.06H:EAX[0] = 1` | mechanism | S1, S6. These advertise the thermal status MSR and its core DTS fields. Package thermal management (`CPUID.06H:EAX[6]`) is independent and does not gate this path (S7). |
| `CPUID.07H.00H:EDX[15] = 0` | mechanism | S3, S6. A homogeneous 6+0 target. If the flag is set, the target is outside this scope (see Open questions). Leaf 1AH is not consulted. |
| One active Windows processor group and one physical package with validated, non-overlapping core masks covering the package | mechanism | S7. Defines physical-core attribution and identity. |
| Existing signed `IntelMSR` route, and a disposable sampling thread that confirms affinity to the representative, rechecks vendor, signature, the two capability bits and the clear hybrid flag there, and restores affinity | safety | S7, S8. Restricts privileged access and confirms the target before and after reads. |
| Both reads succeed, Reading Valid is set, status bit 23 is clear, target in 50–120 °C inclusive and result within `0 < temperature ≤ target` | safety | S1, S7, S8, and the project gate for the bit 23 conflict with lead-only S9. |

| Scope | Status | Default enablement |
| --- | --- | --- |
| Non-hybrid Alder Lake-S 6+0 die, exact `0x00090675`, hybrid flag clear | S1–S3 establish the display model, the 6+0 signature, the core-scoped status fields, the calibrated target and the offset-independent decode. S4 names the stepping H0. S6 independently observes the signature with DTS set, hybrid clear and leaf 1AH zero on Core i3-12100, Core i5-12500T, Pentium Gold G7400 and Core i5-12500. G7400 and i5-12500 also provide valid `0x19C`/`0x1A2` pairs on every logical CPU. These product names are evidence identities and best-evidenced members. They do not gate the read. Other products with this signature and a clear hybrid flag are Experimental under ADR 0011. The bit 23 rejection rests on S1. S9 is the lead-only (copyleft) source for the conflicting mask, and no scope depends on it. | Experimental under the mechanism and safety gates. No member is Verified until a maintainer-accepted hardware report confirms the HardwareVisualizer path. |
| `0x00090675` with the hybrid flag set | S5 lists this identification for the client row, but no observation at this signature reports it. | Disabled under this specification. |
| Other signatures, including `0x00090672` with the hybrid flag clear, `0x000906A4`, and Raptor Lake `0x000B06F2` and `0x000B06F5` | Outside this mechanism. Approved Alder Lake and Raptor Lake hybrid rows follow [Intel core DTS](cpu-intel-core-dts-msr.md). | Disabled under this specification. |

Product names, brand strings, core counts, SMT and board identity are
evidence identities ([ADR 0031](../../adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md)).
The 6+0 die appears in S2 Table 1 with 6, 4 and 2 enabled P-cores.
S6 records 4-core and 2-core members at the same signature.
Use the actual Windows physical-core records, and do not infer an
expected core count from a product or a capture.

## Register map (facts)

| Address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |
| MSR `0x19C` | `IA32_THERM_STATUS` | 22:16 | Digital readout below the calibrated target for the calling core (Core scope) | Unsigned degrees Celsius | S1 Tables 2-2 and 2-39, S2 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 26:23 | Reserved | — | S1 Table 2-39 |
| MSR `0x19C` | `IA32_THERM_STATUS` | 31 | Reading Valid | Flag, set when the readout is valid | S1 Tables 2-2 and 2-39 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 23:16 | Factory-calibrated TjMAX, the DTS reference | Degrees Celsius | S1 Table 2-20, S2 §4.1.3.1 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 29:24 | TCC activation offset, a thermal-control setting that is not used in the decode | Degrees Celsius | S2 §4.1.3.1 |
| MSR `0x1A2` | `MSR_TEMPERATURE_TARGET` | 6:0 | TCC offset averaging window (Tau), a thermal-control setting that is not used in the decode | — | S2 §4.1.3.1 |

S1 lists `0x1A2` with Unique scope in Table 2-20. Read both registers on
the same confirmed representative, as on the existing path (S7). The
result describes the architectural DTS readout of that representative's
core. It does not identify a physical diode or its location on the die.

## Read procedure and decode

1. Apply the discovery gates before any core topology query. On a
   failed gate, make no topology query and no core MSR read under this
   scope (S7).
2. Use the common non-hybrid procedure of S7. Select one representative
   per validated Windows physical-core record. Pin the dedicated thread
   to it and confirm the running processor. Recheck vendor, signature,
   thermal status and DTS bits, and the clear hybrid flag on that
   processor. Skip the target before MSR access if any check fails.
3. Read `0x1A2` and then `0x19C` through `IntelMSR` on the same calling
   processor (S8). Confirm the running processor again while pinned,
   then restore affinity. Discard the result if the processor changed
   or restoration failed (S7).
4. Require both reads to succeed, a target byte from 50 to 120 °C
   inclusive, status bit 31 set and status bit 23 clear. Subtract the
   unsigned bits 22:16 from the target. Accept only
   `0 < temperature ≤ target`. A valid zero readout returns the target.
   Bits 29:24 and 6:0 of `0x1A2` do not enter the conversion (S1, S2,
   S7).
5. A fault, a rejected field or a failed gate leaves only that core
   unavailable and preserves eligible siblings. Existing package
   temperature and ACPI fallback stay independent. The core path makes
   no `0x1B1` read. Apply S7's demand, rate limit, topology recheck,
   cache, disposable-thread and timeout rules unchanged.

## Quirks

| S6 capture | Logical CPUs and cores | `0x1A2` low 32 bits | `0x19C` low 32 bits observed | Decoded under this spec |
| --- | --- | --- | --- | --- |
| Pentium Gold G7400 | 4 logical processors, 2 cores with SMT (lines 319–322) | `0x80641400` on all four | `0x883D0000`, `0x88360000`, `0x883D0000`, `0x883D0000` (final reads) | 100 °C target and zero offset. Readouts of 61 and 54 give 39 °C and 46 °C. |
| Core i5-12500 | 12 logical processors, 6 cores with SMT (lines 939–950) | `0x80641400` on all twelve | Readouts from `0x35` to `0x41`, with bits 26:23 clear in every word | 100 °C target and zero offset. Results from 35 °C to 47 °C. |

Every captured status word with Reading Valid set has bit 23 clear.
Bits 15:8 of the observed target word (`0x14`) are outside every field
this scope uses. The captures are observations. Every result uses the
target read live on its own representative.

The S3 errata summary lists no S-column exception to the core status
or target reads (ADL001–ADL094). S2 states a DTS accuracy of ±5 °C
across the operating range. The positive-result gate already excludes
the region below 0 °C.

## Safety notes

- Use only the two allow-listed reads on each confirmed representative.
  The fields used are read-only. The status log bits clear only through
  a write, and this scope makes no write (S1, S8).
- Never write `0x1A2`, thresholds, logs or any other MSR. The core path
  needs no platform-ID or board-specific access path (S7, S8).
- Inherit S7's bounded affinity interval, fail-closed restoration,
  topology rechecks and request budget.

## Acceptance fixtures

All cases are synthetic unless they cite S6 register observations.

| Case | Required result |
| --- | --- |
| Exact `0x00090675`, thermal status and DTS set, hybrid clear, one group and package, six SMT cores | One target per physical core. The second logical processor of each core adds no channel. Repeat with package thermal management clear. The core path is still eligible, and no core-path `0x1B1` read is made. |
| Same valid discovery with leaf 1AH reporting zero, `0x40`, or a maximum basic leaf below 1AH | The same admission in all three cases. Leaf 1AH is not consulted. |
| Same valid discovery with two cores and no SMT, or four cores with SMT | One target per actual Windows core record. A product or capture core count is never used. |
| `0x00090675` with the hybrid flag set, DTS clear, thermal status bit clear, wrong vendor, or maximum basic leaf below 07H | No core topology query and no core MSR read under this scope. |
| `0x00090672` with the hybrid flag clear, `0x00090674`, `0x00090676`, `0x000906A4`, `0x000B06F2` or `0x000B06F5` | No core topology query and no core MSR read under this scope. Other approved specifications keep their own gates. |
| Eligible discovery, but one pinned representative reports a different signature, a set hybrid flag or a clear DTS bit | Skip both reads on that target. Preserve eligible siblings and independent package and fallback data. |
| S6 G7400 pairs `0x80641400 / 0x883D0000` and `0x80641400 / 0x88360000`, and an S6 i5-12500 pair `0x80641400 / 0x88350000` | 39 °C, 46 °C and 47 °C. |
| Target word `0x8A641400` (offset 10) with status `0x88320000`, then the same with a nonzero averaging window | 50 °C in both cases. The offset and window do not change the decode. |
| Target 100 °C with status `0x889E0000` on one core and `0x881E0000` on another, both with Reading Valid set | The first core is unavailable and the second returns 70 °C. Repeat with the cores exchanged. |
| Valid zero readout at targets of 50 °C and 120 °C. Targets of 0, 49 °C and 121 °C. Reading Valid clear. A readout equal to or greater than the target. A faulted read | Accept the first two at the target temperature. Every other case leaves that core unavailable and preserves valid siblings. |
| Multiple groups or packages, malformed or changing topology, timeout, or failed affinity restoration | Apply S7's no-result, cache-invalidation and fresh-thread rules. Package and fallback results stay independent. |

## Open questions

- Non-blocking for the #2700 exact `0x00090675` Experimental scope: the path admits only a clear hybrid flag, which every S6 capture reports.
  S5 lists `CPUID.0x7.EDX[15] = 1` for the Alder Lake S client row at
  this signature and `EDX[15] = 0` with leaf 1AH type `0x40` for the
  Catlow G7400/G7400T row. All four S6 captures, including a G7400,
  report the flag clear and leaf 1AH zero. A part that reports the
  flag set stays disabled under this specification. Widening needs an
  observation of a set flag at this signature, together with the
  per-target core type it reports.
- Non-blocking for the #2700 exact `0x00090675` Experimental scope: collection rejects a sample with bit 23 set before applying Intel's seven-bit decode.
  S1 marks bits 26:23 reserved for `06_97H`. Lead-only S9 includes bit 23
  in its `06_97` mask. Every S6 status word has the bit clear. The
  meaning of a set bit on hardware remains unresolved.
- Non-blocking for the #2700 exact `0x00090675` Experimental scope: no HardwareVisualizer Windows PawnIO run exists for this signature.
  S6 supplies raw register words from two products, and their
  acquisition OS is not stated. Graduation of a member to Verified uses
  the [#2500](https://github.com/shm11C3/HardwareVisualizer/issues/2500)
  Hardware Compatibility Report path.
- Non-blocking for the #2700 exact `0x00090675` Experimental scope: other 6+0 configurations are not admitted.
  Products whose E-cores are disabled on the `0x00090672` C0 die, and the
  Raptor Lake `0x000B06F2` C0 and `0x000B06F5` H0 signatures, need their
  own hybrid-flag observations and errata applicability. S5 lists
  `06_BFH` steppings 2 and 5 with the hybrid flag set. S4 names
  `06-bf-05` the RPL-S H0 stepping.

## Deferred-scope disposition (#2700)

| #2700 deferred row | Disposition in this revision |
| --- | --- |
| Other client, Atom and Xeon signatures, unsupported core types, and additional steppings | Alder Lake-S H0 `0x00090675` (hybrid flag clear) is specified here as Experimental. Raptor Lake `0x000B06F2` and `0x000B06F5`, Alder Lake U `0x000906A4`, Arrow Lake H `0x000C0652` and U `0x000B0650`, Panther Lake (`06_CCH`), Emerald Rapids and the Lunar Lake LP E-cores remain open under #2700. |
| Pre-Nehalem Core 2 and earlier Atom | Unchanged. This revision does not touch it. |
| Larger or multi-package topology | Unchanged. This scope requires one group and one package. |

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-28 | Proposes Implementation-ready Experimental coverage for the exact non-hybrid Alder Lake-S H0 `0x00090675` core DTS mechanism with the hybrid flag clear, pending maintainer approval. Pins the 6+0 signature, the calibrated reference and its independence from the TCC offset, and errata applicability. Records four independent CPUID captures and two raw thermal-register captures, and the conflict with Intel's hybrid identification. |
