# Spec: PawnIO driver, library, and module IOCTL interface

| Field | Value |
| --- | --- |
| Revision | 7 |
| Status | Implementation-ready (rev 7) |
| Scope | Facts needed to integrate a Rust user-mode client with PawnIO: installation/detection, the PawnIOLib API, the module execution model, and the IOCTL contracts of the `IntelMSR`, `RyzenSMU`, `AMDFamily17`, and `LpcIO` modules. Excludes: writing new Pawn modules, driver internals. |
| Issue phase | Phase 1 (#1635) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | namazso, *PawnIO* repository, <https://github.com/namazso/PawnIO> (README / LICENSE) | Primary; license facts |
| S2 | namazso, *PawnIO.Modules* repository, <https://github.com/namazso/PawnIO.Modules> | Primary; module list, license |
| S3 | PawnIO.Modules wiki, "Using PawnIO Modules", <https://github.com/namazso/PawnIO.Modules/wiki/Using-PawnIO-Modules> | Primary; user-mode API |
| S4 | PawnIO.Modules wiki, "Getting started with PawnIO", <https://github.com/namazso/PawnIO.Modules/wiki/Getting-started-with-PawnIO> | Primary; toolchain, signing |
| S5 | Module sources `IntelMSR.p`, `RyzenSMU.p`, `AMDFamily17.p`, `LpcIO.p` in S2 (LGPL-2.1-or-later); all four re-verified at tag `0.2.11` (annotated tag object `7345a80`, commit `52a7e536dff3e53c96917a28caac5e0fa6510696`), diffed against tag `0.2.8` (annotated tag object `754635b`, commit `dcd5c1f67e015542e4b5b6f570e921ff60571a73`) | Upstream-published interface definitions of the API this project calls across the IOCTL boundary (public `ioctl_*` contracts, allow-lists, caller-mutex `@warning` docs); the PawnIO project is the authoritative source for its own interfaces. Not used as a source for any hardware register fact. No code was copied. |
| S6 | `PawnIOLib/include/PawnIOLib.h` in S1 (LGPL-2.1-or-later, © 2026 namazso) | Primary; exact user-mode API prototypes and doc comments |
| S7 | PawnIO driver source in S1 (GPL-2.0 with IOCTL exception): `PawnIO/src/natives_impl_windows.cpp`, `PawnIO/include/pawnio_um.h` | Native semantics (execution context of `msr_read`, affinity natives) and device path. Interface facts only; no code was copied. |
| S8 | PawnIO.Modules `README.md` and GitHub Releases, <https://github.com/namazso/PawnIO.Modules/releases>; CI workflow `.github/workflows/ci.yml` in S2 | Primary; module-blob distribution channels and signing status. Release 0.2.11 (published 2026-08-30) assets, per `gh release view 0.2.11 -R namazso/PawnIO.Modules`: `release_0_2_11.zip` (69,582 bytes, SHA-256 `43608cb89bc84247fef1368a139013f7d043e17db6d6c8dfc9b46bf0905a81f4`) + source archives; archive contents listed from the downloaded asset on 2026-09-26. Earlier pin: release 0.2.8, `release_0_2_8.zip`. `README.md` and `.github/workflows/ci.yml` are unchanged between tags `0.2.8` and `0.2.11` (upstream compare) |
| S9 | PawnIO repo in S1: `PawnIOUtil/PawnIOUtil.cpp` (the `sign` command and signed-blob layout), `PawnIO/PawnIO.inf.in` (device security descriptor), `PawnIO/src/driver.cpp` (`IoCreateDevice`) | Primary; signed-module format and device access control. Interface facts only; no code was copied |
| S10 | Implementer field validation on AMD Ryzen 7 7800X3D (Windows), reported 2026-06-13: installed module file names, `pawnio_open` access-denied without elevation, `Global\Access_PCI` open-vs-create behavior | Independent runtime observation (clean-room: the implementer ran the actual hardware and PawnIO; no prohibited source consulted). Corroborates the primary-source facts above |
| S11 | PawnIO.Modules PR #85, "RyzenSMU: Fix Bergamo family", <https://github.com/namazso/PawnIO.Modules/pull/85> (commit `a8706056`, merged 2026-07-26 via `75cea484`), and the release 0.2.10 notes, <https://github.com/namazso/PawnIO.Modules/releases/tag/0.2.10> (published 2026-07-27, lists PR #85) | Primary (upstream project's own description of its interface change); the PR cites the Bergamo CPUID `0xAA0F02` from the InstLatx64 CPUID dump `AuthenticAMD0AA0F02_K19_Bergamo_04_CPUID.txt`. The PR author states it was not tested on real hardware. No code was copied |
| S12 | AMD, *Revision Guide for AMD Family 19h Models A0h-AFh Processors*, publication 57926, revision 1.05 (November 2025), docs.amd.com document id `aBaGVEhC_kN7n61TYp_76A`; references PPR order # 57228 (*PPR for AMD Family 19h Model A0h, Revision A2 Processors*) | Primary (vendor document); Overview p. 5 (covered products: EPYC 9004, EPYC 8004), Tables 2–3 p. 8 (CPUID `00AA0F02h` per package). Family 1Ah document list from a docs.amd.com search on 2026-09-26 |

## Licensing facts

- The PawnIO driver is licensed **GPL-2.0 with an exception** that
  explicitly permits combining it with independent modules that
  communicate "through the device IO control interface", and with
  LGPL code. A user-mode client that only talks to the driver via
  IOCTLs is such an independent module. (S1)
- **PawnIOLib** (the user-mode library/DLL) is **LGPL-2.1-or-later**
  (S6). The client loads the system-installed DLL dynamically and
  ships none of its code, so this repository's own license is
  unaffected.
- The modules in PawnIO.Modules are **LGPL-2.1-or-later**. Our client
  invokes them through the driver's IOCTL interface and ships none of
  their code, so this repository's own license is unaffected.
  Redistributing the compiled module blobs with the installer requires
  complying with LGPL-2.1 distribution terms (source offer /
  attribution in third-party notices). (S1, S2)
- This repository is licensed GPL-3.0-or-later from the revision
  recorded in
  [ADR 0020](../../adr/0020-relicense-to-gpl-3.0-or-later.md); it was
  MIT before. The facts above do not depend on the client's license:
  the driver's IOCTL exception and the LGPL-2.1-or-later terms apply
  to an independent user-mode client either way.
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
  Latest release at re-verification (2026-09-26): **0.2.11
  (2026-08-30)**, verified against the upstream git tag `0.2.11`
  (commit `52a7e53`). (S8)
- The release ships as a **single archive** (0.2.11:
  `release_0_2_11.zip`, alongside the auto-generated source archives),
  not as individual per-module assets. The signed module files live
  inside that archive. (S8)
- The 0.2.11 archive holds a flat set of signed `*.bin` modules plus
  `COPYING`; it contains `IntelMSR.bin`, `RyzenSMU.bin`,
  `AMDFamily17.bin`, and `LpcIO.bin` under the same file names as in
  0.2.8. Releases 0.2.9–0.2.11 added modules this project does not
  load; no module this project loads was renamed or removed. (S8)
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
  `IntelMSR.bin` for Phase 1; `AMDFamily17.bin` additionally for the
  CPU package-power phase; `LpcIO.bin` for the Super I/O phases), not
  self-built `.amx` copies.
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
- The read allow-list also includes the RAPL MSRs used by
  [`cpu-intel-rapl-msr.md`](cpu-intel-rapl-msr.md): `0x606`
  (MSR_RAPL_POWER_UNIT) and `0x611` (MSR_PKG_ENERGY_STATUS), plus
  further RAPL-domain registers this project does not decode
  (`0x610`, `0x613`, `0x614`, `0x619`, `0x61B`, `0x61C`, `0x639`,
  `0x641`, `0x64D`, …). `0x611` is not on the write allow-list.
  Verified at tag `0.2.11`. (S5)
