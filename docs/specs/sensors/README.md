# Sensor Hardware Specifications (Clean-Room)

This directory is the specification library for native CPU and Super I/O
sensor monitoring via [PawnIO](https://github.com/namazso/PawnIO)
(issue #1635). Every document here is a **fact-only hardware/interface
specification** produced by the "spec author" role of the clean-room
process described below.

These documents are the **only** external technical input the
implementation role is allowed to use. Keeping them factual, sourced,
and free of third-party code is what lets the resulting Rust code stay
MIT-licensed.

## Clean-room process (two roles)

| Role | May read | Must not do |
| --- | --- | --- |
| Spec author ("dirty room") | Vendor datasheets and manuals (primary); hardware dumps; MPL/GPL/LGPL implementations **only to extract facts** (register addresses, bit layouts, procedures, quirks) | Copy code excerpts, code structure, or implementation identifier names into spec documents |
| Implementer ("clean room") | `docs/specs/sensors/**` and this repository only | Read LibreHardwareMonitor / OpenHardwareMonitor / Linux kernel / lm-sensors sources, or any decompiled monitoring tool |

Names that are part of a public API contract (for example PawnIO module
function names such as `ioctl_read_msr`) are interface facts required
for interoperability, not implementation identifiers; they may appear
in spec documents.

## Hard rules for documents in this directory

- State **facts**, with a source note (provenance) for each fact or
  fact group: document title, document/order number, and section or
  page where known. Use `TODO(provenance)` when a page-level citation
  still needs to be pinned.
- No code excerpts, no code structure, and no identifier names taken
  from copyrighted implementations.
- Facts extracted from MPL/GPL/LGPL sources are allowed but must be
  recorded as factual statements with the source named.
- Anything uncertain goes in the document's **Open questions** section,
  not in the fact tables.
- Read-only orientation: documents describe register *reads*. Writes
  are documented only where a read transaction requires them (for
  example configuration-mode entry keys or bank selection), and must be
  marked as such.

## Document conventions

- One document per access domain or chip family, lowercase kebab-case
  filenames (see [`docs/documentation-guide.md`](../../documentation-guide.md)).
- Start from [`spec-template.md`](spec-template.md).
- Each document carries a **revision number** and a revision history
  table. Any change to facts increments the revision.
- Implementation PRs must pin the spec they were built from in the PR
  body, e.g.:

  ```text
  Implemented from docs/specs/sensors/cpu-amd-zen-smn.md revision 1
  (commit <sha>). No other external sensor documentation was used.
  ```

  This is the audit trail demonstrating clean-room provenance.

## Current documents

| Document | Covers | Issue phase |
| --- | --- | --- |
| [`pawnio-interface.md`](pawnio-interface.md) | PawnIO driver/library API, module IOCTL contracts, mutex conventions, licensing facts | Phase 1 |
| [`cpu-intel-dts-msr.md`](cpu-intel-dts-msr.md) | Intel digital thermal sensor via MSRs (package/core temperature) | Phase 1 |
| [`cpu-amd-zen-smn.md`](cpu-amd-zen-smn.md) | AMD Zen Tctl/Tdie via SMN thermal controller | Phase 1 |
| [`superio-access.md`](superio-access.md) | Generic Super I/O configuration access, chip detection, ISA mutex | Phases 2–4 (mechanism) |

Per-chip Super I/O register maps (Nuvoton NCT67xx, ITE IT86xx/87xx) are
**not yet written**; they are the Phase 3 / Phase 4 deliverables and
will be added as separate documents validated against user-submitted
register dumps.

## Safety policy (applies to all documents and implementations)

- Read-only register access. No writes that alter chip configuration,
  fan control, limits, or power state in any phase of #1635.
- Honor the ecosystem mutex conventions
  (`Global\Access_ISABUS.HTP.Method`, `Global\Access_PCI`) so that
  concurrent monitors (HWiNFO, LibreHardwareMonitor, FanControl) do not
  corrupt each other's multi-step read transactions. Details in
  [`pawnio-interface.md`](pawnio-interface.md).
- When PawnIO is not installed, the application degrades gracefully to
  the ACPI thermal-zone path introduced by PR #1633.
