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
Commands → Services → Platform (via Factory) → OS APIs
```

#### Layer Responsibilities

1. **Commands Layer** (`src/commands/`)
   - Tauri command handlers (UI interface)
   - Input validation and output formatting
   - Delegates to services layer for business logic

2. **Services Layer** (`src/services/`)
   - Application business logic and hardware data processing
   - Platform abstraction through Factory pattern
   - Hardware monitoring state management
   - Data aggregation and formatting

3. **Platform Layer** (`src/platform/`)
   - OS-specific hardware access implementations
   - Trait-based platform abstraction (`MemoryPlatform`, `GpuPlatform`, `NetworkPlatform`)
   - Factory pattern for automatic platform detection
   - Direct OS API interactions

#### Design Patterns Used

- **Strategy Pattern**: Platform-specific implementations via trait objects
- **Factory Pattern**: Automatic platform detection and instance creation
- **Adapter Pattern**: OS-specific implementations adapting to common trait interfaces
- **Service Layer Pattern**: Business logic abstraction from UI and platform concerns

#### Service Layer Implementation

```rust
// Services layer uses Factory to access platform functionality
use crate::platform::factory::PlatformFactory;

pub async fn fetch_memory_detail() -> Result<MemoryInfo, String> {
  let platform = PlatformFactory::create()
    .map_err(|e| format!("Failed to create platform: {e}"))?;
  platform.get_memory_info_detail().await
}

pub fn fetch_network_info() -> Result<Vec<NetworkInfo>, BackendError> {
  let platform = PlatformFactory::create()
    .map_err(|_| BackendError::UnexpectedError)?;
  platform.get_network_info()
    .map_err(|_| BackendError::UnexpectedError)
}
```

#### Platform Abstraction

```rust
// Platform traits define hardware access contracts
pub trait MemoryPlatform: Send + Sync {
  fn get_memory_info(&self) -> Pin<Box<dyn Future<Output = Result<MemoryInfo, String>> + Send + '_>>;
  fn get_memory_info_detail(&self) -> Pin<Box<dyn Future<Output = Result<MemoryInfo, String>> + Send + '_>>;
}

pub trait GpuPlatform: Send + Sync {
  fn get_gpu_usage(&self) -> Pin<Box<dyn Future<Output = Result<f32, String>> + Send + '_>>;
  fn get_gpu_temperature(&self, unit: TemperatureUnit) -> Pin<Box<dyn Future<Output = Result<Vec<NameValue>, String>> + Send + '_>>;
  fn get_gpu_info(&self) -> Pin<Box<dyn Future<Output = Result<Vec<GraphicInfo>, String>> + Send + '_>>;
}

pub trait NetworkPlatform: Send + Sync {
  fn get_network_info(&self) -> Result<Vec<NetworkInfo>, BackendError>;
}

// Unified Platform trait combining all hardware access
pub trait Platform: MemoryPlatform + GpuPlatform + NetworkPlatform {}

// Factory for automatic platform detection
impl PlatformFactory {
  pub fn create() -> Result<Box<dyn Platform>, PlatformError> {
    #[cfg(target_os = "windows")]
    {
      let platform = WindowsPlatform::new()
        .map_err(|e| PlatformError::InitializationFailed(e.to_string()))?;
      Ok(Box::new(platform))
    }
    #[cfg(target_os = "linux")]
    {
      let platform = LinuxPlatform::new()
        .map_err(|e| PlatformError::InitializationFailed(e.to_string()))?;
      Ok(Box::new(platform))
    }
    #[cfg(target_os = "macos")]
    {
      let platform = MacOSPlatform::new()
        .map_err(|e| PlatformError::InitializationFailed(e.to_string()))?;
      Ok(Box::new(platform))
    }
  }
}
```

### Dependency Rules

- **Unidirectional Flow**: Commands → Services → Platform, no reverse dependencies
- **Factory Encapsulation**: Services use Factory for platform access, never direct platform instantiation
- **Trait Abstraction**: Platform traits provide clean interfaces hiding OS-specific complexity
- **Conditional Compilation**: Platform selection handled at compile time via `#[cfg(target_os)]`
- **Service Isolation**: Services handle business logic, platforms handle hardware access only

### Current Architecture Benefits

- **Simplified Design**: Removed intermediate repository layer for cleaner data flow
- **Direct Platform Access**: Services directly use Factory for platform functionality
- **Better Performance**: Fewer abstraction layers reduce overhead
- **Clear Separation**: Business logic in services, hardware access in platform layer
- **Automatic Platform Detection**: Factory handles OS detection transparently

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
