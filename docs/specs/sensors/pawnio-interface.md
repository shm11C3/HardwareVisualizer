# Spec: PawnIO driver, library, and module IOCTL interface

| Field | Value |
| --- | --- |
| Revision | 4 |
| Status | Implementation-ready (rev 4) |
| Scope | Facts needed to integrate a Rust user-mode client with PawnIO: installation/detection, the PawnIOLib API, the module execution model, and the IOCTL contracts of the `IntelMSR`, `RyzenSMU`, and `LpcIO` modules. Excludes: writing new Pawn modules, driver internals. |
| Issue phase | Phase 1 (#1635) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | namazso, *PawnIO* repository, <https://github.com/namazso/PawnIO> (README / LICENSE) | Primary; license facts |
| S2 | namazso, *PawnIO.Modules* repository, <https://github.com/namazso/PawnIO.Modules> | Primary; module list, license |
| S3 | PawnIO.Modules wiki, "Using PawnIO Modules", <https://github.com/namazso/PawnIO.Modules/wiki/Using-PawnIO-Modules> | Primary; user-mode API |
| S4 | PawnIO.Modules wiki, "Getting started with PawnIO", <https://github.com/namazso/PawnIO.Modules/wiki/Getting-started-with-PawnIO> | Primary; toolchain, signing |
| S5 | Module sources `IntelMSR.p`, `RyzenSMU.p`, `LpcIO.p` in S2 (LGPL-2.1-or-later) | Upstream-published interface definitions of the API this project calls across the IOCTL boundary (public `ioctl_*` contracts, allow-lists, caller-mutex `@warning` docs); the PawnIO project is the authoritative source for its own interfaces. Not used as a source for any hardware register fact. No code was copied. |
| S6 | `PawnIOLib/include/PawnIOLib.h` in S1 (LGPL-2.1-or-later, © 2026 namazso) | Primary; exact user-mode API prototypes and doc comments |
| S7 | PawnIO driver source in S1 (GPL-2.0 with IOCTL exception): `PawnIO/src/natives_impl_windows.cpp`, `PawnIO/include/pawnio_um.h` | Native semantics (execution context of `msr_read`, affinity natives) and device path. Interface facts only; no code was copied. |
| S8 | PawnIO.Modules `README.md` and GitHub Releases, <https://github.com/namazso/PawnIO.Modules/releases>; CI workflow `.github/workflows/ci.yml` in S2 | Primary; module-blob distribution channels and signing status. Release 0.2.8 assets (via the release `expanded_assets` fragment): `release_0_2_8.zip` + source archives |
| S9 | PawnIO repo in S1: `PawnIOUtil/PawnIOUtil.cpp` (the `sign` command and signed-blob layout), `PawnIO/PawnIO.inf.in` (device security descriptor), `PawnIO/src/driver.cpp` (`IoCreateDevice`) | Primary; signed-module format and device access control. Interface facts only; no code was copied |
| S10 | Implementer field validation on AMD Ryzen 7 7800X3D (Windows), reported 2026-06-13: installed module file names, `pawnio_open` access-denied without elevation, `Global\Access_PCI` open-vs-create behavior | Independent runtime observation (clean-room: the implementer ran the actual hardware and PawnIO; no prohibited source consulted). Corroborates the primary-source facts above |

## Licensing facts

- The PawnIO driver is licensed **GPL-2.0 with an exception** that
  explicitly permits combining it with independent modules that
  communicate "through the device IO control interface", and with
  LGPL code. A user-mode client that only talks to the driver via
  IOCTLs is such an independent module. (S1)
- **PawnIOLib** (the user-mode library/DLL) is **LGPL-2.1-or-later**
  (S6). The client loads the system-installed DLL dynamically and
  ships none of its code, so MIT licensing of this repository is
  unaffected.
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
- The driver and PawnIOLib runtime are distributed from
  <https://pawnio.eu> (signed installer). (S1, S4)
- **The core PawnIO installer does not bundle the sensor modules.** A
  core install contains the runtime and tooling only — `PawnIOLib.dll`,
  `PawnIOLib.h`, `PawnIOUtil.exe`, the uninstaller — reflecting the
  PawnIO (runtime) vs PawnIO.Modules (modules) repository split. The
  `IntelMSR` / `RyzenSMU` module files come separately from the
  PawnIO.Modules release and must be supplied by this application; do
  not assume they exist under the PawnIO install directory. (S1, S2;
  observed by S10)

### Privilege requirement (Windows)

- **`pawnio_open` requires Administrator / elevation.** The driver's
  device object DACL is set by the INF to
  `D:P(A;;GA;;;SY)(A;;GA;;;BA)` (S9), i.e. `GENERIC_ALL` for Local
  System (`SY`) and Built-in Administrators (`BA`) only, with no ACE
  for normal users. A non-elevated caller therefore fails `pawnio_open`
  with `0x80070005` (`E_ACCESSDENIED`), even though `PawnIOLib.dll`
  loads and `pawnio_version` succeeds, and even when the `PawnIO`
  kernel service is installed and running. (S9; observed by S10)
- Detection must distinguish three states so the caller can react
  correctly: library/driver **absent** → fall back to ACPI zones
  (#1633); present but **access-denied** (`0x80070005`) → report that
  elevation is required rather than "unsupported"; present and
  **openable** → proceed.

## Module blob distribution

- **Signed module blobs are distributed via the PawnIO.Modules
  GitHub Releases** — the repository README states "Signed builds can
  be found in Releases." This is the channel an end-user deployment
  must use, because the production driver loads signed modules only.
  Latest release at authoring: **0.2.8 (2026-06-12)**, verified
  against the upstream git tag `0.2.8` (commit `754635b`). (S8)
- The release ships as a **single archive** (0.2.8: `release_0_2_8.zip`,
  alongside the auto-generated source archives), not as individual
  per-module assets. The signed module files live inside that archive.
  (S8)
- **Two distinct artifact forms, distinguished by extension:**
  - `*.amx` — the raw `pawncc` output (`-C64 -iinclude`), **unsigned**.
    This is what the PawnIO.Modules CI `build` workflow uploads as a
    per-commit artifact and what a local `pawncc` build produces. The
    production (signed) driver will **not** load these; they require
    the unrestricted driver + Windows test-signing. (S8)
  - `*.bin` — the **signed** module: `PawnIOUtil sign` wraps an `.amx`
    as `[u32 little-endian signature length][signature][amx bytes]`
    and writes it to a separate file (S9). These are the files shipped
    inside the release archive and installed on disk as, e.g.,
    `RyzenSMU.bin` / `IntelMSR.bin`; they are what the production
    driver loads. (S9; observed by S10)
- `pawnio_load` takes an **in-memory blob and is extension-agnostic**
  (S6) — it does not require any particular file name. The client must
  therefore not hard-require `.amx`: load the signed `.bin` that the
  release/install provides, treating the module file name/extension as
  configuration (default `.bin`).
- Consequence for this project: bundle the **signed `.bin`** modules
  from a pinned PawnIO.Modules release (`RyzenSMU.bin` and
  `IntelMSR.bin` for Phase 1), not self-built `.amx` copies.
  Redistribution must comply with the modules' LGPL-2.1 terms (see
  Licensing facts).
- Absence of PawnIO is a supported state: the client must detect the
  missing library/driver and report "unavailable" so the caller can
  fall back to the ACPI thermal-zone source (PR #1633).

## PawnIOLib user-mode API

Prototypes and semantics verified against `PawnIOLib.h` (S6). Every
function exists in three variants: HRESULT (`pawnio_*`), Win32 BOOL
(`pawnio_*_win32`), and NTSTATUS (`pawnio_*_nt`); the HRESULT forms
are listed here.

```c
HRESULT pawnio_version(PULONG version);
  // version = (major << 16) | (minor << 8) | patch
HRESULT pawnio_open(PHANDLE handle);      // open an executor
HRESULT pawnio_load(HANDLE handle, const UCHAR* blob, SIZE_T size);
HRESULT pawnio_execute(HANDLE handle, PCSTR name,
                       const ULONG64* in,  SIZE_T in_size,
                       PULONG64 out,       SIZE_T out_size,
                       PSIZE_T return_size);
HRESULT pawnio_close(HANDLE handle);
```

- `in_size` / `out_size` are documented as "Input/Output buffer
  count" and `return_size` as "Entries written" — i.e. counts of
  64-bit `ULONG64` cells, not bytes. (S6)
- Asynchronous variants exist (`pawnio_execute_async`,
  `pawnio_execute_async_nt`, OVERLAPPED mandatory); this project uses
  only the synchronous form. (S6)
- The library is loaded dynamically and functions are resolved by
  name (e.g. via `GetProcAddress`). (S3)
- The driver device object is `\Device\PawnIO`
  (`pawnio_um.h` `k_device_path`); clients normally reach it through
  PawnIOLib rather than opening the device directly. (S7)
- Module functions are addressed by **string name** following the
  `ioctl_*` convention. (S3, S5)
- Buffers are arrays of **64-bit cells**: PawnIO only supports a
  64-bit cell size (modules are compiled with `-C64`). (S3, S4, S6)
- Functions return NTSTATUS-style status codes; module-level denials
  observed in the module sources use `STATUS_ACCESS_DENIED` /
  `STATUS_NOT_SUPPORTED`. (S5)
- One handle holds one loaded module; this project uses one executor
  handle per module (see Open questions for reload semantics).

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
  `0x1A2` (MSR_TEMPERATURE_TARGET) — verified in
  `is_allowed_msr_read`. Reads of MSRs outside the allow-list fail
  with `STATUS_ACCESS_DENIED`. `ioctl_read_msr` is declared with
  exactly 1 input and 1 output cell. (S5)
- **Execution context:** the driver's `msr_read` native executes
  `__readmsr` on the calling thread's current processor and sets no
  affinity; the `IntelMSR` module does not use the
  `cpu_set_affinity` / `cpu_restore_affinity` natives either. The
  CPU a read targets is therefore controlled by the user-mode
  caller's thread affinity. Package-scope MSRs (`0x1B1`, `0x1A2`)
  read identically from any logical CPU of the package. (S5, S7)
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
  device 0, function 0; named constants `SMU_PCI_ADDR_REG`/`..DATA..`
  = `0xC4`/`0xC8`); the client never performs raw PCI access itself.
  (S5; informative internal detail, not a contract this project
  depends on)
- **The module does not acquire any mutex itself.** Each SMU ioctl is
  documented with: "You should acquire the
  `\BaseNamedObjects\Access_PCI` mutant before calling this" — i.e.
  the **caller** must hold it. (S5)

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
- **The module does not acquire any mutex itself.** Each port/config
  ioctl is documented with: "You should acquire the
  `\BaseNamedObjects\Access_ISABUS.HTP.Method` mutant before calling
  this" — i.e. the **caller** must hold it. (S5)

## Mutex conventions

- The Windows user-mode name `Global\X` and the kernel object path
  `\BaseNamedObjects\X` denote the same named object. The ecosystem
  conventions are therefore:
  - **ISA / Super I/O:** `Global\Access_ISABUS.HTP.Method`
  - **PCI / SMN:** `Global\Access_PCI`
- The modules do **not** acquire these mutants; every relevant ioctl
  documents that the caller must hold the mutant before calling (S5).
  The user-mode client therefore holds `Global\Access_PCI` around
  each `RyzenSMU` call, and `Global\Access_ISABUS.HTP.Method` across
  each Super I/O transaction — which spans many IOCTLs (enter config
  mode, select bank, read index/data, exit) and must be held for the
  whole multi-step sequence, matching the behavior of HWiNFO /
  LibreHardwareMonitor / FanControl. (S5; convention per issue #1635)
- Mutex acquisition must use a bounded timeout and treat timeout as a
  failed (skipped) sample, never as permission to proceed unlocked.
- **Open an existing mutant before creating one.** These mutants are
  shared with other monitors (HWiNFO / LibreHardwareMonitor /
  FanControl), and whichever process creates one first sets its ACL.
  Calling `CreateMutexW` against an already-existing, restrictively
  ACL'd object can fail with access-denied. Acquire with
  `OpenMutexW(MUTEX_MODIFY_STATE | SYNCHRONIZE, FALSE, name)` first and
  fall back to `CreateMutexW` only when the object does not yet exist;
  request only the minimal rights needed to `WaitForSingleObject` /
  `ReleaseMutex`. (Win32 semantics; observed by S10 — `CreateMutexW`
  on an existing `Global\Access_PCI` returned access-denied, while
  open-then-create succeeded.)

## Open questions

- Non-blocking for Phase 1: the client uses one executor handle per
  module and never reloads a different blob on the same handle.
  Whether `pawnio_load` may be called twice on one handle is not
  documented upstream (S6); resolve only if a future phase needs it.
- Resolved (rev 4): signed blobs come from the PawnIO.Modules GitHub
  Releases as a single archive (`release_<version>.zip`) containing
  signed `*.bin` modules; see "Module blob distribution". When pinning
  a release, confirm the exact in-archive module file names against
  that release.

## Follow-up scope (out of Phase 1)

A user-friendly deployment needs more than this document; an
installer/setup helper is expected to handle, and to surface
diagnostics for, each of: installing and starting the PawnIO kernel
driver; ensuring `PawnIOLib.dll` is present; placing the signed
`IntelMSR` / `RyzenSMU` `.bin` modules where the app loads them;
obtaining elevation for `pawnio_open`; and reporting the distinct
failure modes (missing DLL, missing module, driver access-denied,
module load failure). Captured here from S10; tracked as a later
phase of #1635, not part of the Phase 1 read path.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-06-10 | Initial version |
| 2 | 2026-06-11 | Provenance resolved against upstream sources: exact `PawnIOLib.h` API (incl. `pawnio_close`, cell-count semantics), device path, `msr_read` execution context and affinity natives, blob naming. Corrected mutex ownership: modules document caller-held mutants and acquire none themselves. Status → Implementation-ready. |
| 3 | 2026-06-13 | Added "Module blob distribution" section: signed blobs ship via the PawnIO.Modules GitHub Releases (README-stated; latest 0.2.8, 2026-06-12, verified against the upstream git tag), CI artifacts/self-builds are unsigned, driver/PawnIOLib from pawnio.eu. Resolved the blob-source open question (asset packaging left as a narrow non-blocking confirmation). Added source S8. Status remains Implementation-ready. |
| 4 | 2026-06-13 | Implementer field-validation corrections (Ryzen 7 7800X3D, S10), all cross-checked against PawnIO primary sources (S9): signed modules are `*.bin` (`PawnIOUtil sign` blob layout) shipped inside the release archive `release_0_2_8.zip`, vs unsigned `*.amx` build output — `pawnio_load` is extension-agnostic, so dropped the `.amx`-only naming claim; `pawnio_open` requires elevation (device DACL `D:P(A;;GA;;;SY)(A;;GA;;;BA)`; non-elevated → `0x80070005`), added three-state detection; core installer excludes modules; mutex acquisition must open-before-create to avoid ACL failures on shared mutants; added a follow-up-scope note for installer UX. Resolved the asset-packaging open question. Status remains Implementation-ready. |
