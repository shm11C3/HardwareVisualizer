---
id: LRN-20260815-deliver-pull-requests-through-review
status: promoted
cause_status: confirmed
scope: pull request creation, CI follow-through, and automated review handling
trigger: an agent is asked to create or publish a HardwareVisualizer pull request
failure_signature: work stopped at PR creation, review follow-through expanded a simple workflow into speculative handling, or every correction triggered another broad review
root_cause: discovery and verification reviews were not separated, so new review scope was introduced on every correction instead of converging on approval
guardrail: .agents/skills/deliver-pull-request/SKILL.md owns primary review, strict triage, incremental verification, and an approval gate; .coderabbit.yml keeps push-triggered review incremental
canonical_refs: AGENTS.md, .agents/skills/deliver-pull-request/SKILL.md
verification: confirm the skill performs one broad primary review, uses incremental verification after batched corrections, fixes only in-scope merge-blocking findings, tracks out-of-scope critical findings separately, requires approval, and escalates non-converging review
evidence: "maintainer corrections during HardwareVisualizer PR #1934"
revalidate_when: repository PR completion criteria, automatic review behavior, or sandbox execution policy changes
---

# Deliver Pull Requests Without Growing The Workflow

PR creation is not completion: the PR must pass CI and reach approval. Perform
one broad primary review, batch its accepted corrections, and use later reviews
only to verify those decisions and the correction diff.

Treat feedback as evidence and decline unsupported or out-of-scope suggestions
with a specific reason and owner. Let correction pushes receive CodeRabbit
incremental review, but never restart a full review. If two verification
reviews do not converge on approval, stop automatic correction and escalate the
unresolved decision. Later reviews may still introduce a verified
merge-blocking security, data-integrity, supported-path failure, or contract
finding. Fix it in the PR only when it belongs to the current requirement;
otherwise create a separate Issue, or use the private security reporting path
for a vulnerability. Minor new suggestions do not justify another push. Never
request Codex review manually; Codex decides when review is needed.
