# Windows Sensor Service

This crate supplies the Tauri-independent Windows service executable for
[ADR 0029](../docs/adr/0029-windows-sensor-service.md). Core owns providers and
authenticated local IPC; the app owns collection policy, settings and all
history. The service has no archive database, WebView or runtime downloads.

## Build and Packaging

The Windows Tauri build hook runs `build-bundle.ps1` before the frontend build.
Use PowerShell for this Windows-only packaging work: it shares the environment
and exit-code conventions of the existing signing and MSI tools. Node remains
the frontend toolchain; this native packaging step does not need a JS wrapper
or an additional Go toolchain. A shared Unix build step may use `sh` separately.

```powershell
./sensor-service/build-bundle.ps1 -BuildProfile dev
./sensor-service/check-msi.ps1 target/debug/bundle/msi/HardwareVisualizer_0.0.0_x64_en-US.msi
```

Release is the default profile; CI sets `HARDVIZ_SENSOR_SERVICE_PROFILE=ci`.
The hook resolves Cargo's actual target directory, passes the Tauri version
into the executable's version resource, stages the executable under the WiX
owner's ignored `generated/` directory and signs it when
`ENABLE_WINDOWS_SIGNING=true`. Both the MSI File and embedded directory
validator use those same bytes. The service is a standard MSI feature, not a
sidecar the app can accidentally execute from a writable installation.

`--validate-install-dir <directory>` performs read-only path/ACL validation
from the installer's embedded copy before `InstallFiles`. With no arguments,
the executable is started only by the Windows Service Control Manager.

Setup and repair use the registered MSI feature; no runtime IPC operation can
install, download, load a client-specified module or execute a command.

New and upgraded protected MSI installations include it without an opt-in or
prior-selection registry marker. Its feature follows the required application
parent and exists as a repair boundary. `check-msi.ps1` verifies the compiled
tables and costs fresh/full-UI, silent, upgrade-property and unprotected plans
without executing installation. PawnIO driver setup remains a separate choice.
