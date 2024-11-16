# HardwareVisualizer

<p align="left">
  <img alt="GitHub Release" src="https://img.shields.io/github/v/release/shm11C3/HardwareVisualizer?&display_name=release">
  <img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/shm11C3/HardwareVisualizer/publish.yaml">
  <img alt="Windows Support Only" src="https://img.shields.io/badge/platform-Windows-blue?logo=windows">
  <img alt="GitHub Downloads (all assets, all releases)" src="https://img.shields.io/github/downloads/shm11C3/HardwareVisualizer/total">
</p>

![image](https://github.com/user-attachments/assets/c474a132-5768-4046-9703-766e74ee3e66)

## Supported OS
| OS                       | Status |
| ----------------------------- | ------ |
| Windows          | ✅     |
| MacOS          |  ⏳    |
| linux |   ⏳   |


## Features

| Feature                       | Status |
| ----------------------------- | ------ |
| CPU Usage Monitoring          | ✅     |
| RAM Usage Monitoring          | ✅     |
| GPU Usage Monitoring (Nvidia) | ✅     |
| GPU Usage Monitoring (AMD)    | ✅     |
| GPU Usage Monitoring (Intel)  | ✅     |
| Temperature Monitoring        | ⏳     |
| Fan Monitoring                | ⏳     |
| Customizable Chart Themes     | ✅     |
| Customizable Dashboard        | ⏳     |
| Custom Background Image  | ✅     |

### Dashboard

![image](https://github.com/user-attachments/assets/9a2bf54f-d6e5-4c20-b0e4-f249fd5b8433)

### Usage Graph

![image](https://github.com/user-attachments/assets/ef3e1630-e567-47a1-a437-f9a3981dd587)


### Custom Graph

![image](https://github.com/user-attachments/assets/814eff68-9190-4c39-a67d-a7458778ec95)

### Background Image

![image](https://github.com/user-attachments/assets/6ab09e8a-ebef-449a-b73f-07ae44626e20)


## Development

[![Linted with Biome](https://img.shields.io/badge/Linted_with-Biome-60a5fa?style=flat&logo=biome)](https://biomejs.dev)

### Requirements

- [Node.js 20+](https://nodejs.org/)
- [Rust](https://www.rust-lang.org/)

### Setup

1. Clone the repository:

   ```bash
   git clone https://github.com/shm11C3/HardwareVisualizer.git
   cd HardwareVisualizer
   ```

2. Install dependencies:

   ```bash
   npm ci
   ```

3. Launch in development mode:

   ```bash
   npm run tauri dev
   ```

4. Build the app for production:

   ```bash
   npm run tauri build
   ```
