---
id: LRN-20261104-prefer-powershell-for-windows-packaging
status: promoted
cause_status: confirmed
scope: Windows-only sensor-service build and MSI packaging
trigger: choosing a language for native Windows packaging orchestration
failure_signature: a JS wrapper was selected just because the frontend already used Node although the workflow was Windows-only and used PowerShell signing and MSI tools
root_cause: tool availability was treated as sufficient justification instead of the platform and owning workflow
guardrail: the sensor-service owner documents PowerShell orchestration and the Windows Tauri hook invokes it directly
canonical_refs: sensor-service/README.md, sensor-service/build-bundle.ps1, src-tauri/tauri.windows.conf.json
verification: build-bundle.ps1 builds and stages the service; check-msi.ps1 verifies its embedded validator, optional feature and service lifecycle
evidence: maintainer correction during issue 2508 implementation and the Windows-specific packaging and signing call chain
revalidate_when: packaging becomes genuinely shared across operating systems or the native signing toolchain changes
---

# Prefer PowerShell for Windows Packaging

Choose the language with the workflow owner. The sensor-service bundling step
builds a Windows executable, prepares WiX inputs and invokes the existing
PowerShell signer. Keep that orchestration in PowerShell and explicitly check
native exit codes. Node's presence for the frontend does not justify a second
wrapper around this Windows-only work; `sh` remains appropriate for shared
Unix workflows rather than Windows-specific path and signing operations.

The maintainer also caught `Profile` colliding with PowerShell's automatic
`$PROFILE` variable. The packaging parameter is `BuildProfile`; run
PSScriptAnalyzer on the script, including its automatic-variable rule, rather
than relying only on parser success or a completed build.
