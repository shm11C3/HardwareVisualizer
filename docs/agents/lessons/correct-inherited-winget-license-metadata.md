---
id: LRN-20261003-correct-inherited-winget-license-metadata
status: promoted
cause_status: confirmed
scope: .github/workflows/winget.yml
trigger: changing package licenses or WinGet release submission
failure_signature: WinGet 1.11.0 declared MIT after the application switched to GPL-3.0-or-later
root_cause: wingetcreate update inherited the previous locale License and the workflow submitted it without correction
guardrail: correct generated locale license metadata before submission using the target release tag
canonical_refs: docs/adr/0020-relicense-to-gpl-3.0-or-later.md, .github/workflows/winget.yml, .github/scripts/update-winget-license.mjs, .github/scripts/test-update-winget-license.mjs
verification: node .github/scripts/test-update-winget-license.mjs
evidence: v1.11.0 package.json and src-tauri/tauri.conf.json; microsoft/winget-pkgs manifests/s/shm11C3/HardwareVisualizer/1.11.0/shm11C3.HardwareVisualizer.locale.en-US.yaml
revalidate_when: wingetcreate generation behavior, manifest format, release license source or relicensing policy changes
---

# Correct Inherited WinGet License Metadata

The application declared GPL-3.0-or-later at v1.11.0, but its published WinGet
default locale still declared MIT. ADR 0020 already identified that
`wingetcreate update` does not rewrite this field. Updating repository package
metadata alone did not update the distribution catalog.

The submission workflow now generates manifests locally, corrects locale
`License` and `LicenseUrl`, and submits the corrected directory. The license
comes from `package.json` at the target release tag; using the current branch
would incorrectly relabel historical MIT releases. The license URL points to
`LICENSE` at the same tag.

Focused tests check GPL and historical MIT metadata, optional locale fields,
idempotence, missing default metadata, and unchanged installer/version files.
The v1.11.0 catalog correction is a separate manual contribution to the external
repository; fixing the next release does not correct existing version entries.
