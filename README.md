# HardwareVisualizer

[English](README.md) | [日本語](docs/README.ja.md)

[![Release](https://img.shields.io/github/v/release/shm11C3/HardwareVisualizer?&display_name=release)](https://github.com/shm11C3/HardwareVisualizer/releases)
[![CI develop](https://github.com/shm11C3/HardwareVisualizer/actions/workflows/ci.yml/badge.svg?branch=develop)](https://github.com/shm11C3/HardwareVisualizer/actions/workflows/ci.yml)
![Platforms](https://img.shields.io/badge/platform-Windows%20|%20Linux-blue)
![Downloads](https://img.shields.io/github/downloads/shm11C3/HardwareVisualizer/total)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2Fshm11C3%2FHardwareVisualizer.svg?type=shield)](https://app.fossa.com/projects/git%2Bgithub.com%2Fshm11C3%2FHardwareVisualizer?ref=badge_shield)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/shm11C3/HardwareVisualizer)

![image](https://github.com/user-attachments/assets/c474a132-5768-4046-9703-766e74ee3e66)

HardwareVisualizer is a tool for real-time monitoring of your computer's hardware performance. It provides an intuitive dashboard, detailed usage graphs, and customizable settings to help you keep track of your system’s vital statistics.

Web: <https://hardviz.com/>

> [!NOTE]
>
> ## Official downloads & security notice
>
> HardwareVisualizer is officially distributed **only** through the channels below:
>
> - GitHub Releases: https://github.com/shm11C3/HardwareVisualizer/releases
> - Official website: https://hardviz.com/
>
> Any other distribution (e.g. third-party mirrors or listings on download sites such as
> SourceForge) is **not affiliated** with this project.
>
> In particular, the SourceForge project named `Hardware Visualizer`
> (`https://sourceforge.net/projects/hardware-visualizer/`) was created without my
> involvement. I cannot verify the authenticity or safety of the ZIP archives
> published there. Use them at your own risk.

## Table of Contents

- [HardwareVisualizer](#hardwarevisualizer)
  - [Table of Contents](#table-of-contents)
  - [Installation Guide](#installation-guide)
    - [Download](#download)
    - [Windows Installation](#windows-installation)
    - [Linux Installation](#linux-installation)
    - [First-time Setup](#first-time-setup)
  - [Features](#features)
  - [Supported OS](#supported-os)
  - [Screenshots](#screenshots)
    - [Dashboard](#dashboard)
    - [Usage Graph](#usage-graph)
    - [Insight](#insight)
    - [Custom Graph](#custom-graph)
    - [Background Image](#background-image)
  - [Development](#development)
    - [Prerequisites](#prerequisites)
    - [Linux Build Dependencies](#linux-build-dependencies)
    - [Build \& Run](#build--run)
    - [Production Build](#production-build)
    - [Common scripts](#common-scripts)
  - [Permissions \& Security Notes](#permissions--security-notes)
  - [Roadmap (Excerpt)](#roadmap-excerpt)
  - [Contributing](#contributing)
    - [Bug Report](#bug-report)
    - [Feature Request](#feature-request)
  - [FAQ](#faq)
  - [License](#license)


[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2Fshm11C3%2FHardwareVisualizer.svg?type=large)](https://app.fossa.com/projects/git%2Bgithub.com%2Fshm11C3%2FHardwareVisualizer?ref=badge_large)

## Installation Guide

### Download

Choose your platform and download the latest installer:

- **Official Website**: [hardviz.com/#download](https://hardviz.com/#download)
- **GitHub Releases**: [Latest Release](https://github.com/shm11C3/HardwareVisualizer/releases/latest) > Assets section

### Windows Installation

1. Download `HardwareVisualizer_x.x.x_x64-setup_windows.exe` or `HardwareVisualizer_x.x.x_x64_en-US_windows.msi` from the download page
2. Run the installer (`.exe` or `.msi` file)
3. Follow the installation wizard
4. Launch **HardwareVisualizer** from Start Menu or Desktop shortcut

> [!NOTE]
> No additional permissions required on Windows

### Linux Installation

1. Download `hardware-visualizer_x.x.x_amd64.deb` from the download page
2. Install via package manager:

   ```bash
   sudo dpkg -i hardware-visualizer_*.deb
   sudo apt-get install -f  # Install dependencies if needed
   ```

3. Launch from application menu or terminal:

   ```bash
   hardware-visualizer
   ```

> [!TIP]
>
> ### Missing hardware data?
>
> Some metrics require elevated privileges. Restart with sudo for full hardware access:
>
> ```bash
> sudo hardware-visualizer
> ```

### First-time Setup

After launching the app:

1. Navigate to **Settings** (⚙️ icon in sidebar)
2. Choose your preferred **theme** and **language**
3. (Optional) Set a custom **background image**

## Features

| Category                 | Status | Notes                            |
| ------------------------ | ------ | -------------------------------- |
| CPU / RAM Usage          | ✅     | Realtime + history               |
| GPU Usage                | ✅     | NVIDIA full / others partial     |
| GPU Temperature          | ✅     | NVIDIA full / others partial     |
| Fan Monitoring           | ⏳     | Planned                          |
| Storage Monitoring       | ✅     | Device summary                   |
| Network Monitoring       | ✅     | Basic interfaces / Usage planned |
| Custom Graph Themes      | ✅     | Persistent                       |
| Dashboard Customization  | ✅     | Layout editing partial           |
| Background Image         | ✅     | Local assets                     |
| Historical Insights      | ✅     | Default Up to 30 days            |
| GPU Insight (non-NVIDIA) | ⚠️     | Limited now                      |

## Supported OS

| OS      | Status | Download                                  |
| ------- | ------ | ----------------------------------------- |
| Windows | ✅     | [Download](https://hardviz.com/#download) |
| Linux   | ✅     | [Download](https://hardviz.com/#download) |
| macOS   | 🔒     | Planned (v2)                              |

## Screenshots

### Dashboard

The current status of the hardware can be checked at a glance.

![image](https://github.com/user-attachments/assets/a578909a-5b85-4d3a-98cb-a885dc10eaec)

### Usage Graph

The resource utilization for the last 1 minute can be checked.

![image](https://github.com/user-attachments/assets/ef3e1630-e567-47a1-a437-f9a3981dd587)

![image](https://github.com/user-attachments/assets/7b786e00-12c0-4627-8b2a-cc3482072eb7)

### Insight

View resource utilization for up to the past 30 days.  
Usage rates are calculated on a minute-by-minute basis.

![image](https://github.com/user-attachments/assets/dd849d54-37a0-4f00-bec8-9c7f994d49fa)

![image](https://github.com/user-attachments/assets/7c3f9ddd-37c1-45b1-9c3a-9f661817e797)

\*GPU Insight is available on Nvidia GPU only.

![image](https://github.com/user-attachments/assets/2d3d2045-ccc0-46ee-9a3a-6cde3e13981e)

### Custom Graph

Flexible graph customization available.

![image](https://github.com/user-attachments/assets/b6b2436b-c4c7-4252-9654-c5f2ca89e499)

### Background Image

![image](https://github.com/user-attachments/assets/6ab09e8a-ebef-449a-b73f-07ae44626e20)

## Development

[![Linted with Biome](https://img.shields.io/badge/Linted_with-Biome-60a5fa?style=flat&logo=biome)](https://biomejs.dev)

### Prerequisites

[Node.js v22](https://nodejs.org/)

```bash
$ node -v
v22.16.0
```

[Rust 1.90](https://www.rust-lang.org/)

```bash
$ rustc -V
rustc 1.90.0 (1159e78c4 2025-09-14)
```

### Linux Build Dependencies

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

### Build & Run

```bash
npm ci
npm run tauri dev
```

### Production Build

```bash
npm run tauri build
```

### Common scripts

```bash
npm run lint
npm run format
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1 --nocapture
```

## Permissions & Security Notes

| Context               | Reason                                                  |
| --------------------- | ------------------------------------------------------- |
| Linux sudo            | Access to certain device files (GPU, sensors)           |
| Windows WMI           | GPU/memory extended metrics                             |
| No outbound telemetry | No telemetry; the app does not send any data externally |

## Roadmap (Excerpt)

| Item                         | Target |
| ---------------------------- | ------ |
| macOS Support                | v2     |
| Unified Platform Abstraction | v1.x   |
| Fan / Temp Full Cross Vendor | v1.x   |
| Dashboard Layout Editor      | v2.x   |
| Power Consumption Estimation | v2.x   |

## Contributing

1. Fork & branch (`feat/...` or `fix/...`)
2. Run lint & tests
3. Open PR with concise description
4. Link related issue

Style:

- Rust: `cargo fmt` (enforced by CI if added)
- TS: Biome config defaults

### Bug Report

[Create Issue](https://github.com/shm11C3/HardwareVisualizer/issues/new?assignees=&labels=bug&projects=&template=bug_report.md&title=%5BBUG%5D)

### Feature Request

[Create Issue](https://github.com/shm11C3/HardwareVisualizer/issues/new?assignees=shm11C3&labels=enhancement&projects=&template=feature_request.md&title=%5BFeature+request%5D)

## FAQ

**Q: Why sudo on Linux?**  
Access to privileged hardware interfaces for sensors.

**Q: NVIDIA only historic GPU charts?**  
Vendor APIs differ; expansion planned.

## License

[MIT License](LICENSE)
