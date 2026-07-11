---
id: LRN-20260711-guidance-follows-canonical-architecture
status: promoted
cause_status: confirmed
scope: AGENTS.md, AI rules, skills, architecture guidance, and repository maps
trigger: an AI-facing file describes ownership, paths, commands, or supported branch conventions
failure_signature: AGENTS.md and Copilot guidance continued to describe the pre-Core/App-split layout and a skill proposed branch prefixes rejected by CI
root_cause: volatile architecture and policy details were duplicated across several always-on instruction files without drift validation
guardrail: AGENTS.md is a short router; canonical architecture lives in docs/architecture/backend.md; check-agent-guidance validates known drift points
canonical_refs: AGENTS.md, docs/architecture/backend.md, .github/scripts/check-agent-guidance.mjs
verification: npm run check:agent-guidance and compare AI ownership statements with owner README files and enforcement workflows
evidence: docs/adr/0002-core-app-split.md, docs/architecture/backend.md, CONTRIBUTING.md, and .github/workflows/pr-branch-name.yml
revalidate_when: crate ownership, repository layout, branch policy, or agent instruction formats change
---

# Guidance Follows Canonical Architecture

Always-on AI files should contain durable constraints and links, not large
copies of changing file trees or code examples. When architecture changes, the
architecture document and owner README files change first; AI routers then
point to those sources.

Any rule that claims to enforce repository policy must be checked against the
actual CI or configuration that enforces it.
