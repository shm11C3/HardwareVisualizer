---
id: LRN-20261109-gate-on-hardware-mechanism-not-observation
status: promoted
cause_status: confirmed
scope: docs/specs/sensors/**, native sensor Detection sections, and their Rust gates under core/src/infrastructure/providers/**
trigger: a sensor specification or provider proposes an exact brand string, baseboard identity, firmware version, part number or bus address as a runtime condition
failure_signature: "the #2510 coverage wave shipped fourteen scopes in three days, almost all gated on the identity of the observation (five Arrow Lake brands, three Vermeer brands, one Gigabyte baseboard tuple, one BIOS version per MSI board, one ODROID-H4 board/firmware/DIMM tuple); the Intel core DTS spec admits Alder Lake H/P by shared signature while the Arrow Lake spec requires a brand match for the same hybrid mechanism"
root_cause: the spec-author role derived each scope from the identity of the dump or report that supplied the evidence instead of from the hardware mechanism the evidence describes, so verification confidence became a runtime allowlist again, the pattern ADR 0011 had already rejected
guardrail: docs/adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md, the gate-kind table in docs/specs/sensors/spec-template.md, and the status-transition checklist item in docs/specs/sensors/README.md
canonical_refs: docs/adr/0031-runtime-sensor-gates-follow-hardware-mechanism.md, docs/specs/sensors/spec-template.md, docs/specs/sensors/README.md, .agents/rules/clean-room-sensors.md
verification: "every row of a Detection gate table is tagged mechanism or safety; grep the Detection section for brand strings, Win32_BaseBoard fields, bios_version, DIMM part numbers or PCI addresses and confirm each appears only in a Status column or fixture"
evidence: "issue #2510; docs/specs/sensors/cpu-intel-arrow-lake-dts-msr.md rev 1 Detection versus cpu-intel-core-dts-msr.md rev 5 Alder Lake H/P row; core/src/infrastructure/providers/windows/pawn_io/cpu/ccd_temperature.rs addresses_for_identity; core/src/infrastructure/providers/windows/pawn_io/super_io/ite.rs is_ga_b75m_d3h_baseboard; core/src/infrastructure/providers/linux/nct6687_hwmon.rs identify_profile; core/src/infrastructure/providers/linux/dimm_temperature.rs collect_from; maintainer direction of 2026-11-09"
revalidate_when: ADR 0011 or ADR 0031 changes, the spec template's Detection section changes, or the six re-scoping revisions named in ADR 0031 are all merged
---

# Gate on the hardware mechanism, not on the observation

## Observation

Each #2510 child specification started from one piece of evidence: an
InstLatx64 CPUID capture, an LKML fan test, a Linux hwmon transcript. The
Detection section then required, at runtime, the exact identity found in that
evidence: the brand string, the baseboard tuple, the BIOS version, the DIMM
part. The Rust gates mirrored those identities. Coverage grew one SKU or one
firmware build per PR, and two specifications gated the same Intel hybrid
DTS mechanism differently within a day.

## Confirmed cause

Evidence identity and hardware mechanism were conflated. A brand string or a
BIOS version tells you where the evidence came from; it does not change the
register map, the decode or the hazard. ADR 0011 had already separated
verification confidence from runtime enablement for family allowlists, but
the rule was not written down for the finer identities that the new
specifications introduced, so the allowlist pattern returned at SKU and
firmware granularity.

## Promotion

ADR 0031 fixes the rule: a runtime gate is a mechanism gate or a safety
gate; evidence identities live in the Status column and in fixtures and
graduate a row's members from Experimental to Verified. The spec template
gains a gate-kind table and the status-transition checklist checks it. The
six existing identity gates are re-scoped one specification revision at a
time, in the order ADR 0031 lists.
