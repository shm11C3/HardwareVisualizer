---
id: LRN-20260814-nested-apply-patch-payloads
status: promoted
cause_status: confirmed
scope: agent edit hooks and free-form apply_patch tool payloads
trigger: a valid apply_patch edit is rejected because the pre-tool hook cannot determine any edited path
failure_signature: the generated-file guard reported Cannot determine edited paths from agent hook payload for patches with explicit file headers
root_cause: agent-hook.mjs inspected only direct tool_input fields while the desktop tool runtime can nest a free-form patch string inside wrapper objects
guardrail: recursively inspect hook payload values for explicit apply_patch file headers while retaining repository-bound path normalization and generated-file rejection
canonical_refs: .github/scripts/agent-hook.mjs, .github/scripts/test-agent-hook.mjs
verification: node .github/scripts/test-agent-hook.mjs
evidence: two rejected apply_patch calls with Add File headers followed by a passing nested-payload regression case
revalidate_when: the agent hook payload schema or apply_patch file-header syntax changes
---

# Accept Nested Apply-patch Payloads Without Weakening Path Guards

The edit hook must derive paths from explicit `*** Add File`, `*** Update File`,
`*** Delete File`, and `*** Move to` headers even when the free-form patch is
nested inside a tool-runtime wrapper. Derived paths still pass through the same
repository-bound normalization before the generated bindings guard runs.

Do not infer paths from arbitrary prose. If no explicit path can be recovered,
the pre-tool hook must continue to reject the edit.
