# Elevated Startup Mode

Some Windows sensor providers, including PawnIO-backed CPU package temperature collection, require the process that opens the provider to have administrator privileges. We decided to add Elevated Startup Mode as an App-owned preference that restarts the whole Tauri process with Windows elevation, instead of introducing a separate elevated Core helper or Windows service in this slice. `hardviz-core` is linked into the app process, so elevating only Core is not possible without adding a new process boundary, IPC contract, installer work, and service/helper lifecycle.
