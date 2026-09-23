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
   `scripts/<area>/` (for example `scripts/diagnostics/`).
3. Only GitHub Actions plumbing lives here, together with its tests: change
   detection, the merge gate, CI telemetry, cache reporting, release
   publishing and signing steps, dependency automation, and PR comments.

Every file in this directory must be listed in the index below with its
consumers. `npm run check:agent-guidance` fails when the directory and the
index disagree, and the agent pre-edit hook refuses to create an unlisted file
here. Add the index row only after the placement rule above says this
directory is the owner.

## Index

| Script | Consumers |
| --- | --- |
| `cache-inventory.cjs` | `ci.yml`, `cache-health.yml` |
| `check-duckdb-license-version.ts` | `ci.yml` |
| `check-licenses.ts` | `ci.yml`, `publish.yml` |
| `check-tauri-deps-changed.ts` | `ci.yml` change detection, `auto-update-tauri.yml` |
| `ci-telemetry/aggregate.mts` | `ci-telemetry.yml` |
| `ci-telemetry/cli.mts` | CI telemetry scripts |
| `ci-telemetry/markers.mts` | CI telemetry scripts |
| `ci-telemetry/record.mts` | CI telemetry scripts |
| `ci-telemetry/render.mts` | CI telemetry scripts |
| `ci-telemetry/schema.mts` | CI telemetry scripts |
| `comment-e2e-captures.sh` | `ci.yml` |
| `evaluate-release-vulnerabilities.mjs` | `ci.yml`, `osv-scan.yml` |
| `extract-apache-notices.ts` | `ci.yml` |
| `generate-release-checksums.sh` | `publish.yml` |
| `merge-gate.ts` | `ci.yml` merge gate |
| `sign-codesigntool.ps1` | `publish.yml` (Tauri `signCommand`) |
| `tauri-updater.sh` | `auto-update-tauri.yml` |
| `update-tauri-config.ts` | `publish.yml` |
| `test-cache-inventory.cjs` | `ci.yml` |
| `test-cache-metrics.cjs` | `ci.yml` |
| `test-check-tauri-deps-changed.mjs` | `ci.yml` |
| `test-ci-telemetry-action.mts` | `ci.yml` |
| `test-ci-telemetry-aggregate.mts` | `ci.yml`, `ci-telemetry.yml` |
| `test-evaluate-release-vulnerabilities.mjs` | `ci.yml` |
| `generate-licenses.ts` | `publish.yml`, `auto-update-licenses.yml`, npm `gen:licenses:*` (exception, see below) |
| `test-generate-licenses.mjs` | `ci.yml` (exception, see below) |
| `agent-hook.mjs` | Claude and Codex hooks, npm (exception, see below) |
| `check-agent-guidance.mjs` | `agent-guidance.yml`, npm, agent hooks (exception, see below) |
| `guidance-paths.mjs` | agent guidance scripts (exception, see below) |
| `test-agent-guidance.mjs` | `agent-guidance.yml`, npm (exception, see below) |
| `test-agent-hook.mjs` | `agent-guidance.yml`, npm (exception, see below) |
| `prune-build-dirs.mjs` | agent Stop hook, npm (exception, see below) |

## Known exceptions

These files predate the placement rule and have consumers outside GitHub
Actions. Move them to their owner when they next change substantially; do not
add new files to these groups here.

- Agent guidance and hook tooling (`agent-hook.mjs`, `check-agent-guidance.mjs`,
  `guidance-paths.mjs`, `prune-build-dirs.mjs`, `test-agent-guidance.mjs`,
  `test-agent-hook.mjs`):
  consumed by `.claude/settings.json`, `.codex/hooks.json`, and npm scripts,
  and owned by the shared agent guidance under `.agents/`.
- License notice generation (`generate-licenses.ts`,
  `test-generate-licenses.mjs`): also run by developers through npm
  `gen:licenses:*`, and it produces `docs/licenses/**` and the bundled notices.
