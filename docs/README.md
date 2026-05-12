# Project Documentation

This directory contains project documentation that is not part of the primary
user-facing root README.

Start here when looking for development, architecture, release, or maintenance
documentation.

## Main Entry Points

- [Backend architecture](ARCHITECTURE/BACKEND_ARCHITECTURE.md)
- [Frontend architecture](../src/README.md)
- [Core crate guide](../core/README.md)
- [Tauri app crate guide](../src-tauri/README.md)
- [Add a new language](DEVELOPMENT/add-language.md)
- [Download verification](download-verification.md)
- [Documentation guide](documentation-guide.md)

## Documentation Map

```text
docs/
├── README.md                       # Documentation index
├── documentation-guide.md          # Documentation placement and naming rules
├── ARCHITECTURE/                   # Architecture documents
├── DEVELOPMENT/                    # Developer task guides
├── THIRD_PARTY_NOTICES/            # Generated and manual third-party notices
├── download-verification.md        # Download verification guide
├── download-verification.ja.md     # Japanese download verification guide
└── internal/                       # Maintainer/internal operations docs
```

## Current Layout Notes

- The Japanese user-facing README lives at [`../README.ja.md`](../README.ja.md)
  beside the English root README. It is not a translation of this documentation
  index.
- `docs/ARCHITECTURE/`, `docs/DEVELOPMENT/`, and
  `docs/THIRD_PARTY_NOTICES/` still use legacy uppercase directory names.
  New documentation should follow the naming rules in
  [documentation-guide.md](documentation-guide.md). Renaming existing
  directories should be handled separately because it affects links, workflows,
  and generated files.
- `tmp/THIRD_PARTY_NOTICES.md` is tracked and used as the current Tauri bundled
  third-party notices resource. Although it lives under `tmp/`, it is not a
  disposable scratch file. Moving it to a clearer runtime resource location,
  such as `src-tauri/resources/THIRD_PARTY_NOTICES.md`, should be handled as a
  separate release/bundling change.

## Root-Level Documents

Some documents intentionally live at the repository root because GitHub or
contributors expect them there:

- [`README.md`](../README.md)
- [`CONTRIBUTING.md`](../CONTRIBUTING.md)
- [`SECURITY.md`](../SECURITY.md)
- [`CODE_SIGNING_POLICY.md`](../CODE_SIGNING_POLICY.md)
- [`LICENSE`](../LICENSE)
