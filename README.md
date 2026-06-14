# HardwareVisualizer

[English](README.md) | [日本語](README.ja.md)

[![Release](https://img.shields.io/github/v/release/shm11C3/HardwareVisualizer?&display_name=release)](https://github.com/shm11C3/HardwareVisualizer/releases)
[![CI develop](https://github.com/shm11C3/HardwareVisualizer/actions/workflows/ci.yml/badge.svg?branch=develop)](https://github.com/shm11C3/HardwareVisualizer/actions/workflows/ci.yml)
![Platforms](https://img.shields.io/badge/platform-Windows%20|%20Linux%20|%20MacOS-blue)
![Downloads](https://img.shields.io/github/downloads/shm11C3/HardwareVisualizer/total?link=https%3A%2F%2Fgithub.com%2Fshm11C3%2FHardwareVisualizer%2Freleases%2Flatest)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2Fshm11C3%2FHardwareVisualizer.svg?type=shield)](https://app.fossa.com/projects/git%2Bgithub.com%2Fshm11C3%2FHardwareVisualizer?ref=badge_shield)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/shm11C3/HardwareVisualizer/badge)](https://scorecard.dev/viewer/?uri=github.com/shm11C3/HardwareVisualizer)
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
      - [Using the Installer](#using-the-installer)
      - [Using Winget](#using-winget)
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
  - [Permissions \& Security Notes](#permissions--security-notes)
  - [Roadmap](#roadmap)
  - [Contributing](#contributing)
  - [FAQ](#faq)
  - [Code Signing Policy](#code-signing-policy)
  - [License](#license)

## Installation Guide

### Download

Choose your platform and download the latest installer:

- **Official Website**: [hardviz.com/#download](https://hardviz.com/#download)
- **GitHub Releases**: [Latest Release](https://github.com/shm11C3/HardwareVisualizer/releases/latest) > Assets section

For checksum and provenance checks, see the
[download verification guide](docs/download-verification.md).

### Windows Installation

#### Using the Installer

1. Download `HardwareVisualizer_x.x.x_x64-setup_windows.exe` or `HardwareVisualizer_x.x.x_x64_en-US_windows.msi` from the download page
2. Run the installer (`.exe` or `.msi` file)
3. Follow the installation wizard
4. Launch **HardwareVisualizer** from Start Menu or Desktop shortcut

#### Using Winget

You can also install using Windows Package Manager (Winget).
Run the following command in PowerShell or Command Prompt:

```powershell
winget install shm11C3.HardwareVisualizer
```

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

| Category                | Status | Notes                            |
| ----------------------- | ------ | -------------------------------- |
| CPU / RAM Usage         | ✅     | Realtime + history               |
| GPU Usage               | ✅     | NVIDIA full / others partial     |
| GPU Temperature         | ✅     | NVIDIA full / others partial     |
| CPU / Sensor Temperature | ✅    | Windows only (ACPI thermal zones, best-effort) |
| Fan Monitoring          | ⏳     | Planned                          |
| Storage Monitoring      | ✅     | Device summary                   |
| Network Monitoring      | ✅     | Basic interfaces / Usage planned |
| Custom Graph Themes     | ✅     | Persistent                       |
| Dashboard Customization | ✅     | Layout editing partial           |
| Background Image        | ✅     | Local assets                     |
| Historical Insights     | ✅     | Default Up to 30 days            |
| GPU Insight             | ✅     | NVIDIA full / others partial     |
| Language Support        | ✅     | English, Japanese, Russian       |

## Supported OS

| OS      | Status | Download                                  |
| ------- | ------ | ----------------------------------------- |
| Windows | ✅     | [Download](https://hardviz.com/#download) |
| Linux   | ✅     | [Download](https://hardviz.com/#download) |
| macOS   | ✅     | [Download](https://hardviz.com/#download) |

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

![image](https://github.com/user-attachments/assets/2d3d2045-ccc0-46ee-9a3a-6cde3e13981e)

### Custom Graph

Flexible graph customization available.

![image](https://github.com/user-attachments/assets/b6b2436b-c4c7-4252-9654-c5f2ca89e499)

### Background Image

<img width="1920" height="1055" alt="bg-image" src="https://github.com/user-attachments/assets/01734d4d-8e8f-4ca5-a73b-fba9a428d3d0" />


## Permissions & Security Notes

| Context               | Reason                                                  |
| --------------------- | ------------------------------------------------------- |
| Linux sudo            | Access to certain device files (GPU, sensors)           |
| Windows WMI           | Memory and system extended metrics, thermal zones       |
| Windows PDH           | GPU engine utilization                                  |
| No outbound telemetry | No telemetry; the app does not send any data externally |

## Roadmap

| Item                         | Target   |
| ---------------------------- | -------- |
| macOS Support                | ✅ Done  |
| AMD GPU compatible           | ✅ Done  |
| Fan / Temp Full Cross Vendor | Research |
| Game Mode                    | Planned  |
| Power Consumption Estimation | Idea     |
| Plugin System                | Idea     |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

Developer and maintainer documentation starts at
[docs/README.md](docs/README.md).

## Code Signing Policy

See [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md) for signing status and the
[download verification guide](docs/download-verification.md) for checksum and
provenance checks.

## Special Thanks

HardwareVisualizer is made possible by many open-source projects, tools, and contributors.

- [Tauri](https://tauri.app/) — for providing the foundation for building lightweight cross-platform desktop applications.
- [sysinfo](https://github.com/GuillaumeGomez/sysinfo) — for cross-platform system information collection.
- [nvapi-rs](https://github.com/arcnmx/nvapi-rs) — for enabling access to NVIDIA's NVAPI from Rust.
- [macmon](https://github.com/vladkens/macmon) — for the MIT-licensed macOS monitoring implementation that informed parts of HardwareVisualizer's macOS sensor support.
- [PawnIO](https://pawnio.eu/) and [PawnIO.Modules](https://github.com/namazso/PawnIO.Modules) — for providing the low-level Windows interface that HardwareVisualizer can integrate with when available for optional native CPU temperature support.

Note: This acknowledgement does not mean that all listed projects are bundled with HardwareVisualizer or used in every build. The Windows PawnIO CPU temperature implementation is implemented from the repository's clean-room sensor specifications, not by porting third-party monitoring implementations.

## License

[MIT License](LICENSE)
