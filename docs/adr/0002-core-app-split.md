# Core / App Split

Status: accepted

After the platform boundary existed, we split the backend in [#1402](https://github.com/shm11C3/HardwareVisualizer/issues/1402) into `hardviz-core` and the Tauri app crate. Core owns Tauri-independent sensor collection, realtime history, platform access, persistence workers, and Core-consumed settings; the App crate owns Tauri commands, event adapters, lifecycle, plugins, UI-owned settings, and generated frontend bindings.

This preserves the platform-layer design while making sensor collection and persistence testable without a Tauri runtime, and it keeps `MetricsSnapshot` fan-out separate from Tauri event emission.

We also kept Core-owned and App-owned preferences in the same top-level `settings.json` instead of splitting them into separate files. Existing users already had one settings file, and many settings are presented as one Settings screen even though their consumers differ. Each side therefore reads and writes only its owned keys while preserving unknown keys, which keeps migration small without letting Core depend on Tauri or forcing App code to own Core behavior.
