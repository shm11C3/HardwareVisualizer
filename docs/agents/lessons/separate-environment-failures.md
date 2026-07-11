---
id: LRN-20260711-separate-environment-failures
status: promoted
cause_status: confirmed
scope: local validation, sandboxed agents, Rust builds, and Biome checks
trigger: a validation command fails before exercising the changed product behavior
failure_signature: blocked HOME writes, exhausted build storage, or tool IO noise was at risk of being reported as a product regression
root_cause: environment and tooling preconditions were not separated from the code path under test
guardrail: AGENTS.md requires exact error capture and environment/product classification; Biome excludes agent-local configuration directories
canonical_refs: AGENTS.md, and biome.jsonc
verification: capture command exit status and causal stderr, then rerun the focused check after correcting only the environment precondition
evidence: command exit status, exact stderr, writable HOME retry, target directory size, and a focused rerun of the affected test
revalidate_when: sandbox policy, Rust toolchain storage, Biome configuration, or agent config locations change
---

# Separate Environment Failures From Product Regressions

Record the exact command, exit status, and first causal error. If the failure is
a blocked settings write, use a writable temporary `HOME` while keeping the
configured Rust toolchain and Cargo homes. If the error is `No space left on
device`, inspect build artifacts before changing product code.

Do not ignore a failing command because a similar historical warning was noise.
First prove whether the current command reached the code path under test.
