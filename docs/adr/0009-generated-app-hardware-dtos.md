# Generated App Hardware DTOs

Status: accepted

The Tauri app generates its pure hardware wire DTOs from `core/src/models/hardware.rs` during the `src-tauri` build.

Core remains the owner of platform-facing hardware data models and must not depend on Tauri, `specta`, or tauri-specta. The App crate owns the wire-facing types, TypeScript binding surface, and any presentation-specific serialization policy. To keep those boundaries without repeating the same struct fields twice, `src-tauri/build.rs` parses the Core hardware model file and writes the selected App DTOs and their `From<core::...>` conversions to `OUT_DIR/hardware_models.rs`.

This keeps ADR 0002 intact: Core still has no Tauri dependency, and the App boundary still converts between Core-owned data and wire-owned DTOs. The generated file is not committed.

Generation is intentionally allowlisted. Event-specific types such as `HardwareMonitorUpdate`, `GpuMonitorData`, motherboard display values, and the App-side `FanSpeedStatus` remain hand-written because they are not pure mirrors of `core/src/models/hardware.rs`.

Field-specific wire policy stays in the generator. Today that includes:

- `ProcessInfo.cpu_usage` and `ProcessInfo.memory_usage`, which serialize as strings for the existing frontend contract.
- Core-only field types such as `DiskKind` and `SizeUnit`, which are rewritten to the App wire enums and converted via existing `From` impls.

Adding a pure field-mapped hardware value now starts in the Core model. If the value is emitted through an existing App DTO, the generated DTO and conversion update with the Core field. If the value changes an event projection, the event builder is still the second place that must be updated before regenerating TypeScript bindings.
