# Add a New Language

This guide describes how to add a new officially supported UI language.

Language support crosses the frontend and Rust app layer:

- the frontend owns i18next resources and renders translated strings;
- the Rust app layer owns first-run language fallback through
  `SUPPORTED_LANGUAGES`;
- Settings persists the selected language through `settings.json`.

When adding a language, update both frontend and Rust support together.

## Language Code

Use a stable short language code such as `en`, `ja`, or `ru`.

The current implementation stores this code directly in Settings and uses it as
the i18next resource key. If a future language needs a regional code, check the
Rust locale extraction logic in `src-tauri/src/services/language_service.rs`
before choosing the code.

## Steps

1. Add `src/lang/<code>.json`.

   Use `src/lang/en.json` as the source structure. Keep the same key hierarchy
   and include `lang.<code>` so the Settings language selector can show the
   language name.

2. Register the resource in `src/lib/i18n.ts`.

   Import the JSON file and add it to the `resources` object:

   ```ts
   import xx from "@/lang/xx.json";

   const resources = {
     // existing languages...
     xx: {
       translation: xx,
     },
   };
   ```

3. Add the resource type in `src/types/i18next.d.ts`.

   Import the JSON type and add it to the `Resources` type.

4. Add the language code to Rust `SUPPORTED_LANGUAGES`.

   Update `src-tauri/src/services/language_service.rs` so first-run OS language
   fallback recognizes the new language as officially supported.

5. Update Rust language service tests.

   Add a test matching the existing `supported_languages_contains_*` tests in
   `src-tauri/src/services/language_service.rs`.

6. Update frontend i18n tests.

   Add a resource-bundle assertion in `src/lib/i18n.test.ts`. If the new
   language intentionally relies on English fallback for some keys, add a
   focused fallback assertion.

7. Check locale-sensitive formatting.

   Search for code that branches on `settings.language`. Some process and
   history table code currently chooses `ja-JP` or `en-US` explicitly. Update
   those call sites if the new language needs a locale-specific formatter.

8. Check Settings UI.

   `LanguageSelect` derives available options from registered i18next
   resources. After step 2, the new language should appear automatically.

## Validation

Run the relevant checks from the repository root:

```bash
npm test
cargo tauri-test
```

If the translation changes UI layout-sensitive text, manually inspect the
Settings screen and any updated feature screens.
