# perf-monitor

A standalone CLI tool that launches the HardwareVisualizer binary, measures CPU and memory usage via [sysinfo](https://crates.io/crates/sysinfo), and asserts that the values stay within configurable thresholds.

## Quick Start

```bash
# Build (from repository root)
cargo build --release --manifest-path perf-monitor/Cargo.toml

# Run against a built binary
cargo perf-test -- --binary src-tauri/target/release/hardware-visualizer.exe
```

## CLI Options

| Option                 | Default                | Description                        |
| ---------------------- | ---------------------- | ---------------------------------- |
| `--binary <PATH>`      | _(required)_           | Path to the target binary          |
| `--config <PATH>`      | `perf-thresholds.toml` | Path to the threshold config file  |
| `--warmup <SECONDS>`   | From config (10)       | Warmup duration before measurement |
| `--duration <SECONDS>` | From config (30)       | Measurement duration               |
| `--output <FORMAT>`    | `text`                 | Output format: `text` or `json`    |

### Examples

```bash
# Short local test with text output
cargo perf-test -- --binary path/to/hardware-visualizer --warmup 5 --duration 10

# JSON output for CI
cargo perf-test -- --binary path/to/hardware-visualizer --output json > results.json
```

When `--output json` is used, machine-readable JSON is written to stdout and the normal text report is written to stderr. This keeps redirected JSON artifacts valid while still showing the full performance result in CI logs.

## Configuration

Thresholds are defined in `perf-thresholds.toml` at the repository root.

```toml
[thresholds]
max_avg_cpu_percent = 5.0      # Average CPU (normalized 0-100%)
max_p95_cpu_percent = 10.0     # 95th percentile CPU
max_avg_app_memory_mb = 100.0  # Average launched app process RSS in MB
max_p95_app_memory_mb = 100.0  # 95th percentile launched app process RSS in MB
max_app_memory_growth_mb = 50.0 # App process memory growth over measurement
max_avg_memory_mb = 450.0      # Average process-tree RSS in MB, including WebView
max_p95_memory_mb = 500.0      # 95th percentile process-tree RSS in MB, including WebView
max_memory_growth_mb = 50.0    # Process-tree memory growth over measurement

[timing]
warmup_seconds = 10            # Skip initial startup spike
measurement_seconds = 30       # Measurement window
sample_interval_ms = 1000      # Sampling frequency
```

### Platform Overrides

Per-platform threshold overrides can be specified under `[platforms.<os>]`:

```toml
[platforms.windows]
max_avg_app_memory_mb = 80.0
max_p95_app_memory_mb = 100.0
max_avg_memory_mb = 600.0
max_p95_memory_mb = 600.0

[platforms.linux]
max_avg_app_memory_mb = 150.0
max_p95_app_memory_mb = 150.0
max_avg_memory_mb = 550.0
max_p95_memory_mb = 550.0

[platforms.macos]
max_avg_app_memory_mb = 150.0
max_p95_app_memory_mb = 150.0
max_avg_memory_mb = 600.0
max_p95_memory_mb = 650.0
```

Only the fields you specify are overridden; others inherit from `[thresholds]`.

## How It Works

1. **Launch** - Spawns the target binary as a subprocess
2. **Warmup** - Waits for the app to stabilize (skips initialization spike)
3. **Measure** - Samples CPU and process-tree RSS at `sample_interval_ms` intervals using `sysinfo`
4. **Terminate** - Kills the process (RAII guard ensures cleanup on errors)
5. **Report** - Computes statistics (avg, max, P95, memory growth) and checks against thresholds

Memory samples track both the launched app process RSS and the total process-tree RSS that includes associated helper processes. Helpers are discovered by walking the `parent()` chain from the launched PID. Windows and macOS also include WebView helper processes that were created after launch and expose the target app identity in process metadata, because WebView helpers are not always reliably parented under the app PID.

### Metrics

| Metric           | Description                                                              |
| ---------------- | ------------------------------------------------------------------------ |
| CPU Avg / P95    | Launched process CPU usage normalized by logical CPU count (0-100%)      |
| App Memory Avg / P95 | Launched app process Resident Set Size (RSS) in MB                   |
| Total Memory Avg / P95 | Total process-tree RSS in MB, including WebView                  |
| Memory Growth    | Last RSS sample minus first sample for each memory budget                 |
| Memory Breakdown | Parent, WebView, and other helper RSS breakdown in text and JSON reports |

### Exit Code

- `0` - All thresholds passed
- `1` - One or more thresholds exceeded, or an error occurred

### Troubleshooting

If the monitor fails with `Process exited during warmup (exit status: exit code: 0)`, check for an already-running HardwareVisualizer instance. The app uses Tauri's single-instance plugin, so a second launch can hand off to the existing instance and exit before sampling begins.

```powershell
Get-Process hardware-visualizer -ErrorAction SilentlyContinue
Stop-Process -Name hardware-visualizer
```

## CI Integration

The GitHub Actions workflow (`.github/workflows/perf-test.yml`) runs daily at 03:00 JST:

- Builds the app and perf-monitor on the active performance matrix
- Currently runs the performance gate on Windows only
- Uploads JSON results as artifacts
- Creates or updates a GitHub Issue when any threshold is exceeded
- Labels performance test issues with `type:performance`, `status:regression`,
  and `source:performance-test`

## Project Structure

```
perf-monitor/
  Cargo.toml          # Crate manifest
  src/
    main.rs            # CLI entry point (clap)
    config.rs          # TOML config loader with platform overrides and validation
    monitor.rs         # Process launch, sysinfo sampling, statistics
    report.rs          # Text and JSON report formatting
```
