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
- Report a failed read where its data would appear: the hook exposes
  `hasError` and `retry`, the panel renders the shared failure component, and
  secondary lanes log only. Never open an OS dialog for a read; `features/hardware`
  cannot import `useTauriDialog` (lint). See the Failure Reporting section of
  `docs/design/frontend-architecture.md`.
- Keep high-frequency updates from rerendering unrelated subtrees; add a focused
  regression test when changing fan-out.
- Add user-visible text to the language files and use existing i18n patterns.
- Keep imports downward-only per
  [ADR 0027](../../docs/adr/0027-frontend-layer-boundaries-and-state-ownership.md):
  no cross-feature imports, no `../`, and atoms only in `store/` modules.
  Pending exceptions in `.config/biome/frontend-boundaries.jsonc` only
  shrink. Prove any edit to that file with canaries, because a mistyped scope
  silently disables the rule.
- Verify visual and interaction changes in rendered desktop and compact views.
  Inspect E2E screenshots/artifacts before weakening selectors.
