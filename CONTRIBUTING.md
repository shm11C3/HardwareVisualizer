# HardwareVisualizer Contributing Guide

[![Linted with Biome](https://img.shields.io/badge/Linted_with-Biome-60a5fa?style=flat&logo=biome)](https://biomejs.dev)

Thank you for your interest in contributing!  
HardwareVisualizer is an open-source project, and we welcome improvements from the community.

## How to Contribute

- **New features:** Please create an Issue before opening a Pull Request.
- **Small fixes / refactoring:** An Issue is optional.
- **Bug reports and ideas:** Opening an Issue is also a great contribution.

### Bug Report

[Create Issue](https://github.com/shm11C3/HardwareVisualizer/issues/new?assignees=&labels=bug&projects=&template=bug_report.md&title=%5BBUG%5D)

### Feature Request

[Create Issue](https://github.com/shm11C3/HardwareVisualizer/issues/new?assignees=shm11C3&labels=enhancement&projects=&template=feature_request.md&title=%5BFeature+request%5D)

## Pull Request Guidelines

HardwareVisualizer automatically assigns labels based on the branch name.  
Branch naming convention:

- Features: `feat/<short-description-or-issue-number>`
- Bug fixes: `fix/<short-description-or-issue-number>`
- Documentation: `docs/<short-description-or-issue-number>`
- Refactoring: `refactor/<short-description-or-issue-number>`
- Other: `chore/<short-description-or-issue-number>`

When submitting a Pull Request (PR), please:

- Provide a concise description of the change
- Link any related Issue (if applicable)
- Ensure CI checks pass

## Development Guide

### Setup

Development requires the following tools:

- [Node.js (LTS recommended)](https://nodejs.org/)
- [Rust Stable](https://www.rust-lang.org/)

In addition, you will need the [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/#system-dependencies).
Please make sure to install these as well.

Next, install the dependencies:

```bash
npm ci
```

### Run Development App

```bash
npm run tauri dev
```

### Production Build

```bash
npm run tauri build
```

### Run linting, formatting, and tests locally before opening a PR

If there are errors in linting, formatting, or tests, the PR cannot be merged.  
Run these before opening a PR:

For JavaScript/TypeScript:

```bash
npm run lint
npm run format
npm test
```

For Rust:

```bash
cargo tauri-fmt
cargo tauri-lint
cargo tauri-test
```

> Note: We use Cargo aliases defined at the project root (`.cargo/config.toml`).

## Security

If you discover a vulnerability, do not open a public issue. Instead, please contact the maintainer directly via GitHub or via email at: `m11c3.sh@gmail.com`.

## License

By contributing to HardwareVisualizer, you agree that your contributions will be licensed under the [MIT License](LICENSE).
