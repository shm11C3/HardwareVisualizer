# Windows Hardware Coverage Survey

Survey date: 2026-09-03. Scope: the Windows sensor paths that feed
Cooling Insight (#1666) and the Dashboard, which hardware they leave
unsupported today, and how feasible each gap is to close under the existing
clean-room and design rules.

This is a research document, not a spec and not a decision. Hardware facts
that are needed to close a gap still have to enter
[`docs/specs/sensors/`](../specs/sensors/) through the spec-author role before
any implementation. Verification status stays owned by the specs
([ADR 0011](../adr/0011-experimental-sensor-enablement.md)).

## Why this matters for #1666

Cooling Insight relates thermal input, response, and cooling activity on one
time axis. Every lane is capability-dependent and reads the archive columns
below, so hardware coverage is the ceiling of what the view can show:

| Cooling Insight lane / signal | Archive input | Windows producer today |
| --- | --- | --- |
| Temperature lane, baseline, load bands | `TemperatureSample.cpu_temperature` | PawnIO Intel DTS / AMD SMN package temperature, else ACPI thermal zone (`core/src/platform/windows/sensors.rs`) |
| Power lane | `PowerDraw.cpu_watts` | PawnIO Intel / AMD RAPL package energy (`cpu_power.rs`); no fallback |
| Fan lane | `motherboard_fan_speeds` rows | PawnIO `LpcIO` Nuvoton NCT6799D direct RPM only (`super_io_motherboard.rs`) |
| Ambient lane / ΔT | environmental provider | SwitchBot Meter over BLE (cross-platform; #2062 extends it) |
| Not yet a lane | CPU clock, throttling state, GPU temperature | Not collected on Windows (see gaps below) |

The unsupported-sensor note in the view names the power and fan lanes only
when the archive proves they never recorded, so on most Windows machines
without PawnIO both lanes are absent and the view degrades to temperature
versus load from ACPI zones, which many self-built desktops do not expose at
all.

## Evidence used

- Provider code under `core/src/infrastructure/providers/windows/` and
  `core/src/platform/windows/`.
- Scoped-enablement tables in `docs/specs/sensors/*.md` at their current
  revisions (cited per row below).
- [`windows-sensor-external-components.md`](../architecture/windows-sensor-external-components.md)
  scope boundaries.
- The upstream PawnIO.Modules repository file listing (retrieved 2026-09-03)
  for which access modules exist. Module names are interface facts; no module
  source was read for this survey.
- Issues #1635, #1666, #1824 and their sub-issues.

No prohibited monitoring implementation was consulted. Chip identifiers below
are limited to those already pinned in the repository's specs.

## Current coverage matrix

Status vocabulary follows ADR 0011: **Verified**, **Experimental**,
**Unsupported**. "Fallback" marks an OS path that needs no PawnIO.

### CPU package temperature

| Hardware | Path | Status | Source |
| --- | --- | --- | --- |
| Intel CPUs advertising DTS + package thermal management (`CPUID.06H:EAX[0]` and `[6]`) | PawnIO `IntelMSR`, `0x1B1` / `0x1A2` | Verified, capability-gated (no model allowlist) | `cpu-intel-dts-msr.md` rev 2 |
| Intel CPUs without package thermal management, or where `0x1A2` reads zero | none | Unsupported → ACPI fallback | same spec, Detection |
| AMD Family 17h (Zen–Zen 2), 19h (Zen 3/4) | PawnIO `RyzenSMU`, SMN `0x00059800` | Verified | `cpu-amd-zen-smn.md` rev 4 |
| AMD Family 1Ah (Zen 5) | same path | Experimental (module recognizes it; THM facts not pinned) | same spec; ADR 0011; #1824 |
| AMD Family 15h / 16h and older | none | Unsupported (module rejects) | `cpu_temperature.rs` `amd_family_enablement` |
| AMD Threadripper / EPYC multi-die | same path | Verified path, die-selection semantics unverified | spec Open questions |
| Non-x86 (Windows on Arm) | none | Unsupported → ACPI fallback | `cpu_identity.rs` returns `Other` |
| Any CPU, PawnIO absent / not elevated / module missing | ACPI `MSAcpi_ThermalZoneTemperature` or thermal-zone perf counters | Fallback, best-effort; many desktop boards expose no zone | `thermal_zone.rs`; #1635 background |

Per-core readings and hybrid P/E-core handling are out of scope of every
current spec.

### CPU package power

| Hardware | Path | Status | Source |
| --- | --- | --- | --- |
| Intel Sandy Bridge and newer | PawnIO `IntelMSR`, `0x606` / `0x611`, probe-gated | Verified | `cpu-intel-rapl-msr.md` rev 3 |
| Intel Silvermont Atom (`06_37H`, `06_4AH`, `06_5AH`, `06_5DH`) | none | Unsupported (deviant units, excluded by signature) | same spec |
| Intel pre-Sandy Bridge, or probe fault | none | Unsupported | same spec |
| AMD 17h models 01h/08h, 19h models 21h/61h, 1Ah model 02h | PawnIO `AMDFamily17`, `0xC0010299` / `0xC001029B` | Verified | `cpu-amd-zen-rapl-msr.md` rev 3 |
| Other AMD 17h/19h/1Ah models | same path | Experimental (width-agnostic decode) | same spec |
| AMD 1Ah model 44h | same path | Experimental; socket-vs-CCX domain ambiguous | same spec, Open questions |
| AMD pre-17h, `CPUID_Fn80000007_EDX[14]` clear, non-x86 | none | Unsupported | `cpu_power.rs` |
| Any CPU, PawnIO absent | none | **No fallback exists**; `cpu_watts` stays `None` | `sensors.rs` `sample_power_draw` |

### Motherboard temperatures and fan RPM (Super I/O)

| Hardware | Path | Status | Source |
| --- | --- | --- | --- |
| Nuvoton raw chip ID `0xD802` / NCT6799D: bank 4 temperatures `0x90`–`0x95`, direct RPM `0xC0`–`0xCB` | PawnIO `LpcIO`, normal HM base | Verified | `superio-nuvoton-nct67xx.md` rev 5 |
| NCT6799D read-only HM base, count-based RPM, AUXFANIN4 / seventh fan, voltages, PWM | — | Disabled by spec | same spec, Scoped enablement |
| Nuvoton `0xD421` / NCT6796D | datasheet-pinned facts exist, no hardware dump | Disabled (spec says primary-source pinned, not hardware-validated) | same spec |
| Every other Nuvoton / Winbond ID | — | Unsupported | ADR 0011 (no chip profile) |
| ITE exact `0x8728` / IT8728F/EX: `TMPIN1`–`TMPIN3` | PawnIO `LpcIO`, Environment Controller base | Experimental (no accepted dump yet) | `superio-ite-it86xx-it87xx.md` rev 2; #2039 |
| IT8728F FAN1–5, voltages, `0x8721` response | — | Disabled / Unsupported | same spec |
| Every other ITE IT86xx/IT87xx ID (including boards with a second ITE chip on the `0x4E`/`0x4F` pair) | — | Unsupported; raw chip ID is still reported by the Phase 2 diagnostic | same spec; `superio-access.md` rev 3 |
| Fintek, SMSC, and other Super I/O vendors | — | Unsupported; the diagnostic only tries Nuvoton and ITE entry sequences | `superio-access.md` |
| Boards whose sensors sit behind an embedded controller instead of an LPC Super I/O (most laptops, some desktop boards) | — | Unsupported; no EC spec | — |

Motherboard fan RPM today therefore requires NCT6799D specifically. The fan
lane in Cooling Insight is empty on every other board.

### GPU

GPU readings come from vendor APIs and OS counters, not PawnIO, so no
clean-room gate applies. Coverage is per vendor:

| Vendor | Usage | Temperature | VRAM | Fan | Power | Clock | Source |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NVIDIA | NVAPI | NVAPI (GPU core) | NVAPI | Cooler level % only (`gpu_cooler_level`; no RPM, not archived as a fan) | None | NVAPI graphics clock in static info | `nvapi_provider.rs` |
| AMD | ADL OD5 / OD8 | ADL (core, hotspot, memory, VRM, liquid, SoC, PLX where OD7/OD8 exposes them) | None | None | None | Static info only | `adl_provider.rs` |
| Intel (Arc, iGPU) | PDH engine counters | None | None | None | None | Reported as 0 | `directx.rs`, `pdh_provider.rs` |
| Any other adapter | PDH | None | None | None | None | None | `gpu.rs` |

`PowerDraw.gpu_watts` is never populated on Windows.

### Other signals

| Signal | Windows status | Note |
| --- | --- | --- |
| Storage temperature | Available through native SMART / NVMe paths and `smartctl` fallback | Not a Cooling Insight input; no gap for #1666 |
| CPU clock frequency | Not collected on Windows | Listed by #1666 as a future input |
| Thermal throttling state | Not collected | Listed by #1666 as a future input; see gap T1 |
| Memory (DIMM) temperature | Not collected | Would need SMBus SPD sensor access |
| Chipset / PCH temperature | Not collected | Only appears when firmware exposes an ACPI zone |
| Ambient temperature | SwitchBot Meter over BLE only | #2062 adds Hub-family devices |

## Cross-cutting limiter: PawnIO setup friction

Every native path above shares the same prerequisites, and they, not chip
coverage, are the first reason most Windows users see no PawnIO data:

- PawnIO runtime installed by the user (never bundled).
- The module blob (`IntelMSR.bin`, `RyzenSMU.bin`, `AMDFamily17.bin`,
  `LpcIO.bin`) copied manually into the PawnIO directory; the core installer
  does not ship modules.
- `pawnio_open` requires elevation (`0x80070005` otherwise), so users need
  Elevated Startup Mode ([ADR 0007](../adr/0007-elevated-startup-mode.md)).

Widening chip support raises the ceiling; module bundling and installer
integration (the deferred follow-up scope in `pawnio-interface.md`) raise the
floor for the hardware that is already supported. Both matter for #1666, and
the second needs no hardware research, only third-party-notice and packaging
work.

## Gap inventory and feasibility

Feasibility is judged on three questions: does a read-only access path
already exist, does a primary source for the hardware facts exist, and what
evidence is needed to reach Experimental or Verified. Cost is the clean-room
cost (spec authoring, hardware dumps, implementation).

### T1. Thermal throttling state (Intel first)

- **Gap:** #1666 lists throttling frequency as a future extension; nothing
  collects it.
- **Access path:** already exists. `0x19C` and `0x1B1` are on the `IntelMSR`
  read allow-list and the package temperature path reads `0x1B1` every tick.
  The Intel SDM documents package thermal status, PROCHOT#, critical
  temperature, and power-limitation status and sticky log bits in the same
  register.
- **Work:** a `cpu-intel-dts-msr.md` revision pinning the status bits, then a
  small decode extension plus a new snapshot field and archive column. Log
  bits are sticky and clearing them is a write, so only the live status bits
  fit the read-only policy; the archive would count minutes with status set.
- **AMD:** no equivalent fact is pinned; PROCHOT / thermal-limit state via
  SMN or MSR would need new spec research first.
- **Feasibility:** high for Intel, unknown for AMD. No new hardware, no new
  module, no new elevation requirement.

### T2. AMD Family 1Ah (Zen 5) temperature and 1Ah/44h power graduation

- **Gap:** enabled experimentally; verification is spec-only work.
- **Work:** pin `THM_TCON_CUR_TMP` from a Family 1Ah PPR when AMD publishes
  it, or accept a maintainer-approved hardware dump. For power, the socket
  versus CCX question on model 44h needs the dump comparison already described
  in the spec's Open questions.
- **Feasibility:** high, spec-author role only; user value is confidence, not
  new readings.

### T3. Super I/O coverage beyond NCT6799D

This is the largest user-visible gap for the fan lane and for motherboard
temperatures. Three sub-tiers:

1. **NCT6796D (`0xD421`).** The vendor datasheet is already the pinned
   primary source S1 of the Nuvoton spec, including the chip ID and the
   family register semantics. What is missing is a hardware dump. Under ADR
   0011 a datasheet-backed profile is not a guess, so a spec revision can
   enable `0xD421` as Experimental using the datasheet facts and graduate on
   the first accepted dump. Cost: spec revision plus a chip-ID table entry.
2. **IT8728F fans (FAN1–3).** Register and formula facts exist; the divisor,
   split-counter consistency, and stopped/invalid encodings are unresolved.
   Needs one accepted IT8728F dump captured with the procedure in the ITE
   spec's "Manual feedback dump" section. Cost: dump plus spec revision plus
   fan decode in the existing ITE sampler.
3. **Other Nuvoton and ITE chips.** Each needs its own datasheet or an
   accepted dump before a chip-ID mapping can exist; a chip-ID-only mapping
   from resemblance is prohibited. The Phase 2 diagnostic already reports raw
   chip IDs, so the practical route is a user-submitted diagnostic program:
   an issue template that asks for the diagnostic JSON and the elevated HM
   dump, then one spec revision per chip. No telemetry, consistent with
   DP-01.

Voltages, PWM, and the read-only HM base remain out of scope; none of them
feed #1666.

### T4. GPU fan RPM, power, and Intel GPU temperature

- **Gap:** the fan lane archives motherboard fans only; the NVIDIA cooler
  level is a percentage and is not archived. GPU power is absent on Windows.
  Intel GPUs have no temperature at all.
- **Access path:** public vendor SDKs, no clean-room gate. Candidates are
  NVAPI tachometer and power readings (check whether the pinned `nvapi`
  crate exposes them before adding raw entry points), ADL Overdrive fan speed
  and PMLog power/fan sensors, and Intel oneAPI Level Zero sysman for
  temperature and fan on Arc.
- **Work:** provider extensions plus a decision on whether GPU fans join the
  motherboard fan rows (`MotherboardFanSpeed` is motherboard-scoped by name)
  or get their own archive series.
- **Feasibility:** medium. Runtime proof needs the physical GPUs. #1666
  names GPU temperature support as a future extension, so this is aligned
  but not on the critical path.

### T5. CPU package power without PawnIO

- **Gap:** the power lane has no fallback, unlike temperature.
- **Options:** none of the OS-level sources reports package power on
  desktops. Battery discharge rate (`Win32_Battery`) is system power on
  battery only. The `Processor Information` performance counters give
  frequency and utility, not watts.
- **Feasibility:** low. The realistic improvement is module bundling and
  installer integration so the PawnIO path is reachable.

### T6. Laptops and embedded-controller boards

- **Gap:** laptop fans and board temperatures live behind vendor EC firmware.
  PawnIO.Modules publishes EC-oriented modules (`LpcACPIEC`, `IsaBridgeEC`,
  `DellSMM`), so an access path could exist.
- **Blocker:** EC register maps are vendor- and model-specific and almost
  never documented in public primary sources. Community knowledge of them
  lives in copyleft drivers and decompiled tools, which the clean-room rules
  prohibit. Without a vendor document there is nothing a spec author can pin.
- **Feasibility:** low today. Keep the ACPI thermal-zone fallback, which is
  more often populated on laptops than on desktops.

### T7. Windows on Arm and pre-Zen / pre-Nehalem x86

- **Gap:** no native path and, on Arm, no MSR concept the current specs
  cover. An `ARMMSR` module exists upstream but no spec.
- **Feasibility:** low value for #1666 (small installed base among
  self-built desktops). Accept ACPI fallback.

### T8. DIMM temperature over SMBus

- **Gap:** memory temperature is not collected.
- **Access path:** `SmbusI801` / `SmbusPIIX4` modules exist upstream; the
  DDR4 SPD temperature sensor (JEDEC TSE2004) and DDR5 SPD hub thermal sensor
  (JEDEC SPD5118) are public standards, so a primary source exists.
- **Feasibility:** medium cost, but not a #1666 input. Defer.

## Suggested order relative to #1666 value

1. **Module bundling / installer integration** (cross-cutting): raises the
   share of users who get any PawnIO lane at all. Packaging and notices only.
2. **T3.1 NCT6796D Experimental enablement** and **T3.2 IT8728F fans**:
   cheapest widening of the fan lane using sources already pinned.
3. **T1 Intel throttling state**: adds the throttling signal #1666 asked for
   from a register already read every tick.
4. **T3.3 user-submitted diagnostic program**: turns the shipped chip-ID
   diagnostic into a steady stream of chip profiles without buying boards.
5. **T4 GPU fan / power / Intel temperature**: no clean-room gate, extends the
   fan lane and prepares the GPU temperature extension.
6. **T2 Zen 5 graduation**: spec-only confidence work; schedule when AMD
   publishes the PPR or a dump arrives.
7. T5–T8: defer; record the reasoning here so they are not re-investigated
   from scratch.

## What would change these conclusions

- A verified count of which Super I/O chip IDs users actually report (from
  the diagnostic program) would reorder T3.
- A public NCT6799D datasheet would let the spec drop the dump-only
  scoping and cover the disabled NCT6799D features.
- A change in PawnIO's device DACL or a HardwareVisualizer elevated helper
  would remove the elevation dependency and make coverage widening more
  valuable.
- Confirming the `nvapi` crate surface for tachometer and power readings
  decides whether T4 is a provider extension or a new FFI binding.
