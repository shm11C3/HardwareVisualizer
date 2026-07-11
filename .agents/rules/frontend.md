---
scope: "src/**/*.ts,src/**/*.tsx,src/**/*.css,src/lang/**/*.json"
---

# Frontend Instructions

Follow `src/AGENTS.md` and `docs/design-principles.md`.

- Use generated commands from `@/rspc/bindings`; never hand-edit the generated
  binding file.
- Keep Application Preferences behind typed Rust settings commands. Tauri Store
  is only for resettable UI-local/transient state.
- Preserve missing/unsupported/stale states instead of displaying invented zero
  or healthy values.
- Keep high-frequency updates from rerendering unrelated subtrees; add a focused
  regression test when changing fan-out.
- Add user-visible text to the language files and use existing i18n patterns.
- Verify visual and interaction changes in rendered desktop and compact views.
  Inspect E2E screenshots/artifacts before weakening selectors.
