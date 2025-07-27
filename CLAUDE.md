# HardwareVisualizer Project Guidelines

## Project Overview

**HardwareVisualizer** is a cross-platform hardware monitoring application built with Tauri (Rust + React/TypeScript). It provides real-time hardware performance monitoring with customizable dashboards, detailed usage graphs, and historical data insights.

## Tech Stack

- **Frontend**: React 19, TypeScript, Tailwind CSS 4.x, Vite 7
- **Backend**: Rust (Tauri 2.x), SQLite
- **UI Components**: Radix UI, Lucide Icons, Recharts
- **State Management**: Jotai
- **Testing**: Vitest, Testing Library
- **Linting/Formatting**: Biome
- **Build Tool**: Tauri CLI
- **CI/CD**: GitHub Actions

## Project Structure

```
├── src/                    # React frontend
│   ├── features/          # Feature-based modules
│   │   ├── hardware/      # Hardware monitoring logic
│   │   ├── settings/      # Application settings
│   │   └── menu/          # Navigation menu
│   ├── components/        # Reusable UI components
│   ├── hooks/            # Custom React hooks
│   └── lib/              # Utility functions
├── src-tauri/            # Rust backend
│   ├── src/              # Rust source code
│   └── capabilities/     # Tauri permissions
|── .github/               # GitHub Actions workflows
│   |── scripts/         # Automation scripts
│   ├── workflows/       # CI/CD pipelines
│   ├── issue-templates/ # Issue templates
│   └── dependabot.yml   # Dependabot configuration
└── claude-reports/      # AI analysis reports
```

## Development Commands

| Command                 | Description                                  |
| ----------------------- | -------------------------------------------- |
| `npm run dev`           | Start development server with React DevTools |
| `npm run tauri dev`     | Launch Tauri development mode                |
| `npm run build`         | Build for production                         |
| `npm run lint`          | Run Biome linter and formatter               |
| `npm run format`        | Format code with Biome                       |
| `npm run test:unit`     | Run frontend unit tests                      |
| `npm run test:tauri`    | Run Rust tests                               |
| `npm run test:unit-cov` | Run tests with coverage                      |

## Code Quality Standards

### Linting & Formatting

- **Biome** for JavaScript/TypeScript linting and formatting
- **rustfmt** for Rust code formatting
- Run `npm run lint` before committing

### Testing Strategy

- **Unit Tests**: Vitest for frontend, Cargo test for Rust
- **Coverage**: Aim for comprehensive test coverage
- **Test Location**: `/test/unit/` for frontend, `/src-tauri/src/_tests/` for Rust

### TypeScript Configuration

- Strict TypeScript enabled
- Path aliases configured for clean imports
- Custom types in `/src/types/`

## Platform Support

| Platform | Status       | Requirements            |
| -------- | ------------ | ----------------------- |
| Windows  | ✅ Supported | WebView2                |
| Linux    | ✅ Supported | webkit2gtk, sudo access |
| macOS    | 🔒 Planned   | No ETA                  |

## Key Features

- Real-time CPU, RAM, GPU, Storage, Network monitoring
- Customizable dashboard with drag-and-drop
- Historical data insights (up to 30 days)
- Custom background images
- Multi-language support (EN/JA)
- Auto-updater functionality

## Hardware Data Collection

- **Permissions**: Requires elevated privileges on Linux (`sudo`)
- **Database**: SQLite for historical data storage
- **Real-time**: WebSocket-like updates via Tauri events
- **GPU Support**: NVIDIA (full), AMD/Intel (limited)

## Build & Distribution

- **Tauri Bundle**: Cross-platform native executables
- **GitHub Actions**: Automated CI/CD pipeline
- **Release**: GitHub Releases with auto-updater
- **Dependencies**: Listed in Linux .deb package requirements

## Development Notes

- **Memory Management**: Efficient data handling for continuous monitoring
- **Performance**: Optimized rendering for real-time updates
- **Error Handling**: Comprehensive error boundaries and logging
- **Internationalization**: i18next for multi-language support

## Security Considerations

- **Tauri CSP**: Configured for secure WebView
- **Permissions**: Minimal required capabilities
- **Data Privacy**: Local-only data storage
- **Elevated Access**: Required for hardware information access
