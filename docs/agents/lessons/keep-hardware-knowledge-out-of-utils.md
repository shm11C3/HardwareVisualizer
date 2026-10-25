---
id: LRN-20261025-keep-hardware-knowledge-out-of-utils
status: promoted
cause_status: confirmed
scope: core/src/utils/** and new pure helpers for sensor, vendor, or protocol facts
trigger: adding a decoder, register table, chip profile, or protocol helper and choosing between core/src/utils and a provider module
failure_signature: Super I/O helpers and later a 15-row Nuvoton chip profile table accumulated in core/src/utils/super_io.rs because they were pure functions, although they encode vendor register facts
root_cause: pure and testable was treated as the placement criterion, so core/src/utils became the default home for any side-effect-free code instead of staying limited to domain-neutral helpers
guardrail: core/AGENTS.md states that core/src/utils holds only domain-neutral helpers and that hardware, vendor, and protocol knowledge lives with its provider; the Super I/O code moves to core/src/infrastructure/providers/windows/super_io/
canonical_refs: core/AGENTS.md, core/README.md
verification: core/src/utils contains no file named after a sensor, vendor, chip, or bus; sensor decoders and profile tables live under core/src/infrastructure/providers/**
evidence: "maintainer correction on 2026-10-25 during PR #2463 review; core/README.md already described utils as logger, formatter, IP, and rounding helpers; the CPU decoders already live in core/src/infrastructure/providers/windows/cpu_*_decode.rs"
revalidate_when: Core gains a cross-platform sensor layer that several providers share, or the utils directory is removed
---

# Keep Hardware Knowledge Out Of Utils

A helper belongs to the owner of the facts it encodes, not to the directory
for side-effect-free code. A configuration-port table, a chip-id combination
rule, a byte-temperature decoder, a plausibility range, or a chip profile table
is Super I/O knowledge, so it lives with the Super I/O provider, the same way
the CPU MSR decoders live next to the CPU providers. `core/src/utils/` stays
for domain-neutral helpers: logging macros, formatting, rounding, IP parsing.

The mistake was easy to make because the clean-room process asks for pure,
fixture-tested decoders, and "pure" looked like "utility". A file under
`utils/` named after a sensor or vendor is the signal that the placement is
wrong. Group such code as a module folder under the provider, one file per
specification document, so the clean-room provenance pin and the ownership
boundary coincide.
