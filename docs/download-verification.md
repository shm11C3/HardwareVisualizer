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

GitHub Releases are planned to include `SHA256SUMS.txt` starting with v1.8.1
in the release Assets section as the canonical checksum list for release
assets.

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

GitHub Artifact Attestations are planned to be generated for release assets
starting with v1.8.1.

This is an advanced verification step. Most users should first verify that the
file matches the SHA-256 value published in `SHA256SUMS.txt`.

This check requires the GitHub CLI and network access to GitHub. The `-R` flag
scopes verification to attestations associated with this repository, and the
command verifies the default SLSA provenance predicate for the local file.

```bash
gh attestation verify ./HardwareVisualizer_x.x.x_x64_en-US.msi -R shm11C3/HardwareVisualizer
```

For releases before v1.8.1, GitHub Artifact Attestations may not be available.

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
