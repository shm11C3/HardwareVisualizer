# ADL Diagnostic Tool

Standalone diagnostic binary that loads AMD's `atiadlxx.dll` and walks through
every step of the ADL pipeline.  
Use it to investigate **why a specific machine cannot retrieve GPU values from
the AMD Display Library (ADL)**.

## Prerequisites

- Windows machine with AMD GPU (discrete or APU/iGPU)
- Rust toolchain (see `rust-toolchain.toml`)

## Running

From the repository root:

```bash
cd src-tauri
cargo run --example adl_diagnostic
```

> No additional flags or elevated privileges are required, though results may
> differ when run as administrator.

## What it checks

The tool runs the following steps in order and prints detailed results for each:

| Step | Description                                                                                                                       |
| ---- | --------------------------------------------------------------------------------------------------------------------------------- |
| 1    | Load `atiadlxx.dll` (shipped with AMD GPU drivers)                                                                                |
| 2    | Resolve **mandatory** ADL2 symbols (`Main_Control_Create`, `Adapter_NumberOfAdapters_Get`, etc.)                                  |
| 3    | Resolve **optional** symbols (Overdrive 5 / N / PMLog APIs)                                                                       |
| 4    | Create ADL context via `ADL2_Main_Control_Create`                                                                                 |
| 5    | Enumerate adapters (`ADL2_Adapter_NumberOfAdapters_Get` + `AdapterInfo_Get`)                                                      |
| 6    | For each adapter: print name, vendor ID, bus/device/function, present/exist flags, and active status                              |
| 7    | Query **Overdrive Capabilities** (`ADL2_Overdrive_Caps` → OD5 fallback)                                                           |
| 8    | Query **OD5 Temperature** (`ADL2_Overdrive5_Temperature_Get`)                                                                     |
| 9    | Query **OD5 Activity / Usage** (`ADL2_Overdrive5_CurrentActivity_Get`)                                                            |
| 10   | Query **ODN Temperatures** (Edge, Memory, VR VDDC, VR MVDD, Liquid, PLX, Hot Spot)                                                |
| 11   | Query **PMLog / OD8 Sensors** — prints all known temperature & usage sensors, plus a full dump of every supported PMLog sensor ID |

## Reading the output

### Healthy discrete GPU (e.g. RX 6800)

```
Vendor ID: 0x1002          ← standard AMD PCI Vendor ID
Overdrive level: 8         ← OD8 / PMLog path used
OD5 Temperature: OK
PMLog sensors: many supported
```

### APU / iGPU (e.g. Ryzen 5 5600U — Radeon Graphics)

```
Vendor ID: 0x03EA          ← non-standard; fixed in adl_provider.rs
Overdrive level: 5         ← OD Caps returns rc=-8, falls back to OD5
OD5 Temperature: rc=-100   ← OD5 temp API not supported on this APU
OD5 Activity: OK           ← usage works via OD5
PMLog Temp Edge: value=70  ← temperature available through PMLog
PMLog GPU Usage: value=28  ← usage also available through PMLog
```

### No AMD GPU

```
[Step 1] Loading atiadlxx.dll...
  FAIL: Could not load atiadlxx.dll: ...
```

## Known vendor ID values

| `vendor_id` | Meaning                                                                 |
| ----------- | ----------------------------------------------------------------------- |
| `0x1002`    | Standard AMD/ATI PCI Vendor ID (discrete GPUs)                          |
| `0x03EA`    | Reported by some AMD APU/iGPU drivers (Renoir, Lucienne, Barcelo, etc.) |

Both are accepted by `adl_provider.rs` via the `AMD_VENDOR_IDS` constant.
An adapter-name fallback (`"AMD"` / `"Radeon"`) is also in place for any
future vendor ID variants.

## Relationship to production code

The diagnostic tool mirrors the logic in
`src-tauri/src/infrastructure/providers/windows/adl_provider.rs` but with
verbose `println!` output at every decision point.  
If you fix a bug in the provider, consider updating the diagnostic tool to
match (and vice-versa).

## Source

`src-tauri/examples/adl_diagnostic.rs`
