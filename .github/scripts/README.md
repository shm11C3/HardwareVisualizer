# GitHub Actions Scripts

This directory is for scripts whose job is to serve GitHub Actions workflows
and composite actions. Being a script, or being run by CI, is not a reason to
put a file here.

## Where a script belongs

1. A script that checks, generates, or operates on files owned by one area
   lives next to that owner, and CI calls it by that path. For example, the
   MSI table check lives in `src-tauri/windows/wix/`, next to the WiX fragment
   it verifies.
2. A developer or diagnostic tool with no single owning area lives under
   `scripts/<area>/` (for example `scripts/diagnostics/`,
   `scripts/build-cache/`, or `scripts/licenses/`).
3. Agent guidance and hook tooling (the guidance checker, the Claude and Codex
   hooks, and their tests) lives in `.agents/tooling/`; `agent-guidance.yml`
   runs it from there.
4. Only GitHub Actions plumbing lives here, together with its tests: change
   detection, the merge gate, CI telemetry, cache reporting, release
   publishing and signing steps, dependency automation, and PR comments.

Every file in this directory must be listed in the index below with its
consumers. `npm run check:agent-guidance` fails when the directory and the
index disagree, and the agent pre-edit hook refuses to create an unlisted file
here. Add the index row only after the placement rule above says this
directory is the owner.

## Subdirectories

Each script lives in the subdirectory of the workflow concern it serves, next
to its test.

- `ci/`: the merge gate, E2E capture comments, and the E2E CI policy test, for
  `ci.yml`.
- `deps/`: Tauri dependency change detection and automated updates, for
  `ci.yml` and `auto-update-tauri.yml`.
- `cache/`: Actions cache inventory, for `ci.yml` and `cache-health.yml`.
- `ci-telemetry/`: CI telemetry aggregation, its shared schema, and its test,
  for `ci-telemetry.yml` and the `ci-telemetry` action.
- `release/`: release checksums, Tauri config and signing, WinGet license
  correction, and release vulnerability evaluation, for `publish.yml`,
  `winget.yml`, `osv-scan.yml`, and `ci.yml`.

Tests for composite actions live next to the action
(`.github/actions/cache-sccache/report/test-metrics.cjs` and
`.github/actions/ci-telemetry/test-metrics.mts`).

## Index

| Script | Consumers |
| --- | --- |
| `ci/merge-gate.ts` | `ci.yml` merge gate |
| `ci/comment-e2e-captures.sh` | `ci.yml` |
| `ci/test-e2e-ci-policy.mjs` | `ci.yml` change detection |
| `deps/tauri-updater.sh` | `auto-update-tauri.yml` |
| `deps/check-tauri-deps-changed.ts` | `ci.yml` change detection, `auto-update-tauri.yml` |
| `deps/test-check-tauri-deps-changed.mjs` | `ci.yml` |
| `cache/cache-inventory.cjs` | `ci.yml`, `cache-health.yml` |
| `cache/test-cache-inventory.cjs` | `ci.yml` |
| `ci-telemetry/aggregate.mts` | `ci-telemetry.yml` |
| `ci-telemetry/cli.mts` | CI telemetry scripts |
| `ci-telemetry/markers.mts` | CI telemetry scripts |
| `ci-telemetry/record.mts` | CI telemetry scripts |
| `ci-telemetry/render.mts` | CI telemetry scripts |
| `ci-telemetry/schema.mts` | CI telemetry scripts |
| `ci-telemetry/test-ci-telemetry-aggregate.mts` | `ci.yml`, `ci-telemetry.yml` |
| `release/generate-release-checksums.sh` | `publish.yml` |
| `release/update-tauri-config.ts` | `publish.yml` |
| `release/sign-codesigntool.ps1` | `publish.yml` (Tauri `signCommand`) |
| `release/test-sign-codesigntool.ps1` | `ci.yml` Windows installer validation |
| `release/update-winget-license.mjs` | `winget.yml` |
| `release/test-update-winget-license.mjs` | `ci.yml` |
| `release/evaluate-release-vulnerabilities.mjs` | `ci.yml`, `osv-scan.yml` |
| `release/test-evaluate-release-vulnerabilities.mjs` | `ci.yml` |
