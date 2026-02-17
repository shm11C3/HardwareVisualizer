# Backend Architecture

## Layered Architecture Overview

The HardwareVisualizer backend follows a strict layered architecture pattern with clear separation of concerns and unidirectional dependencies:

```
Commands → Services → Platform (via Factory) → OS APIs
```

## Directory Structure

```
src/
├── commands/              # Application layer (Tauri interface)
│   ├── background_image.rs
│   ├── hardware.rs
│   ├── mod.rs
│   ├── settings.rs
│   ├── system.rs
│   └── ui.rs
├── services/             # Business logic layer
│   ├── cpu_service.rs
│   ├── gpu_service.rs
│   ├── hardware_service.rs
│   ├── memory_service.rs
│   ├── network_service.rs
│   ├── process_service.rs
│   ├── monitoring_service.rs
│   ├── setting_service.rs
│   └── language.rs
├── platform/             # Platform abstraction layer
│   ├── traits.rs         # Platform interfaces
│   ├── factory.rs        # Platform instance creation
│   ├── mod.rs
│   ├── windows/          # Windows-specific implementations
│   │   ├── gpu.rs
│   │   ├── memory.rs
│   │   ├── network.rs
│   │   └── mod.rs
│   ├── linux/            # Linux-specific implementations
│   │   ├── gpu.rs
│   │   ├── memory.rs
│   │   ├── network.rs
│   │   ├── cache.rs
│   │   └── mod.rs
│   └── macos/            # macOS-specific implementations
│       └── mod.rs
├── infrastructure/       # External resources layer
│   ├── directx.rs        # DirectX interactions
│   ├── dmidecode.rs      # Hardware detection
│   ├── drm_sys.rs        # Linux DRM
│   ├── kernel.rs         # Kernel interfaces
│   ├── lspci.rs          # PCI device enumeration
│   ├── net_sys.rs        # Network interfaces
│   ├── nvapi.rs          # NVIDIA API
│   ├── procfs.rs         # Linux proc filesystem
│   ├── sysinfo_provider.rs
│   └── wmi_provider.rs   # Windows WMI
├── database/             # SQLite data layer
│   ├── db.rs
│   ├── gpu_archive.rs
│   ├── hardware_archive.rs
│   ├── migration.rs
│   ├── mod.rs
│   └── process_stats.rs
├── structs/              # Shared data structures
│   ├── hardware.rs
│   ├── hardware_archive.rs
│   ├── mod.rs
│   └── settings.rs
├── enums/                # Type definitions
│   ├── error.rs
│   ├── hardware.rs
│   ├── mod.rs
│   └── settings.rs
├── utils/                # Utility functions
│   ├── color.rs
│   ├── file.rs
│   ├── formatter.rs
│   ├── ip.rs
│   ├── logger.rs
│   ├── mod.rs
│   ├── rounding.rs
│   └── tauri.rs
├── backgrounds/          # Background services
│   ├── hardware_archive.rs
│   ├── mod.rs
│   ├── system_monitor.rs
│   └── updater.rs
└── _tests/               # Test modules
    ├── commands/
    ├── services/
    ├── utils/
    └── mod.rs
```

## Layer Responsibilities

### Commands Layer (`src/commands/`)

- **Purpose**: Tauri command handlers providing UI interface
- **Responsibilities**:
  - Input validation and sanitization
  - Output formatting for frontend consumption
  - Error handling and user-friendly error messages
  - Delegating business logic to services layer
- **Dependencies**: Services layer only
- **Example**: `get_memory_info` command delegates to `memory_service::fetch_memory_detail()`

### Services Layer (`src/services/`)

- **Purpose**: Business logic and application orchestration
- **Responsibilities**:
  - Hardware data aggregation and processing
  - Platform abstraction through Factory pattern
  - Hardware monitoring state management
  - Business rules enforcement and data formatting
- **Dependencies**: Platform layer (via Factory)
- **Key Services**:
  - `memory_service`: Memory data processing and platform access
  - `gpu_service`: GPU monitoring and thermal management
  - `network_service`: Network interface information
  - `hardware_service`: Overall hardware coordination

### Platform Layer (`src/platform/`)

- **Purpose**: OS-specific hardware access implementations
- **Responsibilities**:
  - Cross-platform hardware access abstraction
  - OS-specific API interactions
  - Platform detection and automatic selection
  - Hardware capability differences handling
- **Dependencies**: OS APIs and infrastructure layer
- **Components**:
  - `traits.rs`: Platform interface definitions (`MemoryPlatform`, `GpuPlatform`, `NetworkPlatform`)
  - `factory.rs`: Automatic platform instance creation
  - Platform-specific modules: Windows, Linux, macOS implementations

## Design Patterns

### Strategy Pattern

- **Purpose**: Unified interfaces for platform-specific implementations
- **Benefits**:
  - Runtime platform selection
  - Platform differences encapsulation
  - Clean trait-based abstraction

```rust
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
```

### Factory Pattern

- **Purpose**: Creates appropriate platform instances automatically
- **Benefits**:
  - Automatic platform detection via conditional compilation
  - Instance creation centralization
  - Configuration complexity hiding

```rust
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

### Service Layer Pattern

- **Purpose**: Business logic abstraction from UI and platform concerns
- **Benefits**:
  - Clean separation between commands and platform access
  - Centralized business logic
  - Easy testing and mocking

```rust
// Services use Factory for platform access
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

## Dependency Rules

1. **Unidirectional Flow**: Commands → Services → Platform, no reverse dependencies
2. **Factory Encapsulation**: Services use Factory for platform access, never direct platform instantiation
3. **Trait Abstraction**: Platform traits provide clean interfaces hiding OS-specific complexity
4. **Conditional Compilation**: Platform selection handled at compile time via `#[cfg(target_os)]`
5. **Service Isolation**: Services handle business logic, platforms handle hardware access only

## Error Handling Strategy

- **Commands Layer**: User-friendly error messages and proper Tauri error responses
- **Services Layer**: Business error validation and domain-specific errors
- **Platform Layer**: OS-specific error translation to common error types
- **Infrastructure Layer**: Low-level error handling and retry logic

## Testing Strategy

- **Unit Tests**: Each layer tested in isolation with mocked dependencies
- **Integration Tests**: Cross-layer interaction testing
- **Platform Tests**: Platform-specific implementation validation
- **Test Location**: `src/_tests/` directory with layer-specific test modules

## Architecture Benefits

1. **Simplified Design**: Removed intermediate repository layer for cleaner data flow
2. **Better Performance**: Fewer abstraction layers reduce overhead
3. **Clear Separation**: Business logic in services, hardware access in platform layer
4. **Automatic Platform Detection**: Factory handles OS detection transparently
5. **Maintainability**: Clear separation of concerns and single responsibility
6. **Testability**: Easy mocking and dependency injection through traits
7. **Scalability**: Easy addition of new platforms and hardware features
8. **Flexibility**: Runtime platform switching and configuration

## Performance Considerations

- **Async Operations**: Platform operations use async/await for non-blocking calls
- **Caching**: Linux platform implements caching for expensive operations
- **Memory Management**: Efficient data structures and minimal allocations
- **Error Recovery**: Graceful degradation when hardware access fails
