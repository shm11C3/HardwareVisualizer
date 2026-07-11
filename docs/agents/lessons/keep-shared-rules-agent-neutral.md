---
id: LRN-20260711-keep-shared-rules-agent-neutral
status: promoted
cause_status: confirmed
scope: AI guidance layout, shared rules, and GitHub integrations
trigger: adding or relocating reusable agent instructions
failure_signature: agent-neutral rules were stored in a GitHub Copilot-specific directory even though Copilot was not an active consumer
root_cause: tool discovery conventions were treated as the primary ownership boundary instead of the agents that actually use the guidance
guardrail: .agents/rules/ owns shared path-scoped rules; .github/ contains GitHub-specific configuration and optional adapters only
canonical_refs: AGENTS.md, .agents/rules/README.md, docs/documentation-guide.md, .github/workflows/agent-guidance.yml
verification: npm run check:agent-guidance verifies required shared rules, references, hooks, and the path-filtered workflow
evidence: maintainer direction that GitHub Copilot is rarely used and current repository guidance layout
revalidate_when: an actively used GitHub integration requires a documented rule adapter or the repository adopts another shared agent-rule standard
---

# Keep Shared Rules Agent-neutral

Shared constraints belong in `.agents/rules/`, where Codex, Claude, and future
agents can reference the same source. The `scope` field helps an agent select a
rule; it does not claim automatic tool enforcement.

Keep `.github/` for GitHub Actions, PR and issue templates, CodeRabbit, and an
explicitly needed integration adapter. Do not recreate `.github/instructions/`
only for visual symmetry or a speculative future Copilot setup.
