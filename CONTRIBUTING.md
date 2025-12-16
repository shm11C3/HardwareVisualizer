# HardwareVisualizer Contributing Guide

Thank you are interested in contributing to HardwareVisualizer!  
HardwareVisualizer is an open-source project, and we welcome improvements from the community.

## Pull Request Guidelines

- For new features, we recommend creating an Issue before implementation.
- For bug fixes, creating an Issue is optional.

Branch naming convention:

- Features: `feat/<short-description or issue-number>`
- Bug fixes: `fix/<short-description or issue-number>`

When submitting a Pull Request (PR), please:

- Provide a concise description of the change
- Link any related Issue
- Ensure CI checks pass

## Development Guide

### Setup

Development requires the following tools:

- [Node.js v22](https://nodejs.org/)
- [Rust 1.89](https://www.rust-lang.org/)

If you are using Linux, you may need to install additional dependencies:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

Next, install the dependencies:

```bash
npm ci
```

### Run lint & tests locally before opening a PR

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
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1 --nocapture
```

## Security

If you discover a vulnerability, do not open a public issue. Instead, please contact the maintainer directly via GitHub or via email at: `m11c3.sh@gmail.com`.

## License

By contributing to HardwareVisualizer, you agree that your contributions will be licensed under the [MIT License](LICENSE).