- Change since tag `0.2.8`: `0x1A4` (the hardware-prefetcher control
  MSR) was added to **both** the read and the write allow-list
  (upstream release 0.2.10). No MSR was removed from either list, the
  vendor gate and the `ioctl_*` surface are unchanged, and none of the
  thermal or RAPL MSRs above moved onto the write allow-list. This
  project neither reads nor writes `0x1A4`. (S5)
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

Target: AMD x86-64 CPUs, families `0x17`, `0x19`, `0x1A`, plus (from
tag `0.2.11`) family `0x15` restricted to the pre-Zen Carrizo, Bristol
Ridge, and Stoney Ridge parts. Module load additionally requires a
family/model combination the module recognizes; unrecognized models
and every other family/vendor get `STATUS_NOT_SUPPORTED`. This project
does not enable family `0x15` (see
[`cpu-amd-zen-smn.md`](cpu-amd-zen-smn.md)). (S5, tag `0.2.11`)

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
- Changes since tag `0.2.8` (verified at tag `0.2.11`), none of which
  alters the contract this project uses (S5):
  - The `ioctl_*` names and their input/output cell counts are
    unchanged (`ioctl_read_smu_register`: 1 input cell, 1 output
    cell), and every SMU ioctl keeps the caller-held `Access_PCI`
    `@warning`.
  - The SMN window `0x56000`–`0x5AFFF` is still accepted for every
    supported family. The additional windows added in release 0.2.11
    (an MMIO mailbox page and an SMU fuse range) are accepted only on
    the three family `0x15` parts.
  - The codename values returned by `ioctl_get_code_name` for the
    Zen families are unchanged; the three pre-Zen codenames were
    appended after the existing values (release 0.2.11).
  - Model `0xA0` recognition (family correction). At tags `0.2.8` and
    `0.2.9` the Bergamo entry was listed under family `0x1A` model
    `0xA0`, and family `0x19` model `0xA0` was not recognized. Upstream
    PR #85 ("RyzenSMU: Fix Bergamo family", commit `a8706056`, released
    in 0.2.10) describes that entry as incorrectly listed under family
    `0x1A` and moves it to family `0x19` model `0xA0`; Bergamo reports
    CPUID `00AA0F02h` (family `0x19`, model `0xA0`, stepping 2) (S5,
    S11). From release 0.2.10 the module therefore recognizes family
    `0x19` model `0xA0` as Bergamo and rejects family `0x1A` model
    `0xA0` at load (`STATUS_NOT_SUPPORTED`) (S5).
  - AMD documents CPUID `00AA0F02h` (Zen4c-A2) for AMD EPYC 9004
    Series (SP5) and AMD EPYC 8004 Series (SP6) processors, both
    within Family 19h Models A0h–AFh (S12, Overview p. 5 and
    Tables 2–3 p. 8). No AMD document for a Family 1Ah model `0xA0`
    part was found at authoring (2026-09-26 search of the AMD
    documentation portal; AMD Family 1Ah documents found cover
    models 00h–0Fh, 10h–1Fh, 11h, 50h–57h, and 70h) (S12).
  - Consequence for this project: no known shipping CPU loses
    `RyzenSMU` support from this change. Family `0x19` model `0xA0`
    parts (EPYC 9004 "Bergamo" / EPYC 8004 "Siena"), which the `0.2.8`
    module rejected at load, are now recognized. A future Family
    `0x1A` model `0xA0` part, if one ships, would be rejected at load
    by the `0.2.11` module; this affects only the Experimental family
    `0x1A` Tctl path (S5, S12).
  - From release 0.2.10, module load no longer resolves or maps the SMU
    PM table; the table is mapped on the first PM-table read instead.
    A PM-table failure therefore no longer fails module load.
  - The host-bridge sanity check at load accepts vendor ID `0x1022` or
    `0x1002` (informative internal detail).

