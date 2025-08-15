# HardwareVisualizer

[![Release](https://img.shields.io/github/v/release/shm11C3/HardwareVisualizer?&display_name=release)](https://github.com/shm11C3/HardwareVisualizer/releases)
[![CI test-build](https://img.shields.io/github/actions/workflow/status/shm11C3/HardwareVisualizer/test-build.yml?label=test-build)](https://github.com/shm11C3/HardwareVisualizer/actions)
[![CI develop](https://github.com/shm11C3/HardwareVisualizer/actions/workflows/ci.yml/badge.svg?branch=develop)](https://github.com/shm11C3/HardwareVisualizer/actions/workflows/ci.yml)
![Platforms](https://img.shields.io/badge/platform-Windows%20|%20Linux-blue)
![Downloads](https://img.shields.io/github/downloads/shm11C3/HardwareVisualizer/total)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

![image](https://github.com/user-attachments/assets/c474a132-5768-4046-9703-766e74ee3e66)

HardwareVisualizer is a tool for real-time monitoring of your computer's hardware performance. It provides an intuitive dashboard, detailed usage graphs, and customizable settings to help you keep track of your system’s vital statistics.

Web: <https://hardviz.com/>

## Table of Contents

- [HardwareVisualizer](#hardwarevisualizer)
  - [Table of Contents](#table-of-contents)
  - [Features](#features)
  - [Supported OS](#supported-os)
  - [Quick Start (End Users)](#quick-start-end-users)
  - [Screenshots](#screenshots)
  - [Bug Report](#bug-report)
  - [Feature Request](#feature-request)
  - [Development](#development)
  - [Permissions \& Security Notes](#permissions--security-notes)
  - [Roadmap (Excerpt)](#roadmap-excerpt)
  - [Contributing](#contributing)
  - [FAQ](#faq)
  - [Tips](#tips)
  - [License](#license)

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

## Quick Start (End Users)

1. Download installer from Website
2. Launch app
3. (Linux) If some metrics missing → restart via `sudo hardware-visualizer`
4. Open Settings → adjust theme & polling intervals

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

## Bug Report

[Create Issue](https://github.com/shm11C3/HardwareVisualizer/issues/new?assignees=&labels=bug&projects=&template=bug_report.md&title=%5BBUG%5D)

## Feature Request

[Create Issue](https://github.com/shm11C3/HardwareVisualizer/issues/new?assignees=shm11C3&labels=enhancement&projects=&template=feature_request.md&title=%5BFeature+request%5D)

## Development

[![Linted with Biome](https://img.shields.io/badge/Linted_with-Biome-60a5fa?style=flat&logo=biome)](https://biomejs.dev)

### Prerequisites

[Node.js v22](https://nodejs.org/)

```bash
$ node -v
v22.16.0
```

[Rust 1.89](https://www.rust-lang.org/)

```bash
$ rustc -V
rustc 1.89.0 (29483883e 2025-08-04)
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
npm run lint        # biome
npm run format
npm run test:unit   # frontend
npm run test:tauri  # rust
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

## FAQ

**Q: Why sudo on Linux?**  
Access to privileged hardware interfaces for sensors.

**Q: NVIDIA only historic GPU charts?**  
Vendor APIs differ; expansion planned.

## Tips

### Launching the app by Linux command line

You can launch the app by running the following command in the terminal:

```bash
sudo hardware-visualizer
```

> [!NOTE]
> 🔒 sudo is required to access certain system information such as CPU, memory, or GPU stats.

This works on both native Linux and WSL (Windows Subsystem for Linux) with GUI support (WSLg).
If installed correctly, a Tauri window will open on execution.

## License

[MIT License](LICENSE)
