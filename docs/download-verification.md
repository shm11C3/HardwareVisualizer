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

GitHub Releases starting with v1.8.1 include `SHA256SUMS.txt` in the release
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

## GitHub build provenance attestations

Release assets starting with v1.8.1 also include GitHub build provenance
attestations.

This check requires the GitHub CLI and network access to GitHub. The `-R` flag
scopes verification to attestations associated with this repository, and the
command verifies the default SLSA provenance predicate for the local file.

```bash
gh attestation verify ./HardwareVisualizer_x.x.x_x64_en-US.msi -R shm11C3/HardwareVisualizer
```

For releases before v1.8.1, attestations may not be available.

## Winget

Winget is an official Windows installation path where the package is available.

```powershell
winget install shm11C3.HardwareVisualizer
winget show shm11C3.HardwareVisualizer
```

Winget is an installation channel. It does not replace Authenticode signing,
SHA-256 checksums, or GitHub build provenance attestations.

For Winget manifest checks on v1.8.1 and later, use the SHA-256 value for the
Windows installer from `SHA256SUMS.txt` to populate or verify
`InstallerSha256`.
