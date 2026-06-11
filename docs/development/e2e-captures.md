# E2E capture harness (Playwright web/mock)

Automated E2E scenario runs that save PNG evidence captures of the UI.
This is the web-only harness from issue #1609: it runs the React frontend in
a plain Chromium browser with **mocked Tauri IPC and events**, so no Rust
backend, live hardware data, or Tauri runtime is required.

## Scope

- In scope: deterministic scenario runs + PNG capture artifacts.
- Out of scope: **visual snapshot regression / baseline comparison** is
  intentionally not part of this harness. Captures are evidence for human
  review, not compared against golden images.
- The native Tauri smoke path (tauri-driver/WebDriver) is tracked separately
  in issue #1610. Note that tauri-driver only supports Linux and Windows;
  macOS has no WKWebView driver.

## Running locally

```bash
npm run test:e2e
```

Playwright starts its own Vite dev server on port `1521` with
`VITE_E2E_MOCK=true` (a regular `npm run dev` server on `1520` is never
reused). Captures are written to:

```text
test-results/captures/<scenario>.png
```

The first page load pays Vite's cold module-transform cost
(babel + react-compiler) and can take >10 seconds; specs use an extended
timeout for the initial render assertion.

## How the mocks work

- `src/main.tsx` installs the mocks before importing the app, only when
  `VITE_E2E_MOCK=true`. The branch is statically false in production builds,
  so the mock code is dead-code eliminated from release bundles.
- `src/e2e/mocks/installTauriMocks.ts` is the single mock entry point:
  - `mockIPC(..., { shouldMockEvents: true })` from `@tauri-apps/api/mocks`
    intercepts every `invoke()` (generated tauri-specta commands and
    `plugin:<name>|<command>` plugin calls) and implements
    `plugin:event|listen/emit/unlisten`.
  - `mockWindows("main")` fakes the current window label.
  - `window.__TAURI_OS_PLUGIN_INTERNALS__` is set directly because
    `platform()` from `@tauri-apps/plugin-os` is a synchronous global read,
    not an IPC call.
  - The Tauri store plugin is backed by an in-memory `Map`, seeded from
    `src/e2e/fixtures/store.ts`.
  - Unhandled commands throw `[e2e-mock] Unhandled invoke: <cmd>` so coverage
    gaps surface immediately when new screens are added to scenarios.
- Fixture data lives in `src/e2e/fixtures/` (settings, hardware info,
  process list, storage health, and a deterministic
  `hardware-monitor-update` series built from fixed sine waves).
- Tests push hardware events through `window.__E2E__.emitHardwareUpdate` /
  `emitHardwareUpdateSeries`, exposed by the mock installer.

## Covered scenarios

| Spec | Captures |
|------|----------|
| `e2e/dashboard.spec.ts` | `dashboard`, `dashboard-gpu-secondary` (GPU tab switch) |
| `e2e/usage.spec.ts` | `usage` (mixed CPU/RAM/GPU chart) |
| `e2e/cpu-detail.spec.ts` | `cpu-detail` (info table + per-core charts) |
| `e2e/insights.spec.ts` | `insights-main`, `insights-process` |
| `e2e/settings.spec.ts` | `settings` (General/About sections) |

Insights pins the clock with `page.clock.setFixedTime(...)` because its
archive query ranges derive from `Date.now()`; the mocked archive commands
synthesize records from the requested start/end range
(`src/e2e/fixtures/archive.ts`), so charts stay deterministic.

Pass criteria are DOM-level: fixture content visible via accessible
selectors, interactions reflected in ARIA state, and captures saved.
Pixels are not compared (no baselines). One targeted style guard exists:
the usage spec compares the **computed** stroke of each chart series
against the fixture colors, catching invalid color plumbing that would
render series black while remaining "visible" to all other assertions.

## Writing a scenario

Specs live in `e2e/*.spec.ts` (outside `src/`, so Vitest does not pick them
up). Shared helpers live in `e2e/helpers.ts`. The shape of a scenario:

1. `await gotoApp(page)` — loads `/` and waits for fixture content
   (first load can take >10s, see above).
2. Seed chart history: `await seedHardwareHistory(page)`.
3. Navigate with `await navigateTo(page, "<screen>")` (clicks the side
   menu's accessible `open <type>` buttons) and interact via accessible
   selectors (`getByRole("tab", ...)`, aria-labels, headings).
4. Save the capture: `await saveCapture(page, "<name>")` — writes a
   full-page PNG (the whole scrollable page, not just the viewport) into
   `test-results/captures/`.

Determinism rules:

- Locked viewport (1280x800), `deviceScaleFactor: 1`, dark color scheme,
  `en-US` locale, UTC timezone, reduced motion (see `playwright.config.ts`).
- Use fixture data only — never live hardware values.

## Render performance smoke

Lightweight render performance checks reuse the same web/mock harness, but run
as a separate suite:

```bash
npm run test:perf:render
```

The suite writes a JSON report and Playwright artifacts under:

```text
test-results/render-perf/
```

These checks target coarse, CI-stable signals such as DOM element count,
Long Task count, and generous interaction timing. They intentionally avoid
strict FPS, CPU, and memory gates. Thresholds are moderate rather than
extremely loose because this suite is meant to catch obvious render regressions
even while it starts as an observation signal. In CI the `test-render-perf` job
runs only for frontend pull requests, uploads artifacts, and stays outside the
merge gate.

## CI

The `test-e2e-web` job in `.github/workflows/ci.yml` runs the suite on
`ubuntu-latest` and uploads `test-results/captures/` as the `e2e-captures`
artifact with `if: always()`, so captures survive failed runs. The Playwright
HTML report and per-test output are uploaded only on failure.

For same-repo pull requests the job also posts the captures inline as a
sticky PR comment (`.github/scripts/comment-e2e-captures.sh`): images are
force-pushed to the single-commit `e2e-captures` branch under
`pr-<number>/` and embedded via raw.githubusercontent.com URLs. The branch
is disposable — it can be deleted at any time and will be recreated by the
next run. Fork PRs are skipped (their token is read-only).
