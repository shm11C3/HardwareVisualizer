# Code signing and download authenticity policy

HardwareVisualizer publishes release artifacts through GitHub Releases and the
official website. The signing and verification status differs by platform.

## Official distribution and installation locations

Official downloads and installations are available only from:

- GitHub Releases: <https://github.com/shm11C3/HardwareVisualizer/releases>
- Official website: <https://hardviz.com/>
- Winget for Windows, where available

Third-party mirrors, download sites, file-sharing links, YouTube description
links, shortened URLs, and password-protected archives are not official
distribution channels.

Fake sites impersonating official download pages are a real malware distribution
risk. Users should verify the domain before downloading.

## Current signing status

| Platform      | Signing status                 | Verification                               |
| ------------- | ------------------------------ | ------------------------------------------ |
| Windows       | Signed (v1.9.0+ installers)    | Authenticode, Winget, SHA-256, attestation |
| macOS         | Signed and notarized           | Gatekeeper, codesign, SHA-256, attestation |
| Linux         | Unsigned packages              | SHA-256, attestation                       |
| Tauri updater | Signed updater assets          | Update-path only                           |

Windows release installers are Authenticode signed starting with v1.9.0.
Earlier Windows releases may be unsigned. For all platforms, use official
distribution locations and verify GitHub Release downloads with SHA-256
checksums and GitHub Artifact Attestations where available.

SHA-256 checksums and GitHub Artifact Attestations are generated for v1.8.1 and
later releases that include verification metadata.

Tauri updater `.sig` assets protect the in-app update path. They do not replace
platform code signing, Linux package signing, SHA-256 checksums, or GitHub
Artifact Attestations for manual downloads.

## Windows

Status: Authenticode signed for v1.9.0 and later release installers.

Windows `.exe` and `.msi` release installers built by the official publish
workflow are Authenticode signed through SSL.com eSigner starting with v1.9.0.
Earlier Windows release installers may be unsigned.

Verify Windows downloads using:

- official distribution locations
- Authenticode signature validation
- SHA-256 checksums
- GitHub Artifact Attestations

Use Winget as the recommended Windows installation path where available:

```powershell
winget install shm11C3.HardwareVisualizer
winget show shm11C3.HardwareVisualizer
```

Winget is an official installation path, but it is not a replacement for
Authenticode signing, SHA-256 checksums, or GitHub Artifact Attestations.

Windows SmartScreen may still show a warning for a validly signed installer
while publisher or file reputation is being established. Authenticode signature
validation and SmartScreen reputation are related but separate checks.

### Signing process

The SSL.com signing process applies to Windows installer packages, such as
`.exe` and `.msi` files, published on GitHub Releases.

- Artifacts are built from this repository using CI.
- Only CI-built artifacts will be signed for release distribution.
- Certificate material and signing access are handled through the SSL.com
  signing workflow.

### Team roles

- Authors, with commit access:
  - <https://github.com/shm11C3>

- Reviewers, for changes proposed by non-committers:
  - <https://github.com/shm11C3>
  - Policy: external pull requests are reviewed by the maintainer before merge.

- Approvers, for each signing request:
  - <https://github.com/shm11C3>
  - Policy: each signing request requires explicit approval by the maintainer.

## macOS

Status: signed with Apple Developer ID and notarized by Apple.

Users can verify macOS artifacts with Gatekeeper and code signing tools. See the
[download verification guide](docs/download-verification.md)
for copy-pasteable commands.

## Linux

Status: Linux packages are not signed with a Linux package-signing mechanism
yet.

Linux artifacts, such as AppImage, `.deb`, and `.rpm` files, are published
through GitHub Releases.

Until Linux package signing is implemented, verify downloads using:

- official distribution locations
- SHA-256 checksums
- GitHub Artifact Attestations

Release assets ending in `.sig`, including Linux `.sig` assets, are Tauri
updater signatures. They are not GPG, Sigstore/cosign, repository, or package
manager signatures for manual Linux package verification.

We may add Linux artifact signing, such as Sigstore/cosign or GPG, in a future
release.

## Release integrity controls

For v1.8.1 and later releases that include verification metadata, the release
workflow provides two repository-generated verification layers:

- `SHA256SUMS.txt` is attached to the GitHub Release and lists SHA-256 checksums
  for all release assets except itself.
- GitHub Artifact Attestations are generated for release assets and
  `SHA256SUMS.txt`. They are available through GitHub's attestation service and
  can be verified with GitHub CLI.

`SHA256SUMS.txt` is the canonical checksum source for user documentation,
website download metadata, and Winget manifest updates.

For Winget, use the Windows installer entry from `SHA256SUMS.txt` to populate or
verify `InstallerSha256`.

The official website may also provide a browser-based verification page that
computes SHA-256 locally without uploading the selected file.

## Tauri updater signatures

Tauri updater artifacts are signed using the Tauri updater signing mechanism.

These signatures protect the application update path, but they are not a
replacement for Windows Authenticode signing, macOS notarization, Linux package
signing, SHA-256 checksums, or GitHub Artifact Attestations.

## Verification guide

Users can verify release files by following the
[download verification guide](docs/download-verification.md).