### `AMDFamily17`

Target: AMD x86-64 CPUs, families `0x17`–`0x1A` only; other vendors,
architectures, and families get `STATUS_NOT_SUPPORTED` at module
load. (S5, tag `0.2.11`; unchanged since tag `0.2.8`)

| Function | Input cells | Output cells | Semantics |
| --- | --- | --- | --- |
| `ioctl_read_msr` | `in[0]` = MSR index | `out[0]` = MSR value | Read an allow-listed MSR |
| `ioctl_write_msr` | `in[0]` = MSR index, `in[1]` = value | — | Write an allow-listed MSR (NOT used by this project — read-only policy) |
| `ioctl_read_smn` | `in[0]` = SMN address | `out[0]` = 32-bit register value | Read an SMN register (not used by this project; the SMN path goes through `RyzenSMU`) |

- The read allow-list includes the RAPL MSRs used by
  [`cpu-amd-zen-rapl-msr.md`](cpu-amd-zen-rapl-msr.md):
  `0xC0010299` (RAPL_PWR_UNIT), `0xC001029A` (CORE_ENERGY_STAT),
  `0xC001029B` (PKG_ENERGY_STAT), plus P-state/CPPC/performance
  registers this project does not decode. Reads outside the
  allow-list fail with `STATUS_ACCESS_DENIED`. None of the three
  RAPL MSRs is on the write allow-list. `ioctl_read_msr` is declared
  with exactly 1 input and 1 output cell. (S5)
