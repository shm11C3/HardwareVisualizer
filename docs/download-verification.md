# Download verification

[English](download-verification.md) | [日本語](download-verification.ja.md)

Use this guide to verify HardwareVisualizer release files downloaded from the
official distribution channels.

## Official sources

Official downloads and installations are available only from:

- GitHub Releases: <https://github.com/shm11C3/HardwareVisualizer/releases>
- Official website: <https://hardviz.com/>
- Winget for Windows, where available

Third-party mirrors, download sites, file-sharing links, YouTube description
links, and password-protected archives are not official distribution channels.
Malware campaigns have also used fake sites that impersonate official download
pages to distribute malicious installers. Check the domain carefully before
downloading, and verify GitHub Release assets with the checks below when
available.

## SHA-256 checksums

Starting with v1.8.1, GitHub Releases include `SHA256SUMS.txt` in the release
Assets section as the canonical checksum list for release assets.

Download `SHA256SUMS.txt` from the same GitHub Release as your installer and
compare the SHA-256 value for the matching filename.

Windows:

```powershell
Get-FileHash .\HardwareVisualizer_x.x.x_x64_en-US.msi -Algorithm SHA256
```

macOS:

```bash
shasum -a 256 HardwareVisualizer_x.x.x_aarch64.dmg
```

Linux:

```bash
sha256sum hardware-visualizer_x.x.x_amd64.deb
```

For releases before v1.8.1, `SHA256SUMS.txt` may not be available.

## GitHub Artifact Attestations

Starting with v1.8.1, GitHub Artifact Attestations are generated for release
assets.

This is an advanced verification step. Most users should first verify that the
file matches the SHA-256 value published in `SHA256SUMS.txt`.

This check requires the GitHub CLI and network access to GitHub. The `-R` flag
scopes verification to attestations associated with this repository, and the
command verifies the default SLSA provenance predicate for the local file.

```bash
gh attestation verify ./HardwareVisualizer_x.x.x_x64_en-US.msi -R shm11C3/HardwareVisualizer
```

For releases before v1.8.1, GitHub Artifact Attestations may not be available.

## Windows Authenticode signature

Windows `.exe` and `.msi` release installers are Authenticode signed starting
with v1.9.0. Earlier Windows release installers may be unsigned.

Verify the installer signature with PowerShell:

```powershell
Get-AuthenticodeSignature .\HardwareVisualizer_x.x.x_x64_en-US.msi | Format-List
```

For the NSIS setup executable:

```powershell
Get-AuthenticodeSignature .\HardwareVisualizer_x.x.x_x64-setup.exe | Format-List
```

Successful output should report `Status: Valid`. You can also inspect the
signer certificate and timestamp details in the command output.

If you have Windows SDK tools installed, `signtool` can perform the same policy
check:

```powershell
signtool verify /pa /v .\HardwareVisualizer_x.x.x_x64-setup.exe
```

Windows SmartScreen may still show a reputation warning for a validly signed
installer while publisher or file reputation is being established.

## macOS signature and notarization

macOS downloads are signed with Apple Developer ID and notarized by Apple.

Verify the downloaded disk image signature:

```bash
codesign --verify --verbose=2 HardwareVisualizer_x.x.x_aarch64.dmg
```

Verify Gatekeeper acceptance and notarization status for the disk image:

```bash
spctl -a -vv --type open HardwareVisualizer_x.x.x_aarch64.dmg
```

If you already copied the app bundle to `/Applications`, verify the installed
app bundle signature:

```bash
codesign --verify --deep --strict --verbose=2 /Applications/HardwareVisualizer.app
```

Successful `spctl` output should report `accepted`, and the detailed output
should identify a Developer ID source.

## Linux package signing

Linux packages, such as AppImage, `.deb`, and `.rpm` files, are not currently
signed with a Linux package-signing mechanism such as GPG, Sigstore/cosign, or
repository metadata signing. Verify Linux downloads with `SHA256SUMS.txt` and
GitHub Artifact Attestations when available.

## Tauri updater `.sig` assets

Release assets ending in `.sig` are Tauri updater signatures for the in-app
update path. They do not replace Windows Authenticode signing, macOS
notarization, Linux package signing, SHA-256 checksums, or GitHub Artifact
Attestations for manual downloads.

## Winget

Winget is an official Windows installation path where the package is available.

```powershell
winget install shm11C3.HardwareVisualizer
winget show shm11C3.HardwareVisualizer
```

Winget is an installation channel. It does not replace Authenticode signing,
SHA-256 checksums, or GitHub Artifact Attestations.

For Winget manifest checks on v1.8.1 and later, use the SHA-256 value for the
Windows installer from `SHA256SUMS.txt` to populate or verify
`InstallerSha256`.
