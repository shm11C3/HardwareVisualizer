# HardwareVisualizer

<p align="left">
  <img alt="GitHub Release" src="https://img.shields.io/github/v/release/shm11C3/HardwareVisualizer?&display_name=release">
  <img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/shm11C3/HardwareVisualizer/publish.yaml">
  <img alt="Windows Support Only" src="https://img.shields.io/badge/platform-Windows-blue?logo=windows">
  <img alt="GitHub Downloads (all assets, all releases)" src="https://img.shields.io/github/downloads/shm11C3/HardwareVisualizer/total">
</p>

![image](https://github.com/user-attachments/assets/c474a132-5768-4046-9703-766e74ee3e66)

## Supported OS

| OS      | Status                                                                       |
| ------- | ---------------------------------------------------------------------------- |
| Windows | ✅ [Download](https://github.com/shm11C3/HardwareVisualizer/releases/latest) |
| MacOS   | ⏳                                                                           |
| linux   | ⏳                                                                           |

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
| Hardware Data History     | ⏳     |

### Dashboard

![image](https://github.com/user-attachments/assets/e56a701d-a2e0-417a-9bf1-edca8be014a5)

### Usage Graph

![image](https://github.com/user-attachments/assets/ef3e1630-e567-47a1-a437-f9a3981dd587)

### Custom Graph

![image](https://github.com/user-attachments/assets/814eff68-9190-4c39-a67d-a7458778ec95)

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

[Rust 1.84](https://www.rust-lang.org/)

```bash
$ rustc -V
rustc 1.84.1 (e71f9a9a9 2025-01-27)
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
   npm run test:unit ## frontend unit tests
   npm run test:tauri ## rust tests
  ```
