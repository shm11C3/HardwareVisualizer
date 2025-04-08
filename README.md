# HardwareVisualizer

<p align="left">
  <img alt="GitHub Release" src="https://img.shields.io/github/v/release/shm11C3/HardwareVisualizer?&display_name=release">
  <img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/shm11C3/HardwareVisualizer/test-build.yml">
  <img alt="CI" src="https://github.com/shm11C3/HardwareVisualizer/actions/workflows/ci.yml/badge.svg?branch=develop">
  <img alt="Windows Support Only" src="https://img.shields.io/badge/platform-Windows-blue?logo=windows">
  <img alt="GitHub Downloads (all assets, all releases)" src="https://img.shields.io/github/downloads/shm11C3/HardwareVisualizer/total">
</p>

![image](https://github.com/user-attachments/assets/c474a132-5768-4046-9703-766e74ee3e66)

HardwareVisualizer is a tool for real-time monitoring of your computer's hardware performance. It provides an intuitive dashboard, detailed usage graphs, and customizable settings to help you keep track of your system’s vital statistics.

## Supported OS

| OS      | Status                                                                       |
| ------- | ---------------------------------------------------------------------------- |
| Windows | ✅ [Download](https://github.com/shm11C3/HardwareVisualizer/releases/latest) |
| MacOS   | ⏳                                                                           |
| Linux   | ⏳                                                                           |

## Features

| Feature                   | Status |
| ------------------------- | ------ |
| CPU Usage Monitoring      | ✅     |
| RAM Usage Monitoring      | ✅     |
| GPU Usage Monitoring      | ✅     |
| Temperature Monitoring    | ⏳     |
| Fan Monitoring            | ⏳     |
| Storage Monitoring        | ✅     |
| Network Monitoring        | ✅     |
| Customizable Chart Themes | ✅     |
| Customizable Dashboard    | ⏳     |
| Custom Background Image   | ✅     |
| Insights (Hardware Data History)     | ✅     |

## Screenshots

### Dashboard

The current status of the hardware can be checked at a glance.

![image](https://github.com/user-attachments/assets/afc91145-f4fd-4556-aca3-f24bd6c1be28)

### Usage Graph

The resource utilization for the last 1 minute can be checked.

![image](https://github.com/user-attachments/assets/ef3e1630-e567-47a1-a437-f9a3981dd587)

### Insight

View resource utilization for up to the past 30 days.  
Usage rates are calculated on a minute-by-minute basis.

![image](https://github.com/user-attachments/assets/dd849d54-37a0-4f00-bec8-9c7f994d49fa)

![image](https://github.com/user-attachments/assets/7c3f9ddd-37c1-45b1-9c3a-9f661817e797)

*GPU Insight is available on Nvidia GPU only.

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

### Requirements

[Node.js v22](https://nodejs.org/)

```bash
$ node -v
v22.14.0
```

[Rust 1.85](https://www.rust-lang.org/)

```bash
$ rustc -V
rustc 1.85.0 (4d91de4e4 2025-02-17)
```

### Getting Started

- Install dependencies:

  ```bash
  npm ci
  ```

- Launch in development mode:

  ```bash
  npm run tauri dev
  ```

- Build the app for production:

  ```bash
  npm run tauri build
  ```

- Lint the code:

  ```bash
   npm run lint
  ```

- Format the code:

  ```bash
   npm run format
  ```

- Run tests:

  ```bash
   npm run test:unit # frontend unit tests
   npm run test:tauri # rust tests
  ```
