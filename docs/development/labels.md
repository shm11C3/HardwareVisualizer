# GitHub Label Guide

HardwareVisualizer uses labels to answer different questions on issues and pull
requests. Keep these label groups separate so issue triage, pull request
automation, and release notes do not overload the same labels.

## Label Groups

### Issue Type

Use exactly one `type:` label on open issues whenever possible. These labels
describe the kind of work being requested, not the files that may change.

- `type:bug` - user-visible malfunction or incorrect behavior.
- `type:feature` - new user-facing capability or meaningful enhancement.
- `type:performance` - runtime, build, bundle, memory, or responsiveness work.
- `type:investigation` - research or diagnosis before the implementation shape
  is known.
- `type:security` - public security hardening or vulnerability-adjacent work.
  Do not use public issues for private vulnerability reports.
- `type:task` - maintenance work that is not naturally a bug, feature,
  performance, docs, or CI item.
- `type:docs` - documentation-only issue.

Do not use `change:` labels on issues. `change:` describes a pull request.

### Pull Request Change

Use one or more `change:` labels on pull requests. These labels drive release
note categories and are assigned mostly by `.github/labeler.yml`, Dependabot,
or repository automation.

- `change:feature` - feature branch work (`feat/` or `feature/`).
- `change:fix` - bug-fix branch work (`fix/`).
- `change:docs` - documentation changes.
- `change:refactor` - refactoring changes.
- `change:ci` - CI, GitHub Actions, or repository automation changes.
- `change:deps` - dependency or lockfile updates.
- `change:config` - build, TypeScript, Vite, Vitest, Cargo, or Tauri config.
- `change:test` - test harness, fixtures, or coverage changes.

Release notes read `change:` labels only. Do not add unprefixed project labels
such as `feature`, `bug`, `docs`, or `dependencies` as release-note fallbacks.

Dependabot is the special case to watch. By default, Dependabot can create and
apply `dependencies` plus ecosystem labels such as `javascript`, `rust`, or
`github_actions`. Each `updates` entry in `.github/dependabot.yml` must define
explicit `labels:` so Dependabot uses the project labels instead of those
defaults. Create the referenced labels before merging label automation changes;
Dependabot ignores custom labels that do not already exist.

### Area

Use `area:` labels for the code or repository boundary affected by a pull
request. They can also be useful on implementation-ready issues, but avoid
guessing before the likely owner is clear.

- `area:frontend` - React, TypeScript, UI components, browser tests, or Vite.
- `area:backend` - backend behavior that spans App and Core.
- `area:core` - `core/**` or Tauri-independent domain/runtime code.
- `area:tauri` - `src-tauri/**`, Tauri commands, app wiring, or bundling.
- `area:github-actions` - workflows and composite actions under `.github/**`.
- `area:javascript` - JavaScript/TypeScript dependency or tooling work.
- `area:docs` - documentation files or docs automation.
- `area:config` - project configuration files.

Rust work is labeled by crate boundary, not by language alone. Use
`area:core` for the Tauri-independent Core crate and `area:tauri` for the
Tauri/App crate. Use both labels for root Cargo, toolchain, or workspace changes
that can affect both crates.

### Screen

Use `screen:` labels when an issue is clearly tied to one visible app surface.
These labels answer "where does the user see this?"

- `screen:dashboard`
- `screen:insight`
- `screen:process`
- `screen:snapshot`
- `screen:settings`
- `screen:tray`
- `screen:updater`

If the work crosses screens or is backend-only, skip `screen:` unless one screen
is the primary user-facing entry point.

### Feature

Use `feature:` labels for product or platform capability areas. These labels
answer "what capability is affected?"

- `feature:hardware-monitoring`
- `feature:storage-health`
- `feature:gpu-monitoring`
- `feature:process-stats`
- `feature:live-metrics`
- `feature:charts`
- `feature:background-images`
- `feature:tray-widget`
- `feature:external-components`
- `feature:e2e`
- `feature:i18n`
- `feature:release`
- `feature:supply-chain`
- `feature:design-system`

Prefer one or two precise `feature:` labels over broad combinations.

### Status And Source

Use `status:` for current workflow state and `source:` for automated origin.
These labels are optional and should not replace the type/change labels.

- `status:investigating` - actively being investigated.
- `status:regression` - known regression from a previous working state.
- `status:needs-info` - waiting for reporter or maintainer information.
- `status:blocked` - cannot proceed until an external dependency changes.
- `source:automation` - created or maintained by repository automation.
- `source:performance-test` - created by the scheduled performance workflow.

Keep GitHub standard helper labels such as `good first issue`, `help wanted`,
`duplicate`, `invalid`, `question`, and `wontfix` when they describe workflow
state better than project-specific labels.

## Migration Notes

The following legacy labels were removed from GitHub after the replacement
labels were created:

- `bug` -> `type:bug` on issues, `change:fix` on pull requests.
- `enhancement` and `feature` -> `type:feature` on issues,
  `change:feature` on pull requests.
- `performance` -> `type:performance` on issues. Pull requests should use the
  appropriate `change:` label plus a feature or area label.
- `performance-regression` -> `type:performance` + `status:regression` +
  `source:performance-test`.
- `docs` and `documentation` -> `type:docs` on issues, `change:docs` and
  `area:docs` on pull requests.
- `ci` and `github_actions` -> `change:ci` + `area:github-actions`.
- `dependencies` -> `change:deps`.
- `configuration` -> `change:config` + `area:config`.
- `frontend`, `backend`, `javascript`, `tauri`, `hardviz_core`, and
  `hardviz_tauri` -> `area:*`.
- `rust` -> `area:core`, `area:tauri`, or both, depending on the affected crate.
- `hardware` and `monitoring` -> use a precise `feature:*` label.
- `testing` and `automation testing` -> `change:test` or `feature:e2e`.
- `automated` -> `source:automation`.

Do not recreate removed unprefixed project labels. Keep only GitHub standard
helper labels, such as `good first issue`, `help wanted`, `duplicate`,
`invalid`, `question`, and `wontfix`, when they describe workflow state better
than project-specific labels.
