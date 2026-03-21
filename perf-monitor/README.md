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

## Configuration

Thresholds are defined in `perf-thresholds.toml` at the repository root.

```toml
[thresholds]
max_avg_cpu_percent = 5.0      # Average CPU (normalized 0-100%)
max_p95_cpu_percent = 10.0     # 95th percentile CPU
max_avg_memory_mb = 100.0      # Average RSS in MB
max_p95_memory_mb = 100.0      # 95th percentile RSS in MB
max_memory_growth_mb = 50.0    # Memory growth over measurement (leak detection)

[timing]
warmup_seconds = 10            # Skip initial startup spike
measurement_seconds = 30       # Measurement window
sample_interval_ms = 1000      # Sampling frequency
```

### Platform Overrides

Per-platform threshold overrides can be specified under `[platforms.<os>]`:

```toml
[platforms.windows]
max_avg_memory_mb = 70.0

[platforms.linux]
max_avg_memory_mb = 70.0

[platforms.macos]
max_avg_memory_mb = 70.0
```

Only the fields you specify are overridden; others inherit from `[thresholds]`.

## How It Works

1. **Launch** — Spawns the target binary as a subprocess
2. **Warmup** — Waits for the app to stabilize (skips initialization spike)
3. **Measure** — Samples CPU and memory at `sample_interval_ms` intervals using `sysinfo`
4. **Terminate** — Kills the process (RAII guard ensures cleanup on errors)
5. **Report** — Computes statistics (avg, max, P95, memory growth) and checks against thresholds

### Metrics

| Metric           | Description                                                |
| ---------------- | ---------------------------------------------------------- |
| CPU Avg / P95    | Process CPU usage normalized by logical CPU count (0-100%) |
| Memory Avg / P95 | Resident Set Size (RSS) in MB                              |
| Memory Growth    | Last sample minus first sample (detects obvious leaks)     |

### Exit Code

- `0` — All thresholds passed
- `1` — One or more thresholds exceeded, or an error occurred

## CI Integration

The GitHub Actions workflow (`.github/workflows/perf-test.yml`) runs daily at 03:00 JST:

- Builds the app and perf-monitor on Windows, Linux, and macOS
- Runs performance tests on each platform (Linux uses `xvfb-run` for virtual display)
- Uploads JSON results as artifacts
- Creates a GitHub Issue labeled `performance-regression` if any threshold is exceeded

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
