---
id: LRN-20261110-prefer-types-and-tables-over-strings-and-branches
status: promoted
cause_status: confirmed
scope: Rust and TypeScript code that classifies values, reports failures, or selects behavior per hardware model or profile
trigger: representing an error, state, or kind as a String / string or bare number, matching on message text, or adding another if / match arm that selects a model, chip, or profile
failure_signature: PawnIO failures were String values, and retry, unsupported, and PawnIO guidance decisions recovered their kind with starts_with / contains / == on the text; ITE and CPU admission accumulated per-model if / == branches while Nuvoton already used a profile table
root_cause: each new failure path or model was added in its local form (a formatted reason or one more branch) without first asking whether a type or a table row could carry the meaning, so the classification and the per-model selection were spread across helpers that depended on exact wording
guardrail: the root AGENTS.md Change Justification And Simplicity section requires checking for a precise type before using a string or bare number, forbids deciding behavior by matching message text, and asks whether accumulated branches are data that a table can express, while keeping protocol steps and safety gates as code
canonical_refs: AGENTS.md
verification: no code in core/src/infrastructure/providers/windows/pawn_io/** or core/src/platform/windows/sensors.rs (tests excluded) classifies a failure with starts_with, contains, or == on its message
evidence: "maintainer direction on 2026-11-09 and 2026-11-10; issue #2639 and PRs #2640, #2641, #2643 replaced string-classified PawnIO errors with PawnIoError, MutexError, and MotherboardSensorError; core/src/infrastructure/providers/windows/pawn_io/super_io/nuvoton.rs shows the table-driven profile form"
revalidate_when: a lint or test enforces the string-classification ban, or the AGENTS.md simplicity section is restructured
---

# Prefer Types And Tables Over Strings And Branches

A string that later code inspects to find out what happened is a type that was
not written down. The PawnIO providers formatted every failure as text. Then
`is_retryable_init_error` rebuilt the prefix of every Experimental chip to test
`starts_with`, and `sensors.rs` compared against exported prefix constants to
decide whether to offer guidance. Changing a word in a message could silently
change retry or guidance behavior. A typed error made each of those decisions a
`match`, and `Display` kept the surfaced text unchanged.

The same habit shows up in branches. When every newly supported model adds
another `if signature == ...` or `profile == ...`, the branches are really rows
of data. Nuvoton profiles are rows in a table, so adding a chip there means
adding a row. ITE profiles are an enum with predicates compared throughout
discovery and sampling.

Tables are for selection data, not procedures. Configuration enter and exit,
activation and base checks, mutex ownership, and double reads are ordered steps
from a specification. Expressing them as data would only move the branches into
an interpreter that neither the compiler nor a spec reviewer can check.
