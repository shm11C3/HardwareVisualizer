# HardwareVisualizer

<p align="left">
  <img alt="GitHub Release" src="https://img.shields.io/github/v/release/shm11C3/HardwareVisualizer?&display_name=release">
  <img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/shm11C3/HardwareVisualizer/publish.yaml">
  <img alt="Windows Support Only" src="https://img.shields.io/badge/platform-Windows-blue?logo=windows">
  <img alt="GitHub Downloads (all assets, all releases)" src="https://img.shields.io/github/downloads/shm11C3/HardwareVisualizer/total">
</p>

<div align="center" style="padding-top: 6px;">
  <img alt="logo" src="https://github.com/user-attachments/assets/2067cc20-d886-4480-91d0-5c801186e2b2">
</div>

## Features

| Feature                       | Status |
| ----------------------------- | ------ |
| CPU Usage Monitoring          | ✅     |
| RAM Usage Monitoring          | ✅     |
| GPU Usage Monitoring (Nvidia) | ✅     |
| GPU Usage Monitoring (AMD)    | ⏳     |
| GPU Usage Monitoring (Intel)  | ⏳     |
| Temperature Monitoring        | ⏳     |
| Fan Monitoring                | ⏳     |
| Customizable Chart Themes     | ✅     |
| Customizable Dashboard        | ⏳     |
| CustomizableBackground Image  | ⏳     |

### Dashboard

![image](https://github.com/user-attachments/assets/9a2bf54f-d6e5-4c20-b0e4-f249fd5b8433)

### Usage Graph

![image](https://github.com/user-attachments/assets/b8fa7d67-a015-487f-aeb4-f43306d28f54)

### Custom Themes

![image](https://github.com/user-attachments/assets/acb5a432-1339-4b29-a81c-590e87bd8b77)

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

3. Run the app in development mode:

   ```bash
   npm run tauri dev
   ```

4. Build the app for production:

   ```bash
   npm run tauri build
   ```
