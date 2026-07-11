# Frontend Instructions

These instructions add to the repository root `AGENTS.md` for work under
`src/`.

## Boundaries

- Call the backend through generated commands from `@/rspc/bindings` and handle
  the generated Result shape. Never edit `src/rspc/bindings.ts` manually.
- Use Jotai for shared frontend state and keep state close to its feature.
- Application Preferences go through typed Rust settings commands and
  `settings.json`. Use Tauri Store only for resettable UI-local/transient state.
- `showGpuUsageSource` is a known legacy exception that still uses Tauri Store.
  Do not copy it as a persistence pattern or migrate it opportunistically; see
  `docs/agents/lessons/legacy-gpu-source-display-preference.md`.
- Backend `error_event` is handled by `useErrorModalListener` in
  `src/hooks/useTauriEventListener.ts`; preserve the existing error boundary
  unless the product flow needs a more specific user action.
- Add user-visible strings to the language files under `src/lang/` and use
  `useTranslation()`.

Read [`src/README.md`](README.md),
[`docs/design-principles.md`](../docs/design-principles.md), and `CONTEXT.md`
before introducing a new product term, persisted setting, or monitoring view.
Also read [`frontend.md`](../.agents/rules/frontend.md) and, for persistence,
[`settings.md`](../.agents/rules/settings.md).

## UX And Performance

- Preserve partial availability in the UI. Do not label missing data as zero,
  healthy, disconnected, or failed without evidence.
- Keep live, archived, daily-record, selected, and focus concepts distinct.
- Do not let high-frequency updates rerender unrelated subtrees. Add a focused
  render/behavior regression test when changing live event fan-out.
- For visual or interaction changes, inspect the rendered result at relevant
  desktop and compact viewports. A passing unit test or screenshot capture alone
  is not visual approval.
- Keep controls readable and stable across supported backgrounds and window
  states. Verify overlays and first-run dialogs before weakening E2E selectors.

## Validation

Run the smallest relevant test first, then broaden as needed:

```bash
npx vitest run path/to/focused.test.tsx
npm run lint:ci
npm test
npm run build
```
