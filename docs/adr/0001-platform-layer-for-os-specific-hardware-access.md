# Platform Layer for OS-specific Hardware Access

Status: accepted

HardwareVisualizer supports Windows, Linux, and macOS, but each OS exposes hardware data through different APIs, permissions, vendor SDKs, and fallback paths. We decided in [#526](https://github.com/shm11C3/HardwareVisualizer/issues/526) to route hardware access through a platform layer that hides OS-specific differences behind shared hardware access contracts, so command and service code can ask for hardware facts without choosing a Windows, Linux, or macOS implementation directly.

This keeps OS-specific behavior replaceable inside the platform boundary and prevents cross-platform product behavior from being scattered through frontend-facing command handlers or higher-level services.
