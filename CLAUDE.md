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
│   │   ├── commands/     # Tauri command layer (UI interface)
│   │   ├── services/     # Application business logic layer
│   │   ├── platform/     # Platform abstraction layer
│   │   │   ├── traits.rs # Common interfaces
│   │   │   ├── factory.rs # Platform selection
│   │   │   ├── common/   # Shared utilities
│   │   │   ├── windows/  # Windows-specific implementations
│   │   │   ├── linux/    # Linux-specific implementations
│   │   │   └── macos/    # macOS-specific implementations
│   │   ├── structs/      # Data type definitions
│   │   ├── utils/        # Utility functions
│   │   └── _tests/       # Test modules
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

## Architecture Design

### Layered Architecture Pattern

The backend follows a strict layered architecture with unidirectional dependencies:

```
Commands → Services → Repositories → Platform (via Factory) → OS APIs
```

#### Layer Responsibilities

1. **Commands Layer** (`src/commands/`)
   - Tauri command handlers (UI interface)
   - Input validation and output formatting
   - Calls services layer for business logic

2. **Services Layer** (`src/services/`)
   - Application business logic
   - Hardware data aggregation and processing
   - Settings management and system information

3. **Repositories Layer** (`src/repositories/`)
   - Data access abstraction layer
   - Platform-agnostic data retrieval interfaces
   - Handles platform-specific implementation details
   - Error handling and data format standardization

4. **Platform Layer** (`src/platform/`)
   - OS-specific hardware access implementations
   - Factory pattern for platform instance creation
   - Clean abstraction for cross-platform compatibility

#### Design Patterns Used

- **Repository Pattern**: Abstracts data access and platform-specific logic
- **Strategy Pattern**: Unified interfaces for platform services
- **Adapter Pattern**: OS-specific implementations adapting to common interfaces  
- **Factory Pattern**: Runtime platform detection and service creation

#### Repository Pattern Implementation

```rust
// Repository abstracts platform complexity
pub trait MemoryRepository: Send + Sync {
    async fn get_memory_info(&self) -> Result<MemoryInfo, String>;
    async fn get_memory_info_detail(&self) -> Result<MemoryInfo, String>;
}

// Platform-agnostic repository implementation
pub struct MemoryRepositoryImpl {
    platform: PlatformInstance,
}

impl MemoryRepository for MemoryRepositoryImpl {
    async fn get_memory_info(&self) -> Result<MemoryInfo, String> {
        self.platform.get_memory_info().await
    }
}
```

#### Platform Abstraction

```rust
// Platform traits define hardware access contracts
pub trait MemoryPlatform: Send + Sync {
    fn get_memory_info(&self) -> Pin<Box<dyn Future<Output = Result<MemoryInfo, String>> + Send + '_>>;
    fn get_memory_info_detail(&self) -> Pin<Box<dyn Future<Output = Result<MemoryInfo, String>> + Send + '_>>;
    fn platform_name(&self) -> &'static str;
}

// Factory creates appropriate platform instances
impl PlatformFactory {
    pub fn create() -> Result<PlatformInstance, PlatformError> {
        let platform_type = PlatformType::detect();
        Self::create_for_platform(platform_type)
    }
}

// Platform-specific implementations
impl MemoryPlatform for WindowsPlatform { /* Windows implementation */ }
impl MemoryPlatform for LinuxPlatform { /* Linux implementation */ }
impl MemoryPlatform for MacOSPlatform { /* macOS implementation */ }
```

### Dependency Rules

- **Single Direction**: Upper layers can only depend on lower layers
- **No Cross-References**: Services cannot reference commands, repositories cannot reference services
- **Interface Segregation**: Platform interfaces are kept minimal and focused
- **Factory Responsibility**: Factory focuses solely on instance creation, not business logic
- **Repository Responsibility**: Repository handles platform complexity and data access abstraction

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
