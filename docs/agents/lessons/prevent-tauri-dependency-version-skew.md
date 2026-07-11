---
id: LRN-20260711-prevent-tauri-dependency-version-skew
status: promoted
cause_status: confirmed
scope: src-tauri/Cargo.toml, Cargo.lock, Dependabot, and macOS release builds
trigger: a Tauri-adjacent dependency changes directly or through dependency automation
failure_signature: multiple window-vibrancy versions produced duplicate Objective-C symbols only in the macOS release LTO build
root_cause: the direct dependency was upgraded independently from Tauri and the lockfile retained both dependency versions
guardrail: current Dependabot exception plus cargo tree inspection; promote to CI if the invariant can be checked without freezing an obsolete version
canonical_refs: src-tauri/Cargo.toml, Cargo.lock, .github/dependabot.yml, and docs/agents/lessons/prevent-tauri-dependency-version-skew.md
verification: cargo tree -p hardware_visualizer -i window-vibrancy --locked --offline resolves one compatible version; validate macOS release LTO before changing the exception
evidence: src-tauri/Cargo.toml, Cargo.lock, .github/dependabot.yml, cargo tree -p hardware_visualizer -i window-vibrancy --locked, and the macOS release build
revalidate_when: Tauri changes its window-vibrancy dependency or the project intentionally validates a compatible upgrade
---

# Prevent Tauri Dependency Version Skew

The current direct `window-vibrancy` pin is a compatibility exception, not a
timeless design rule. Before changing it, inspect the full Cargo graph, confirm
that Tauri and the direct dependency resolve compatibly, remove obsolete
lockfile entries, and run the macOS release-profile validation that exposed the
original failure.

Do not promote a particular version number into `AGENTS.md`. The manifest,
lockfile, Dependabot exception, and this revalidation trigger own the temporary
fact.