- Changes since tag `0.2.8` (verified at tag `0.2.11`), upstream
  release 0.2.10: the read allow-list gained the machine-check global
  MSRs `0x179`–`0x17B`, the SMCA per-bank diagnostic MSRs within
  `0xC0002000`–`0xC00023FF` (only banks reported by `0x179` and only
  specific architected register offsets within each bank), and the
  load-store / cache configuration MSRs `0xC0011020`, `0xC0011021`,
  `0xC0011022`, `0xC001102B`; the write allow-list gained those four
  configuration MSRs. Nothing was removed, and the three RAPL MSRs
  remain absent from the write allow-list. The `ioctl_*` surface,
  family gate, and `@warning` documentation are unchanged. (S5)
- `ioctl_read_msr` carries **no caller-mutex requirement**; only
  `ioctl_read_smn` is documented with "You should acquire the
  `\BaseNamedObjects\Access_PCI` mutant before calling this". (S5)
- **Execution context:** as with `IntelMSR`, the driver's `msr_read`
  native executes `RDMSR` on the calling thread's current processor,
  and the module uses no affinity natives; the targeted CPU is
  controlled by the user-mode caller's thread affinity. (S5, S7)
- The module's SMN read goes through a host-bridge index/data pair at
  PCI config offsets `0x60`/`0x64` and requires the host bridge
  vendor ID `0x1022`; informative internal detail only — this
  project performs SMN access via `RyzenSMU`, not this module. (S5)
- The signed blob `AMDFamily17.bin` is contained in the 0.2.11 release
  archive `release_0_2_11.zip` alongside `IntelMSR.bin` /
  `RyzenSMU.bin` / `LpcIO.bin` (verified by listing the downloaded
  archive; it was likewise present in the 0.2.8 archive). (S8)

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
- From tag `0.2.10` (verified at tag `0.2.11`): ports `0xCF8`–`0xCFF`
  (the PCI configuration mechanism #1 ports) are never allowed for
  port I/O, and `ioctl_find_bars` does not record a candidate window
  that overlaps them; the number of recorded BAR windows is bounded
  (128). The `ioctl_*` names, input/output cell counts, and slot
  mapping are unchanged since tag `0.2.8`. (S5)
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
| 5 | 2026-08-30 | Added the `AMDFamily17` module contract for the CPU package-power phase (family gate `0x17`–`0x1A`, `ioctl_read_msr`/`ioctl_write_msr`/`ioctl_read_smn`, RAPL MSR read allow-list membership, no caller mutex on MSR reads, execution context, `AMDFamily17.bin` in the 0.2.8 release archive), verified against tag `0.2.8` (commit `754635b`). Recorded the `IntelMSR` RAPL read allow-list additions (`0x606`, `0x611`, and other RAPL-domain registers). Status remains Implementation-ready. |
| 6 | 2026-09-03 | Licensing facts reworded for the repository relicense from MIT to GPL-3.0-or-later (ADR 0020) and a note added that the client's license does not change the IOCTL-exception / LGPL facts. No hardware, API, or IOCTL fact changed. Status remains Implementation-ready. |
| 7 | 2026-09-26 | Re-verified all module facts against PawnIO.Modules tag `0.2.11` (commit `52a7e536dff3e53c96917a28caac5e0fa6510696`, release asset `release_0_2_11.zip`) by diffing `IntelMSR.p`, `RyzenSMU.p`, `AMDFamily17.p`, and `LpcIO.p` against tag `0.2.8` and listing the release archive. Unchanged: every `ioctl_*` name and input/output cell count, the caller-mutex `@warning` docs, the `IntelMSR` vendor gate and `AMDFamily17` family gate, allow-list membership of `0x19C`/`0x1B1`/`0x1A2`/`0x606`/`0x611` and `0xC0010299`/`0xC001029A`/`0xC001029B` (none on a write allow-list), the `RyzenSMU` SMN window `0x56000`–`0x5AFFF`, the signed `.bin` file names, and the README/CI distribution facts. Recorded additive changes: `IntelMSR` read/write allow-list gained `0x1A4`; `AMDFamily17` read allow-list gained machine-check and SMCA diagnostic MSRs and read/write gained four cache-configuration MSRs; `RyzenSMU` accepts three family `0x15` parts, moved the Bergamo model `0xA0` recognition from family `0x1A` to family `0x19` (upstream PR #85 describes the old entry as a wrongly assigned family, S11; AMD RG 57926 places EPYC 9004/8004 at CPUID `00AA0F02h`, S12; no AMD Family 1Ah model `0xA0` document found at authoring, so no known shipping CPU loses support), and no longer maps the PM table at load; `LpcIO` never allows ports `0xCF8`–`0xCFF` and bounds the BAR list. Corrected the provenance label: `754635b` identifies the `0.2.8` annotated tag object, whose commit is `dcd5c1f`. No blocking change. Status remains Implementation-ready. |
