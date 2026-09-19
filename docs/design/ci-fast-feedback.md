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
Rust cache. A dedicated runner pool requires a separate provider decision.

The existing step-level background and wait controls remain in the main CI
workflow where they provide measured overlap. The repository's pinned
actionlint version does not understand those controls, so its main-workflow
job remains disabled. Before publishing workflow changes, inspect a normalized
sequential projection and structural invariants locally rather than silently
removing the existing overlap. The temporary measurement workflow uses
ordinary GitHub Actions syntax and shell process waiting.

## Measurement

Hosted comparisons must hold the commit, runner image, dependency lockfiles,
and cache configuration equal. Compare the old and new rustfmt steps and the
old frontend check/build jobs with the new frontend lint/check jobs. Record
workflow creation to completion, each check's queue and runner duration, and
the critical path; repeat the comparison enough times to expose runner noise.
Keep test identities, coverage generation, and Merge Gate membership unchanged.

The temporary timing workflow omits the production coverage-comment action so
measurement runs do not write pull-request comments. It still runs the same
lint, test, build, and coverage-generation commands; required CI retains the
coverage report action. The measurement is evidence for this change only. It
does not justify splitting the Core or Tauri test suites, changing cache
ownership, or reducing required checks without a separate decision and review.

Reference: [GitHub concurrency](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency).
