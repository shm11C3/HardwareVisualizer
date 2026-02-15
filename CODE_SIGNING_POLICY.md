# Code signing policy

This project signs and distributes release artifacts. The signing method differs by platform.

## Windows — SignPath Foundation (pending)

We are applying to the SignPath Foundation program.

Planned statement (required by the program, if approved):
"Free code signing provided by SignPath.io, certificate by SignPath Foundation"

Status: Pending approval.

### What will be signed

- Windows installer packages (e.g. .exe, .msi) published on GitHub Releases.

### Build and signing process

- Artifacts are built from this repository using CI.
- Only CI-built artifacts will be submitted to SignPath for signing.
- The private key is held by SignPath (HSM-backed). This project does not store the private key.

### Team roles (single-maintainer project)

- Authors (commit access, can modify the repository without additional reviews):
  - <https://github.com/shm11C3>

- Reviewers (review required for changes proposed by non-committers, e.g. pull requests):
  - <https://github.com/shm11C3>
  - Policy: All external pull requests are reviewed by the maintainer before merge.

- Approvers (approve each signing request):
  - <https://github.com/shm11C3>
  - Policy: Each signing request requires explicit approval by the maintainer.

## macOS

- Signed with Apple Developer ID and notarized by Apple.

## Linux (currently unsigned)

Status: Not implemented yet.

### What is distributed

- Linux artifacts (e.g. AppImage, .deb, .rpm) published on GitHub Releases.

### Verification

- At this time, Linux artifacts are not cryptographically signed by this project.
- Users should obtain artifacts only from the official GitHub Releases page.

### Future plan (non-binding)

- We may add artifact signing (e.g. Sigstore/cosign or GPG) in a future release.

## Distribution locations

- <https://github.com/shm11C3/HardwareVisualizer/releases>

## Privacy policy

This program will not transfer any information to other networked systems unless specifically requested by the user.
