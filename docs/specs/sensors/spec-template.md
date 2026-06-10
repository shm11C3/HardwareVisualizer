# Spec: <Domain / Chip family — short title>

<!--
Copy this file to a new lowercase kebab-case name and fill every
section. Delete the HTML comments. Keep facts and sources together.
Rules: docs/specs/sensors/README.md
-->

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Draft |
| Scope | <what this document specifies, and what it deliberately excludes> |
| Issue phase | <phase from #1635> |

## Sources

<!--
Primary sources first (vendor datasheets / manuals). For facts taken
from MPL/GPL/LGPL implementations, name the project and state that only
facts were extracted. Pin page/section where possible; otherwise add
TODO(provenance).
-->

| ID | Source | Notes |
| --- | --- | --- |
| S1 | <Vendor>, *<Document title>*, document/order no. <N>, rev. <R>, §<section>/p.<page> | Primary |
| S2 | … | … |

## Detection

<!--
How an implementation decides this document applies: CPUID checks,
chip ID registers, presence probes. Each fact tagged with a source ID.
-->

## Register map (facts)

<!--
Tables only. Address, name (vendor mnemonic), bit fields, units,
access width, source tag. No prose interpretation here.
-->

| Address | Name (vendor mnemonic) | Bits | Meaning | Units / encoding | Source |
| --- | --- | --- | --- | --- | --- |

## Read procedure and decode

<!--
Ordered factual steps (prose / arithmetic, no code), including required
ordering, validity checks, and the exact decode formula with units.
-->

## Quirks

<!--
Per-model deviations, errata, offsets. Each entry: factual statement +
source note. Quirks learned from GPL/MPL sources are facts with the
project named.
-->

## Safety notes

<!--
Which operations write anything (e.g. bank select), mutex requirements,
and what must never be written.
-->

## Open questions

<!--
Anything not yet verified against a primary source, with what evidence
exists so far. Implementers must treat these as unresolved.
-->

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | <YYYY-MM-DD> | Initial version |
