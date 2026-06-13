# Project Documentation

This directory contains project documentation that is not part of the primary
user-facing root README.

Start here when looking for development, architecture, release, or maintenance
documentation.

## Main Entry Points

- [Backend architecture](architecture/backend.md)
- [External components](user/external-components.md)
- [Windows sensor external components](architecture/windows-sensor-external-components.md)
- [Architecture decision records](adr/)
- [Sensor hardware specs (clean-room)](specs/sensors/)
- [Frontend architecture](../src/README.md)
- [Core crate guide](../core/README.md)
- [Tauri app crate guide](../src-tauri/README.md)
- [Add a new language](development/add-language.md)
- [E2E capture harness](development/e2e-captures.md)
- [Download verification](download-verification.md)
- [Documentation guide](documentation-guide.md)

## Documentation Map

```text
docs/
├── README.md                       # Documentation index
├── documentation-guide.md          # Documentation placement and naming rules
├── adr/                            # Architecture decision records
├── architecture/                   # Architecture documents
├── development/                    # Developer task guides
├── specs/                          # Clean-room hardware specification documents
│   └── sensors/                    # Sensor specs for PawnIO-based monitoring
├── third-party-notices/            # Generated and manual third-party notices
├── download-verification.md        # Download verification guide
├── download-verification.ja.md     # Japanese download verification guide
└── internal/                       # Maintainer/internal operations docs
```

## Current Layout Notes

- The Japanese user-facing README lives at [`../README.ja.md`](../README.ja.md)
  beside the English root README. It is not a translation of this documentation
  index.
- Documentation directories and handwritten Markdown files should use
  lowercase kebab-case. Generated legal notice files named
  `THIRD_PARTY_NOTICES.md` are the current exception.
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
