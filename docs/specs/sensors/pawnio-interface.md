# Spec: PawnIO driver, library, and module IOCTL interface

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Draft — not implementation-ready |
| Scope | Facts needed to integrate a Rust user-mode client with PawnIO: installation/detection, the PawnIOLib API, the module execution model, and the IOCTL contracts of the `IntelMSR`, `RyzenSMU`, and `LpcIO` modules. Excludes: writing new Pawn modules, driver internals. |
| Issue phase | Phase 1 (#1635) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | namazso, *PawnIO* repository, <https://github.com/namazso/PawnIO> (README / LICENSE) | Primary; license facts |
| S2 | namazso, *PawnIO.Modules* repository, <https://github.com/namazso/PawnIO.Modules> | Primary; module list, license |
| S3 | PawnIO.Modules wiki, "Using PawnIO Modules", <https://github.com/namazso/PawnIO.Modules/wiki/Using-PawnIO-Modules> | Primary; user-mode API |
| S4 | PawnIO.Modules wiki, "Getting started with PawnIO", <https://github.com/namazso/PawnIO.Modules/wiki/Getting-started-with-PawnIO> | Primary; toolchain, signing |
| S5 | Module sources `IntelMSR.p`, `RyzenSMU.p`, `LpcIO.p` in S2 (LGPL-2.1-or-later) | Upstream-published interface definitions of the API this project calls across the IOCTL boundary (public `ioctl_*` contracts, allow-lists, mutex names); the PawnIO project is the authoritative source for its own interfaces. Not used as a source for any hardware register fact. No code was copied. |

## Licensing facts

- The PawnIO driver is licensed **GPL-2.0 with an exception** that
  explicitly permits combining it with independent modules that
  communicate "through the device IO control interface", and with
  LGPL code. A user-mode client that only talks to the driver via
  IOCTLs is such an independent module. (S1)
- The modules in PawnIO.Modules are **LGPL-2.1-or-later**. Our client
  invokes them through the driver's IOCTL interface and ships none of
  their code, so MIT licensing of this repository's code is unaffected.
  Redistributing the compiled module blobs with the installer requires
  complying with LGPL-2.1 distribution terms (source offer /
  attribution in third-party notices). (S1, S2)
- PawnIO is the WinRing0 replacement adopted by LibreHardwareMonitor,
  FanControl, and OpenRGB after WinRing0 was added to Microsoft's
  vulnerable-driver blocklist. (Issue #1635 background; S1)

## Installation and detection

- PawnIO ships as a signed driver with an installer; installation
  requires administrator rights once. (S1, S4)
- The installed location of PawnIOLib is discovered via the registry
  value `InstallLocation` under
  `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO`,
  with `%ProgramFiles%\PawnIO` as the documented fallback. (S3)
- The production (signed) driver validates module signatures; loading
  unsigned modules requires the separate "unrestricted" build of the
  driver plus Windows test-signing mode. An end-user deployment
  therefore uses the signed driver with the signed module blobs
  released by the PawnIO project. (S4)
- Absence of PawnIO is a supported state: the client must detect the
  missing library/driver and report "unavailable" so the caller can
  fall back to the ACPI thermal-zone source (PR #1633).

## PawnIOLib user-mode API

Facts from S3. The normative reference for exact prototypes is
`PawnIOLib.h` shipped in the PawnIO installation directory
(`TODO(provenance)`: pin exact signatures from the installed header
during implementation).

- The library is loaded dynamically and functions are resolved by
  name (e.g. via `GetProcAddress`).
- Call sequence:
  1. `pawnio_open(&handle)` — obtain a handle to the driver.
  2. `pawnio_load(handle, blob, blob_size)` — load a compiled module
     blob (`.amx`) into that handle's context.
  3. `pawnio_execute(handle, "ioctl_<name>", in_buf, in_count,
     out_buf, out_count, &return_size)` — invoke a module function by
     its string name with input/output buffers.
- Module functions are addressed by **string name** following the
  `ioctl_*` convention. (S3)
- Buffers are arrays of **64-bit cells**: PawnIO only supports a
  64-bit cell size (modules are compiled with `-C64`). Sizes are given
  in cells, not bytes. (S3, S4)
- Functions return NTSTATUS-style status codes; module-level denials
  observed in the module sources use `STATUS_ACCESS_DENIED` /
  `STATUS_NOT_SUPPORTED`. (S5)
- One handle holds one loaded module; use one handle per module.
  (`TODO(provenance)`: confirm against `PawnIOLib.h` whether a handle
  may reload a different blob; also confirm the close/cleanup function
  name, expected to be `pawnio_close`.)

## Module IOCTL contracts

Interface facts extracted from the public surfaces of the modules
(S5). Cell layouts: `in[i]` / `out[i]` are 64-bit cells.

### `IntelMSR`

Target: Intel x86-64 CPUs only; other vendors get
`STATUS_NOT_SUPPORTED`.

| Function | Input cells | Output cells | Semantics |
| --- | --- | --- | --- |
| `ioctl_read_msr` | `in[0]` = MSR index | `out[0]` = MSR value | Read an allow-listed MSR |
| `ioctl_write_msr` | `in[0]` = MSR index, `in[1]` = value | — | Write an allow-listed MSR (NOT used by this project — read-only policy) |

- The read allow-list includes the thermal MSRs this project needs:
  `0x19C` (IA32_THERM_STATUS), `0x1B1` (IA32_PACKAGE_THERM_STATUS),
  `0x1A2` (MSR_TEMPERATURE_TARGET). Reads of MSRs outside the
  allow-list fail with `STATUS_ACCESS_DENIED`. (S5)
- The write allow-list is small (power-limit / mailbox registers) and
  is irrelevant here; this project performs no MSR writes.

### `RyzenSMU`

Target: AMD x86-64 CPUs, families `0x17`, `0x19`, `0x1A` only.

| Function | Input cells | Output cells | Semantics |
| --- | --- | --- | --- |
| `ioctl_read_smu_register` | `in[0]` = SMN address | `out[0]` = 32-bit register value | Read a validated SMN register |
| `ioctl_get_code_name` | — | `out[0]` = codename enum | CPU codename detected by the module |
| `ioctl_get_smu_version` | — | `out[0]` = version | SMU firmware version |
| `ioctl_resolve_pm_table` / `ioctl_update_pm_table` / `ioctl_read_pm_table` | — | table metadata / contents | PM-table access (not needed for Phase 1) |
| `ioctl_write_smu_register`, `ioctl_send_smu_command` | … | … | Write paths (NOT used — read-only policy) |

- SMN reads are validated against allowed address windows, including
  `0x56000`–`0x5AFFF`, which contains the thermal controller register
  `0x59800` used for Tctl (see
  [`cpu-amd-zen-smn.md`](cpu-amd-zen-smn.md)). (S5)
- Internally the module performs SMN access through an index/data
  register pair in the host bridge PCI configuration space (bus 0,
  device 0, function 0); the client never performs raw PCI access
  itself. (S5; informative internal detail, not a contract this
  project depends on)
- The module synchronizes several operations on the named kernel
  mutant `\BaseNamedObjects\Access_PCI`. (S5)

### `LpcIO`

Target: x86-64 systems; provides port I/O for Super I/O chips.

| Function | Input cells | Output cells | Semantics |
| --- | --- | --- | --- |
| `ioctl_select_slot` | `in[0]` = slot (0 or 1) | — | Select config port pair: slot 0 → `0x2E`/`0x2F`, slot 1 → `0x4E`/`0x4F` |
| `ioctl_find_bars` | — | — | Discover and allow the I/O BAR ranges of the selected chip |
| `ioctl_superio_inb` | `in[0]` = config register | `out[0]` = byte | Read a Super I/O configuration register |
| `ioctl_superio_inw` | `in[0]` = config register | `out[0]` = word | 16-bit configuration read |
| `ioctl_superio_outb` | `in[0]` = config register, `in[1]` = byte | — | Write a Super I/O configuration register |
| `ioctl_pio_inb` | `in[0]` = port | `out[0]` = byte | Read an allowed I/O port (config pair or discovered BARs) |
| `ioctl_pio_outb` | `in[0]` = port, `in[1]` = byte | — | Write an allowed I/O port |

- Port access is restricted to the selected configuration register
  pair and BAR ranges discovered by `ioctl_find_bars` (clamped to
  8-byte-aligned windows). (S5)
- The module takes the named kernel mutant
  `\BaseNamedObjects\Access_ISABUS.HTP.Method` around its operations.
  (S5)

## Mutex conventions

- The Windows user-mode name `Global\X` and the kernel object path
  `\BaseNamedObjects\X` denote the same named object. The ecosystem
  conventions are therefore:
  - **ISA / Super I/O:** `Global\Access_ISABUS.HTP.Method`
  - **PCI / SMN:** `Global\Access_PCI`
- The modules acquire these mutants **per IOCTL call** (S5). A Super
  I/O read transaction spans many IOCTLs (enter config mode, select
  bank, read index/data, exit), so per-call locking alone cannot keep
  the transaction atomic. The client must additionally hold the
  corresponding `Global\…` mutex in user mode for the whole multi-step
  transaction, matching the behavior of HWiNFO / LibreHardwareMonitor /
  FanControl. (Convention; issue #1635)
- Mutex acquisition must use a bounded timeout and treat timeout as a
  failed (skipped) sample, never as permission to proceed unlocked.

## Open questions

- On which logical CPU does `IntelMSR::ioctl_read_msr` execute the
  read? Expected: the calling thread's current processor, making
  per-core readings controllable via thread affinity. Phase 1 only
  needs the package-scope MSR (any core of the package), but this must
  be verified before adding per-core readings.
- Exact `PawnIOLib.h` prototypes (including the close function and
  version query) need to be pinned from an installed copy.
- Module blob file names and installed locations of the signed module
  releases (needed for installer integration) need to be confirmed
  from a PawnIO release package.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-06-10 | Initial version |
