---
id: LRN-20260923-place-scripts-with-their-owner
status: promoted
cause_status: confirmed
scope: new scripts anywhere in the repository, especially .github/scripts
trigger: adding a script, check, or tool that CI, an npm script, or an agent hook will run
failure_signature: new scripts repeatedly landed in .github/scripts because they were scripts or ran in CI, although they checked or served files owned elsewhere
root_cause: .github/scripts was treated as the default home for any script instead of as the owner of GitHub Actions plumbing only
guardrail: .github/scripts/README.md states the placement rule and indexes every file with its consumers; the guidance checker fails when the directory and index disagree; the agent pre-edit hook refuses to create an unlisted file there
canonical_refs: AGENTS.md, .github/scripts/README.md, .github/scripts/check-agent-guidance.mjs, .github/scripts/agent-hook.mjs
verification: npm run test:agent-guidance and npm run test:agent-hooks cover an unlisted script, a missing indexed script, and creating an unlisted file; npm run check:agent-guidance validates the live index
evidence: "maintainer correction on PR #2211, where the MSI table check moved to src-tauri/windows/wix/; the agent tooling in .github/scripts, including prune-build-dirs.mjs added in #2213; the known exceptions listed in .github/scripts/README.md"
revalidate_when: the agent tooling or license generation exceptions move to their owners, or GitHub Actions stops being the only CI surface
---

# Place Scripts With Their Owner

A script belongs to the owner of what it checks, generates, or operates on, not
to the tool that runs it. An installer table check lives next to the WiX
fragment, and CI calls it by that path. A developer tool with no single owner
goes under `scripts/<area>/`. `.github/scripts/` is only for GitHub Actions
plumbing such as change detection, the merge gate, CI telemetry, and release
steps.

The mistake repeated because every script run in CI looked like a GitHub
script. The index turns placement into an explicit, reviewable statement: a
new file there needs a row naming its GitHub Actions consumer, and the hook
stops an agent before it creates an unlisted file.
