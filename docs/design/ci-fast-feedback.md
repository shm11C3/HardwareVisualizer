# CI fast feedback

## Decision

Return cheap failures earlier and reduce repeated setup without changing the
required test coverage. Keep GitHub Actions as the scheduler. This follows
DP-09 and DP-10: workflow structure is verifiable locally, while speedups
require hosted-runner measurements.

Cancel superseded pull-request runs independently in CI and CodeQL. Give each
push and scheduled run a unique concurrency group so cache-populating builds
are neither cancelled nor displaced by a pending run.

Give frontend lint its own check. Run frontend tests and the frontend build in
one job with one dependency installation, and keep both in the Merge Gate.
Run rustfmt with only the pinned toolchain and formatter; it needs neither a
Cargo dependency cache nor compilation. Preserve the existing path conditions.

## Sharing boundaries

Clippy, normal tests, coverage, the release build, and bindings export have
different Cargo modes, features, or instrumentation. Combining their cache
keys does not provide a reusable build, so keep these jobs separate. CodeQL
starts independently; retain all three language categories and its distinct
Rust cache. Same-repository pull requests whose Rust inputs are unchanged
re-upload the base Rust analysis instead of re-running it. A dedicated runner
pool requires a separate provider decision.

The existing step-level background and wait controls remain in the main CI
workflow where they provide measured overlap. The repository's pinned
actionlint version does not understand those controls, so its main-workflow
job remains disabled. Before publishing workflow changes, inspect a normalized
sequential projection and structural invariants locally rather than silently
removing the existing overlap. The temporary measurement workflow uses
ordinary GitHub Actions syntax and shell process waiting.

## Native E2E compilation

Keep the real Tauri first-run and IPC smoke on frontend changes. Cache compiled
app dependencies under a dedicated debug-build key and use the existing trusted
R2 compiler cache for Rust and bundled DuckDB C++. The old driver-only cache did
not retain app targets, even when its key was an exact hit.

A frontend change still rebuilds the application because its binary embeds the
current frontend. Reusing a whole binary would exercise stale UI. Limit bindings
regeneration to its Rust producers and the generated file instead of every
frontend consumer; keep push, automation, and unclassified-input checks.

## Measurement

Hosted comparisons must hold the commit, runner image, dependency lockfiles,
and cache configuration equal. Compare the old and new rustfmt steps and the
old frontend check/build jobs with the new frontend lint/check jobs. Record
workflow creation to completion, each check's queue and runner duration, and
the critical path; repeat the comparison enough times to expose runner noise.
Keep test identities, coverage generation, and Merge Gate membership unchanged.

For the cancellation probe, start a PR run and confirm CI and CodeQL are
in progress before pushing one small documentation-only update. Record both
run IDs: superseded PR runs should end as cancelled, the newest PR runs should
complete, and a develop push run should keep its distinct run-id group and
remain unaffected.

The temporary timing workflow omits the production coverage-comment action so
measurement runs do not write pull-request comments. It still runs the same
lint, test, build, and coverage-generation commands; required CI retains the
coverage report action. The measurement is evidence for this change only. It
does not justify splitting the Core or Tauri test suites, changing cache
ownership, or reducing required checks without a separate decision and review.

Reference: [GitHub concurrency](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency).

For Native E2E, target a 30% reduction in hosted job duration, including setup
and post steps. Record the target/compiler cache state, compilation, app launch
and smoke durations separately. Compare cold and warm caches separately; a
new target-cache key starts saving on develop after publication.

On 2026-10-11, four recent Native E2E jobs took 646–821s. In
[run 38100815964](https://github.com/shm11C3/HardwareVisualizer/actions/runs/38100815964),
the native app compilation alone took 8m 34s, versus about 41s from WebDriver
session start to the successful capture. During compilation the four-thread
runner was near full CPU utilization. The bottleneck is uncached compilation;
a driver installation cache does not address it. These are baseline timings;
the updated hosted job's improvement remains to be measured.
