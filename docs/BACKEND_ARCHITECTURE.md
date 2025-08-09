# Backend Architecture

## Layered Architecture Overview

The HardwareVisualizer backend follows a strict layered architecture pattern with clear separation of concerns and unidirectional dependencies:

```
Commands → Services → Repositories → Platform (via Factory) → OS APIs
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
│   ├── hardware.rs
│   ├── settings.rs
│   ├── background_image.rs
│   └── language.rs
├── repositories/         # Data access abstraction layer
│   ├── mod.rs
│   └── memory_repository.rs # Platform-agnostic data access
├── platform/             # Platform abstraction layer
│   ├── traits.rs         # Platform interfaces
│   ├── factory.rs        # Platform instance creation
│   ├── mod.rs
│   ├── windows/          # Windows-specific implementations
│   │   └── mod.rs
│   ├── linux/            # Linux-specific implementations
│   │   └── mod.rs
│   └── macos/            # macOS-specific implementations
│       └── mod.rs
├── infrastructure/       # External resources layer
│   ├── database/         # SQLite wrapper
│   ├── logging/          # Log output
│   └── config/           # Configuration management
├── structs/              # Shared data structures
│   └── hardware.rs
├── types/                # Type definitions
├── utils/                # Utility functions
└── _tests/               # Test modules
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
- **Example**: `get_memory_info` command validates input and calls `MemoryService`

### Services Layer (`src/services/`)

- **Purpose**: Business logic and application orchestration
- **Responsibilities**:
  - Hardware data aggregation and processing
  - Business rules enforcement
  - Cross-domain logic coordination
  - High-level error handling
- **Dependencies**: Repositories layer
- **Key Services**:
  - `MemoryService`: Memory data business logic
  - `PlatformService`: Platform service coordination

### Repositories Layer (`src/repositories/`)

- **Purpose**: Data access abstraction and platform complexity management
- **Responsibilities**:
  - Platform-agnostic data retrieval interfaces
  - Platform-specific implementation details handling
  - Data format standardization
  - Low-level error handling and retry logic
- **Dependencies**: Platform layer
- **Key Repositories**:
  - `MemoryRepository`: Abstracts memory data access across platforms

### Platform Layer (`src/platform/`)

- **Purpose**: OS-specific hardware access implementations
- **Responsibilities**:
  - Cross-platform hardware access abstraction
  - OS-specific API interactions
  - Platform detection and selection
  - Hardware capability differences handling
- **Dependencies**: OS APIs only
- **Components**:
  - `traits.rs`: Platform interface definitions
  - `factory.rs`: Platform instance creation
  - Platform-specific modules: Windows, Linux, macOS implementations

## Design Patterns

### Repository Pattern

- **Purpose**: Abstracts data access complexity from business logic
- **Benefits**:
  - Platform-specific logic encapsulation
  - Testability through dependency injection
  - Clean separation between business logic and data access

```rust
pub trait MemoryRepository: Send + Sync {
    async fn get_memory_info(&self) -> Result<MemoryInfo, String>;
    async fn get_memory_info_detail(&self) -> Result<MemoryInfo, String>;
}
```

### Factory Pattern

- **Purpose**: Creates appropriate platform instances at runtime
- **Benefits**:
  - Platform detection automation
  - Instance creation centralization
  - Configuration complexity hiding

```rust
impl PlatformFactory {
    pub fn create() -> Result<PlatformInstance, PlatformError> {
        let platform_type = PlatformType::detect();
        Self::create_for_platform(platform_type)
    }
}
```

### Strategy Pattern

- **Purpose**: Unified interfaces for platform-specific implementations
- **Benefits**:
  - Runtime algorithm switching
  - Platform differences encapsulation
  - Code duplication reduction

```rust
pub trait MemoryPlatform: Send + Sync {
    fn get_memory_info(&self) -> Pin<Box<dyn Future<Output = Result<MemoryInfo, String>> + Send + '_>>;
    fn platform_name(&self) -> &'static str;
}
```

## Dependency Rules

1. **Unidirectional Dependencies**: Higher layers can only depend on lower layers
2. **Interface Segregation**: Layers communicate through well-defined interfaces
3. **No Cross-References**: Peer layers cannot directly reference each other
4. **Abstraction Dependency**: Depend on abstractions, not concretions

## Error Handling Strategy

- **Commands Layer**: User-friendly error messages and HTTP-like status codes
- **Services Layer**: Business error validation and domain-specific errors
- **Repositories Layer**: Data access error handling and retry logic
- **Platform Layer**: OS-specific error translation to common error types

## Testing Strategy

- **Unit Tests**: Each layer tested in isolation with mocked dependencies
- **Integration Tests**: Cross-layer interaction testing
- **Platform Tests**: Platform-specific implementation validation
- **Test Location**: `src/_tests/` directory with layer-specific test modules

## Benefits of This Architecture

1. **Maintainability**: Clear separation of concerns and single responsibility
2. **Testability**: Easy mocking and dependency injection
3. **Scalability**: Easy addition of new platforms and features
4. **Flexibility**: Runtime platform switching and configuration
5. **Reusability**: Platform-agnostic business logic and repository abstractions
